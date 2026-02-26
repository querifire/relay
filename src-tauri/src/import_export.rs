use crate::profiles::Profile;
use crate::proxy_instance::SavedInstance;
use crate::scheduler::Schedule;
use crate::settings::AppSettings;
use crate::split_tunnel::RoutingRule;
use anyhow::Result;
use serde::{Deserialize, Serialize};
use std::path::PathBuf;

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ExportBundle {
    pub version: u32,
    pub exported_at: u64,
    pub settings: AppSettings,
    pub instances: Vec<SavedInstance>,
    pub profiles: Vec<Profile>,
    pub split_tunnel_rules: Vec<RoutingRule>,
    pub schedules: Vec<Schedule>,
}

pub fn default_export_path() -> PathBuf {
    let base = dirs::download_dir().unwrap_or_else(|| PathBuf::from("."));
    let ts = std::time::SystemTime::now()
        .duration_since(std::time::UNIX_EPOCH)
        .unwrap_or_default()
        .as_secs();
    base.join(format!("relay-export-{}.json", ts))
}

pub async fn save_bundle(path: &str, bundle: &ExportBundle) -> Result<()> {
    let p = PathBuf::from(path);
    if let Some(parent) = p.parent() {
        tokio::fs::create_dir_all(parent).await?;
    }
    let json = serde_json::to_string_pretty(bundle)?;
    crate::atomic_write::atomic_write_async(&p, &json).await?;
    Ok(())
}

pub async fn load_bundle(path: &str) -> Result<ExportBundle> {
    let content = tokio::fs::read_to_string(path).await?;
    Ok(serde_json::from_str::<ExportBundle>(&content)?)
}
