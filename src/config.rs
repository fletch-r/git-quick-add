use std::env;
use std::fs;
use std::path::PathBuf;

#[derive(Debug, Clone, Copy, Default)]
pub struct AppConfig {
    pub uppercase: bool,
}

impl AppConfig {
    pub fn load() -> Self {
        let Some(path) = config_path() else {
            return Self::default();
        };

        let Ok(contents) = fs::read_to_string(path) else {
            return Self::default();
        };

        parse_config_contents(&contents)
    }
}

fn config_path() -> Option<PathBuf> {
    if let Ok(config_home) = env::var("XDG_CONFIG_HOME") {
        return Some(PathBuf::from(config_home).join("git-quick-add").join("config.toml"));
    }

    env::var("HOME")
        .ok()
        .map(|home| PathBuf::from(home).join(".config").join("git-quick-add").join("config.toml"))
}

fn parse_config_contents(contents: &str) -> AppConfig {
    let mut config = AppConfig::default();

    for raw_line in contents.lines() {
        let line = raw_line.split('#').next().unwrap_or("").trim();
        if line.is_empty() {
            continue;
        }

        let Some((key, value)) = line.split_once('=') else {
            continue;
        };

        if key.trim() != "uppercase" {
            continue;
        }

        match value.trim() {
            "true" => config.uppercase = true,
            "false" => config.uppercase = false,
            _ => {}
        }
    }

    config
}

#[cfg(test)]
mod tests {
    use super::{AppConfig, parse_config_contents};

    #[test]
    fn defaults_to_false_when_missing() {
        assert!(!parse_config_contents("").uppercase);
    }

    #[test]
    fn parses_uppercase_true() {
        assert!(parse_config_contents("uppercase = true").uppercase);
    }

    #[test]
    fn ignores_comments_and_whitespace() {
        let config = parse_config_contents("  uppercase = true  # enable uppercase");
        assert_eq!(config.uppercase, AppConfig { uppercase: true }.uppercase);
    }
}
