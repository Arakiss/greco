use std::path::PathBuf;

pub const HELP: &str = "\
greco - a Rust coding-agent harness that evolves its local skill catalog

Usage:
  greco --help
  greco --version
  greco status [--json]
  greco ask --input <text> [--stream]
  greco tool read <path>
  greco tool write <path> <content>
  greco tool edit <path> <find> <replace>
  greco tool bash <command> [--timeout <seconds>]
  greco catalog list [--json]
  greco validate-skill <path> [--json]
  greco tui --snapshot

Environment:
  OPENAI_API_KEY   OpenAI API key, read from env, .env.local, or ~/.config/greco/env
  GRECO_MODEL      Model slug, defaults to gpt-5.4
  GRECO_HOME       Local archive directory, defaults to .greco
";

pub const SYSTEM_PROMPT: &str = "\
You are Greco, a minimal coding-agent harness. Use primitive tools carefully. \
Propose reusable skills only when a repeated pattern is evident. A proposed \
skill is not active until Greco validates it empirically.";

#[derive(Debug, Clone, PartialEq, Eq)]
pub enum Command {
    Help,
    Version,
    Status { json: bool },
    Ask { input: String, stream: bool },
    Tool(ToolCommand),
    CatalogList { json: bool },
    ValidateSkill { path: PathBuf, json: bool },
    TuiSnapshot,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub enum ToolCommand {
    Read {
        path: PathBuf,
    },
    Write {
        path: PathBuf,
        content: String,
    },
    Edit {
        path: PathBuf,
        find: String,
        replace: String,
    },
    Bash {
        command: String,
        timeout_seconds: u64,
    },
}

pub fn parse_args(args: Vec<String>) -> Result<Command, String> {
    if args.is_empty() {
        return Ok(Command::Help);
    }

    match args[0].as_str() {
        "-h" | "--help" | "help" => Ok(Command::Help),
        "-V" | "--version" | "version" => Ok(Command::Version),
        "status" => Ok(Command::Status {
            json: args.iter().any(|arg| arg == "--json"),
        }),
        "ask" => parse_ask(&args[1..]),
        "tool" => parse_tool(&args[1..]),
        "catalog" => parse_catalog(&args[1..]),
        "validate-skill" => parse_validate_skill(&args[1..]),
        "tui" => {
            if args.iter().any(|arg| arg == "--snapshot") {
                Ok(Command::TuiSnapshot)
            } else {
                Err(
                    "interactive TUI is not implemented yet; use `greco tui --snapshot`"
                        .to_string(),
                )
            }
        }
        other => Err(format!("unknown command `{other}`; run `greco --help`")),
    }
}

fn parse_ask(args: &[String]) -> Result<Command, String> {
    let mut input = None;
    let mut stream = false;
    let mut index = 0;
    while index < args.len() {
        match args[index].as_str() {
            "--input" => {
                index += 1;
                input = args.get(index).cloned();
            }
            "--stream" => stream = true,
            other => return Err(format!("unknown ask option `{other}`")),
        }
        index += 1;
    }
    Ok(Command::Ask {
        input: input.ok_or_else(|| "`greco ask` requires --input <text>".to_string())?,
        stream,
    })
}

fn parse_catalog(args: &[String]) -> Result<Command, String> {
    if args.first().map(String::as_str) != Some("list") {
        return Err("expected `greco catalog list`".to_string());
    }
    Ok(Command::CatalogList {
        json: args.iter().any(|arg| arg == "--json"),
    })
}

fn parse_tool(args: &[String]) -> Result<Command, String> {
    match args.first().map(String::as_str) {
        Some("read") => Ok(Command::Tool(ToolCommand::Read {
            path: required_path(args, 1, "read path")?,
        })),
        Some("write") => Ok(Command::Tool(ToolCommand::Write {
            path: required_path(args, 1, "write path")?,
            content: args
                .get(2)
                .cloned()
                .ok_or_else(|| "`greco tool write` requires content".to_string())?,
        })),
        Some("edit") => Ok(Command::Tool(ToolCommand::Edit {
            path: required_path(args, 1, "edit path")?,
            find: args
                .get(2)
                .cloned()
                .ok_or_else(|| "`greco tool edit` requires find text".to_string())?,
            replace: args
                .get(3)
                .cloned()
                .ok_or_else(|| "`greco tool edit` requires replacement text".to_string())?,
        })),
        Some("bash") => {
            let command = args
                .get(1)
                .cloned()
                .ok_or_else(|| "`greco tool bash` requires command text".to_string())?;
            let mut timeout_seconds = 30;
            let mut index = 2;
            while index < args.len() {
                match args[index].as_str() {
                    "--timeout" => {
                        index += 1;
                        timeout_seconds = args
                            .get(index)
                            .ok_or_else(|| "--timeout requires seconds".to_string())?
                            .parse()
                            .map_err(|_| "--timeout must be an integer".to_string())?;
                    }
                    other => return Err(format!("unknown bash option `{other}`")),
                }
                index += 1;
            }
            Ok(Command::Tool(ToolCommand::Bash {
                command,
                timeout_seconds,
            }))
        }
        Some(other) => Err(format!("unknown tool `{other}`")),
        None => Err("expected `greco tool <read|write|edit|bash>`".to_string()),
    }
}

fn required_path(args: &[String], index: usize, name: &str) -> Result<PathBuf, String> {
    args.get(index)
        .map(PathBuf::from)
        .ok_or_else(|| format!("missing {name}"))
}

fn parse_validate_skill(args: &[String]) -> Result<Command, String> {
    let path = args
        .iter()
        .find(|arg| !arg.starts_with("--"))
        .map(PathBuf::from)
        .ok_or_else(|| "`greco validate-skill` requires a path".to_string())?;
    Ok(Command::ValidateSkill {
        path,
        json: args.iter().any(|arg| arg == "--json"),
    })
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn parses_status_json() {
        assert_eq!(
            parse_args(vec!["status".into(), "--json".into()]).unwrap(),
            Command::Status { json: true }
        );
    }

    #[test]
    fn rejects_missing_ask_input() {
        assert!(parse_args(vec!["ask".into()]).is_err());
    }
}
