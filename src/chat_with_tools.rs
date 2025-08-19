use crate::api::{DeepSeekClient, Message};
use crate::tools::ToolRegistry;
use anyhow::Result;
use colored::*;
use serde_json::json;
use std::io::{self, Write};

pub async fn interactive_mode_with_tools(
    client: DeepSeekClient,
    system_prompt: Option<String>,
) -> Result<()> {
    println!("{}", "DeepSeek Interactive Chat with Tools".bold().cyan());
    println!("{}", "Available tools: shell, calculator, read_file, write_file".green());
    println!("{}", "Type 'exit' or 'quit' to end the session".dimmed());
    println!("{}", "Type 'clear' to clear chat history".dimmed());
    println!();

    let mut messages = Vec::new();
    let registry = ToolRegistry::new();
    let tools = registry.get_tool_definitions();

    if let Some(sys) = system_prompt {
        messages.push(Message {
            role: "system".to_string(),
            content: Some(sys),
            tool_calls: None,
            tool_call_id: None,
        });
        println!("{}", "System prompt set".green());
    }

    loop {
        print!("{} ", "You:".bold().green());
        io::stdout().flush()?;

        let mut input = String::new();
        io::stdin().read_line(&mut input)?;
        let input = input.trim();

        if input.is_empty() {
            continue;
        }

        match input.to_lowercase().as_str() {
            "exit" | "quit" => {
                println!("{}", "Goodbye!".yellow());
                break;
            }
            "clear" => {
                messages.clear();
                println!("{}", "Chat history cleared".yellow());
                continue;
            }
            _ => {}
        }

        messages.push(Message {
            role: "user".to_string(),
            content: Some(input.to_string()),
            tool_calls: None,
            tool_call_id: None,
        });

        // Get response with tools
        let response = client
            .complete_with_tools(messages.clone(), tools.clone(), 0.7)
            .await?;

        if let Some(choice) = response.choices.first() {
            let assistant_msg = &choice.message;

            // Check if the model wants to use tools
            if let Some(tool_calls) = &assistant_msg.tool_calls {
                println!("{}", "DeepSeek (using tools):".bold().blue());
                
                // Add assistant's message with tool calls
                messages.push(assistant_msg.clone());

                for tool_call in tool_calls {
                    let func_name = &tool_call.function.name;
                    let func_args = &tool_call.function.arguments;

                    println!(
                        "  {} {} with args: {}",
                        "→ Calling".dimmed(),
                        func_name.yellow(),
                        func_args.dimmed()
                    );

                    // Execute the tool
                    let result = match registry.execute(func_name, func_args).await {
                        Ok(res) => res,
                        Err(e) => format!("Error: {}", e),
                    };

                    println!("  {} {}", "← Result:".dimmed(), result.green());

                    // Add tool response to messages
                    messages.push(Message {
                        role: "tool".to_string(),
                        content: Some(result),
                        tool_calls: None,
                        tool_call_id: Some(tool_call.id.clone()),
                    });
                }

                // Get final response after tool execution
                println!();
                print!("{} ", "DeepSeek:".bold().blue());
                io::stdout().flush()?;
                
                let final_response = client
                    .complete_with_history(messages.clone(), 0.7, true)
                    .await?;

                messages.push(Message {
                    role: "assistant".to_string(),
                    content: Some(final_response),
                    tool_calls: None,
                    tool_call_id: None,
                });
            } else if let Some(content) = &assistant_msg.content {
                // Normal response without tools
                print!("{} ", "DeepSeek:".bold().blue());
                io::stdout().flush()?;
                println!("{}", content);
                messages.push(assistant_msg.clone());
            }
        }

        println!();
    }

    Ok(())
}