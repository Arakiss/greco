use std::{
    collections::BTreeMap,
    fs,
    path::{Path, PathBuf},
    time::{SystemTime, UNIX_EPOCH},
};

use serde::{Deserialize, Serialize};

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
pub struct SkillManifest {
    pub id: String,
    pub version: String,
    pub kind: SkillKind,
    pub entrypoint: String,
    pub description: String,
    #[serde(default)]
    pub lineage: Option<SkillLineage>,
    #[serde(default)]
    pub validation: Option<ValidationSpec>,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
#[serde(rename_all = "snake_case")]
pub enum SkillKind {
    Script,
    Rust,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
pub struct SkillLineage {
    pub parent_id: Option<String>,
    pub source_trace: Option<String>,
    pub mutation_reason: Option<String>,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
pub struct ValidationSpec {
    pub command: Option<String>,
    pub timeout_seconds: Option<u64>,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
#[serde(rename_all = "snake_case")]
pub enum SkillState {
    Candidate,
    Active,
    Rejected,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
pub struct SkillEntry {
    pub id: String,
    pub version: String,
    pub key: String,
    pub description: String,
    pub path: PathBuf,
    pub state: SkillState,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
pub struct ActiveSkill {
    pub id: String,
    pub version: String,
    pub description: String,
    pub path: PathBuf,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Default)]
pub struct SkillScore {
    pub attempts: u64,
    pub passes: u64,
    pub failures: u64,
    pub score: f64,
    pub last_validated_at_unix_ms: Option<u128>,
    pub last_promoted_at_unix_ms: Option<u128>,
    pub last_rejected_at_unix_ms: Option<u128>,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Default)]
pub struct ScoreBook {
    pub skills: BTreeMap<String, SkillScore>,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq, Default)]
pub struct Catalog {
    pub active: Vec<ActiveSkill>,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
pub struct CatalogSnapshot {
    pub candidates: Vec<SkillEntry>,
    pub active: Vec<SkillEntry>,
    pub rejected: Vec<SkillEntry>,
    pub scores: BTreeMap<String, SkillScore>,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
pub struct CandidateDraft {
    pub id: String,
    pub version: String,
    pub description: String,
    pub entrypoint: String,
    pub script: String,
    pub validation_command: String,
    pub timeout_seconds: u64,
    pub lineage: Option<SkillLineage>,
    pub overwrite: bool,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
pub struct CandidateCreated {
    pub key: String,
    pub path: PathBuf,
    pub manifest: SkillManifest,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
pub struct PromotionResult {
    pub key: String,
    pub from: PathBuf,
    pub to: PathBuf,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
pub struct RejectionResult {
    pub key: String,
    pub from: PathBuf,
    pub to: PathBuf,
    pub reason: String,
}

impl Catalog {
    pub fn load(home: &Path) -> Result<Self, String> {
        let active = list_entries(home, SkillState::Active)?
            .into_iter()
            .map(|entry| ActiveSkill {
                id: entry.id,
                version: entry.version,
                description: entry.description,
                path: entry.path,
            })
            .collect();
        Ok(Self { active })
    }
}

impl CandidateDraft {
    pub fn manifest(&self) -> SkillManifest {
        SkillManifest {
            id: self.id.clone(),
            version: self.version.clone(),
            kind: SkillKind::Script,
            entrypoint: self.entrypoint.clone(),
            description: self.description.clone(),
            lineage: self.lineage.clone(),
            validation: Some(ValidationSpec {
                command: Some(self.validation_command.clone()),
                timeout_seconds: Some(self.timeout_seconds),
            }),
        }
    }
}

pub fn snapshot(home: &Path) -> Result<CatalogSnapshot, String> {
    Ok(CatalogSnapshot {
        candidates: list_entries(home, SkillState::Candidate)?,
        active: list_entries(home, SkillState::Active)?,
        rejected: list_entries(home, SkillState::Rejected)?,
        scores: load_scores(home)?.skills,
    })
}

pub fn list_entries(home: &Path, state: SkillState) -> Result<Vec<SkillEntry>, String> {
    let dir = state_dir(home, &state);
    let Ok(entries) = fs::read_dir(dir) else {
        return Ok(Vec::new());
    };

    let mut skills = Vec::new();
    for entry in entries {
        let entry = entry.map_err(|err| format!("catalog read failed: {err}"))?;
        if !entry.path().is_dir() {
            continue;
        }
        let manifest = read_manifest(&entry.path())?;
        skills.push(entry_from_manifest(manifest, entry.path(), state.clone()));
    }
    skills.sort_by(|left, right| left.key.cmp(&right.key));
    Ok(skills)
}

pub fn create_candidate(home: &Path, draft: &CandidateDraft) -> Result<CandidateCreated, String> {
    validate_draft(draft)?;
    let manifest = draft.manifest();
    let key = skill_key(&manifest.id, &manifest.version);
    let target = state_dir(home, &SkillState::Candidate).join(&key);
    if target.exists() {
        if draft.overwrite {
            fs::remove_dir_all(&target)
                .map_err(|err| format!("cannot replace existing candidate: {err}"))?;
        } else {
            return Err(format!("candidate already exists: {key}"));
        }
    }
    fs::create_dir_all(&target).map_err(|err| format!("cannot create candidate dir: {err}"))?;
    let manifest_json = serde_json::to_string_pretty(&manifest).map_err(|err| err.to_string())?;
    fs::write(target.join("manifest.json"), manifest_json)
        .map_err(|err| format!("cannot write manifest: {err}"))?;
    fs::write(target.join(&draft.entrypoint), &draft.script)
        .map_err(|err| format!("cannot write entrypoint: {err}"))?;
    make_executable(&target.join(&draft.entrypoint))?;
    validate_manifest(&manifest, &target)?;
    Ok(CandidateCreated {
        key,
        path: target,
        manifest,
    })
}

pub fn promote_candidate(home: &Path, key_or_id: &str) -> Result<PromotionResult, String> {
    let from = find_skill_dir(home, SkillState::Candidate, key_or_id)?;
    let manifest = read_manifest(&from)?;
    let key = skill_key(&manifest.id, &manifest.version);
    let to = state_dir(home, &SkillState::Active).join(&key);
    if to.exists() {
        return Err(format!("active skill already exists: {key}"));
    }
    fs::create_dir_all(state_dir(home, &SkillState::Active))
        .map_err(|err| format!("cannot create active catalog dir: {err}"))?;
    fs::rename(&from, &to).map_err(|err| format!("cannot promote candidate: {err}"))?;
    update_score(home, &key, ScoreEvent::Promotion)?;
    Ok(PromotionResult { key, from, to })
}

pub fn reject_candidate(
    home: &Path,
    key_or_id: &str,
    reason: String,
) -> Result<RejectionResult, String> {
    if reason.trim().is_empty() {
        return Err("rejection reason is required".to_string());
    }
    let from = find_skill_dir(home, SkillState::Candidate, key_or_id)?;
    let manifest = read_manifest(&from)?;
    let key = skill_key(&manifest.id, &manifest.version);
    let to = state_dir(home, &SkillState::Rejected).join(&key);
    if to.exists() {
        fs::remove_dir_all(&to)
            .map_err(|err| format!("cannot replace existing rejection: {err}"))?;
    }
    fs::create_dir_all(state_dir(home, &SkillState::Rejected))
        .map_err(|err| format!("cannot create rejected catalog dir: {err}"))?;
    fs::write(
        from.join("rejection.json"),
        serde_json::to_string_pretty(&serde_json::json!({
            "reason": reason.clone(),
            "rejected_at_unix_ms": now_millis()
        }))
        .map_err(|err| err.to_string())?,
    )
    .map_err(|err| format!("cannot write rejection metadata: {err}"))?;
    fs::rename(&from, &to).map_err(|err| format!("cannot reject candidate: {err}"))?;
    update_score(home, &key, ScoreEvent::Rejection)?;
    Ok(RejectionResult {
        key,
        from,
        to,
        reason,
    })
}

pub fn find_candidate_dir(home: &Path, key_or_id: &str) -> Result<PathBuf, String> {
    find_skill_dir(home, SkillState::Candidate, key_or_id)
}

pub fn read_manifest(skill_dir: &Path) -> Result<SkillManifest, String> {
    let path = skill_dir.join("manifest.json");
    let content = fs::read_to_string(&path)
        .map_err(|err| format!("cannot read {}: {err}", path.display()))?;
    serde_json::from_str(&content)
        .map_err(|err| format!("invalid manifest {}: {err}", path.display()))
}

pub fn validate_manifest(manifest: &SkillManifest, skill_dir: &Path) -> Result<(), String> {
    validate_slug(&manifest.id, "manifest id")?;
    if manifest.version.trim().is_empty() {
        return Err("manifest version is required".to_string());
    }
    if manifest.version.contains('/') || manifest.version.contains('\\') {
        return Err("manifest version must not contain path separators".to_string());
    }
    if manifest.description.trim().is_empty() {
        return Err("manifest description is required".to_string());
    }
    if manifest.entrypoint.trim().is_empty() {
        return Err("manifest entrypoint is required".to_string());
    }
    if manifest.entrypoint.contains("..")
        || manifest.entrypoint.starts_with('/')
        || manifest.entrypoint.starts_with('\\')
    {
        return Err("manifest entrypoint must stay inside the skill directory".to_string());
    }
    let entrypoint = skill_dir.join(&manifest.entrypoint);
    if !entrypoint.exists() {
        return Err(format!(
            "entrypoint does not exist: {}",
            entrypoint.display()
        ));
    }
    Ok(())
}

pub fn record_validation(
    home: &Path,
    manifest: &SkillManifest,
    accepted: bool,
) -> Result<(), String> {
    update_score(
        home,
        &skill_key(&manifest.id, &manifest.version),
        ScoreEvent::Validation { accepted },
    )
}

fn validate_draft(draft: &CandidateDraft) -> Result<(), String> {
    validate_slug(&draft.id, "candidate id")?;
    if draft.version.trim().is_empty() {
        return Err("candidate version is required".to_string());
    }
    if draft.description.trim().is_empty() {
        return Err("candidate description is required".to_string());
    }
    if draft.entrypoint.trim().is_empty() {
        return Err("candidate entrypoint is required".to_string());
    }
    if draft.entrypoint.contains("..")
        || draft.entrypoint.starts_with('/')
        || draft.entrypoint.starts_with('\\')
    {
        return Err("candidate entrypoint must stay inside the skill directory".to_string());
    }
    if draft.script.trim().is_empty() {
        return Err("candidate script is required".to_string());
    }
    if draft.validation_command.trim().is_empty() {
        return Err("candidate validation command is required".to_string());
    }
    if draft.timeout_seconds == 0 {
        return Err("candidate timeout must be greater than zero".to_string());
    }
    Ok(())
}

fn validate_slug(value: &str, name: &str) -> Result<(), String> {
    if value.trim().is_empty() {
        return Err(format!("{name} is required"));
    }
    if value
        .chars()
        .all(|ch| ch.is_ascii_alphanumeric() || ch == '-' || ch == '_')
    {
        Ok(())
    } else {
        Err(format!(
            "{name} may only contain ASCII letters, digits, hyphen, or underscore"
        ))
    }
}

fn entry_from_manifest(manifest: SkillManifest, path: PathBuf, state: SkillState) -> SkillEntry {
    let key = skill_key(&manifest.id, &manifest.version);
    SkillEntry {
        id: manifest.id,
        version: manifest.version,
        key,
        description: manifest.description,
        path,
        state,
    }
}

fn find_skill_dir(home: &Path, state: SkillState, key_or_id: &str) -> Result<PathBuf, String> {
    for entry in list_entries(home, state)? {
        if entry.key == key_or_id || entry.id == key_or_id {
            return Ok(entry.path);
        }
    }
    Err(format!("skill not found: {key_or_id}"))
}

fn state_dir(home: &Path, state: &SkillState) -> PathBuf {
    let name = match state {
        SkillState::Candidate => "candidates",
        SkillState::Active => "active",
        SkillState::Rejected => "rejected",
    };
    home.join("catalog").join(name)
}

fn score_path(home: &Path) -> PathBuf {
    home.join("catalog").join("scores.json")
}

fn load_scores(home: &Path) -> Result<ScoreBook, String> {
    let path = score_path(home);
    let Ok(content) = fs::read_to_string(&path) else {
        return Ok(ScoreBook::default());
    };
    serde_json::from_str(&content)
        .map_err(|err| format!("invalid scores {}: {err}", path.display()))
}

fn save_scores(home: &Path, scores: &ScoreBook) -> Result<(), String> {
    let path = score_path(home);
    if let Some(parent) = path.parent() {
        fs::create_dir_all(parent).map_err(|err| format!("cannot create score dir: {err}"))?;
    }
    fs::write(
        &path,
        serde_json::to_string_pretty(scores).map_err(|err| err.to_string())?,
    )
    .map_err(|err| format!("cannot write scores {}: {err}", path.display()))
}

enum ScoreEvent {
    Validation { accepted: bool },
    Promotion,
    Rejection,
}

fn update_score(home: &Path, key: &str, event: ScoreEvent) -> Result<(), String> {
    let mut scores = load_scores(home)?;
    let score = scores.skills.entry(key.to_string()).or_default();
    let now = now_millis();
    match event {
        ScoreEvent::Validation { accepted } => {
            score.attempts += 1;
            if accepted {
                score.passes += 1;
            } else {
                score.failures += 1;
            }
            score.last_validated_at_unix_ms = Some(now);
        }
        ScoreEvent::Promotion => {
            score.last_promoted_at_unix_ms = Some(now);
        }
        ScoreEvent::Rejection => {
            score.last_rejected_at_unix_ms = Some(now);
        }
    }
    score.score = if score.attempts == 0 {
        0.0
    } else {
        score.passes as f64 / score.attempts as f64
    };
    save_scores(home, &scores)
}

fn skill_key(id: &str, version: &str) -> String {
    format!("{id}@{version}")
}

fn now_millis() -> u128 {
    SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .unwrap_or_default()
        .as_millis()
}

#[cfg(unix)]
fn make_executable(path: &Path) -> Result<(), String> {
    use std::os::unix::fs::PermissionsExt;
    let mut permissions = fs::metadata(path)
        .map_err(|err| format!("cannot stat entrypoint: {err}"))?
        .permissions();
    permissions.set_mode(0o755);
    fs::set_permissions(path, permissions)
        .map_err(|err| format!("cannot make entrypoint executable: {err}"))
}

#[cfg(not(unix))]
fn make_executable(_path: &Path) -> Result<(), String> {
    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::{
        fs,
        time::{SystemTime, UNIX_EPOCH},
    };

    #[test]
    fn parses_minimal_manifest() {
        let manifest: SkillManifest = serde_json::from_str(
            r#"{
                "id": "demo",
                "version": "0.1.0",
                "kind": "script",
                "entrypoint": "run.sh",
                "description": "Demo skill"
            }"#,
        )
        .unwrap();
        assert_eq!(manifest.id, "demo");
        assert_eq!(manifest.kind, SkillKind::Script);
    }

    #[test]
    fn rejects_unsafe_candidate_ids() {
        let draft = CandidateDraft {
            id: "../demo".to_string(),
            version: "0.1.0".to_string(),
            description: "Demo".to_string(),
            entrypoint: "run.sh".to_string(),
            script: "#!/bin/sh\n".to_string(),
            validation_command: "sh run.sh".to_string(),
            timeout_seconds: 5,
            lineage: None,
            overwrite: false,
        };
        assert!(create_candidate(Path::new("/tmp/greco-nope"), &draft).is_err());
    }

    #[test]
    fn creates_promotes_rejects_and_scores_candidates() {
        let home = temp_dir("catalog-cycle");
        fs::create_dir_all(&home).unwrap();

        let created = create_candidate(&home, &draft("demo_skill", false)).unwrap();
        assert_eq!(created.key, "demo_skill@0.1.0");
        assert!(created.path.join("manifest.json").exists());

        let promoted = promote_candidate(&home, "demo_skill").unwrap();
        assert!(promoted.to.join("manifest.json").exists());
        let first_snapshot = snapshot(&home).unwrap();
        assert_eq!(first_snapshot.active.len(), 1);
        assert!(first_snapshot.candidates.is_empty());
        assert!(
            first_snapshot.scores["demo_skill@0.1.0"]
                .last_promoted_at_unix_ms
                .is_some()
        );

        let created = create_candidate(&home, &draft("reject_me", false)).unwrap();
        assert!(created.path.exists());
        let rejected = reject_candidate(&home, "reject_me", "not useful".to_string()).unwrap();
        assert!(rejected.to.join("rejection.json").exists());
        let second_snapshot = snapshot(&home).unwrap();
        assert_eq!(second_snapshot.rejected.len(), 1);
        assert!(
            second_snapshot.scores["reject_me@0.1.0"]
                .last_rejected_at_unix_ms
                .is_some()
        );

        fs::remove_dir_all(home).unwrap();
    }

    fn draft(id: &str, overwrite: bool) -> CandidateDraft {
        CandidateDraft {
            id: id.to_string(),
            version: "0.1.0".to_string(),
            description: "Demo skill".to_string(),
            entrypoint: "run.sh".to_string(),
            script: "#!/bin/sh\nset -eu\nprintf '%s\\n' demo\n".to_string(),
            validation_command: "sh run.sh".to_string(),
            timeout_seconds: 5,
            lineage: None,
            overwrite,
        }
    }

    fn temp_dir(label: &str) -> PathBuf {
        let nanos = SystemTime::now()
            .duration_since(UNIX_EPOCH)
            .unwrap()
            .as_nanos();
        std::env::temp_dir().join(format!(
            "greco-catalog-test-{label}-{nanos}-{}",
            std::process::id()
        ))
    }
}
