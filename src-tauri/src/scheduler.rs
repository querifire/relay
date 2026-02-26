use anyhow::Result;
use serde::{Deserialize, Serialize};
use std::path::PathBuf;

#[derive(Debug, Clone, Serialize, Deserialize)]
pub enum ScheduleAction {
    StartInstance { instance_id: String },
    StopInstance { instance_id: String },
    ChangeIp { instance_id: String },
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub enum ScheduleRepeat {
    Once,
    Daily,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Schedule {
    pub id: String,
    pub name: String,
    
    pub time: String,
    pub action: ScheduleAction,
    pub repeat: ScheduleRepeat,
    pub enabled: bool,
    
    pub last_run_day: Option<u32>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct SaveScheduleRequest {
    pub id: Option<String>,
    pub name: String,
    pub time: String,
    pub action: ScheduleAction,
    pub repeat: ScheduleRepeat,
    pub enabled: bool,
}

fn schedules_path() -> PathBuf {
    let base = dirs::config_dir().unwrap_or_else(|| PathBuf::from("."));
    base.join("relay").join("schedules.json")
}

pub async fn list_schedules() -> Vec<Schedule> {
    let path = schedules_path();
    if !path.exists() {
        return Vec::new();
    }
    match tokio::fs::read_to_string(path).await {
        Ok(content) => serde_json::from_str::<Vec<Schedule>>(&content).unwrap_or_default(),
        Err(_) => Vec::new(),
    }
}

pub async fn save_all(schedules: &[Schedule]) -> Result<()> {
    let path = schedules_path();
    if let Some(parent) = path.parent() {
        tokio::fs::create_dir_all(parent).await?;
    }
    let json = serde_json::to_string_pretty(schedules)?;
    crate::atomic_write::atomic_write_async(&path, &json).await?;
    Ok(())
}

pub async fn upsert_schedule(req: SaveScheduleRequest) -> Result<Schedule> {
    let mut schedules = list_schedules().await;
    if let Some(id) = req.id.clone() {
        if let Some(schedule) = schedules.iter_mut().find(|s| s.id == id) {
            schedule.name = req.name;
            schedule.time = req.time;
            schedule.action = req.action;
            schedule.repeat = req.repeat;
            schedule.enabled = req.enabled;
            let out = schedule.clone();
            save_all(&schedules).await?;
            return Ok(out);
        }
    }
    let schedule = Schedule {
        id: uuid::Uuid::new_v4().to_string(),
        name: req.name,
        time: req.time,
        action: req.action,
        repeat: req.repeat,
        enabled: req.enabled,
        last_run_day: None,
    };
    schedules.push(schedule.clone());
    save_all(&schedules).await?;
    Ok(schedule)
}

pub async fn delete_schedule(id: &str) -> Result<()> {
    let mut schedules = list_schedules().await;
    schedules.retain(|s| s.id != id);
    save_all(&schedules).await
}

fn today_ordinal() -> u32 {
    use std::time::{SystemTime, UNIX_EPOCH};
    let days = SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .unwrap_or_default()
        .as_secs()
        / 86_400;
    (days % 366) as u32
}

fn now_hhmm() -> String {
    use chrono::Timelike;
    let now = chrono::Local::now();
    format!("{:02}:{:02}", now.hour(), now.minute())
}

pub async fn take_due_schedules() -> Result<Vec<Schedule>> {
    let mut schedules = list_schedules().await;
    let now = now_hhmm();
    let day = today_ordinal();
    let mut due = Vec::new();

    for s in schedules.iter_mut() {
        if !s.enabled || s.time != now {
            continue;
        }
        if s.last_run_day == Some(day) {
            continue;
        }
        s.last_run_day = Some(day);
        if matches!(s.repeat, ScheduleRepeat::Once) {
            s.enabled = false;
        }
        due.push(s.clone());
    }

    save_all(&schedules).await?;
    Ok(due)
}
