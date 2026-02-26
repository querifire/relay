use crate::plugin_sdk::{PluginContext, PluginType, RelayPlugin};
use anyhow::{anyhow, Context, Result};
use flate2::read::GzDecoder;
use serde_json::Value;
use sha2::Digest;
use std::fs::File;
use std::io::Write;
use std::path::{Component, Path, PathBuf};
use tar::Archive;

const TOR_BUNDLE_URL_WINDOWS_X64: &str = "https://archive.torproject.org/tor-package-archive/torbrowser/14.5.7/tor-expert-bundle-windows-x86_64-14.5.7.tar.gz";
const TOR_SHA256SUMS_URL: &str = "https://archive.torproject.org/tor-package-archive/torbrowser/14.5.7/sha256sums-signed-build.txt";

pub struct TorDownloaderPlugin;

impl TorDownloaderPlugin {
    pub fn new() -> Self {
        Self
    }

    fn tor_install_dir(ctx: &PluginContext) -> PathBuf {
        ctx.app_data_dir.join("tor")
    }

    fn download_url() -> Result<String> {
        #[cfg(all(target_os = "windows", target_arch = "x86_64"))]
        {
            Ok(std::env::var("RELAY_TOR_BUNDLE_URL")
                .ok()
                .filter(|v| !v.trim().is_empty())
                .unwrap_or_else(|| TOR_BUNDLE_URL_WINDOWS_X64.to_string()))
        }
        #[cfg(not(all(target_os = "windows", target_arch = "x86_64")))]
        {
            Err(anyhow!(
                "Tor Downloader currently supports only Windows x86_64"
            ))
        }
    }

    fn download_bundle(url: &str, target_file: &Path) -> Result<()> {
        
        let bytes = Self::download_bytes_in_thread(url)?;
        let mut out = File::create(target_file).with_context(|| {
            format!(
                "Failed to create temporary Tor bundle file: {}",
                target_file.display()
            )
        })?;
        out.write_all(&bytes)
            .context("Failed to write downloaded Tor bundle to disk")?;
        out.flush()
            .context("Failed to flush downloaded Tor bundle to disk")?;
        Ok(())
    }

    fn download_bytes_in_thread(url: &str) -> Result<Vec<u8>> {
        let url = url.to_string();
        let (tx, rx) = std::sync::mpsc::channel::<Result<Vec<u8>>>();
        std::thread::spawn(move || {
            let result = (|| -> Result<Vec<u8>> {
                let bytes = reqwest::blocking::get(&url)
                    .with_context(|| format!("Failed to download Tor bundle: {}", url))?
                    .error_for_status()
                    .with_context(|| format!("Tor bundle URL returned error: {}", url))?
                    .bytes()
                    .context("Failed to read Tor bundle bytes")?;
                Ok(bytes.to_vec())
            })();
            let _ = tx.send(result);
        });
        rx.recv()
            .unwrap_or_else(|_| Err(anyhow::anyhow!("Tor download thread terminated unexpectedly")))
    }

    fn verify_bundle_sha256(bundle_path: &Path, bundle_filename: &str) -> Result<()> {
        tracing::info!("[plugin:tor] Downloading SHA256 checksum file…");
        let sums_bytes = Self::download_bytes_in_thread(TOR_SHA256SUMS_URL)
            .context("Failed to download Tor SHA256 checksum file")?;
        let sums_text = String::from_utf8(sums_bytes)
            .context("SHA256 checksum file is not valid UTF-8")?;

        let expected_hash = sums_text
            .lines()
            .find_map(|line| {
                let line = line.trim();
                let (hash, rest) = line.split_once(|c: char| c.is_ascii_whitespace())?;
                let fname = rest.trim().trim_start_matches('*');
                if fname == bundle_filename {
                    Some(hash.to_ascii_lowercase())
                } else {
                    None
                }
            })
            .ok_or_else(|| {
                anyhow!(
                    "SHA256 for '{}' not found in checksum file",
                    bundle_filename
                )
            })?;

        tracing::info!("[plugin:tor] Computing SHA256 of downloaded bundle…");
        let bundle_bytes = std::fs::read(bundle_path)
            .with_context(|| format!("Failed to read bundle for verification: {}", bundle_path.display()))?;
        let actual_hash = format!("{:x}", sha2::Sha256::digest(&bundle_bytes));

        if actual_hash != expected_hash {
            return Err(anyhow!(
                "Tor bundle SHA256 mismatch! expected={} actual={}",
                expected_hash,
                actual_hash
            ));
        }

        tracing::info!("[plugin:tor] SHA256 verified OK");
        Ok(())
    }

    fn extract_bundle(archive_path: &Path, destination: &Path) -> Result<()> {
        let tar_gz = File::open(archive_path).with_context(|| {
            format!(
                "Failed to open downloaded Tor archive: {}",
                archive_path.display()
            )
        })?;
        let tar = GzDecoder::new(tar_gz);
        let mut archive = Archive::new(tar);

        for entry in archive.entries().context("Failed to read tar entries")? {
            let mut entry = entry.context("Failed to read tar entry")?;
            let entry_path = entry.path().context("Failed to get tar entry path")?.into_owned();

            if entry_path.is_absolute() {
                return Err(anyhow!(
                    "Tor archive contains absolute path (possible path traversal): {}",
                    entry_path.display()
                ));
            }
            for component in entry_path.components() {
                if matches!(component, Component::ParentDir) {
                    return Err(anyhow!(
                        "Tor archive contains '..' component (path traversal attempt): {}",
                        entry_path.display()
                    ));
                }
            }

            let dest = destination.join(&entry_path);

            if let (Ok(canon_dest), Ok(canon_base)) =
                (dest.parent().map(|p| p.to_path_buf()).unwrap_or_else(|| dest.clone()).canonicalize().or_else(|_| Ok::<_, std::io::Error>(dest.clone())),
                 destination.canonicalize().or_else(|_| Ok::<_, std::io::Error>(destination.to_path_buf())))
            {
                if !canon_dest.starts_with(&canon_base) {
                    return Err(anyhow!(
                        "Tor archive entry escapes destination directory: {}",
                        entry_path.display()
                    ));
                }
            }

            if entry.header().entry_type().is_dir() {
                std::fs::create_dir_all(&dest).with_context(|| {
                    format!("Failed to create directory during extraction: {}", dest.display())
                })?;
            } else {
                if let Some(parent) = dest.parent() {
                    std::fs::create_dir_all(parent).with_context(|| {
                        format!("Failed to create parent directory: {}", parent.display())
                    })?;
                }
                entry.unpack(&dest).with_context(|| {
                    format!("Failed to extract file: {}", dest.display())
                })?;
            }
        }

        Ok(())
    }

    fn find_tor_binary(root: &Path) -> Result<PathBuf> {
        fn walk(dir: &Path) -> Result<Option<PathBuf>> {
            for entry in std::fs::read_dir(dir)
                .with_context(|| format!("Failed to read directory: {}", dir.display()))?
            {
                let entry = entry?;
                let path = entry.path();
                if path.is_dir() {
                    if let Some(found) = walk(&path)? {
                        return Ok(Some(found));
                    }
                    continue;
                }
                if let Some(name) = path.file_name().and_then(|n| n.to_str()) {
                    if name.eq_ignore_ascii_case("tor.exe") || name.eq_ignore_ascii_case("tor") {
                        return Ok(Some(path));
                    }
                }
            }
            Ok(None)
        }

        walk(root)?.ok_or_else(|| anyhow!("Tor binary was not found after extraction"))
    }

    fn update_settings_binary_path(ctx: &PluginContext, binary_path: Option<&Path>) -> Result<()> {
        let mut root = if ctx.settings_path.exists() {
            let content = std::fs::read_to_string(&ctx.settings_path).with_context(|| {
                format!(
                    "Failed to read settings file: {}",
                    ctx.settings_path.display()
                )
            })?;
            serde_json::from_str::<Value>(&content).unwrap_or_else(|_| Value::Object(Default::default()))
        } else {
            Value::Object(Default::default())
        };

        let binary_value = binary_path
            .map(|p| Value::String(p.to_string_lossy().to_string()))
            .unwrap_or(Value::Null);

        if !root.is_object() {
            root = Value::Object(Default::default());
        }
        if let Some(obj) = root.as_object_mut() {
            obj.insert("tor_binary_path".to_string(), binary_value);
            let mut tor_cfg = obj
                .get("tor_config")
                .cloned()
                .unwrap_or_else(|| Value::Object(Default::default()));
            if !tor_cfg.is_object() {
                tor_cfg = Value::Object(Default::default());
            }
            if let Some(tor_cfg_obj) = tor_cfg.as_object_mut() {
                tor_cfg_obj.insert(
                    "binary_path".to_string(),
                    obj.get("tor_binary_path").cloned().unwrap_or(Value::Null),
                );
            }
            obj.insert("tor_config".to_string(), tor_cfg);
        }

        if let Some(parent) = ctx.settings_path.parent() {
            std::fs::create_dir_all(parent).with_context(|| {
                format!(
                    "Failed to create settings parent directory: {}",
                    parent.display()
                )
            })?;
        }

        let json = serde_json::to_string_pretty(&root)?;
        let tmp_path = ctx.settings_path.with_extension("json.tmp");
        std::fs::write(&tmp_path, json)
            .with_context(|| format!("Failed to write temp settings file: {}", tmp_path.display()))?;
        std::fs::rename(&tmp_path, &ctx.settings_path).with_context(|| {
            format!(
                "Failed to atomically replace settings file: {}",
                ctx.settings_path.display()
            )
        })?;
        Ok(())
    }
}

impl RelayPlugin for TorDownloaderPlugin {
    fn id(&self) -> &str {
        "builtin-tor-downloader"
    }

    fn name(&self) -> &str {
        "Tor Downloader"
    }

    fn version(&self) -> &str {
        "1.0.0"
    }

    fn description(&self) -> &str {
        "Downloads and installs tor-expert-bundle for Relay."
    }

    fn plugin_type(&self) -> PluginType {
        PluginType::Builtin
    }

    fn on_install(&self, ctx: &PluginContext) -> Result<()> {
        let install_dir = Self::tor_install_dir(ctx);
        std::fs::create_dir_all(&install_dir).with_context(|| {
            format!("Failed to create Tor install directory: {}", install_dir.display())
        })?;

        let url = Self::download_url()?;
        let archive_path = install_dir.join("tor-expert-bundle.tar.gz");

        tracing::info!("[plugin:tor] Downloading Tor bundle: {}", url);
        Self::download_bundle(&url, &archive_path)?;

        let bundle_filename = archive_path
            .file_name()
            .and_then(|n| n.to_str())
            .unwrap_or("tor-expert-bundle.tar.gz");
        Self::verify_bundle_sha256(&archive_path, bundle_filename)?;

        tracing::info!("[plugin:tor] Extracting Tor bundle...");
        Self::extract_bundle(&archive_path, &install_dir)?;
        let _ = std::fs::remove_file(&archive_path);

        let tor_binary = Self::find_tor_binary(&install_dir)?;
        Self::update_settings_binary_path(ctx, Some(&tor_binary))?;
        tracing::info!(
            "[plugin:tor] Tor installed at {}",
            tor_binary.to_string_lossy()
        );
        Ok(())
    }

    fn on_uninstall(&self, ctx: &PluginContext) -> Result<()> {
        let install_dir = Self::tor_install_dir(ctx);
        if install_dir.exists() {
            std::fs::remove_dir_all(&install_dir).with_context(|| {
                format!("Failed to remove Tor directory: {}", install_dir.display())
            })?;
        }
        Self::update_settings_binary_path(ctx, None)?;
        Ok(())
    }

    fn on_enable(&self, _ctx: &PluginContext) -> Result<()> {
        Ok(())
    }

    fn on_disable(&self, _ctx: &PluginContext) -> Result<()> {
        Ok(())
    }

    fn is_installed(&self, ctx: &PluginContext) -> bool {
        let install_dir = Self::tor_install_dir(ctx);
        Self::find_tor_binary(&install_dir).is_ok()
    }
}
