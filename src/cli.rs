use std::path::PathBuf;

pub const HELP: &str = "\
greco - a Rust coding-agent harness that evolves its local skill catalog

Usage:
  greco --help
  greco --version
  greco status [--json]
  greco ask --input <text> [--max-turns <n>] [--stream]
  greco tool read <path>
  greco tool write <path> <content>
  greco tool edit <path> <find> <replace>
  greco tool bash <command> [--timeout <seconds>]
  greco propose-skill --task <text> [--overwrite] [--json]
  greco catalog list [--state <active|candidates|rejected|all>] [--json]
  greco catalog create-candidate --id <id> --description <text> --script <text> --validation-command <command> [--json]
  greco catalog validate <candidate-id> [--json]
  greco catalog promote <candidate-id> [--json]
  greco catalog reject <candidate-id> --reason <text> [--json]
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
    Status {
        json: bool,
    },
    Ask {
        input: String,
        stream: bool,
        max_turns: usize,
    },
    Tool(ToolCommand),
    ProposeSkill(ProposeSkillCommand),
    Catalog(CatalogCommand),
    ValidateSkill {
        path: PathBuf,
        json: bool,
    },
    TuiSnapshot,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct ProposeSkillCommand {
    pub task: String,
    pub overwrite: bool,
    pub json: bool,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub enum CatalogCommand {
    List {
        state: CatalogListState,
        json: bool,
    },
    CreateCandidate {
        id: String,
        version: String,
        description: String,
        entrypoint: String,
        script: String,
        validation_command: String,
        timeout_seconds: u64,
        overwrite: bool,
        json: bool,
    },
    Validate {
        id: String,
        json: bool,
    },
    Promote {
        id: String,
        json: bool,
    },
    Reject {
        id: String,
        reason: String,
        json: bool,
    },
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub enum CatalogListState {
    Active,
    Candidates,
    Rejected,
    All,
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
        "propose-skill" => parse_propose_skill(&args[1..]),
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
    let mut max_turns = 8;
    let mut index = 0;
    while index < args.len() {
        match args[index].as_str() {
            "--input" => {
                index += 1;
                input = args.get(index).cloned();
            }
            "--stream" => stream = true,
            "--max-turns" => {
                index += 1;
                max_turns = args
                    .get(index)
                    .ok_or_else(|| "--max-turns requires an integer".to_string())?
                    .parse()
                    .map_err(|_| "--max-turns must be an integer".to_string())?;
            }
            other => return Err(format!("unknown ask option `{other}`")),
        }
        index += 1;
    }
    Ok(Command::Ask {
        input: input.ok_or_else(|| "`greco ask` requires --input <text>".to_string())?,
        stream,
        max_turns,
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

fn parse_propose_skill(args: &[String]) -> Result<Command, String> {
    let mut task = None;
    let mut overwrite = false;
    let mut json = false;
    let mut index = 0;
    while index < args.len() {
        match args[index].as_str() {
            "--task" => {
                index += 1;
                task = args.get(index).cloned();
            }
            "--overwrite" => overwrite = true,
            "--json" => json = true,
            other => return Err(format!("unknown propose-skill option `{other}`")),
        }
        index += 1;
    }
    Ok(Command::ProposeSkill(ProposeSkillCommand {
        task: task.ok_or_else(|| "`greco propose-skill` requires --task <text>".to_string())?,
        overwrite,
        json,
    }))
}

fn parse_catalog(args: &[String]) -> Result<Command, String> {
    match args.first().map(String::as_str) {
        Some("list") => parse_catalog_list(&args[1..]),
        Some("create-candidate") => parse_catalog_create_candidate(&args[1..]),
        Some("validate") => Ok(Command::Catalog(CatalogCommand::Validate {
            id: required_arg(args, 1, "candidate id")?,
            json: args.iter().any(|arg| arg == "--json"),
        })),
        Some("promote") => Ok(Command::Catalog(CatalogCommand::Promote {
            id: required_arg(args, 1, "candidate id")?,
            json: args.iter().any(|arg| arg == "--json"),
        })),
        Some("reject") => parse_catalog_reject(args),
        Some(other) => Err(format!("unknown catalog command `{other}`")),
        None => Err(
            "expected `greco catalog <list|create-candidate|validate|promote|reject>`".to_string(),
        ),
    }
}

fn parse_catalog_list(args: &[String]) -> Result<Command, String> {
    let mut state = CatalogListState::Active;
    let mut index = 0;
    while index < args.len() {
        match args[index].as_str() {
            "--state" => {
                index += 1;
                state = match args.get(index).map(String::as_str) {
                    Some("active") => CatalogListState::Active,
                    Some("candidates") => CatalogListState::Candidates,
                    Some("rejected") => CatalogListState::Rejected,
                    Some("all") => CatalogListState::All,
                    Some(other) => return Err(format!("unknown catalog state `{other}`")),
                    None => return Err("--state requires a value".to_string()),
                };
            }
            "--json" => {}
            other => return Err(format!("unknown catalog list option `{other}`")),
        }
        index += 1;
    }
    Ok(Command::Catalog(CatalogCommand::List {
        state,
        json: args.iter().any(|arg| arg == "--json"),
    }))
}

fn parse_catalog_create_candidate(args: &[String]) -> Result<Command, String> {
    let mut id = None;
    let mut version = "0.1.0".to_string();
    let mut description = None;
    let mut entrypoint = "run.sh".to_string();
    let mut script = None;
    let mut validation_command = None;
    let mut timeout_seconds = 5;
    let mut overwrite = false;
    let mut json = false;
    let mut index = 0;
    while index < args.len() {
        match args[index].as_str() {
            "--id" => {
                index += 1;
                id = args.get(index).cloned();
            }
            "--version" => {
                index += 1;
                version = args
                    .get(index)
                    .cloned()
                    .ok_or_else(|| "--version requires a value".to_string())?;
            }
            "--description" => {
                index += 1;
                description = args.get(index).cloned();
            }
            "--entrypoint" => {
                index += 1;
                entrypoint = args
                    .get(index)
                    .cloned()
                    .ok_or_else(|| "--entrypoint requires a value".to_string())?;
            }
            "--script" => {
                index += 1;
                script = args.get(index).cloned();
            }
            "--validation-command" => {
                index += 1;
                validation_command = args.get(index).cloned();
            }
            "--timeout" => {
                index += 1;
                timeout_seconds = args
                    .get(index)
                    .ok_or_else(|| "--timeout requires seconds".to_string())?
                    .parse()
                    .map_err(|_| "--timeout must be an integer".to_string())?;
            }
            "--overwrite" => overwrite = true,
            "--json" => json = true,
            other => return Err(format!("unknown create-candidate option `{other}`")),
        }
        index += 1;
    }
    Ok(Command::Catalog(CatalogCommand::CreateCandidate {
        id: id.ok_or_else(|| "--id is required".to_string())?,
        version,
        description: description.ok_or_else(|| "--description is required".to_string())?,
        entrypoint,
        script: script.ok_or_else(|| "--script is required".to_string())?,
        validation_command: validation_command
            .ok_or_else(|| "--validation-command is required".to_string())?,
        timeout_seconds,
        overwrite,
        json,
    }))
}

fn parse_catalog_reject(args: &[String]) -> Result<Command, String> {
    let id = required_arg(args, 1, "candidate id")?;
    let mut reason = None;
    let mut index = 2;
    while index < args.len() {
        match args[index].as_str() {
            "--reason" => {
                index += 1;
                reason = args.get(index).cloned();
            }
            "--json" => {}
            other => return Err(format!("unknown reject option `{other}`")),
        }
        index += 1;
    }
    Ok(Command::Catalog(CatalogCommand::Reject {
        id,
        reason: reason.ok_or_else(|| "--reason is required".to_string())?,
        json: args.iter().any(|arg| arg == "--json"),
    }))
}

fn required_path(args: &[String], index: usize, name: &str) -> Result<PathBuf, String> {
    args.get(index)
        .map(PathBuf::from)
        .ok_or_else(|| format!("missing {name}"))
}

fn required_arg(args: &[String], index: usize, name: &str) -> Result<String, String> {
    args.get(index)
        .cloned()
        .filter(|arg| !arg.starts_with("--"))
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

    #[test]
    fn parses_propose_skill() {
        assert_eq!(
            parse_args(vec![
                "propose-skill".into(),
                "--task".into(),
                "make a skill".into(),
                "--overwrite".into(),
                "--json".into(),
            ])
            .unwrap(),
            Command::ProposeSkill(ProposeSkillCommand {
                task: "make a skill".to_string(),
                overwrite: true,
                json: true,
            })
        );
    }

    #[test]
    fn parses_catalog_create_candidate() {
        assert_eq!(
            parse_args(vec![
                "catalog".into(),
                "create-candidate".into(),
                "--id".into(),
                "demo".into(),
                "--description".into(),
                "Demo".into(),
                "--script".into(),
                "#!/bin/sh".into(),
                "--validation-command".into(),
                "sh run.sh".into(),
            ])
            .unwrap(),
            Command::Catalog(CatalogCommand::CreateCandidate {
                id: "demo".to_string(),
                version: "0.1.0".to_string(),
                description: "Demo".to_string(),
                entrypoint: "run.sh".to_string(),
                script: "#!/bin/sh".to_string(),
                validation_command: "sh run.sh".to_string(),
                timeout_seconds: 5,
                overwrite: false,
                json: false,
            })
        );
    }
}
