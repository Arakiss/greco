mod catalog;
mod cli;
mod config;
mod provider;
mod tools;
mod tui;
mod validation;

use std::process::ExitCode;

use cli::{Command, ToolCommand, parse_args};
use config::Config;
use provider::{ModelMessage, ModelProvider, ModelRequest, OpenAiProvider};
use serde_json::json;

#[tokio::main]
async fn main() -> ExitCode {
    match run().await {
        Ok(code) => code,
        Err(err) => {
            eprintln!("greco: {err}");
            ExitCode::from(1)
        }
    }
}

async fn run() -> Result<ExitCode, String> {
    let command = parse_args(std::env::args().skip(1).collect())?;
    let config = Config::load()?;

    match command {
        Command::Help => {
            print!("{}", cli::HELP);
            Ok(ExitCode::SUCCESS)
        }
        Command::Version => {
            println!("greco {}", env!("CARGO_PKG_VERSION"));
            Ok(ExitCode::SUCCESS)
        }
        Command::Status { json: as_json } => {
            let status = json!({
                "name": "greco",
                "version": env!("CARGO_PKG_VERSION"),
                "provider": config.provider,
                "model": config.model,
                "workspace": config.workspace.display().to_string(),
                "home": config.home.display().to_string(),
                "api_key": {
                    "present": config.api_key.is_some(),
                    "source": config.api_key_source,
                },
            });
            if as_json {
                println!(
                    "{}",
                    serde_json::to_string_pretty(&status).map_err(|err| err.to_string())?
                );
            } else {
                println!("{}", tui::render_status(&config));
            }
            Ok(ExitCode::SUCCESS)
        }
        Command::Ask { input, stream } => {
            let api_key = config.api_key.clone().ok_or_else(|| {
                "OPENAI_API_KEY is missing; copy .env.example to .env.local".to_string()
            })?;
            let provider = OpenAiProvider::new(api_key, config.model.clone());
            let request = ModelRequest {
                instructions: Some(cli::SYSTEM_PROMPT.to_string()),
                input: vec![ModelMessage::user(input)],
                tools: tools::primitive_tool_specs(),
                store: false,
            };
            if stream {
                let text = provider.stream_text(request).await?;
                println!("{text}");
            } else {
                let response = provider.respond(request).await?;
                println!("{}", response.output_text);
            }
            Ok(ExitCode::SUCCESS)
        }
        Command::Tool(tool) => {
            let result = match tool {
                ToolCommand::Read { path } => tools::read_file(&config.workspace, &path).await?,
                ToolCommand::Write { path, content } => {
                    tools::write_file(&config.workspace, &path, &content).await?
                }
                ToolCommand::Edit {
                    path,
                    find,
                    replace,
                } => tools::edit_file(&config.workspace, &path, &find, &replace).await?,
                ToolCommand::Bash {
                    command,
                    timeout_seconds,
                } => tools::run_bash(&config.workspace, &command, timeout_seconds).await?,
            };
            println!(
                "{}",
                serde_json::to_string_pretty(&result).map_err(|err| err.to_string())?
            );
            Ok(if result.success {
                ExitCode::SUCCESS
            } else {
                ExitCode::from(2)
            })
        }
        Command::CatalogList { json: as_json } => {
            let catalog = catalog::Catalog::load(&config.home)?;
            if as_json {
                println!(
                    "{}",
                    serde_json::to_string_pretty(&catalog.active).map_err(|err| err.to_string())?
                );
            } else if catalog.active.is_empty() {
                println!("No active skills yet.");
            } else {
                for skill in catalog.active {
                    println!("{} {} - {}", skill.id, skill.version, skill.description);
                }
            }
            Ok(ExitCode::SUCCESS)
        }
        Command::ValidateSkill {
            path,
            json: as_json,
        } => {
            let report = validation::validate_skill(&path, &config).await?;
            if as_json {
                println!(
                    "{}",
                    serde_json::to_string_pretty(&report).map_err(|err| err.to_string())?
                );
            } else {
                println!("{}", validation::render_report(&report));
            }
            Ok(if report.accepted {
                ExitCode::SUCCESS
            } else {
                ExitCode::from(2)
            })
        }
        Command::TuiSnapshot => {
            println!("{}", tui::render_snapshot(&config)?);
            Ok(ExitCode::SUCCESS)
        }
    }
}
