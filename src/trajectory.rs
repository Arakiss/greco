use std::{
    path::{Path, PathBuf},
    process,
    time::{SystemTime, UNIX_EPOCH},
};

use serde_json::{Value, json};
use tokio::{fs, io::AsyncWriteExt};

#[derive(Debug, Clone)]
pub struct Trajectory {
    path: PathBuf,
}

impl Trajectory {
    pub async fn start(home: &Path, data: Value) -> Result<Self, String> {
        let directory = home.join("traces").join("sessions");
        fs::create_dir_all(&directory)
            .await
            .map_err(|err| format!("cannot create trace directory: {err}"))?;
        let path = directory.join(format!("{}.jsonl", session_id()));
        let trajectory = Self { path };
        trajectory.record("session_start", data).await?;
        Ok(trajectory)
    }

    pub fn path(&self) -> &Path {
        &self.path
    }

    pub async fn record(&self, event: &str, data: Value) -> Result<(), String> {
        let row = json!({
            "ts_unix_ms": now_millis(),
            "event": event,
            "data": data
        });
        let mut file = fs::OpenOptions::new()
            .create(true)
            .append(true)
            .open(&self.path)
            .await
            .map_err(|err| format!("cannot open trace file: {err}"))?;
        let mut line = serde_json::to_string(&row).map_err(|err| err.to_string())?;
        line.push('\n');
        file.write_all(line.as_bytes())
            .await
            .map_err(|err| format!("cannot write trace row: {err}"))
    }
}

fn session_id() -> String {
    format!("{}-{}", now_nanos(), process::id())
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
