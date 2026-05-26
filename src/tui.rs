use crate::{catalog::Catalog, config::Config};

pub fn render_status(config: &Config) -> String {
    [
        format!("Greco {}", env!("CARGO_PKG_VERSION")),
        format!("provider: {}", config.provider),
        format!("model: {}", config.model),
        format!("workspace: {}", config.workspace.display()),
        format!("home: {}", config.home.display()),
        format!(
            "api key: {}",
            if config.api_key.is_some() {
                "present"
            } else {
                "missing"
            }
        ),
    ]
    .join("\n")
}

pub fn render_snapshot(config: &Config) -> Result<String, String> {
    let catalog = Catalog::load(&config.home)?;
    let mut lines = vec![
        "greco tui snapshot".to_string(),
        "==================".to_string(),
        format!("version: {}", env!("CARGO_PKG_VERSION")),
        format!("provider: {} / {}", config.provider, config.model),
        format!("workspace: {}", config.workspace.display()),
        format!("archive: {}", config.home.display()),
        format!(
            "credential: {}",
            config.api_key_source.as_deref().unwrap_or("missing")
        ),
        String::new(),
        "catalog".to_string(),
        "-------".to_string(),
    ];

    if catalog.active.is_empty() {
        lines.push("active skills: 0".to_string());
    } else {
        lines.push(format!("active skills: {}", catalog.active.len()));
        for skill in catalog.active {
            lines.push(format!(
                "- {} {} :: {}",
                skill.id, skill.version, skill.description
            ));
        }
    }

    lines.extend([
        String::new(),
        "next".to_string(),
        "----".to_string(),
        "1. greco validate-skill <path> --json".to_string(),
        "2. greco ask --input \"...\"".to_string(),
        "3. inspect .greco/traces/validation".to_string(),
    ]);

    Ok(lines.join("\n"))
}
