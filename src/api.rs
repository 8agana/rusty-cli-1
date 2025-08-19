use crate::tools::{Tool, ToolCall};
use anyhow::Result;
use eventsource_stream::Eventsource;
use futures_util::StreamExt;
use reqwest::Client;
use serde::{Deserialize, Serialize};
use serde_json::{json, Value};
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
                Err(e) => {
                    eprintln!("Stream error: {e:?}");
                    break;
                }
            }
        }

        println!();
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
