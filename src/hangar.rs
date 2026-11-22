use reqwest::Client;
use serde_json::Value;

pub struct HangarClient {
    pub base_url: String,
    pub client_id: String,
    pub api_key: Option<String>,
    http: Client,
}

impl HangarClient {
    pub fn new(base_url: String, client_id: String, api_key: Option<String>) -> Self {
        Self { base_url, client_id, api_key, http: Client::new() }
    }

    fn auth(&self, req: reqwest::RequestBuilder) -> reqwest::RequestBuilder {
        let mut r = req.header("X-Hangar-Client", &self.client_id);
        if let Some(k) = &self.api_key {
            r = r.bearer_auth(k);
        }
        r
    }

    pub async fn health(&self) -> bool {
        self.auth(self.http.get(format!("{}/health", self.base_url)))
            .send().await.map(|r| r.status().is_success()).unwrap_or(false)
    }

    pub async fn chat(&self, model: &str, messages: Vec<Value>, max_tokens: u32) -> anyhow::Result<String> {
        let body = serde_json::json!({
            "model": model,
            "messages": messages,
            "max_tokens": max_tokens,
        });
        let resp = self.auth(self.http.post(format!("{}/v1/chat/completions", self.base_url)))
            .json(&body).send().await?;
        let v: Value = resp.json().await?;
        Ok(v["choices"][0]["message"]["content"].as_str().unwrap_or("").to_string())
    }

    pub async fn transcribe(&self, model: &str, audio_path: &std::path::Path) -> anyhow::Result<String> {
        let bytes = std::fs::read(audio_path)?;
        let part = reqwest::multipart::Part::bytes(bytes)
            .file_name(audio_path.file_name().unwrap().to_string_lossy().to_string())
            .mime_str("audio/wav")?;
        let form = reqwest::multipart::Form::new()
            .text("model", model.to_string())
            .part("file", part);
        let resp = self.auth(self.http.post(format!("{}/v1/audio/transcriptions", self.base_url)))
            .multipart(form).send().await?;
        let v: Value = resp.json().await?;
        Ok(v["text"].as_str().unwrap_or("").to_string())
    }
}
