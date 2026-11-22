use serde::{Deserialize, Serialize};
use std::path::PathBuf;

fn default_true() -> bool { true }
fn default_ssh_host() -> String { "carsons-mac-mini".to_string() }
fn default_ssh_user() -> String { "carson".to_string() }
fn default_ssh_local_port() -> u16 { 8500 }
fn default_ssh_remote_port() -> u16 { 8500 }

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Config {
    pub data_dir: PathBuf,
    pub hangar_url: String,
    pub hangar_client_id: String,
    pub hangar_api_key: Option<String>,
    pub stt_base_url: String,
    pub stt_model: String,
    pub chat_base_url: String,
    pub chat_model: String,
    pub filler_words: Vec<String>,
    pub screenshot_dir_name: String,
    #[serde(default = "default_true")]
    pub ssh_tunnel_enabled: bool,
    #[serde(default = "default_ssh_host")]
    pub ssh_tunnel_host: String,
    #[serde(default = "default_ssh_user")]
    pub ssh_tunnel_user: String,
    #[serde(default = "default_ssh_local_port")]
    pub ssh_tunnel_local_port: u16,
    #[serde(default = "default_ssh_remote_port")]
    pub ssh_tunnel_remote_port: u16,
}

impl Default for Config {
    fn default() -> Self {
        let data_dir = dirs::data_dir()
            .unwrap_or_else(|| PathBuf::from("."))
            .join("natus");
        Self {
            data_dir,
            hangar_url: "http://127.0.0.1:8500".to_string(),
            hangar_client_id: "default".to_string(),
            hangar_api_key: None,
            stt_base_url: "http://127.0.0.1:8080".to_string(),
            stt_model: "transcribe".to_string(),
            chat_base_url: "http://127.0.0.1:8090".to_string(),
            chat_model: "flagship".to_string(),
            filler_words: vec![
                "um".into(), "uh".into(), "like".into(), "you know".into(),
                "i mean".into(), "sort of".into(), "kind of".into(), "basically".into(),
            ],
            screenshot_dir_name: "screenshots".to_string(),
            ssh_tunnel_enabled: true,
            ssh_tunnel_host: "carsons-mac-mini".to_string(),
            ssh_tunnel_user: "carson".to_string(),
            ssh_tunnel_local_port: 8500,
            ssh_tunnel_remote_port: 8500,
        }
    }
}

impl Config {
    pub fn load() -> Self {
        let path = dirs::config_dir().unwrap_or_default().join("natus").join("config.json");
        if path.exists() {
            if let Ok(s) = std::fs::read_to_string(&path) {
                if let Ok(c) = serde_json::from_str(&s) {
                    return c;
                }
            }
        }
        Self::default()
    }
    pub fn save(&self) -> anyhow::Result<()> {
        let dir = dirs::config_dir().unwrap_or_default().join("natus");
        std::fs::create_dir_all(&dir)?;
        std::fs::write(dir.join("config.json"), serde_json::to_string_pretty(self)?)?;
        Ok(())
    }
    pub fn session_dir(&self, id: &str) -> PathBuf {
        self.data_dir.join("sessions").join(id)
    }
}
