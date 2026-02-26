use anyhow::{anyhow, Context, Result};
use libloading::{Library, Symbol};
use serde::{Deserialize, Serialize};
use serde_json::Value;
use std::collections::HashMap;
use std::path::{Path, PathBuf};
use std::sync::Arc;

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum PluginType {
    Builtin,
    External,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct PluginInfo {
    pub id: String,
    pub name: String,
    pub version: String,
    pub description: String,
    pub plugin_type: PluginType,
    pub installed: bool,
    pub enabled: bool,
    pub last_error: Option<String>,
}

#[derive(Debug, Clone)]
pub struct PluginContext {
    pub app_data_dir: PathBuf,
    pub plugins_dir: PathBuf,
    pub settings_path: PathBuf,
}

impl PluginContext {
    pub fn new() -> Self {
        let app_data_dir = dirs::config_dir()
            .unwrap_or_else(|| PathBuf::from("."))
            .join("relay");
        let plugins_dir = app_data_dir.join("plugins");
        let settings_path = app_data_dir.join("settings.json");
        Self {
            app_data_dir,
            plugins_dir,
            settings_path,
        }
    }

    pub fn plugin_data_dir(&self, plugin_id: &str) -> PathBuf {
        self.app_data_dir.join("plugin-data").join(plugin_id)
    }
}

pub trait RelayPlugin: Send + Sync {
    fn id(&self) -> &str;
    fn name(&self) -> &str;
    fn version(&self) -> &str;
    fn description(&self) -> &str;
    fn plugin_type(&self) -> PluginType;

    fn on_install(&self, ctx: &PluginContext) -> Result<()>;
    fn on_uninstall(&self, ctx: &PluginContext) -> Result<()>;
    fn on_enable(&self, ctx: &PluginContext) -> Result<()>;
    fn on_disable(&self, ctx: &PluginContext) -> Result<()>;

    fn is_installed(&self, ctx: &PluginContext) -> bool;
    fn settings_schema(&self) -> Option<Value> {
        None
    }
}

#[derive(Debug, Deserialize)]
pub struct PluginManifest {
    pub plugin: PluginManifestMeta,
}

#[derive(Debug, Deserialize)]
pub struct PluginManifestMeta {
    pub id: String,
    pub name: String,
    pub version: String,
    pub description: String,
    pub entry: String,
    pub min_relay_version: Option<String>,
}

impl PluginManifest {
    pub fn from_path(path: &Path) -> Result<Self> {
        let content = std::fs::read_to_string(path)
            .with_context(|| format!("Failed to read manifest: {}", path.display()))?;
        let parsed: Self = toml::from_str(&content)
            .with_context(|| format!("Failed to parse TOML manifest: {}", path.display()))?;
        Ok(parsed)
    }
}

type PluginCreateSymbol = unsafe fn() -> *mut dyn RelayPlugin;

struct ExternalPluginWrapper {
    _library: Arc<Library>,
    plugin: Box<dyn RelayPlugin>,
}

impl RelayPlugin for ExternalPluginWrapper {
    fn id(&self) -> &str {
        self.plugin.id()
    }

    fn name(&self) -> &str {
        self.plugin.name()
    }

    fn version(&self) -> &str {
        self.plugin.version()
    }

    fn description(&self) -> &str {
        self.plugin.description()
    }

    fn plugin_type(&self) -> PluginType {
        PluginType::External
    }

    fn on_install(&self, ctx: &PluginContext) -> Result<()> {
        self.plugin.on_install(ctx)
    }

    fn on_uninstall(&self, ctx: &PluginContext) -> Result<()> {
        self.plugin.on_uninstall(ctx)
    }

    fn on_enable(&self, ctx: &PluginContext) -> Result<()> {
        self.plugin.on_enable(ctx)
    }

    fn on_disable(&self, ctx: &PluginContext) -> Result<()> {
        self.plugin.on_disable(ctx)
    }

    fn is_installed(&self, ctx: &PluginContext) -> bool {
        self.plugin.is_installed(ctx)
    }

    fn settings_schema(&self) -> Option<Value> {
        self.plugin.settings_schema()
    }
}

pub fn load_external_plugin(manifest_path: &Path) -> Result<Box<dyn RelayPlugin>> {
    
    if std::env::var("RELAY_ALLOW_EXTERNAL_PLUGINS").as_deref() != Ok("1") {
        return Err(anyhow!(
            "External plugins are disabled for security. \
             To enable, set the environment variable RELAY_ALLOW_EXTERNAL_PLUGINS=1. \
             WARNING: external plugins run arbitrary native code with full application privileges \
             and no sandboxing. Only load plugins from sources you fully trust."
        ));
    }

    let manifest = PluginManifest::from_path(manifest_path)?;
    let plugin_dir = manifest_path
        .parent()
        .ok_or_else(|| anyhow!("Manifest path has no parent: {}", manifest_path.display()))?;
    let library_path = plugin_dir.join(&manifest.plugin.entry);

    if !library_path.exists() {
        return Err(anyhow!(
            "Plugin entry library does not exist: {}",
            library_path.display()
        ));
    }

    let library = unsafe { Library::new(&library_path) }
        .with_context(|| format!("Failed to load dynamic library: {}", library_path.display()))?;
    let library = Arc::new(library);

    let plugin_box = unsafe {
        let constructor: Symbol<'_, PluginCreateSymbol> = library
            .get(b"relay_create_plugin")
            .with_context(|| {
                format!(
                    "Missing symbol `relay_create_plugin` in {}",
                    library_path.display()
                )
            })?;

        let raw = constructor();
        if raw.is_null() {
            return Err(anyhow!("Plugin constructor returned null pointer"));
        }
        Box::from_raw(raw)
    };

    if plugin_box.id() != manifest.plugin.id {
        return Err(anyhow!(
            "Plugin id mismatch: manifest='{}', runtime='{}'",
            manifest.plugin.id,
            plugin_box.id()
        ));
    }

    Ok(Box::new(ExternalPluginWrapper {
        _library: library,
        plugin: plugin_box,
    }))
}

pub struct PluginRegistry {
    plugins: HashMap<String, Box<dyn RelayPlugin>>,
}

impl PluginRegistry {
    pub fn new() -> Self {
        Self {
            plugins: HashMap::new(),
        }
    }

    pub fn register(&mut self, plugin: Box<dyn RelayPlugin>) -> Result<()> {
        let id = plugin.id().to_string();
        if self.plugins.contains_key(&id) {
            return Err(anyhow!("Plugin with id '{}' already registered", id));
        }
        self.plugins.insert(id, plugin);
        Ok(())
    }

    pub fn get(&self, id: &str) -> Option<&dyn RelayPlugin> {
        self.plugins.get(id).map(|p| p.as_ref())
    }

    pub fn iter(&self) -> impl Iterator<Item = &dyn RelayPlugin> {
        self.plugins.values().map(|p| p.as_ref())
    }
}
