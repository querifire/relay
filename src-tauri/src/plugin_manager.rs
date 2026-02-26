use crate::builtin_plugins::tor_downloader::TorDownloaderPlugin;
use crate::builtin_plugins::geoip_plugin::GeoIpPlugin;
use crate::builtin_plugins::system_proxy_plugin::SystemProxyPlugin;
use crate::plugin_sdk::{load_external_plugin, PluginContext, PluginInfo, PluginRegistry, RelayPlugin};
use anyhow::{anyhow, Context, Result};
use serde::{Deserialize, Serialize};
use std::collections::HashMap;
use std::path::PathBuf;

#[derive(Debug, Clone, Default, Serialize, Deserialize)]
pub struct PluginRuntimeState {
    pub installed: bool,
    pub enabled: bool,
    pub last_error: Option<String>,
}

pub struct PluginManager {
    registry: PluginRegistry,
    states: HashMap<String, PluginRuntimeState>,
    ctx: PluginContext,
}

impl PluginManager {
    pub fn new() -> Result<Self> {
        let ctx = PluginContext::new();
        std::fs::create_dir_all(&ctx.app_data_dir).with_context(|| {
            format!(
                "Failed to create relay app data dir: {}",
                ctx.app_data_dir.display()
            )
        })?;
        std::fs::create_dir_all(&ctx.plugins_dir).with_context(|| {
            format!("Failed to create plugins dir: {}", ctx.plugins_dir.display())
        })?;

        let mut manager = Self {
            registry: PluginRegistry::new(),
            states: Self::load_states(&ctx).unwrap_or_default(),
            ctx,
        };

        manager.register_builtin_plugins()?;
        manager.scan_plugins_dir()?;
        Ok(manager)
    }

    pub fn new_empty() -> Self {
        let ctx = PluginContext::new();
        Self {
            registry: PluginRegistry::new(),
            states: HashMap::new(),
            ctx,
        }
    }

    fn state_file_path(ctx: &PluginContext) -> PathBuf {
        ctx.app_data_dir.join("plugins_state.json")
    }

    fn load_states(ctx: &PluginContext) -> Result<HashMap<String, PluginRuntimeState>> {
        let path = Self::state_file_path(ctx);
        if !path.exists() {
            return Ok(HashMap::new());
        }
        let content = std::fs::read_to_string(&path)
            .with_context(|| format!("Failed to read plugin state file: {}", path.display()))?;
        let parsed = serde_json::from_str(&content)
            .with_context(|| format!("Failed to parse plugin state file: {}", path.display()))?;
        Ok(parsed)
    }

    fn save_states(&self) -> Result<()> {
        let path = Self::state_file_path(&self.ctx);
        let json = serde_json::to_string_pretty(&self.states)?;
        let parent = path.parent().unwrap_or_else(|| std::path::Path::new("."));
        let file_name = path
            .file_name()
            .and_then(|name| name.to_str())
            .unwrap_or("plugins_state.json");
        let tmp = parent.join(format!("{}.tmp", file_name));
        std::fs::write(&tmp, json).with_context(|| {
            format!(
                "Failed to write temporary plugin state file: {}",
                tmp.display()
            )
        })?;
        std::fs::rename(&tmp, &path)
            .with_context(|| format!("Failed to replace plugin state file: {}", path.display()))?;
        Ok(())
    }

    fn register_builtin_plugins(&mut self) -> Result<()> {
        self.registry.register(Box::new(TorDownloaderPlugin::new()))?;
        self.registry.register(Box::new(GeoIpPlugin::new()))?;
        self.registry.register(Box::new(SystemProxyPlugin::new()))?;
        Ok(())
    }

    pub fn scan_plugins_dir(&mut self) -> Result<()> {
        for entry in std::fs::read_dir(&self.ctx.plugins_dir).with_context(|| {
            format!(
                "Failed to read plugins directory: {}",
                self.ctx.plugins_dir.display()
            )
        })? {
            let entry = entry?;
            let path = entry.path();
            if !path.is_dir() {
                continue;
            }

            let manifest_path = path.join("plugin.toml");
            if !manifest_path.exists() {
                continue;
            }

            match load_external_plugin(&manifest_path) {
                Ok(plugin) => {
                    if let Err(e) = self.registry.register(plugin) {
                        tracing::warn!("[plugin] Failed to register plugin from {}: {}", manifest_path.display(), e);
                    }
                }
                Err(e) => {
                    tracing::warn!("[plugin] Failed to load plugin from {}: {}", manifest_path.display(), e);
                }
            }
        }
        Ok(())
    }

    fn plugin_by_id(&self, id: &str) -> Result<&dyn RelayPlugin> {
        self.registry
            .get(id)
            .ok_or_else(|| anyhow!("Plugin '{}' not found", id))
    }

    fn update_state<F>(&mut self, id: &str, f: F) -> Result<()>
    where
        F: FnOnce(&mut PluginRuntimeState),
    {
        let state = self.states.entry(id.to_string()).or_default();
        f(state);
        self.save_states()
    }

    pub fn get_plugins(&self) -> Vec<PluginInfo> {
        let mut list = Vec::new();
        for plugin in self.registry.iter() {
            let state = self.states.get(plugin.id()).cloned().unwrap_or_default();
            list.push(PluginInfo {
                id: plugin.id().to_string(),
                name: plugin.name().to_string(),
                version: plugin.version().to_string(),
                description: plugin.description().to_string(),
                plugin_type: plugin.plugin_type(),
                installed: state.installed || plugin.is_installed(&self.ctx),
                enabled: state.enabled,
                last_error: state.last_error,
            });
        }
        list.sort_by(|a, b| a.id.cmp(&b.id));
        list
    }

    pub fn install_plugin(&mut self, id: &str) -> Result<()> {
        let install_result = {
            let plugin = self.plugin_by_id(id)?;
            plugin.on_install(&self.ctx)
        };
        match install_result {
            Ok(()) => self.update_state(id, |s| {
                s.installed = true;
                s.last_error = None;
            }),
            Err(e) => {
                let msg = e.to_string();
                self.update_state(id, |s| s.last_error = Some(msg.clone()))?;
                Err(anyhow!(msg))
            }
        }
    }

    pub fn uninstall_plugin(&mut self, id: &str) -> Result<()> {
        let uninstall_result = {
            let plugin = self.plugin_by_id(id)?;
            plugin.on_uninstall(&self.ctx)
        };
        match uninstall_result {
            Ok(()) => self.update_state(id, |s| {
                s.installed = false;
                s.enabled = false;
                s.last_error = None;
            }),
            Err(e) => {
                let msg = e.to_string();
                self.update_state(id, |s| s.last_error = Some(msg.clone()))?;
                Err(anyhow!(msg))
            }
        }
    }

    pub fn enable_plugin(&mut self, id: &str) -> Result<()> {
        let enable_result = {
            let plugin = self.plugin_by_id(id)?;
            plugin.on_enable(&self.ctx)
        };
        match enable_result {
            Ok(()) => self.update_state(id, |s| {
                s.enabled = true;
                s.last_error = None;
            }),
            Err(e) => {
                let msg = e.to_string();
                self.update_state(id, |s| s.last_error = Some(msg.clone()))?;
                Err(anyhow!(msg))
            }
        }
    }

    pub fn disable_plugin(&mut self, id: &str) -> Result<()> {
        let disable_result = {
            let plugin = self.plugin_by_id(id)?;
            plugin.on_disable(&self.ctx)
        };
        match disable_result {
            Ok(()) => self.update_state(id, |s| {
                s.enabled = false;
                s.last_error = None;
            }),
            Err(e) => {
                let msg = e.to_string();
                self.update_state(id, |s| s.last_error = Some(msg.clone()))?;
                Err(anyhow!(msg))
            }
        }
    }

    pub fn get_plugin_settings_schema(&self, id: &str) -> Result<Option<serde_json::Value>> {
        let plugin = self.plugin_by_id(id)?;
        Ok(plugin.settings_schema())
    }

    pub fn context(&self) -> &PluginContext {
        &self.ctx
    }
}
