use crate::config::Config;
use crate::hangar::HangarClient;
use serde_json::json;

pub async fn summarize(cfg: &Config, transcript: &str) -> anyhow::Result<String> {
    let prompt = format!(
        "Clean up this dictation transcript: remove filler words, fix punctuation, \
         and produce a concise bullet list of feedback points. Transcript:\n\n{}",
        transcript
    );
    let messages = vec![
        json!({"role":"system","content":"You are a dictation cleanup and summarization assistant."}),
        json!({"role":"user","content":prompt}),
    ];
    let hc = HangarClient::new(
        cfg.hangar_url.clone(),
        cfg.hangar_client_id.clone(),
        cfg.hangar_api_key.clone(),
    );
    if hc.health().await {
        if let Ok(t) = hc.chat(&cfg.chat_model, messages.clone(), 1024).await {
            if !t.trim().is_empty() {
                return Ok(t);
            }
        }
    }
    let client = reqwest::Client::new();
    let body = json!({"model": cfg.chat_model, "messages": messages, "max_tokens": 1024});
    let resp = client
        .post(format!(
            "{}/v1/chat/completions",
            cfg.chat_base_url.trim_end_matches('/')
        ))
        .json(&body)
        .send()
        .await?;
    let v: serde_json::Value = resp.json().await?;
    Ok(v["choices"][0]["message"]["content"]
        .as_str()
        .unwrap_or("")
        .to_string())
}
