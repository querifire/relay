use anyhow::Result;
use serde::{Deserialize, Serialize};
use std::path::PathBuf;

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct RoutingRule {
    pub id: String,
    pub name: String,
    pub domains: Vec<String>,
    pub proxy_instance_id: Option<String>,
    pub enabled: bool,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct SaveRoutingRuleRequest {
    pub id: Option<String>,
    #[serde(default)]
    pub name: String,
    pub domains: Vec<String>,
    pub proxy_instance_id: Option<String>,
    pub enabled: bool,
}

fn rules_path() -> PathBuf {
    let base = dirs::config_dir().unwrap_or_else(|| PathBuf::from("."));
    base.join("relay").join("split_tunnel_rules.json")
}

pub async fn list_rules() -> Vec<RoutingRule> {
    let path = rules_path();
    if !path.exists() {
        return Vec::new();
    }
    match tokio::fs::read_to_string(path).await {
        Ok(content) => serde_json::from_str::<Vec<RoutingRule>>(&content).unwrap_or_default(),
        Err(_) => Vec::new(),
    }
}

pub async fn save_rules(rules: &[RoutingRule]) -> Result<()> {
    let path = rules_path();
    if let Some(parent) = path.parent() {
        tokio::fs::create_dir_all(parent).await?;
    }
    let json = serde_json::to_string_pretty(rules)?;
    crate::atomic_write::atomic_write_async(&path, &json).await?;
    Ok(())
}

pub async fn upsert_rule(req: SaveRoutingRuleRequest) -> Result<RoutingRule> {
    let mut rules = list_rules().await;
    if let Some(id) = req.id.clone() {
        if let Some(rule) = rules.iter_mut().find(|r| r.id == id) {
            rule.name = req.name;
            rule.domains = normalize_domains(req.domains);
            rule.proxy_instance_id = req.proxy_instance_id;
            rule.enabled = req.enabled;
            let out = rule.clone();
            save_rules(&rules).await?;
            return Ok(out);
        }
    }

    let new_rule = RoutingRule {
        id: uuid::Uuid::new_v4().to_string(),
        name: req.name,
        domains: normalize_domains(req.domains),
        proxy_instance_id: req.proxy_instance_id,
        enabled: req.enabled,
    };
    rules.push(new_rule.clone());
    save_rules(&rules).await?;
    Ok(new_rule)
}

pub async fn delete_rule(id: &str) -> Result<()> {
    let mut rules = list_rules().await;
    let before = rules.len();
    rules.retain(|r| r.id != id);
    if rules.len() != before {
        save_rules(&rules).await?;
    }
    Ok(())
}

fn normalize_domains(domains: Vec<String>) -> Vec<String> {
    domains
        .into_iter()
        .map(|d| d.trim().to_ascii_lowercase())
        .filter(|d| !d.is_empty())
        .collect()
}

pub async fn match_domain(hostname: &str) -> Option<String> {
    let host = hostname.trim().to_ascii_lowercase();
    if host.is_empty() {
        return None;
    }
    for rule in list_rules().await {
        if !rule.enabled {
            continue;
        }
        for pattern in &rule.domains {
            if domain_matches(&host, pattern) {
                return rule.proxy_instance_id.clone();
            }
        }
    }
    None
}

fn domain_matches(host: &str, pattern: &str) -> bool {
    if pattern.starts_with("*.") {
        let suffix = &pattern[2..];
        return host == suffix || host.ends_with(&format!(".{}", suffix));
    }
    host == pattern || host.ends_with(&format!(".{}", pattern))
}
