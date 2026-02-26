use crate::plugin_sdk::{PluginContext, PluginType, RelayPlugin};
use anyhow::{Context, Result, anyhow};
use serde_json::Value;
use std::io::Write;
use std::path::PathBuf;
use std::time::Duration;

const GEOIP_DB_FILE: &str = "GeoLite2-Country.mmdb";

const GEOIP_DOWNLOAD_URLS: &[&str] = &[
    "https://github.com/P3TERX/GeoLite.mmdb/releases/latest/download/GeoLite2-Country.mmdb",
    "https://github.com/Loyalsoldier/geoip/releases/latest/download/GeoLite2-Country.mmdb",
];

const MMDB_MAGIC: &[u8] = b"\xab\xcd\xefMaxMind.com";

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

    fn primary_url() -> String {
        std::env::var("RELAY_GEOIP_DB_URL")
            .ok()
            .filter(|v| !v.trim().is_empty())
            .unwrap_or_else(|| GEOIP_DOWNLOAD_URLS[0].to_string())
    }

    fn download_bytes(url: &str) -> Result<Vec<u8>> {
        let client = reqwest::blocking::Client::builder()
            .timeout(Duration::from_secs(120))
            .user_agent("relay-app/1.0")
            .build()
            .context("Failed to build HTTP client")?;

        let response = client
            .get(url)
            .send()
            .with_context(|| format!("Failed to connect to {}", url))?
            .error_for_status()
            .with_context(|| format!("Server returned error for {}", url))?;

        let bytes = response
            .bytes()
            .with_context(|| format!("Failed to read response body from {}", url))?;

        Ok(bytes.to_vec())
    }

    fn download_with_fallback() -> Result<Vec<u8>> {
        let primary = Self::primary_url();
        let mut all_urls: Vec<String> = vec![primary.clone()];

        if !std::env::var("RELAY_GEOIP_DB_URL").is_ok() {
            for &u in GEOIP_DOWNLOAD_URLS.iter().skip(1) {
                all_urls.push(u.to_string());
            }
        }

        let mut last_err: Option<anyhow::Error> = None;
        for url in &all_urls {
            let url_owned = url.clone();
            let url_log = url.clone();
            let (tx, rx) = std::sync::mpsc::channel::<Result<Vec<u8>>>();
            std::thread::spawn(move || {
                let _ = tx.send(Self::download_bytes(&url_owned));
            });

            match rx.recv().unwrap_or_else(|_| Err(anyhow!("Download thread terminated unexpectedly"))) {
                Ok(bytes) => {
                    if bytes.len() < 512 {
                        last_err = Some(anyhow!(
                            "Downloaded file from {} is too small ({} bytes) — may be an LFS pointer or error page",
                            url_log, bytes.len()
                        ));
                        continue;
                    }

                    let magic_pos = bytes
                        .windows(MMDB_MAGIC.len())
                        .rposition(|w| w == MMDB_MAGIC);
                    if magic_pos.is_none() {
                        last_err = Some(anyhow!(
                            "File from {} does not appear to be a valid MaxMind database (magic bytes not found)",
                            url_log
                        ));
                        continue;
                    }

                    return Ok(bytes);
                }
                Err(e) => {
                    tracing::warn!("[plugin:geoip] Download from {} failed: {:#}", url_log, e);
                    last_err = Some(e);
                }
            }
        }

        Err(last_err.unwrap_or_else(|| anyhow!("No download URLs available")))
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

        tracing::info!("[plugin:geoip] Downloading GeoLite2 database...");

        let bytes = Self::download_with_fallback()?;

        let db_path = Self::db_path(ctx);
        let tmp_path = db_path.with_extension("mmdb.tmp");

        let mut file = std::fs::File::create(&tmp_path).with_context(|| {
            format!(
                "Failed to create temporary file: {}",
                tmp_path.display()
            )
        })?;
        file.write_all(&bytes)
            .context("Failed to write GeoIP database to disk")?;
        file.flush()
            .context("Failed to flush GeoIP database to disk")?;
        drop(file);

        std::fs::rename(&tmp_path, &db_path).with_context(|| {
            format!(
                "Failed to move database into place: {}",
                db_path.display()
            )
        })?;

        tracing::info!("[plugin:geoip] Installed database at {} ({} bytes)", db_path.display(), bytes.len());
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
        None
    }
}
