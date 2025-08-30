mod api;
mod chat;
mod chat_with_tools;
mod config;
mod session;
mod tools;

use anyhow::Result;
use clap::{Parser, Subcommand, ValueEnum};
use colored::*;

#[derive(Parser)]
#[command(author, version, about, long_about = None)]
#[command(propagate_version = true)]
struct Cli {
    #[command(subcommand)]
    command: Option<Commands>,

    #[arg(short, long, env = "DEEPSEEK_API_KEY", global = true)]
    api_key: Option<String>,

    #[arg(short, long, default_value = "deepseek-chat", global = true)]
    model: String,

    #[arg(long, global = true)]
    no_stream: bool,

    /// Provider to use: deepseek | openai | grok | groq
    #[arg(long, value_enum, default_value_t = Provider::Deepseek, global = true)]
    provider: Provider,
}

#[derive(Subcommand)]
enum Commands {
    Chat {
        message: Option<String>,

        #[arg(short, long)]
        system: Option<String>,

        #[arg(short, long, default_value = "0.7")]
        temperature: f32,

        #[arg(long)]
        interactive: bool,

        #[arg(long)]
        tools: bool,
    },

    Config {
        #[command(subcommand)]
        action: ConfigAction,
    },

    Models,
}

#[derive(Copy, Clone, Debug, ValueEnum)]
enum Provider {
    Deepseek,
    Openai,
    Grok,
    Groq,
}

#[derive(Subcommand)]
enum ConfigAction {
    Set {
        #[arg(value_enum)]
        key: ConfigKey,
        value: String,
    },
    Get {
        #[arg(value_enum)]
        key: Option<ConfigKey>,
    },
}

#[derive(clap::ValueEnum, Clone)]
enum ConfigKey {
    ApiKey,
    Model,
    DefaultTemperature,
}

#[tokio::main]
async fn main() -> Result<()> {
    // Initialize tracing
    tracing_subscriber::fmt::init();

    let cli = Cli::parse();

    // Models command doesn't need an API key
    if let Some(Commands::Models) = &cli.command {
        println!("{}", "Available DeepSeek models:".bold());
        println!("  • deepseek-chat (latest chat model)");
        println!("  • deepseek-chat-v3");
        println!("  • deepseek-coder (latest coder model)");
        println!("  • deepseek-coder-v2");
        println!("  • deepseek-reasoner (latest reasoning model)");
        println!("  • deepseek-reasoner-r1");
        println!("  • deepseek-reasoner-r1-distill-qwen-32b");
        println!("  • deepseek-reasoner-r1-distill-llama-70b");
        println!();
        println!(
            "{}",
            "Note: You can use any valid DeepSeek model name with -m flag".dimmed()
        );
        return Ok(());
    }

    let client: Box<dyn api::ChatClient> = match cli.provider {
        Provider::Deepseek => {
            let api_key = if let Some(key) = cli.api_key {
                key
            } else if let Ok(key) = std::env::var("DEEPSEEK_API_KEY") {
                key
            } else if let Ok(cfg) = config::Config::load() {
                cfg.api_key
                    .unwrap_or_else(|| prompt_and_save_key().expect("key"))
            } else {
                prompt_and_save_key()?
            };
            let c = api::DeepSeekClient::new(api_key, cli.model.clone());
            // Using trait object for dynamic provider dispatch
            Box::new(c) as Box<dyn api::ChatClient>
        }
        Provider::Openai => {
            let api_key = std::env::var("OPENAI_API_KEY")
                .map_err(|_| anyhow::anyhow!("Set OPENAI_API_KEY"))?;
            let base = "https://api.openai.com".to_string();
            Box::new(api::OaiCompatClient::new(api_key, cli.model.clone(), base))
                as Box<dyn api::ChatClient>
        }
        Provider::Grok => {
            let api_key = std::env::var("XAI_API_KEY")
                .or_else(|_| std::env::var("GROK_API_KEY"))
                .map_err(|_| anyhow::anyhow!("Set XAI_API_KEY or GROK_API_KEY"))?;
            let base = "https://api.x.ai/v1".to_string();
            Box::new(api::OaiCompatClient::new(api_key, cli.model.clone(), base))
                as Box<dyn api::ChatClient>
        }
        Provider::Groq => {
            let api_key =
                std::env::var("GROQ_API_KEY").map_err(|_| anyhow::anyhow!("Set GROQ_API_KEY"))?;
            let base = "https://api.groq.com/openai".to_string();
            Box::new(api::OaiCompatClient::new(api_key, cli.model.clone(), base))
                as Box<dyn api::ChatClient>
        }
    };

    match cli.command {
        Some(Commands::Chat {
            message,
            system,
            temperature,
            interactive,
            tools,
        }) => {
            if tools {
                if interactive || message.is_none() {
                    chat_with_tools::interactive_mode_with_tools(client.as_ref(), system).await?;
                } else {
                    println!(
                        "Tools mode only works in interactive mode. Use --interactive --tools"
                    );
                }
            } else if interactive || message.is_none() {
                chat::interactive_mode(client.as_ref(), system).await?;
            } else if let Some(msg) = message {
                // Build simple messages array and call via trait
                use crate::api::Message;
                let mut msgs = Vec::new();
                if let Some(sys) = system.clone() {
                    msgs.push(Message {
                        role: "system".into(),
                        content: Some(sys),
                        tool_calls: None,
                        tool_call_id: None,
                    });
                }
                msgs.push(Message {
                    role: "user".into(),
                    content: Some(msg),
                    tool_calls: None,
                    tool_call_id: None,
                });
                let response = client
                    .complete_with_history(msgs, temperature, !cli.no_stream)
                    .await?;
                println!("{response}");
            }
        }

        Some(Commands::Config { action }) => match action {
            ConfigAction::Set { key, value } => {
                let mut config = config::Config::load().unwrap_or_default();
                match key {
                    ConfigKey::ApiKey => config.api_key = Some(value),
                    ConfigKey::Model => config.default_model = Some(value),
                    ConfigKey::DefaultTemperature => {
                        config.default_temperature = Some(value.parse()?);
                    }
                }
                config.save()?;
                println!("{}", "Configuration saved".green());
            }
            ConfigAction::Get { key } => {
                let config = config::Config::load()?;
                if let Some(key) = key {
                    match key {
                        ConfigKey::ApiKey => {
                            if let Some(k) = &config.api_key {
                                let masked = if k.len() > 10 {
                                    format!("{}...{}", &k[..6], &k[k.len() - 4..])
                                } else if k.len() > 6 {
                                    format!("{}...{}", &k[..3], &k[k.len() - 3..])
                                } else {
                                    format!("**** ({} chars)", k.len())
                                };
                                println!("API Key: {}", masked);
                            }
                        }
                        ConfigKey::Model => {
                            println!(
                                "Model: {}",
                                config
                                    .default_model
                                    .unwrap_or_else(|| "deepseek-chat".to_string())
                            );
                        }
                        ConfigKey::DefaultTemperature => {
                            println!("Temperature: {}", config.default_temperature.unwrap_or(0.7));
                        }
                    }
                } else {
                    println!("{}", toml::to_string_pretty(&config)?);
                }
            }
        },

        Some(Commands::Models) => {
            // Already handled above
            unreachable!()
        }

        None => {
            let cfg = config::Config::load().unwrap_or_default();
            let picked = pick_provider_and_model_interactive(&cfg).await?;
            chat::interactive_mode(picked.as_ref(), None).await?;
        }
    }

    Ok(())
}

fn prompt_and_save_key() -> anyhow::Result<String> {
    use std::io::{self, Write};
    print!("Enter DEEPSEEK_API_KEY: ");
    io::stdout().flush()?;
    let mut key = String::new();
    io::stdin().read_line(&mut key)?;
    let key = key.trim().to_string();
    if key.is_empty() {
        anyhow::bail!("No API key provided");
    }
    let mut cfg = config::Config::load().unwrap_or_default();
    cfg.api_key = Some(key.clone());
    cfg.save()?;
    println!("Saved key to {}", config::Config::config_path().display());
    Ok(key)
}

async fn pick_provider_and_model_interactive(
    cfg: &config::Config,
) -> anyhow::Result<Box<dyn api::ChatClient>> {
    use std::io::{self, Write};
    let mut items: Vec<(&'static str, Box<dyn api::ChatClient>)> = Vec::new();
    if let Ok(k) = std::env::var("DEEPSEEK_API_KEY").or_else(|_| {
        cfg.api_key
            .clone()
            .ok_or(anyhow::anyhow!("missing"))
            .map_err(|_| std::env::VarError::NotPresent)
    }) {
        items.push((
            "DeepSeek",
            Box::new(api::DeepSeekClient::new(k, "deepseek-chat".into())),
        ));
    }
    if let Ok(k) = std::env::var("OPENAI_API_KEY").or_else(|_| {
        cfg.openai_api_key
            .clone()
            .ok_or(std::env::VarError::NotPresent)
    }) {
        items.push((
            "OpenAI",
            Box::new(api::OaiCompatClient::new(
                k,
                "gpt-4o-mini".into(),
                "https://api.openai.com".into(),
            )),
        ));
    }
    if let Ok(k) = std::env::var("XAI_API_KEY")
        .or_else(|_| std::env::var("GROK_API_KEY"))
        .or_else(|_| {
            cfg.xai_api_key
                .clone()
                .or(cfg.grok_api_key.clone())
                .ok_or(std::env::VarError::NotPresent)
        })
    {
        items.push((
            "Grok (xAI)",
            Box::new(api::OaiCompatClient::new(
                k,
                "grok-code-fast-1".into(),
                "https://api.x.ai/v1".into(),
            )),
        ));
    }
    if let Ok(k) = std::env::var("GROQ_API_KEY").or_else(|_| {
        cfg.groq_api_key
            .clone()
            .ok_or(std::env::VarError::NotPresent)
    }) {
        items.push((
            "Groq",
            Box::new(api::OaiCompatClient::new(
                k,
                "llama3-70b-8192".into(),
                "https://api.groq.com/openai".into(),
            )),
        ));
    }
    if items.is_empty() {
        println!("No provider keys found. Enter DeepSeek key to proceed.");
        let key = prompt_and_save_key()?;
        items.push((
            "DeepSeek",
            Box::new(api::DeepSeekClient::new(key, "deepseek-chat".into())),
        ));
    }
    let mut idx = 0usize;
    if items.len() > 1 {
        println!("Select provider:");
        for (i, (name, _)) in items.iter().enumerate() {
            println!("{:>2}. {}", i + 1, name);
        }
        print!("Enter number: ");
        io::stdout().flush()?;
        let mut s = String::new();
        io::stdin().read_line(&mut s)?;
        idx = s.trim().parse::<usize>().unwrap_or(1).clamp(1, items.len()) - 1;
    }
    let mut client = items.remove(idx).1;
    match client.list_models().await {
        Ok(list) if !list.is_empty() => {
            println!("Select model (Enter to keep '{}'):", client.model_name());
            for (i, m) in list.iter().enumerate().take(50) {
                println!("{:>2}. {}", i + 1, m);
            }
            print!("Model number or name: ");
            io::stdout().flush()?;
            let mut s = String::new();
            io::stdin().read_line(&mut s)?;
            let t = s.trim();
            if !t.is_empty() {
                let chosen = if let Ok(n) = t.parse::<usize>() {
                    if n >= 1 && n <= list.len() {
                        list[n - 1].clone()
                    } else {
                        t.to_string()
                    }
                } else {
                    t.to_string()
                };
                client = client.with_model(&chosen);
            }
        }
        _ => {}
    }
    Ok(client)
}
