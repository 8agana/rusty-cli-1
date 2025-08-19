mod api;
mod chat;
mod chat_with_tools;
mod config;
mod tools;

use anyhow::Result;
use clap::{Parser, Subcommand};
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
        println!("{}", "Note: You can use any valid DeepSeek model name with -m flag".dimmed());
        return Ok(());
    }

    let api_key = if let Some(key) = cli.api_key {
        key
    } else if let Ok(key) = std::env::var("DEEPSEEK_API_KEY") {
        key
    } else if let Ok(config) = config::Config::load() {
        config
            .api_key
            .ok_or_else(|| anyhow::anyhow!("No API key found"))?
    } else {
        return Err(anyhow::anyhow!(
            "No API key found. Set DEEPSEEK_API_KEY or use --api-key"
        ));
    };

    let client = api::DeepSeekClient::new(api_key, cli.model);

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
                    chat_with_tools::interactive_mode_with_tools(client, system).await?;
                } else {
                    println!("Tools mode only works in interactive mode. Use --interactive --tools");
                }
            } else if interactive || message.is_none() {
                chat::interactive_mode(client, system).await?;
            } else if let Some(msg) = message {
                let response = client
                    .complete(msg, system, temperature, !cli.no_stream)
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
            chat::interactive_mode(client, None).await?;
        }
    }

    Ok(())
}
