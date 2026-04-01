use anyhow::Result;
use clap::{Parser, Subcommand};
use uc law_providers::{
    ChatCompletionRequest, Message, ProviderRouter, ProviderConfig,
    get_available_models, resolve_model_alias
};
use std::io::{self, Write};

#[derive(Parser)]
#[command(name = "uc law")]
#[command(author = "Uclaw Team")]
#[command(version = "0.1.0")]
#[command(about = "Uclaw - Unified AI Platform", long_about = None)]
struct Cli {
    #[arg(short, long, default_value = "claude-sonnet-4-6")]
    model: String,
    
    #[arg(short, long, value_enum)]
    provider: Option<String>,
    
    #[arg(short, long)]
    stream: bool,
    
    #[command(subcommand)]
    command: Option<Commands>,
}

#[derive(Subcommand)]
enum Commands {
    Chat {
        #[arg(short, long)]
        system: Option<String>,
        
        prompt: Option<String>,
    },
    Models,
    Session {
        #[command(subcommand)]
        action: SessionAction,
    },
    Config {
        #[command(subcommand)]
        action: ConfigAction,
    },
}

#[derive(Subcommand)]
enum SessionAction {
    List,
    Clear,
    Export { filename: String },
    Import { filename: String },
}

#[derive(Subcommand)]
enum ConfigAction {
    Show,
    Set { key: String, value: String },
}

#[tokio::main]
async fn main() -> Result<()> {
    tracing_subscriber::fmt::init();
    
    let cli = Cli::parse();
    
    match cli.command {
        Some(Commands::Chat { system, prompt }) => {
            chat_mode(&cli.model, system.as_deref(), prompt.as_deref()).await?;
        }
        Some(Commands::Models) => {
            list_models();
        }
        Some(Commands::Session { action }) => {
            handle_session(action).await?;
        }
        Some(Commands::Config { action }) => {
            handle_config(action).await?;
        }
        None => {
            interactive_mode(&cli.model).await?;
        }
    }
    
    Ok(())
}

async fn chat_mode(model: &str, system: Option<&str>, prompt: Option<&str>) -> Result<()> {
    let config = ProviderConfig::from_env();
    let router = ProviderRouter::new(&config)?;
    
    let resolved_model = resolve_model_alias(model);
    
    let mut messages: Vec<Message> = Vec::new();
    
    if let Some(system_prompt) = system {
        messages.push(Message::system(system_prompt));
    }
    
    if let Some(p) = prompt {
        messages.push(Message::user(p));
        
        let request = ChatCompletionRequest {
            model: resolved_model,
            messages,
            max_tokens: Some(4096),
            temperature: None,
            top_p: None,
            stream: false,
            system: None,
        };
        
        let response = router.chat_completion(request).await?;
        
        println!("{}", response.choices[0].message.content);
    }
    
    Ok(())
}

async fn interactive_mode(model: &str) -> Result<()> {
    let config = ProviderConfig::from_env();
    let router = ProviderRouter::new(&config)?;
    
    let resolved_model = resolve_model_alias(model);
    
    println!("============================================================");
    println!("  Uclaw CLI - Unified AI Platform");
    println!("  Model: {}", router.default_model());
    println!("  Type 'exit' to quit, 'clear' to reset, 'help' for commands");
    println!("============================================================");
    println!();
    
    let mut messages: Vec<Message> = Vec::new();
    
    loop {
        print!("You: ");
        io::stdout().flush()?;
        
        let mut input = String::new();
        if io::stdin().read_line(&mut input).is_err() {
            break;
        }
        
        let input = input.trim();
        
        if input.is_empty() {
            continue;
        }
        
        match input.to_lowercase().as_str() {
            "exit" | "quit" | "q" => {
                println!("Goodbye!");
                break;
            }
            "clear" | "reset" => {
                messages.clear();
                println!("Conversation cleared.");
                continue;
            }
            "help" | "h" | "?" => {
                print_help();
                continue;
            }
            "history" | "hist" => {
                for (i, msg) in messages.iter().enumerate() {
                    let role = match msg.role {
                        uc law_providers::MessageRole::User => "User",
                        uc law_providers::MessageRole::Assistant => "Assistant",
                        uc law_providers::MessageRole::System => "System",
                    };
                    println!("[{}] {}: {}", i, role, msg.content);
                }
                continue;
            }
            cmd if cmd.starts_with("model ") => {
                let new_model = &cmd[6..].trim();
                println!("Model switched to: {}", new_model);
                continue;
            }
            _ => {}
        }
        
        messages.push(Message::user(input));
        
        print!("\nUclaw: ");
        io::stdout().flush()?;
        
        let request = ChatCompletionRequest {
            model: resolved_model.clone(),
            messages: messages.clone(),
            max_tokens: Some(4096),
            temperature: None,
            top_p: None,
            stream: false,
            system: None,
        };
        
        match router.chat_completion(request).await {
            Ok(response) => {
                println!();
                let assistant_content = &response.choices[0].message.content;
                println!("{}", assistant_content);
                messages.push(Message::assistant(assistant_content.clone()));
            }
            Err(e) => {
                eprintln!("\nError: {}", e);
            }
        }
        println!();
    }
    
    Ok(())
}

fn list_models() {
    println!("Available Models:");
    println!("==================\n");
    
    let models = get_available_models();
    let mut current_provider = String::new();
    
    for model in models {
        if model.provider != current_provider {
            println!("\n[{}]", model.provider.to_uppercase());
            current_provider = model.provider.clone();
        }
        println!("  {} - {}", model.id, model.display_name);
        if model.supports_vision {
            print!(" (vision)");
        }
        if model.supports_streaming {
            print!(" (streaming)");
        }
        println!();
    }
}

async fn handle_session(action: SessionAction) -> anyhow::Result<()> {
    match action {
        SessionAction::List => {
            println!("Sessions feature coming soon...");
        }
        SessionAction::Clear => {
            println!("Sessions cleared.");
        }
        SessionAction::Export { filename } => {
            println!("Exporting session to {}...", filename);
        }
        SessionAction::Import { filename } => {
            println!("Importing session from {}...", filename);
        }
    }
    Ok(())
}

async fn handle_config(action: ConfigAction) -> anyhow::Result<()> {
    match action {
        ConfigAction::Show => {
            let config = ProviderConfig::from_env();
            println!("Current Configuration:");
            println!("=======================");
            println!("Anthropic: {}", if config.anthropic.is_some() { "configured" } else { "not configured" });
            println!("OpenAI: {}", if config.openai.is_some() { "configured" } else { "not configured" });
            println!("Gemini: {}", if config.gemini.is_some() { "configured" } else { "not configured" });
            println!("Ollama: {}", if config.ollama.is_some() { "configured" } else { "not configured" });
        }
        ConfigAction::Set { key, value } => {
            println!("Setting {} = {} (requires restart)", key, value);
        }
    }
    Ok(())
}

fn print_help() {
    println!(r#"
Commands:
  exit, quit, q  - Exit the program
  clear, reset   - Clear conversation history
  history, hist  - Show conversation history
  model <name>   - Switch to a different model
  help           - Show this help message

Available Models:
  Claude: claude-opus-4-6, claude-sonnet-4-6, claude-haiku-4-5-20251213
  OpenAI: gpt-4o, gpt-4o-mini, gpt-4-turbo
  Gemini: gemini-2.0-flash, gemini-1.5-pro, gemini-1.5-flash

Environment Variables:
  ANTHROPIC_API_KEY - Your Anthropic API key
  OPENAI_API_KEY    - Your OpenAI API key
  GEMINI_API_KEY    - Your Google Gemini API key
"#);
}
