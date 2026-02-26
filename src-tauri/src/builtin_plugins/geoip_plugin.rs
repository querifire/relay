use crate::plugin_sdk::{PluginContext, PluginType, RelayPlugin};
use anyhow::{Context, Result};
use serde_json::Value;
use std::io::Write;
use std::path::PathBuf;

const GEOIP_DB_URL: &str =
    "https://raw.githubusercontent.com/P3TERX/GeoLite.mmdb/download/GeoLite2-Country.mmdb";
const GEOIP_DB_FILE: &str = "GeoLite2-Country.mmdb";

pub struct GeoIpPlugin;

impl GeoIpPlugin {
    pub fn new() -> Self {
        Self
    }

    fn install_dir(ctx: &PluginContext) -> PathBuf {
        ctx.app_data_dir.join("geoip")
    }

    fn db_path(ctx: &PluginContext) -> PathBuf {
        Self::install_dir(ctx).join(GEOIP_DB_FILE)
    }

    fn download_url() -> String {
        std::env::var("RELAY_GEOIP_DB_URL")
            .ok()
            .filter(|v| !v.trim().is_empty())
            .unwrap_or_else(|| GEOIP_DB_URL.to_string())
    }

    fn download_bytes_in_thread(url: &str) -> Result<Vec<u8>> {
        let url = url.to_string();
        let (tx, rx) = std::sync::mpsc::channel::<Result<Vec<u8>>>();
        std::thread::spawn(move || {
            let result = (|| -> Result<Vec<u8>> {
                let bytes = reqwest::blocking::get(&url)
                    .with_context(|| format!("Failed to download GeoIP database: {}", url))?
                    .error_for_status()
                    .with_context(|| format!("GeoIP DB URL returned error: {}", url))?
                    .bytes()
                    .context("Failed to read downloaded GeoIP database bytes")?;
                Ok(bytes.to_vec())
            })();
            let _ = tx.send(result);
        });
        rx.recv()
            .unwrap_or_else(|_| Err(anyhow::anyhow!("GeoIP download thread terminated unexpectedly")))
    }
}

impl RelayPlugin for GeoIpPlugin {
    fn id(&self) -> &str {
        "builtin-geoip-database"
    }

    fn name(&self) -> &str {
        "GeoIP Database"
    }

    fn version(&self) -> &str {
        "1.0.0"
    }

    fn description(&self) -> &str {
        "Downloads GeoLite2 country database for future country-based filtering."
    }

    fn plugin_type(&self) -> PluginType {
        PluginType::Builtin
    }

    fn on_install(&self, ctx: &PluginContext) -> Result<()> {
        let install_dir = Self::install_dir(ctx);
        std::fs::create_dir_all(&install_dir).with_context(|| {
            format!(
                "Failed to create GeoIP install directory: {}",
                install_dir.display()
            )
        })?;

        let url = Self::download_url();
        let db_path = Self::db_path(ctx);
        tracing::info!("[plugin:geoip] Downloading GeoLite2 database from {}", url);

        let bytes = Self::download_bytes_in_thread(&url)?;

        let mut file = std::fs::File::create(&db_path).with_context(|| {
            format!(
                "Failed to create GeoIP database file on disk: {}",
                db_path.display()
            )
        })?;
        file.write_all(&bytes)
            .context("Failed to write GeoIP database to disk")?;
        file.flush()
            .context("Failed to flush GeoIP database to disk")?;

        tracing::info!("[plugin:geoip] Installed database at {}", db_path.display());
        Ok(())
    }

    fn on_uninstall(&self, ctx: &PluginContext) -> Result<()> {
        let install_dir = Self::install_dir(ctx);
        if install_dir.exists() {
            std::fs::remove_dir_all(&install_dir).with_context(|| {
                format!(
                    "Failed to remove GeoIP installation directory: {}",
                    install_dir.display()
                )
            })?;
        }
        Ok(())
    }

    fn on_enable(&self, _ctx: &PluginContext) -> Result<()> {
        Ok(())
    }

    fn on_disable(&self, _ctx: &PluginContext) -> Result<()> {
        Ok(())
    }

    fn is_installed(&self, ctx: &PluginContext) -> bool {
        Self::db_path(ctx).exists()
    }

    fn settings_schema(&self) -> Option<Value> {
        Some(serde_json::json!({
            "type": "object",
            "title": "GeoIP Database",
            "description": "lookup_country API is coming soon in phase 2.",
            "properties": {
                "status": { "type": "string", "const": "coming_soon" }
            }
        }))
    }
}
