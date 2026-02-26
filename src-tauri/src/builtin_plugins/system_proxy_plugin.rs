use crate::plugin_sdk::{PluginContext, PluginType, RelayPlugin};
use crate::system_proxy;
use anyhow::{anyhow, Context, Result};
use serde::Deserialize;
use serde_json::Value;
use std::path::PathBuf;

#[derive(Debug, Deserialize)]
struct SavedInstanceMinimal {
    bind_addr: String,
    port: u16,
}

fn instances_path(ctx: &PluginContext) -> PathBuf {
    ctx.app_data_dir.join("instances.json")
}

fn read_first_instance(ctx: &PluginContext) -> Result<(String, u16)> {
    let path = instances_path(ctx);
    if !path.exists() {
        return Err(anyhow!(
            "No proxy instances found. Create a proxy instance first, then enable System Proxy."
        ));
    }

    let content = std::fs::read_to_string(&path)
        .context("Failed to read proxy instances file")?;

    let instances: Vec<SavedInstanceMinimal> = serde_json::from_str(&content)
        .context("Failed to parse proxy instances file")?;

    instances
        .into_iter()
        .next()
        .map(|i| (i.bind_addr, i.port))
        .ok_or_else(|| anyhow!(
            "No proxy instances configured. Create a proxy instance first, then enable System Proxy."
        ))
}

pub struct SystemProxyPlugin;

impl SystemProxyPlugin {
    pub fn new() -> Self {
        Self
    }
}

impl RelayPlugin for SystemProxyPlugin {
    fn id(&self) -> &str {
        "builtin-system-proxy"
    }

    fn name(&self) -> &str {
        "System Proxy"
    }

    fn version(&self) -> &str {
        "1.0.0"
    }

    fn description(&self) -> &str {
        "Sets Relay as the OS-wide system proxy (Windows). Enable to route all system traffic through your active proxy instance."
    }

    fn plugin_type(&self) -> PluginType {
        PluginType::Builtin
    }

    fn on_install(&self, _ctx: &PluginContext) -> Result<()> {
        Ok(())
    }

    fn on_uninstall(&self, _ctx: &PluginContext) -> Result<()> {
        system_proxy::unset_system_proxy()
            .context("Failed to disable system proxy during uninstall")?;
        Ok(())
    }

    fn on_enable(&self, ctx: &PluginContext) -> Result<()> {
        let (addr, port) = read_first_instance(ctx)?;
        system_proxy::set_system_proxy(&addr, port)
            .with_context(|| format!("Failed to set system proxy to {}:{}", addr, port))?;
        tracing::info!("[plugin:system-proxy] System proxy set to {}:{}", addr, port);
        Ok(())
    }

    fn on_disable(&self, _ctx: &PluginContext) -> Result<()> {
        system_proxy::unset_system_proxy()
            .context("Failed to disable system proxy")?;
        tracing::info!("[plugin:system-proxy] System proxy disabled");
        Ok(())
    }

    fn is_installed(&self, _ctx: &PluginContext) -> bool {
        true
    }

    fn settings_schema(&self) -> Option<Value> {
        None
    }
}
