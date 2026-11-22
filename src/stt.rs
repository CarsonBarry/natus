use crate::config::Config;
use crate::hangar::HangarClient;
use std::path::Path;

pub async fn transcribe(cfg: &Config, audio: &Path) -> anyhow::Result<String> {
    // Try hangar first
    let hc = HangarClient::new(cfg.hangar_url.clone(), cfg.hangar_client_id.clone(), cfg.hangar_api_key.clone());
    if hc.health().await {
        if let Ok(t) = hc.transcribe(&cfg.stt_model, audio).await {
            if !t.trim().is_empty() { return Ok(t); }
        }
    }
    // Direct OpenAI-compat endpoint
    let client = reqwest::Client::new();
    let bytes = std::fs::read(audio)?;
    let part = reqwest::multipart::Part::bytes(bytes)
        .file_name(audio.file_name().unwrap().to_string_lossy().to_string())
        .mime_str("audio/wav")?;
    let form = reqwest::multipart::Form::new()
        .text("model", cfg.stt_model.clone())
        .part("file", part);
    let resp = client.post(format!("{}/v1/audio/transcriptions", cfg.stt_base_url.trim_end_matches('/')))
        .multipart(form).send().await?;
    let v: serde_json::Value = resp.json().await?;
    Ok(v["text"].as_str().unwrap_or("").to_string())
}
