use crate::settings::AppSettings;
use anyhow::{anyhow, Result};
use serde::{Deserialize, Serialize};
use std::path::PathBuf;

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Profile {
    pub id: String,
    pub name: String,
    pub description: Option<String>,
    pub settings: AppSettings,
    pub instances: Vec<String>,
    pub created_at: u64,
    pub updated_at: u64,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct SaveProfileRequest {
    pub id: Option<String>,
    pub name: String,
    pub description: Option<String>,
    pub settings: AppSettings,
    #[serde(default)]
    pub instances: Vec<String>,
}

fn unix_ts() -> u64 {
    std::time::SystemTime::now()
        .duration_since(std::time::UNIX_EPOCH)
        .unwrap_or_default()
        .as_secs()
}

fn profiles_path() -> PathBuf {
    let base = dirs::config_dir().unwrap_or_else(|| PathBuf::from("."));
    base.join("relay").join("profiles").join("profiles.json")
}

pub async fn list_profiles() -> Vec<Profile> {
    let path = profiles_path();
    if !path.exists() {
        return Vec::new();
    }
    match tokio::fs::read_to_string(path).await {
        Ok(content) => serde_json::from_str::<Vec<Profile>>(&content).unwrap_or_default(),
        Err(_) => Vec::new(),
    }
}

pub async fn save_profiles(profiles: &[Profile]) -> Result<()> {
    let path = profiles_path();
    if let Some(parent) = path.parent() {
        tokio::fs::create_dir_all(parent).await?;
    }
    let json = serde_json::to_string_pretty(profiles)?;
    crate::atomic_write::atomic_write_async(&path, &json).await?;
    Ok(())
}

pub async fn upsert_profile(req: SaveProfileRequest) -> Result<Profile> {
    let mut profiles = list_profiles().await;
    let now = unix_ts();
    if let Some(id) = req.id.clone() {
        let profile = profiles
            .iter_mut()
            .find(|p| p.id == id)
            .ok_or_else(|| anyhow!("Profile not found"))?;
        profile.name = req.name;
        profile.description = req.description;
        profile.settings = req.settings;
        profile.instances = req.instances;
        profile.updated_at = now;
        let out = profile.clone();
        save_profiles(&profiles).await?;
        return Ok(out);
    }

    let profile = Profile {
        id: uuid::Uuid::new_v4().to_string(),
        name: req.name,
        description: req.description,
        settings: req.settings,
        instances: req.instances,
        created_at: now,
        updated_at: now,
    };
    profiles.push(profile.clone());
    save_profiles(&profiles).await?;
    Ok(profile)
}

pub async fn delete_profile(id: &str) -> Result<()> {
    let mut profiles = list_profiles().await;
    let before = profiles.len();
    profiles.retain(|p| p.id != id);
    if profiles.len() == before {
        return Err(anyhow!("Profile not found"));
    }
    save_profiles(&profiles).await
}

pub async fn get_profile(id: &str) -> Option<Profile> {
    list_profiles().await.into_iter().find(|p| p.id == id)
}
