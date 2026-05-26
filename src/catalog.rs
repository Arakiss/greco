use std::{
    fs,
    path::{Path, PathBuf},
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
pub struct ActiveSkill {
    pub id: String,
    pub version: String,
    pub description: String,
    pub path: PathBuf,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq, Default)]
pub struct Catalog {
    pub active: Vec<ActiveSkill>,
}

impl Catalog {
    pub fn load(home: &Path) -> Result<Self, String> {
        let active_dir = home.join("catalog/active");
        let Ok(entries) = fs::read_dir(active_dir) else {
            return Ok(Self::default());
        };

        let mut active = Vec::new();
        for entry in entries {
            let entry = entry.map_err(|err| format!("catalog read failed: {err}"))?;
            if !entry.path().is_dir() {
                continue;
            }
            let manifest = read_manifest(&entry.path())?;
            active.push(ActiveSkill {
                id: manifest.id,
                version: manifest.version,
                description: manifest.description,
                path: entry.path(),
            });
        }
        active.sort_by(|left, right| left.id.cmp(&right.id));
        Ok(Self { active })
    }
}

pub fn read_manifest(skill_dir: &Path) -> Result<SkillManifest, String> {
    let path = skill_dir.join("manifest.json");
    let content = fs::read_to_string(&path)
        .map_err(|err| format!("cannot read {}: {err}", path.display()))?;
    serde_json::from_str(&content)
        .map_err(|err| format!("invalid manifest {}: {err}", path.display()))
}

pub fn validate_manifest(manifest: &SkillManifest, skill_dir: &Path) -> Result<(), String> {
    if manifest.id.trim().is_empty() {
        return Err("manifest id is required".to_string());
    }
    if manifest.version.trim().is_empty() {
        return Err("manifest version is required".to_string());
    }
    if manifest.description.trim().is_empty() {
        return Err("manifest description is required".to_string());
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

#[cfg(test)]
mod tests {
    use super::*;

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
}
