use std::time::{SystemTime, UNIX_EPOCH};

use serde::{Deserialize, Serialize};
use serde_json::{Value, json};
use tokio::{fs, io::AsyncWriteExt};

use crate::{
    catalog::{self, CandidateCreated, CandidateDraft, SkillLineage},
    cli,
    config::Config,
    provider::{ModelProvider, ModelRequest, ModelResponse, user_message},
};

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
pub struct SkillProposal {
    pub id: String,
    pub version: String,
    pub description: String,
    pub entrypoint: String,
    pub script: String,
    pub validation_command: String,
    pub timeout_seconds: u64,
}

pub async fn propose_skill<P: ModelProvider>(
    provider: &P,
    config: &Config,
    task: String,
    overwrite: bool,
) -> Result<CandidateCreated, String> {
    let response = provider
        .respond(ModelRequest {
            instructions: Some(proposal_prompt()),
            input: vec![user_message(task.clone())],
            tools: Vec::new(),
            store: false,
            include: Vec::new(),
            parallel_tool_calls: false,
            text_format: Some(proposal_schema()),
        })
        .await?;
    let proposal: SkillProposal = serde_json::from_str(response.output_text.trim())
        .map_err(|err| format!("model did not return a valid skill proposal: {err}"))?;
    let trace_path = write_proposal_trace(config, &task, &proposal, &response).await?;
    catalog::create_candidate(
        config.home.as_path(),
        &proposal.into_draft(task, overwrite, Some(trace_path)),
    )
}

impl SkillProposal {
    pub fn into_draft(
        self,
        task: String,
        overwrite: bool,
        source_trace: Option<String>,
    ) -> CandidateDraft {
        CandidateDraft {
            id: self.id,
            version: self.version,
            description: self.description,
            entrypoint: self.entrypoint,
            script: self.script,
            validation_command: self.validation_command,
            timeout_seconds: self.timeout_seconds,
            lineage: Some(SkillLineage {
                parent_id: None,
                source_trace,
                mutation_reason: Some(task),
            }),
            overwrite,
        }
    }
}

fn proposal_prompt() -> String {
    format!(
        "{} {}",
        cli::SYSTEM_PROMPT,
        "Return one reusable Greco skill proposal as JSON. The skill must be a \
         self-contained POSIX sh script that runs from its own skill directory. \
         Use an id with only ASCII letters, digits, hyphen, or underscore. \
         Use version 0.1.0 unless the task explicitly asks otherwise. \
         The validation_command must pass without network access. Prefer simple \
         commands like `sh run.sh | grep -x MARKER`. Do not wrap validation in \
         nested `sh -c` commands, because shell quoting is fragile."
    )
}

fn proposal_schema() -> serde_json::Value {
    json!({
        "type": "json_schema",
        "name": "greco_skill_proposal",
        "strict": true,
        "schema": {
            "type": "object",
            "properties": {
                "id": {
                    "type": "string",
                    "description": "Stable skill id using only ASCII letters, digits, hyphen, or underscore."
                },
                "version": {
                    "type": "string",
                    "description": "Semantic version for the candidate skill, normally 0.1.0."
                },
                "description": {
                    "type": "string",
                    "description": "Short description of when future Greco runs should use this skill."
                },
                "entrypoint": {
                    "type": "string",
                    "description": "Entrypoint filename, normally run.sh."
                },
                "script": {
                    "type": "string",
                    "description": "Complete POSIX sh script content."
                },
                "validation_command": {
                    "type": "string",
                    "description": "Command to run inside the skill directory to validate the script."
                },
                "timeout_seconds": {
                    "type": "integer",
                    "description": "Validation timeout in seconds."
                }
            },
            "required": [
                "id",
                "version",
                "description",
                "entrypoint",
                "script",
                "validation_command",
                "timeout_seconds"
            ],
            "additionalProperties": false
        }
    })
}

async fn write_proposal_trace(
    config: &Config,
    task: &str,
    proposal: &SkillProposal,
    response: &ModelResponse,
) -> Result<String, String> {
    let directory = config.home.join("traces/proposals");
    fs::create_dir_all(&directory)
        .await
        .map_err(|err| format!("cannot create proposal trace dir: {err}"))?;
    let path = directory.join(format!("{}.jsonl", now_nanos()));
    let row = json!({
        "ts_unix_ms": now_millis(),
        "event": "skill_proposal",
        "data": {
            "task": task,
            "response_id": response.id,
            "proposal": proposal,
            "usage": response.raw.get("usage").cloned().unwrap_or(Value::Null)
        }
    });
    let mut line = serde_json::to_string(&row).map_err(|err| err.to_string())?;
    line.push('\n');
    let mut file = fs::OpenOptions::new()
        .create_new(true)
        .write(true)
        .open(&path)
        .await
        .map_err(|err| format!("cannot open proposal trace: {err}"))?;
    file.write_all(line.as_bytes())
        .await
        .map_err(|err| format!("cannot write proposal trace: {err}"))?;
    Ok(path.display().to_string())
}

fn now_millis() -> u128 {
    SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .unwrap_or_default()
        .as_millis()
}

fn now_nanos() -> u128 {
    SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .unwrap_or_default()
        .as_nanos()
}
