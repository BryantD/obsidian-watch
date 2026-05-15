use anyhow::{Context, Result};
use serde::Deserialize;
use std::collections::HashMap;
use std::path::{Path, PathBuf};

#[derive(Debug, Deserialize)]
pub struct Config {
    pub directories: HashMap<String, DirectoryConfig>,
}

#[derive(Debug, Deserialize)]
pub struct DirectoryConfig {
    pub path: PathBuf,
    pub command: String,
}

impl Config {
    pub fn from_path(path: &Path) -> Result<Self> {
        let text = std::fs::read_to_string(path)
            .with_context(|| format!("reading config file {}", path.display()))?;
        let config: Config =
            toml::from_str(&text).with_context(|| format!("parsing TOML in {}", path.display()))?;
        Ok(config)
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn parses_minimal_config() {
        let toml = r#"
            [directories.notes]
            path = "/tmp/notes"
            command = "echo {FILE}"
        "#;
        let config: Config = toml::from_str(toml).unwrap();
        let dir = config.directories.get("notes").expect("notes section");
        assert_eq!(dir.path, PathBuf::from("/tmp/notes"));
        assert_eq!(dir.command, "echo {FILE}");
    }

    #[test]
    fn parses_multiple_directories() {
        let toml = r#"
            [directories.notes]
            path = "/tmp/notes"
            command = "a.sh"

            [directories.archive]
            path = "/tmp/archive"
            command = "b.sh"
        "#;
        let config: Config = toml::from_str(toml).unwrap();
        assert_eq!(config.directories.len(), 2);
    }

    #[test]
    fn rejects_missing_path_field() {
        let toml = r#"
            [directories.notes]
            command = "echo {FILE}"
        "#;
        let result: Result<Config, _> = toml::from_str(toml);
        assert!(result.is_err());
    }
}
