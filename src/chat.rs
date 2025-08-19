use crate::api::{DeepSeekClient, Message};
use anyhow::Result;
use colored::*;
use std::io::{self, Write};

pub async fn interactive_mode(client: DeepSeekClient, system_prompt: Option<String>) -> Result<()> {
    println!("{}", "DeepSeek Interactive Chat".bold().cyan());
    println!("{}", "Type 'exit' or 'quit' to end the session".dimmed());
    println!("{}", "Type 'clear' to clear chat history".dimmed());
    println!(
        "{}",
        "Type 'system <prompt>' to set a new system prompt".dimmed()
    );
    println!();

    let mut messages = Vec::new();

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
            _ if input.starts_with("system ") => {
                let system_content = input.strip_prefix("system ").unwrap();
                messages.retain(|m| m.role != "system");
                messages.insert(
                    0,
                    Message {
                        role: "system".to_string(),
                        content: Some(system_content.to_string()),
                        tool_calls: None,
                        tool_call_id: None,
                    },
                );
                println!("{}", "System prompt updated".green());
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

        print!("{} ", "DeepSeek:".bold().blue());
        io::stdout().flush()?;

        let response = client
            .complete_with_history(messages.clone(), 0.7, true)
            .await?;

        messages.push(Message {
            role: "assistant".to_string(),
            content: Some(response),
            tool_calls: None,
            tool_call_id: None,
        });

        println!();
    }

    Ok(())
}
