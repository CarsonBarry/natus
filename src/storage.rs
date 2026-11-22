use crate::config::Config;
use chrono::Utc;
use serde::{Deserialize, Serialize};
use std::path::PathBuf;

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct SessionMeta {
    pub id: String,
    pub started_at_ms: i64,
    pub title: String,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct CaptureEntry {
    pub timestamp_ms: i64,
    pub screenshot_path: Option<PathBuf>,
    pub text: String,
    pub cleaned_text: Option<String>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Session {
    pub meta: SessionMeta,
    pub entries: Vec<CaptureEntry>,
}

impl Session {
    pub fn new(title: &str) -> Self {
        Self {
            meta: SessionMeta {
                id: uuid::Uuid::new_v4().to_string(),
                started_at_ms: Utc::now().timestamp_millis(),
                title: title.to_string(),
            },
            entries: Vec::new(),
        }
    }
    pub fn save(&self, cfg: &Config) -> anyhow::Result<PathBuf> {
        let dir = cfg.session_dir(&self.meta.id);
        std::fs::create_dir_all(&dir)?;
        let p = dir.join("session.json");
        std::fs::write(&p, serde_json::to_string_pretty(self)?)?;
        Ok(p)
    }
    pub fn load(cfg: &Config, id: &str) -> anyhow::Result<Self> {
        let p = cfg.session_dir(id).join("session.json");
        let s = std::fs::read_to_string(&p)?;
        Ok(serde_json::from_str(&s)?)
    }
    pub fn list(cfg: &Config) -> Vec<String> {
        let root = cfg.data_dir.join("sessions");
        if !root.exists() { return vec![]; }
        std::fs::read_dir(root).ok().map(|rd| {
            rd.filter_map(|e| e.ok())
                .filter(|e| e.path().join("session.json").exists())
                .map(|e| e.file_name().to_string_lossy().to_string())
                .collect()
        }).unwrap_or_default()
    }
}
