use crate::tools::{Tool, ToolCall};
use anyhow::Result;
use eventsource_stream::Eventsource;
use futures_util::StreamExt;
use reqwest::Client;
use serde::{Deserialize, Serialize};
use serde_json::json;
use std::io::{self, Write};

#[derive(Debug, Clone)]
pub struct DeepSeekClient {
    client: Client,
    api_key: String,
    model: String,
    base_url: String,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Message {
    pub role: String,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub content: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub tool_calls: Option<Vec<ToolCall>>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub tool_call_id: Option<String>,
}

#[derive(Debug, Deserialize)]
pub struct CompletionResponse {
    pub choices: Vec<Choice>,
}

#[allow(dead_code)]
#[derive(Debug, Deserialize)]
pub struct Choice {
    pub message: Message,
    pub finish_reason: Option<String>,
}

#[derive(Debug, Deserialize)]
pub struct StreamChoice {
    pub delta: Delta,
}

#[derive(Debug, Deserialize)]
pub struct Delta {
    pub content: Option<String>,
}

#[derive(Debug, Deserialize)]
pub struct StreamResponse {
    pub choices: Vec<StreamChoice>,
}

impl DeepSeekClient {
    pub fn new(api_key: String, model: String) -> Self {
        Self {
            client: Client::new(),
            api_key,
            model,
            base_url: "https://api.deepseek.com".to_string(),
        }
    }

    pub fn model_name(&self) -> &str {
        &self.model
    }

    #[allow(dead_code)]
    pub async fn complete(
        &self,
        message: String,
        system: Option<String>,
        temperature: f32,
        stream: bool,
    ) -> Result<String> {
        let mut messages = vec![];

        if let Some(sys) = system {
            messages.push(Message {
                role: "system".to_string(),
                content: Some(sys),
                tool_calls: None,
                tool_call_id: None,
            });
        }

        messages.push(Message {
            role: "user".to_string(),
            content: Some(message),
            tool_calls: None,
            tool_call_id: None,
        });

        if stream {
            self.stream_completion(messages, temperature).await
        } else {
            self.simple_completion(messages, temperature).await
        }
    }

    async fn simple_completion(&self, messages: Vec<Message>, temperature: f32) -> Result<String> {
        let response = self
            .client
            .post(format!("{}/v1/chat/completions", self.base_url))
            .header("Authorization", format!("Bearer {}", self.api_key))
            .header("Content-Type", "application/json")
            .json(&json!({
                "model": self.model,
                "messages": messages,
                "temperature": temperature,
                "stream": false,
            }))
            .send()
            .await?;

        if !response.status().is_success() {
            let error_text = response.text().await?;
            return Err(anyhow::anyhow!("API Error: {}", error_text));
        }

        let completion: CompletionResponse = response.json().await?;
        Ok(completion
            .choices
            .first()
            .and_then(|c| c.message.content.clone())
            .unwrap_or_default())
    }

    async fn stream_completion(&self, messages: Vec<Message>, temperature: f32) -> Result<String> {
        let response = self
            .client
            .post(format!("{}/v1/chat/completions", self.base_url))
            .header("Authorization", format!("Bearer {}", self.api_key))
            .header("Accept", "text/event-stream")
            .header("Content-Type", "application/json")
            .json(&json!({
                "model": self.model,
                "messages": messages,
                "temperature": temperature,
                "stream": true,
            }))
            .send()
            .await?;

        if !response.status().is_success() {
            let error_text = response.text().await?;
            return Err(anyhow::anyhow!("API Error: {}", error_text));
        }

        let mut stream = response.bytes_stream().eventsource();
        let mut full_response = String::new();

        let mut errored = false;
        while let Some(event) = stream.next().await {
            match event {
                Ok(event) => {
                    if event.data == "[DONE]" {
                        break;
                    }

                    if let Ok(chunk) = serde_json::from_str::<StreamResponse>(&event.data) {
                        if let Some(choice) = chunk.choices.first() {
                            if let Some(content) = &choice.delta.content {
                                print!("{content}");
                                io::stdout().flush()?;
                                full_response.push_str(content);
                            }
                        }
                    }
                }
                Err(_e) => {
                    // Network hiccup; break and fall back to non-stream if needed
                    errored = true;
                    break;
                }
            }
        }

        println!();
        if errored && full_response.is_empty() {
            // Best-effort fallback
            return self.simple_completion(vec![], temperature).await;
        }
        Ok(full_response)
    }

    pub async fn complete_with_history(
        &self,
        messages: Vec<Message>,
        temperature: f32,
        stream: bool,
    ) -> Result<String> {
        if stream {
            self.stream_completion(messages, temperature).await
        } else {
            self.simple_completion(messages, temperature).await
        }
    }

    pub async fn complete_with_tools(
        &self,
        messages: Vec<Message>,
        tools: Vec<Tool>,
        temperature: f32,
    ) -> Result<CompletionResponse> {
        let response = self
            .client
            .post(format!("{}/v1/chat/completions", self.base_url))
            .header("Authorization", format!("Bearer {}", self.api_key))
            .header("Content-Type", "application/json")
            .json(&json!({
                "model": self.model,
                "messages": messages,
                "temperature": temperature,
                "tools": tools,
                "tool_choice": "auto",
                "stream": false,
            }))
            .send()
            .await?;

        if !response.status().is_success() {
            let error_text = response.text().await?;
            return Err(anyhow::anyhow!("API Error: {}", error_text));
        }

        let completion: CompletionResponse = response.json().await?;
        Ok(completion)
    }
}

#[derive(Debug, Clone)]
pub struct OaiCompatClient {
    client: Client,
    api_key: String,
    model: String,
    base_url: String,
}

impl OaiCompatClient {
    pub fn new(api_key: String, model: String, base_url: String) -> Self {
        Self {
            client: Client::new(),
            api_key,
            model,
            base_url,
        }
    }
    pub fn model_name(&self) -> &str {
        &self.model
    }

    fn completions_url(&self) -> String {
        let base = self.base_url.trim_end_matches('/');
        if base.ends_with("/v1") {
            format!("{}/chat/completions", base)
        } else {
            format!("{}/v1/chat/completions", base)
        }
    }

    pub async fn simple_completion(
        &self,
        messages: Vec<Message>,
        temperature: f32,
    ) -> Result<String> {
        let response = self
            .client
            .post(self.completions_url())
            .header("Authorization", format!("Bearer {}", self.api_key))
            .header("Content-Type", "application/json")
            .json(&json!({
                "model": self.model,
                "messages": messages,
                "temperature": temperature,
                "stream": false,
            }))
            .send()
            .await?;
        if !response.status().is_success() {
            let error_text = response.text().await?;
            return Err(anyhow::anyhow!("API Error: {}", error_text));
        }
        #[derive(Deserialize)]
        struct Resp {
            choices: Vec<Choice>,
            #[serde(default)]
            usage: Option<UsageLike>,
        }
        #[derive(Deserialize)]
        struct UsageLike {
            prompt_tokens: Option<u32>,
            completion_tokens: Option<u32>,
            total_tokens: Option<u32>,
        }
        let completion: Resp = response.json().await?;
        if let Some(u) = completion.usage {
            if let (Some(pi), Some(co), Some(tt)) =
                (u.prompt_tokens, u.completion_tokens, u.total_tokens)
            {
                eprintln!("[usage] in={} out={} total={}", pi, co, tt);
            }
        }
        Ok(completion
            .choices
            .first()
            .and_then(|c| c.message.content.clone())
            .unwrap_or_default())
    }

    pub async fn stream_completion(
        &self,
        messages: Vec<Message>,
        temperature: f32,
    ) -> Result<String> {
        let response = self
            .client
            .post(self.completions_url())
            .header("Authorization", format!("Bearer {}", self.api_key))
            .header("Content-Type", "application/json")
            .json(&json!({
                "model": self.model,
                "messages": messages,
                "temperature": temperature,
                "stream": true,
            }))
            .send()
            .await?;
        if !response.status().is_success() {
            let error_text = response.text().await?;
            return Err(anyhow::anyhow!("API Error: {}", error_text));
        }
        let mut stream = response.bytes_stream().eventsource();
        let mut full = String::new();
        while let Some(ev) = stream.next().await {
            match ev {
                Ok(ev) => {
                    if ev.data == "[DONE]" {
                        break;
                    }
                    if let Ok(chunk) = serde_json::from_str::<StreamResponse>(&ev.data) {
                        if let Some(choice) = chunk.choices.first() {
                            if let Some(content) = &choice.delta.content {
                                print!("{}", content);
                                io::stdout().flush()?;
                                full.push_str(content);
                            }
                        }
                    }
                }
                Err(e) => {
                    eprintln!("Stream error: {e:?}");
                    break;
                }
            }
        }
        println!();
        Ok(full)
    }

    pub async fn complete_with_history(
        &self,
        messages: Vec<Message>,
        temperature: f32,
        stream: bool,
    ) -> Result<String> {
        if stream {
            self.stream_completion(messages, temperature).await
        } else {
            self.simple_completion(messages, temperature).await
        }
    }

    fn models_url(&self) -> String {
        let base = self.base_url.trim_end_matches('/');
        if base.ends_with("/v1") {
            format!("{}/models", base)
        } else {
            format!("{}/v1/models", base)
        }
    }

    pub async fn list_models_inner(&self) -> Result<Vec<String>> {
        #[derive(Deserialize)]
        struct Model {
            id: String,
        }
        #[derive(Deserialize)]
        struct Resp {
            data: Vec<Model>,
        }
        let resp = self
            .client
            .get(self.models_url())
            .header("Authorization", format!("Bearer {}", self.api_key))
            .send()
            .await?;
        if !resp.status().is_success() {
            let t = resp.text().await.unwrap_or_default();
            return Err(anyhow::anyhow!(t));
        }
        let r: Resp = resp.json().await?;
        Ok(r.data.into_iter().map(|m| m.id).collect())
    }
}

#[async_trait::async_trait]
pub trait ChatClient: Send + Sync + 'static {
    fn model_name(&self) -> &str;
    async fn complete_with_history(
        &self,
        messages: Vec<Message>,
        temperature: f32,
        stream: bool,
    ) -> Result<String>;
    async fn complete_with_tools(
        &self,
        messages: Vec<Message>,
        tools: Vec<Tool>,
        temperature: f32,
    ) -> Result<CompletionResponse>;
    async fn list_models(&self) -> Result<Vec<String>>;
    fn with_model(&self, model: &str) -> Box<dyn ChatClient>;
}

#[async_trait::async_trait]
impl ChatClient for DeepSeekClient {
    fn model_name(&self) -> &str {
        self.model_name()
    }
    async fn complete_with_history(
        &self,
        messages: Vec<Message>,
        temperature: f32,
        stream: bool,
    ) -> Result<String> {
        DeepSeekClient::complete_with_history(self, messages, temperature, stream).await
    }
    async fn complete_with_tools(
        &self,
        messages: Vec<Message>,
        tools: Vec<Tool>,
        temperature: f32,
    ) -> Result<CompletionResponse> {
        DeepSeekClient::complete_with_tools(self, messages, tools, temperature).await
    }
    async fn list_models(&self) -> Result<Vec<String>> {
        // DeepSeek is OpenAI-compatible for models list
        #[derive(Deserialize)]
        struct Model {
            id: String,
        }
        #[derive(Deserialize)]
        struct Resp {
            data: Vec<Model>,
        }
        let url = format!("{}/v1/models", self.base_url.trim_end_matches('/'));
        let resp = self
            .client
            .get(url)
            .header("Authorization", format!("Bearer {}", self.api_key.clone()))
            .send()
            .await?;
        if !resp.status().is_success() {
            let t = resp.text().await.unwrap_or_default();
            return Err(anyhow::anyhow!(t));
        }
        let r: Resp = resp.json().await?;
        Ok(r.data.into_iter().map(|m| m.id).collect())
    }
    fn with_model(&self, model: &str) -> Box<dyn ChatClient> {
        Box::new(DeepSeekClient {
            model: model.to_string(),
            ..self.clone()
        })
    }
}

#[async_trait::async_trait]
impl ChatClient for OaiCompatClient {
    fn model_name(&self) -> &str {
        self.model_name()
    }
    async fn complete_with_history(
        &self,
        messages: Vec<Message>,
        temperature: f32,
        stream: bool,
    ) -> Result<String> {
        OaiCompatClient::complete_with_history(self, messages, temperature, stream).await
    }
    async fn complete_with_tools(
        &self,
        messages: Vec<Message>,
        tools: Vec<Tool>,
        temperature: f32,
    ) -> Result<CompletionResponse> {
        // Reuse same OpenAI-compatible endpoint
        let response = self
            .client
            .post(self.completions_url())
            .header("Authorization", format!("Bearer {}", self.api_key))
            .header("Content-Type", "application/json")
            .json(&serde_json::json!({
                "model": self.model,
                "messages": messages,
                "temperature": temperature,
                "tools": tools,
                "tool_choice": "auto",
                "stream": false,
            }))
            .send()
            .await?;
        if !response.status().is_success() {
            let error_text = response.text().await?;
            return Err(anyhow::anyhow!("API Error: {}", error_text));
        }
        let completion: CompletionResponse = response.json().await?;
        Ok(completion)
    }
    async fn list_models(&self) -> Result<Vec<String>> {
        self.list_models_inner().await
    }
    fn with_model(&self, model: &str) -> Box<dyn ChatClient> {
        Box::new(OaiCompatClient {
            model: model.to_string(),
            ..self.clone()
        })
    }
}
