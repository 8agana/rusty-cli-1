use anyhow::Result;
use async_trait::async_trait;
use serde::{Deserialize, Serialize};
use serde_json::{json, Value};
use std::any::Any;
use std::collections::HashMap;
use tokio::process::Command;
use tokio::io::AsyncWriteExt;

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct FunctionCall {
    pub name: String,
    pub arguments: String,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ToolCall {
    pub id: String,
    pub r#type: String,
    pub function: FunctionCall,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Function {
    pub name: String,
    pub description: String,
    pub parameters: Value,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Tool {
    pub r#type: String,
    pub function: Function,
}

#[async_trait]
pub trait ToolExecutor: Send + Sync {
    fn name(&self) -> &str;
    async fn execute(&self, args: &str) -> Result<String>;
    fn as_any(&self) -> &dyn Any;
}

// Example built-in tools

pub struct ShellTool;

#[async_trait]
impl ToolExecutor for ShellTool {
    fn name(&self) -> &str {
        "shell"
    }

    async fn execute(&self, args: &str) -> Result<String> {
        let params: Value = serde_json::from_str(args)?;
        let command = params["command"]
            .as_str()
            .ok_or_else(|| anyhow::anyhow!("Missing command parameter"))?;
        
        let output = Command::new("sh")
            .arg("-c")
            .arg(command)
            .output()
            .await?;
        
        let stdout = String::from_utf8_lossy(&output.stdout);
        let stderr = String::from_utf8_lossy(&output.stderr);
        
        Ok(format!("stdout:\n{}\nstderr:\n{}", stdout, stderr))
    }
    
    fn as_any(&self) -> &dyn Any {
        self
    }
}

pub struct CalculatorTool;

#[async_trait]
impl ToolExecutor for CalculatorTool {
    fn name(&self) -> &str {
        "calculator"
    }

    async fn execute(&self, args: &str) -> Result<String> {
        let params: Value = serde_json::from_str(args)?;
        let expression = params["expression"]
            .as_str()
            .ok_or_else(|| anyhow::anyhow!("Missing expression parameter"))?;
        
        // Simple calculator using bc
        let mut child = Command::new("bc")
            .arg("-l")
            .stdin(std::process::Stdio::piped())
            .stdout(std::process::Stdio::piped())
            .spawn()?;
        
        if let Some(stdin) = child.stdin.as_mut() {
            stdin.write_all(expression.as_bytes()).await?;
            stdin.write_all(b"\n").await?;
        }
        
        let result = child.wait_with_output().await?;
        let answer = String::from_utf8_lossy(&result.stdout).trim().to_string();
        
        Ok(format!("{} = {}", expression, answer))
    }
    
    fn as_any(&self) -> &dyn Any {
        self
    }
}

pub struct FileReadTool;

#[async_trait]
impl ToolExecutor for FileReadTool {
    fn name(&self) -> &str {
        "read_file"
    }

    async fn execute(&self, args: &str) -> Result<String> {
        let params: Value = serde_json::from_str(args)?;
        let path = params["path"]
            .as_str()
            .ok_or_else(|| anyhow::anyhow!("Missing path parameter"))?;
        
        let contents = tokio::fs::read_to_string(path).await?;
        Ok(contents)
    }
    
    fn as_any(&self) -> &dyn Any {
        self
    }
}

pub struct FileWriteTool;

#[async_trait]
impl ToolExecutor for FileWriteTool {
    fn name(&self) -> &str {
        "write_file"
    }

    async fn execute(&self, args: &str) -> Result<String> {
        let params: Value = serde_json::from_str(args)?;
        let path = params["path"]
            .as_str()
            .ok_or_else(|| anyhow::anyhow!("Missing path parameter"))?;
        let content = params["content"]
            .as_str()
            .ok_or_else(|| anyhow::anyhow!("Missing content parameter"))?;
        
        tokio::fs::write(path, content).await?;
        Ok(format!("File written to {}", path))
    }
    
    fn as_any(&self) -> &dyn Any {
        self
    }
}

pub struct ToolRegistry {
    tools: HashMap<String, Box<dyn ToolExecutor>>,
}

impl ToolRegistry {
    pub fn new() -> Self {
        let mut registry = Self {
            tools: HashMap::new(),
        };
        
        // Register default tools
        registry.register(Box::new(ShellTool));
        registry.register(Box::new(CalculatorTool));
        registry.register(Box::new(FileReadTool));
        registry.register(Box::new(FileWriteTool));
        
        registry
    }
    
    pub fn register(&mut self, tool: Box<dyn ToolExecutor>) {
        self.tools.insert(tool.name().to_string(), tool);
    }
    
    pub async fn execute(&self, name: &str, args: &str) -> Result<String> {
        self.tools
            .get(name)
            .ok_or_else(|| anyhow::anyhow!("Tool {} not found", name))?
            .execute(args)
            .await
    }
    
    pub fn get_tool_definitions(&self) -> Vec<Tool> {
        vec![
            Tool {
                r#type: "function".to_string(),
                function: Function {
                    name: "shell".to_string(),
                    description: "Execute a shell command".to_string(),
                    parameters: json!({
                        "type": "object",
                        "properties": {
                            "command": {
                                "type": "string",
                                "description": "The shell command to execute"
                            }
                        },
                        "required": ["command"]
                    }),
                },
            },
            Tool {
                r#type: "function".to_string(),
                function: Function {
                    name: "calculator".to_string(),
                    description: "Perform mathematical calculations".to_string(),
                    parameters: json!({
                        "type": "object",
                        "properties": {
                            "expression": {
                                "type": "string",
                                "description": "Mathematical expression to evaluate"
                            }
                        },
                        "required": ["expression"]
                    }),
                },
            },
            Tool {
                r#type: "function".to_string(),
                function: Function {
                    name: "read_file".to_string(),
                    description: "Read contents of a file".to_string(),
                    parameters: json!({
                        "type": "object",
                        "properties": {
                            "path": {
                                "type": "string",
                                "description": "Path to the file to read"
                            }
                        },
                        "required": ["path"]
                    }),
                },
            },
            Tool {
                r#type: "function".to_string(),
                function: Function {
                    name: "write_file".to_string(),
                    description: "Write content to a file".to_string(),
                    parameters: json!({
                        "type": "object",
                        "properties": {
                            "path": {
                                "type": "string",
                                "description": "Path to the file to write"
                            },
                            "content": {
                                "type": "string",
                                "description": "Content to write to the file"
                            }
                        },
                        "required": ["path", "content"]
                    }),
                },
            },
        ]
    }
}