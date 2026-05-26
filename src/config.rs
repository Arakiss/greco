use std::{
    collections::BTreeMap,
    env, fs,
    path::{Path, PathBuf},
};

#[derive(Debug, Clone)]
pub struct Config {
    pub provider: String,
    pub model: String,
    pub api_key: Option<String>,
    pub api_key_source: Option<String>,
    pub home: PathBuf,
    pub workspace: PathBuf,
}

impl Config {
    pub fn load() -> Result<Self, String> {
        let workspace =
            env::current_dir().map_err(|err| format!("cannot read current dir: {err}"))?;
        let file_env = load_env_files(&workspace);
        let provider =
            read_setting("GRECO_PROVIDER", &file_env).unwrap_or_else(|| "openai".to_string());
        let model = read_setting("GRECO_MODEL", &file_env).unwrap_or_else(|| "gpt-5.4".to_string());
        let home = read_setting("GRECO_HOME", &file_env)
            .map(PathBuf::from)
            .unwrap_or_else(|| workspace.join(".greco"));
        let (api_key, api_key_source) = read_secret("OPENAI_API_KEY", &file_env);

        Ok(Self {
            provider,
            model,
            api_key,
            api_key_source,
            home,
            workspace,
        })
    }
}

fn read_setting(name: &str, file_env: &BTreeMap<String, EnvValue>) -> Option<String> {
    env::var(name)
        .ok()
        .filter(|value| !value.trim().is_empty())
        .or_else(|| {
            file_env
                .get(name)
                .map(|value| value.value.clone())
                .filter(|value| !value.trim().is_empty())
        })
}

fn read_secret(
    name: &str,
    file_env: &BTreeMap<String, EnvValue>,
) -> (Option<String>, Option<String>) {
    if let Ok(value) = env::var(name)
        && !value.trim().is_empty()
    {
        return (Some(value), Some("environment".to_string()));
    }
    file_env
        .get(name)
        .filter(|value| !value.value.trim().is_empty())
        .map(|value| (Some(value.value.clone()), Some(value.source.clone())))
        .unwrap_or((None, None))
}

#[derive(Debug, Clone)]
struct EnvValue {
    value: String,
    source: String,
}

fn load_env_files(workspace: &Path) -> BTreeMap<String, EnvValue> {
    let mut values = BTreeMap::new();
    if let Some(home) = env::var_os("HOME") {
        let user_env = PathBuf::from(home).join(".config/greco/env");
        merge_env_file(&mut values, &user_env);
    }
    merge_env_file(&mut values, &workspace.join(".env.local"));
    values
}

fn merge_env_file(values: &mut BTreeMap<String, EnvValue>, path: &Path) {
    let Ok(content) = fs::read_to_string(path) else {
        return;
    };
    for line in content.lines() {
        let trimmed = line.trim();
        if trimmed.is_empty() || trimmed.starts_with('#') {
            continue;
        }
        let Some((key, value)) = trimmed.split_once('=') else {
            continue;
        };
        values.insert(
            key.trim().to_string(),
            EnvValue {
                value: unquote(value.trim()),
                source: path.display().to_string(),
            },
        );
    }
}

fn unquote(value: &str) -> String {
    let bytes = value.as_bytes();
    if bytes.len() >= 2
        && ((bytes[0] == b'"' && bytes[bytes.len() - 1] == b'"')
            || (bytes[0] == b'\'' && bytes[bytes.len() - 1] == b'\''))
    {
        value[1..value.len() - 1].to_string()
    } else {
        value.to_string()
    }
}
