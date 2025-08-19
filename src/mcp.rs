use anyhow::Result;
use async_trait::async_trait;
use serde::{Deserialize, Serialize};
use serde_json::{json, Value};
use std::collections::HashMap;
use std::process::Stdio;
use tokio::io::{AsyncBufReadExt, AsyncWriteExt, BufReader};
use tokio::process::{Child, Command};
use tokio::sync::Mutex;
use std::sync::Arc;

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct MCPRequest {
    pub jsonrpc: String,
    pub method: String,
    pub params: Option<Value>,
    pub id: Option<Value>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct MCPResponse {
    pub jsonrpc: String,
    pub result: Option<Value>,
    pub error: Option<MCPError>,
    pub id: Option<Value>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct MCPError {
    pub code: i32,
    pub message: String,
    pub data: Option<Value>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct MCPTool {
    pub name: String,
    pub description: Option<String>,
    pub input_schema: Value,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ToolsListResult {
    pub tools: Vec<MCPTool>,
}

pub struct MCPClient {
    process: Arc<Mutex<Child>>,
    stdin: Arc<Mutex<tokio::process::ChildStdin>>,
    reader: Arc<Mutex<BufReader<tokio::process::ChildStdout>>>,
    request_id: Arc<Mutex<u64>>,
}

impl MCPClient {
    pub async fn new(command: &str, args: Vec<String>) -> Result<Self> {
        let mut child = Command::new(command)
            .args(args)
            .stdin(Stdio::piped())
            .stdout(Stdio::piped())
            .stderr(Stdio::null())
            .spawn()?;

        let stdin = child
            .stdin
            .take()
            .ok_or_else(|| anyhow::anyhow!("Failed to get stdin"))?;
        let stdout = child
            .stdout
            .take()
            .ok_or_else(|| anyhow::anyhow!("Failed to get stdout"))?;

        let reader = BufReader::new(stdout);

        let client = Self {
            process: Arc::new(Mutex::new(child)),
            stdin: Arc::new(Mutex::new(stdin)),
            reader: Arc::new(Mutex::new(reader)),
            request_id: Arc::new(Mutex::new(0)),
        };

        // Initialize the MCP server
        client.initialize().await?;

        Ok(client)
    }

    async fn send_request(&self, method: &str, params: Option<Value>) -> Result<Value> {
        let mut id = self.request_id.lock().await;
        *id += 1;
        let request_id = *id;

        let request = MCPRequest {
            jsonrpc: "2.0".to_string(),
            method: method.to_string(),
            params,
            id: Some(json!(request_id)),
        };

        let request_str = serde_json::to_string(&request)?;
        
        let mut stdin = self.stdin.lock().await;
        stdin.write_all(request_str.as_bytes()).await?;
        stdin.write_all(b"\n").await?;
        stdin.flush().await?;

        // Read response
        let mut reader = self.reader.lock().await;
        let mut line = String::new();
        reader.read_line(&mut line).await?;

        let response: MCPResponse = serde_json::from_str(&line)?;

        if let Some(error) = response.error {
            return Err(anyhow::anyhow!("MCP Error: {}", error.message));
        }

        response
            .result
            .ok_or_else(|| anyhow::anyhow!("No result in response"))
    }

    async fn initialize(&self) -> Result<()> {
        let params = json!({
            "protocolVersion": "1.0.0",
            "capabilities": {
                "tools": {}
            },
            "clientInfo": {
                "name": "deepseek-cli",
                "version": "0.1.0"
            }
        });

        self.send_request("initialize", Some(params)).await?;
        Ok(())
    }

    pub async fn list_tools(&self) -> Result<Vec<MCPTool>> {
        let result = self.send_request("tools/list", None).await?;
        let tools_result: ToolsListResult = serde_json::from_value(result)?;
        Ok(tools_result.tools)
    }

    pub async fn call_tool(&self, name: &str, arguments: Value) -> Result<Value> {
        let params = json!({
            "name": name,
            "arguments": arguments
        });

        self.send_request("tools/call", Some(params)).await
    }
}

pub struct MCPToolWrapper {
    client: Arc<MCPClient>,
    tool: MCPTool,
}

impl MCPToolWrapper {
    pub fn new(client: Arc<MCPClient>, tool: MCPTool) -> Self {
        Self { client, tool }
    }

    pub fn to_deepseek_tool(&self) -> crate::tools::Tool {
        crate::tools::Tool {
            r#type: "function".to_string(),
            function: crate::tools::Function {
                name: self.tool.name.clone(),
                description: self
                    .tool
                    .description
                    .clone()
                    .unwrap_or_else(|| "MCP tool".to_string()),
                parameters: self.tool.input_schema.clone(),
            },
        }
    }
}

#[async_trait]
impl crate::tools::ToolExecutor for MCPToolWrapper {
    fn name(&self) -> &str {
        &self.tool.name
    }

    async fn execute(&self, args: &str) -> Result<String> {
        let arguments: Value = serde_json::from_str(args)?;
        let result = self.client.call_tool(&self.tool.name, arguments).await?;
        Ok(serde_json::to_string_pretty(&result)?)
    }
}

pub struct MCPRegistry {
    clients: Vec<Arc<MCPClient>>,
    tools: HashMap<String, Box<dyn crate::tools::ToolExecutor>>,
}

impl MCPRegistry {
    pub async fn new() -> Result<Self> {
        Ok(Self {
            clients: Vec::new(),
            tools: HashMap::new(),
        })
    }

    pub async fn add_mcp_server(&mut self, command: &str, args: Vec<String>) -> Result<()> {
        let client = Arc::new(MCPClient::new(command, args).await?);
        let tools = client.list_tools().await?;

        for tool in tools {
            let wrapper = Box::new(MCPToolWrapper::new(client.clone(), tool));
            self.tools.insert(wrapper.name().to_string(), wrapper);
        }

        self.clients.push(client);
        Ok(())
    }

    pub fn get_tool_definitions(&self) -> Vec<crate::tools::Tool> {
        self.tools
            .values()
            .map(|tool| {
                // This is a bit hacky but works for now
                if let Some(wrapper) = tool.as_any().downcast_ref::<MCPToolWrapper>() {
                    wrapper.to_deepseek_tool()
                } else {
                    // Fallback for non-MCP tools
                    crate::tools::Tool {
                        r#type: "function".to_string(),
                        function: crate::tools::Function {
                            name: tool.name().to_string(),
                            description: format!("Tool: {}", tool.name()),
                            parameters: json!({}),
                        },
                    }
                }
            })
            .collect()
    }

    pub async fn execute(&self, name: &str, args: &str) -> Result<String> {
        self.tools
            .get(name)
            .ok_or_else(|| anyhow::anyhow!("Tool {} not found", name))?
            .execute(args)
            .await
    }
}