use crate::plugin_sdk::{PluginContext, PluginType, RelayPlugin};
use anyhow::{anyhow, Result};
use serde_json::Value;

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
        "Applies Relay proxy as OS-wide system proxy on Windows (coming soon)."
    }

    fn plugin_type(&self) -> PluginType {
        PluginType::Builtin
    }

    fn on_install(&self, _ctx: &PluginContext) -> Result<()> {
        Ok(())
    }

    fn on_uninstall(&self, _ctx: &PluginContext) -> Result<()> {
        Ok(())
    }

    fn on_enable(&self, _ctx: &PluginContext) -> Result<()> {
        Err(anyhow!("System proxy enable flow is coming soon"))
    }

    fn on_disable(&self, _ctx: &PluginContext) -> Result<()> {
        Err(anyhow!("System proxy disable flow is coming soon"))
    }

    fn is_installed(&self, _ctx: &PluginContext) -> bool {
        false
    }

    fn settings_schema(&self) -> Option<Value> {
        Some(serde_json::json!({
            "type": "object",
            "title": "System Proxy",
            "description": "Windows registry integration is planned. Current implementation is a coming-soon stub.",
            "properties": {
                "status": { "type": "string", "const": "coming_soon" }
            }
        }))
    }
}
