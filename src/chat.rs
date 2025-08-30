use crate::api::{ChatClient, Message};
use crate::session::SessionStore;
use crate::tools::ToolRegistry;
use anyhow::Result;
use colored::*;
use std::io::{self, Write};

pub async fn interactive_mode(
    client: &dyn ChatClient,
    system_prompt: Option<String>,
) -> Result<()> {
    println!("{}", "Rusty Interactive Chat".bold().cyan());
    println!("{}", "Type 'exit' or 'quit' to end the session".dimmed());
    println!("{}", "Type 'clear' to clear chat history".dimmed());
    println!("{}", "Type ':new [id]' to start a new session".dimmed());
    println!("{}", "Type ':session <id>' to switch sessions".dimmed());
    println!("{}", "Type ':status' to show current session info".dimmed());
    println!(
        "{}",
        "Type ':models' for model tips; switch provider with --provider at launch".dimmed()
    );
    println!(
        "{}",
        "Type ':tools list' to view tools; ':tools on' to enter tools mode".dimmed()
    );
    println!("{}", "Type ':keys' to set API keys for providers".dimmed());
    println!(
        "{}",
        "Type 'system <prompt>' to set a new system prompt".dimmed()
    );
    println!();

    // Determine session: resume last or start a new one
    let mut session_id = SessionStore::last()?
        .unwrap_or_else(|| format!("s-{}", time::OffsetDateTime::now_utc().unix_timestamp()));
    let mut messages = SessionStore::load(&session_id).unwrap_or_default();
    if !messages.is_empty() {
        println!("{} {}", "Resumed session".yellow(), session_id.dimmed());
    }

    let mut current_system = system_prompt.clone();
    let mut current_model = client.model_name().to_string();
    let mut stream = true;
    let mut cached_models: Vec<String> = Vec::new();
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
            _ if input.starts_with(":new") => {
                session_id = if let Some(rest) = input.split_whitespace().nth(1) {
                    rest.to_string()
                } else {
                    format!("s-{}", time::OffsetDateTime::now_utc().unix_timestamp())
                };
                messages.clear();
                println!("{} {}", "Started new session".green(), session_id.dimmed());
                continue;
            }
            _ if input.starts_with(":session ") => {
                let id = input.split_whitespace().nth(1).unwrap_or("");
                if id.is_empty() {
                    println!("usage: :session <id>");
                } else {
                    session_id = id.to_string();
                    messages = SessionStore::load(&session_id).unwrap_or_default();
                    println!(
                        "{} {} ({} messages)",
                        "Loaded session".green(),
                        session_id.dimmed(),
                        messages.len()
                    );
                }
                continue;
            }
            _ if input == ":status" => {
                println!(
                    "session={} messages={} model={} stream={}",
                    session_id,
                    messages.len(),
                    current_model,
                    stream
                );
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
                current_system = Some(system_content.to_string());
                println!("{}", "System prompt updated".green());
                continue;
            }
            _ if input == ":tools list" => {
                let reg = ToolRegistry::new();
                for t in reg.get_tool_definitions() {
                    println!("- {}: {}", t.function.name, t.function.description);
                }
                continue;
            }
            _ if input.starts_with(":model ") => {
                let arg = input.split_whitespace().nth(1).unwrap_or("");
                if arg.is_empty() {
                    println!("usage: :model <name|index>");
                    continue;
                }
                if let Ok(idx) = arg.parse::<usize>() {
                    if idx == 0 || idx > cached_models.len() {
                        println!("invalid index");
                        continue;
                    }
                    current_model = cached_models[idx - 1].clone();
                } else {
                    current_model = arg.to_string();
                }
                println!("model set to {}", current_model);
                continue;
            }
            _ if input == ":models" => {
                match client.list_models().await {
                    Ok(mut list) => {
                        list.sort();
                        cached_models = list.clone();
                        for (i, m) in list.iter().enumerate().take(50) {
                            println!("{:>2}. {}", i + 1, m);
                        }
                        if list.len() > 50 {
                            println!("... {} more", list.len() - 50);
                        }
                        println!("use :model <number> to select");
                    }
                    Err(e) => eprintln!("models error: {}", e),
                }
                continue;
            }
            _ if input == ":tools on" => {
                println!("Switching to tools mode...");
                let _ = crate::chat_with_tools::interactive_mode_with_tools(
                    client,
                    current_system.clone(),
                )
                .await;
                println!("(exited tools mode)\n");
                continue;
            }
            _ if input == ":keys" => {
                use std::io::{self, Write};
                let mut cfg = crate::config::Config::load().unwrap_or_default();
                println!("Set keys (leave blank to skip):");
                print!("OPENAI_API_KEY: ");
                io::stdout().flush()?;
                let mut s = String::new();
                io::stdin().read_line(&mut s)?;
                let t = s.trim();
                if !t.is_empty() {
                    cfg.openai_api_key = Some(t.to_string());
                }
                s.clear();
                print!("XAI_API_KEY (Grok): ");
                io::stdout().flush()?;
                io::stdin().read_line(&mut s)?;
                let t = s.trim();
                if !t.is_empty() {
                    cfg.xai_api_key = Some(t.to_string());
                }
                s.clear();
                print!("GROQ_API_KEY: ");
                io::stdout().flush()?;
                io::stdin().read_line(&mut s)?;
                let t = s.trim();
                if !t.is_empty() {
                    cfg.groq_api_key = Some(t.to_string());
                }
                s.clear();
                print!("DEEPSEEK_API_KEY: ");
                io::stdout().flush()?;
                io::stdin().read_line(&mut s)?;
                let t = s.trim();
                if !t.is_empty() {
                    cfg.api_key = Some(t.to_string());
                }
                cfg.save().ok();
                println!(
                    "Saved keys to {}",
                    crate::config::Config::config_path().display()
                );
                continue;
            }
            _ if input == ":tools help" => {
                println!(
                    "Examples:
  read_file: {{\"path\": \"src/main.rs\", \"start_line\": 1, \"end_line\": 80}}
  write_file: {{\"path\": \"notes.txt\", \"content\": \"Hello\", \"append\": true}}
  find_text: {{\"root\": \"src\", \"pattern\": \"async fn\", \"max_results\": 50}}
  git_diff: {{\"rev\": \"HEAD\", \"path\": \"src\"}}
  http_get: {{\"url\": \"https://example.com\", \"max_bytes\": 65536}}
  edit_file: {{\"path\": \"src/lib.rs\", \"diff\": \"--- a\\n+++ b\\n@@ -1 +1 @@\\n-old\\n+new\\n\"}}
"
                );
                continue;
            }
            _ if input == ":models" => {
                match client.list_models().await {
                    Ok(mut list) => {
                        list.sort();
                        cached_models = list.clone();
                        for (i, m) in list.iter().enumerate().take(50) {
                            println!("{:>2}. {}", i + 1, m);
                        }
                        if list.len() > 50 {
                            println!("... {} more", list.len() - 50);
                        }
                        println!("use :model <number> to select");
                    }
                    Err(e) => eprintln!("models error: {}", e),
                }
                continue;
            }
            _ if input.starts_with(":model ") => {
                let arg = input.split_whitespace().nth(1).unwrap_or("");
                if arg.is_empty() {
                    println!("usage: :model <name|index>");
                    continue;
                }
                if let Ok(idx) = arg.parse::<usize>() {
                    if idx == 0 || idx > cached_models.len() {
                        println!("invalid index");
                        continue;
                    }
                    current_model = cached_models[idx - 1].clone();
                } else {
                    current_model = arg.to_string();
                }
                println!("model set to {}", current_model);
                continue;
            }
            _ if input.starts_with(":stream ") => {
                let val = input.split_whitespace().nth(1).unwrap_or("");
                stream = matches!(val.to_lowercase().as_str(), "on" | "true" | "1");
                println!("stream={}", stream);
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

        print!("{} ", "Rusty:".bold().blue());
        io::stdout().flush()?;
        // Thinking indicator for non‑streaming responses
        let show_thinking = !stream;
        let thinking = if show_thinking {
            Some(tokio::spawn(async move {
                let mut i = 0u64;
                loop {
                    tokio::time::sleep(std::time::Duration::from_secs(1)).await;
                    i += 1;
                    let status = format!("[thinking {}s]", i).bold().bright_black();
                    print!("\r{} {} ", "Rusty:".bold().blue(), status);
                    let _ = io::stdout().flush();
                }
            }))
        } else {
            None
        };

        let derived = client.with_model(&current_model);
        let response = derived
            .complete_with_history(messages.clone(), 0.7, stream)
            .await;
        if let Some(handle) = thinking {
            handle.abort();
        }
        // Clear the thinking status and restore the label
        if show_thinking {
            print!("\r{} ", "Rusty:".bold().blue());
            io::stdout().flush()?;
        }
        let response = response?;

        messages.push(Message {
            role: "assistant".to_string(),
            content: Some(response),
            tool_calls: None,
            tool_call_id: None,
        });

        // Persist after each turn
        let _ = SessionStore::save(&session_id, &messages);

        println!();
    }

    Ok(())
}
