mod agent;
mod audit;
mod catalog;
mod cli;
mod config;
mod eval;
mod loop_control;
mod modification;
mod proposal;
mod provider;
mod tools;
mod trajectory;
mod tui;
mod validation;

use std::process::ExitCode;

use catalog::{CandidateDraft, SkillLineage, SkillState};
use cli::{
    CatalogCommand, CatalogListState, Command, EvalCommand, LoopCommand, LoopRunMode,
    ModificationCommand, ModificationListState, ToolCommand, parse_args,
};
use config::Config;
use provider::OpenAiProvider;
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
        Command::Ask {
            input,
            stream,
            max_turns,
        } => {
            let api_key = config.api_key.clone().ok_or_else(|| {
                "OPENAI_API_KEY is missing; copy .env.example to .env.local".to_string()
            })?;
            let provider = OpenAiProvider::new(api_key, config.model.clone());
            if stream {
                eprintln!(
                    "greco: --stream is disabled for tool-loop correctness; running buffered"
                );
            }
            let outcome =
                agent::run_agent(&provider, &config, input, agent::AgentOptions { max_turns })
                    .await?;
            println!("{}", outcome.output_text);
            if !outcome.completed {
                eprintln!("greco: model did not finish within {max_turns} turns (partial)");
            }
            eprintln!(
                "greco: trace={} turns={} tools={}",
                outcome.trace_path.display(),
                outcome.turns,
                outcome.tool_calls
            );
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
        Command::ProposeSkill(command) => handle_propose_skill(command, &config).await,
        Command::Catalog(command) => handle_catalog(command, &config).await,
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
        Command::Eval(command) => handle_eval(command, &config).await,
        Command::Propose(command) => handle_propose(command, &config),
        Command::Modification(command) => handle_modification(command, &config).await,
        Command::Loop(command) => handle_loop(command, &config).await,
        Command::Audit(command) => handle_audit(command, &config),
        Command::TuiSnapshot => {
            println!("{}", tui::render_snapshot(&config)?);
            Ok(ExitCode::SUCCESS)
        }
    }
}

fn handle_propose(command: cli::ProposeCommand, config: &Config) -> Result<ExitCode, String> {
    let report = audit::build_window_report(&config.home, &command.since)?;
    let proposed = modification::propose_from_audit(&config.home, &report)?;
    if command.json {
        print_pretty(&proposed)?;
    } else {
        println!("proposed {} at {}", proposed.id, proposed.path.display());
        println!(
            "source: {}",
            proposed.manifest.friction_source.dominant_signal
        );
    }
    Ok(ExitCode::SUCCESS)
}

async fn handle_loop(command: LoopCommand, config: &Config) -> Result<ExitCode, String> {
    match command {
        LoopCommand::Run {
            since,
            mode,
            with_solver,
            json,
        } => {
            let options = loop_control::LoopRunOptions {
                since,
                mode: match mode {
                    LoopRunMode::DryRun => loop_control::LoopMode::DryRun,
                    LoopRunMode::Apply => loop_control::LoopMode::Apply,
                },
            };
            let report = if with_solver {
                let api_key = config.api_key.clone().ok_or_else(|| {
                    "OPENAI_API_KEY is missing; copy .env.example to .env.local".to_string()
                })?;
                let provider = OpenAiProvider::new(api_key, config.model.clone());
                loop_control::run_with_solver(
                    &config.home,
                    &config.workspace,
                    options,
                    &provider,
                    config,
                )
                .await?
            } else {
                loop_control::run(&config.home, &config.workspace, options).await?
            };
            if json {
                print_pretty(&report)?;
            } else {
                println!("{}", loop_control::render_run_report(&report));
            }
            Ok(if report.success {
                ExitCode::SUCCESS
            } else {
                ExitCode::from(2)
            })
        }
        LoopCommand::Status { json } => {
            let report = loop_control::status(&config.home)?;
            if json {
                print_pretty(&report)?;
            } else {
                println!("{}", loop_control::render_status(&report));
            }
            Ok(ExitCode::SUCCESS)
        }
        LoopCommand::Gate { since, json } => {
            let report = loop_control::gate(&config.home, &since)?;
            if json {
                print_pretty(&report)?;
            } else {
                println!("{}", loop_control::render_gate_report(&report));
            }
            Ok(match report.verdict {
                loop_control::LoopGateVerdict::Pass => ExitCode::SUCCESS,
                loop_control::LoopGateVerdict::NeedsMoreData => ExitCode::from(2),
                loop_control::LoopGateVerdict::Fail => ExitCode::from(3),
            })
        }
        LoopCommand::Freeze { reason, json } => {
            let report = loop_control::freeze(&config.home, reason)?;
            if json {
                print_pretty(&report)?;
            } else {
                println!("{}", loop_control::render_status(&report));
            }
            Ok(ExitCode::SUCCESS)
        }
        LoopCommand::Unfreeze { json } => {
            let report = loop_control::unfreeze(&config.home)?;
            if json {
                print_pretty(&report)?;
            } else {
                println!("{}", loop_control::render_status(&report));
            }
            Ok(ExitCode::SUCCESS)
        }
    }
}

async fn handle_modification(
    command: ModificationCommand,
    config: &Config,
) -> Result<ExitCode, String> {
    match command {
        ModificationCommand::List { state, json } => {
            let snapshot = modification::snapshot(&config.home)?;
            if json {
                match state {
                    ModificationListState::Proposed => print_pretty(&snapshot.proposed)?,
                    ModificationListState::Validated => print_pretty(&snapshot.validated)?,
                    ModificationListState::Active => print_pretty(&snapshot.active)?,
                    ModificationListState::Rejected => print_pretty(&snapshot.rejected)?,
                    ModificationListState::Retired => print_pretty(&snapshot.retired)?,
                    ModificationListState::All => print_pretty(&snapshot)?,
                }
            } else {
                print_modification_text(state, snapshot);
            }
            Ok(ExitCode::SUCCESS)
        }
        ModificationCommand::Show { id, json, diff } => {
            let (_path, manifest) = modification::read_by_id(&config.home, &id)?;
            if json {
                print_pretty(&manifest)?;
            } else if diff {
                println!("{}", modification::render_diff(&manifest));
            } else {
                println!(
                    "{:?} {:?} {} {} - {}",
                    manifest.state,
                    manifest.layer,
                    manifest.id,
                    manifest.version,
                    manifest.description
                );
            }
            Ok(ExitCode::SUCCESS)
        }
        ModificationCommand::Validate { id, json } => {
            let result = modification::validate(&config.home, &config.workspace, &id).await?;
            if json {
                print_pretty(&result)?;
            } else {
                println!(
                    "{} {:?}->{:?} at {}",
                    result.id,
                    result.from,
                    result.to,
                    result.path.display()
                );
            }
            Ok(
                if result
                    .manifest
                    .validation
                    .as_ref()
                    .is_some_and(|v| v.accepted)
                {
                    ExitCode::SUCCESS
                } else {
                    ExitCode::from(2)
                },
            )
        }
        ModificationCommand::Compare { id, json } => {
            let api_key = config.api_key.clone().ok_or_else(|| {
                "OPENAI_API_KEY is missing; copy .env.example to .env.local".to_string()
            })?;
            let provider = OpenAiProvider::new(api_key, config.model.clone());
            let report = modification::solver_compare(
                &config.home,
                &config.workspace,
                &id,
                config,
                &provider,
            )
            .await?;
            if json {
                print_pretty(&report)?;
            } else {
                println!("{}", modification::render_solver_comparison(&report));
            }
            Ok(if report.primary_improvement_ppm > 0 {
                ExitCode::SUCCESS
            } else {
                ExitCode::from(2)
            })
        }
        ModificationCommand::Apply { id, json } => {
            let result = modification::apply(&config.home, &id)?;
            print_lifecycle_result(json, &result)?;
            Ok(ExitCode::SUCCESS)
        }
        ModificationCommand::Revert { id, json } => {
            let result = modification::revert(&config.home, &id)?;
            print_lifecycle_result(json, &result)?;
            Ok(ExitCode::SUCCESS)
        }
    }
}

async fn handle_eval(command: EvalCommand, config: &Config) -> Result<ExitCode, String> {
    match command {
        EvalCommand::List { json } => {
            let tasks = eval::list_tasks(&config.home, &config.workspace)?;
            if json {
                let summaries = tasks.iter().map(eval::task_summary).collect::<Vec<_>>();
                print_pretty(&summaries)?;
            } else {
                println!("{}", eval::render_task_list(&tasks));
            }
            Ok(ExitCode::SUCCESS)
        }
        EvalCommand::Solve { task_id, json } => {
            let api_key = config.api_key.clone().ok_or_else(|| {
                "OPENAI_API_KEY is missing; copy .env.example to .env.local".to_string()
            })?;
            let provider = OpenAiProvider::new(api_key, config.model.clone());
            let report =
                eval::solve_task(&config.home, &config.workspace, config, &provider, &task_id)
                    .await?;
            if json {
                print_pretty(&report)?;
            } else {
                println!("{}", eval::render_solve_report(&report));
            }
            Ok(if report.success {
                ExitCode::SUCCESS
            } else {
                ExitCode::from(2)
            })
        }
        EvalCommand::Run { task_id, json } => {
            if task_id == "all" {
                let tasks = eval::list_tasks(&config.home, &config.workspace)?;
                let mut reports = Vec::new();
                for task in tasks {
                    reports.push(eval::run_task(&config.home, &config.workspace, &task.id).await?);
                }
                if json {
                    print_pretty(&reports)?;
                } else {
                    for report in &reports {
                        println!("{}", eval::render_run_report(report));
                    }
                }
                Ok(if reports.iter().all(|report| report.success) {
                    ExitCode::SUCCESS
                } else {
                    ExitCode::from(2)
                })
            } else {
                let report = eval::run_task(&config.home, &config.workspace, &task_id).await?;
                if json {
                    print_pretty(&report)?;
                } else {
                    println!("{}", eval::render_run_report(&report));
                }
                Ok(if report.success {
                    ExitCode::SUCCESS
                } else {
                    ExitCode::from(2)
                })
            }
        }
    }
}

fn handle_audit(command: cli::AuditCommand, config: &Config) -> Result<ExitCode, String> {
    let report = audit::write_report(&config.home, &command.since)?;
    if command.json {
        print_pretty(&report)?;
    } else {
        println!("{}", audit::render_markdown(&report));
    }
    Ok(ExitCode::SUCCESS)
}

fn print_lifecycle_result(
    json: bool,
    result: &modification::LifecycleResult,
) -> Result<(), String> {
    if json {
        print_pretty(result)
    } else {
        println!(
            "{} {:?}->{:?} at {}",
            result.id,
            result.from,
            result.to,
            result.path.display()
        );
        Ok(())
    }
}

async fn handle_propose_skill(
    command: cli::ProposeSkillCommand,
    config: &Config,
) -> Result<ExitCode, String> {
    let api_key = config
        .api_key
        .clone()
        .ok_or_else(|| "OPENAI_API_KEY is missing; copy .env.example to .env.local".to_string())?;
    let provider = OpenAiProvider::new(api_key, config.model.clone());
    let created =
        proposal::propose_skill(&provider, config, command.task, command.overwrite).await?;
    print_json_or_text(
        command.json,
        &created,
        format!(
            "candidate {} created at {}",
            created.key,
            created.path.display()
        ),
    )?;
    Ok(ExitCode::SUCCESS)
}

async fn handle_catalog(command: CatalogCommand, config: &Config) -> Result<ExitCode, String> {
    match command {
        CatalogCommand::List { state, json } => {
            let snapshot = catalog::snapshot(&config.home)?;
            if json {
                match state {
                    CatalogListState::Active => print_pretty(&snapshot.active)?,
                    CatalogListState::Candidates => print_pretty(&snapshot.candidates)?,
                    CatalogListState::Rejected => print_pretty(&snapshot.rejected)?,
                    CatalogListState::All => print_pretty(&snapshot)?,
                }
            } else {
                print_catalog_text(state, snapshot);
            }
            Ok(ExitCode::SUCCESS)
        }
        CatalogCommand::CreateCandidate {
            id,
            version,
            description,
            entrypoint,
            script,
            validation_command,
            timeout_seconds,
            overwrite,
            json,
        } => {
            let created = catalog::create_candidate(
                &config.home,
                &CandidateDraft {
                    id,
                    version,
                    description,
                    entrypoint,
                    script,
                    validation_command,
                    timeout_seconds,
                    lineage: Some(SkillLineage {
                        parent_id: None,
                        source_trace: None,
                        mutation_reason: Some(
                            "created through greco catalog create-candidate".to_string(),
                        ),
                    }),
                    overwrite,
                },
            )?;
            print_json_or_text(
                json,
                &created,
                format!(
                    "candidate {} created at {}",
                    created.key,
                    created.path.display()
                ),
            )?;
            Ok(ExitCode::SUCCESS)
        }
        CatalogCommand::Validate { id, json } => {
            let path = catalog::find_candidate_dir(&config.home, &id)?;
            let report = validation::validate_skill(&path, config).await?;
            if json {
                print_pretty(&report)?;
            } else {
                println!("{}", validation::render_report(&report));
            }
            Ok(if report.accepted {
                ExitCode::SUCCESS
            } else {
                ExitCode::from(2)
            })
        }
        CatalogCommand::Promote { id, json } => {
            let path = catalog::find_candidate_dir(&config.home, &id)?;
            let report = validation::validate_skill(&path, config).await?;
            if !report.accepted {
                if json {
                    print_pretty(&report)?;
                } else {
                    println!("{}", validation::render_report(&report));
                }
                return Ok(ExitCode::from(2));
            }
            let promoted = catalog::promote_candidate(&config.home, &id)?;
            print_json_or_text(
                json,
                &promoted,
                format!("promoted {} to {}", promoted.key, promoted.to.display()),
            )?;
            Ok(ExitCode::SUCCESS)
        }
        CatalogCommand::Reject { id, reason, json } => {
            let rejected = catalog::reject_candidate(&config.home, &id, reason)?;
            print_json_or_text(
                json,
                &rejected,
                format!("rejected {} to {}", rejected.key, rejected.to.display()),
            )?;
            Ok(ExitCode::SUCCESS)
        }
    }
}

fn print_catalog_text(state: CatalogListState, snapshot: catalog::CatalogSnapshot) {
    match state {
        CatalogListState::Active => print_entries("active", &snapshot.active),
        CatalogListState::Candidates => print_entries("candidates", &snapshot.candidates),
        CatalogListState::Rejected => print_entries("rejected", &snapshot.rejected),
        CatalogListState::All => {
            print_entries("candidates", &snapshot.candidates);
            print_entries("active", &snapshot.active);
            print_entries("rejected", &snapshot.rejected);
        }
    }
}

fn print_entries(label: &str, entries: &[catalog::SkillEntry]) {
    if entries.is_empty() {
        println!("No {label} skills.");
        return;
    }
    for entry in entries {
        println!(
            "{} {} {} - {}",
            state_label(&entry.state),
            entry.id,
            entry.version,
            entry.description
        );
    }
}

fn print_modification_text(
    state: ModificationListState,
    snapshot: modification::ModificationSnapshot,
) {
    match state {
        ModificationListState::Proposed => {
            println!("{}", modification::render_entries(&snapshot.proposed));
        }
        ModificationListState::Validated => {
            println!("{}", modification::render_entries(&snapshot.validated));
        }
        ModificationListState::Active => {
            println!("{}", modification::render_entries(&snapshot.active));
        }
        ModificationListState::Rejected => {
            println!("{}", modification::render_entries(&snapshot.rejected));
        }
        ModificationListState::Retired => {
            println!("{}", modification::render_entries(&snapshot.retired));
        }
        ModificationListState::All => {
            println!("proposed\n--------");
            println!("{}", modification::render_entries(&snapshot.proposed));
            println!("\nvalidated\n---------");
            println!("{}", modification::render_entries(&snapshot.validated));
            println!("\nactive\n------");
            println!("{}", modification::render_entries(&snapshot.active));
            println!("\nrejected\n--------");
            println!("{}", modification::render_entries(&snapshot.rejected));
            println!("\nretired\n-------");
            println!("{}", modification::render_entries(&snapshot.retired));
        }
    }
}

fn state_label(state: &SkillState) -> &'static str {
    match state {
        SkillState::Candidate => "candidate",
        SkillState::Active => "active",
        SkillState::Rejected => "rejected",
    }
}

fn print_json_or_text<T: serde::Serialize>(
    json: bool,
    value: &T,
    text: String,
) -> Result<(), String> {
    if json {
        print_pretty(value)
    } else {
        println!("{text}");
        Ok(())
    }
}

fn print_pretty<T: serde::Serialize>(value: &T) -> Result<(), String> {
    println!(
        "{}",
        serde_json::to_string_pretty(value).map_err(|err| err.to_string())?
    );
    Ok(())
}
