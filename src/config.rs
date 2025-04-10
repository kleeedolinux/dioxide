use anyhow::{Context, Result};
use serde::{Deserialize, Serialize};
use std::fs;
use std::path::{Path, PathBuf};

#[derive(Debug, Serialize, Deserialize, Clone)]
pub struct Config {
    pub general: GeneralConfig,
    pub rules: Rules,
}

#[derive(Debug, Serialize, Deserialize, Clone)]
pub struct GeneralConfig {
    pub ignore_patterns: Vec<String>,
    pub exclude_dirs: Vec<String>,
}

#[derive(Debug, Serialize, Deserialize, Clone)]
pub struct Rules {
    pub syntax: SyntaxRules,
    pub dead_code: DeadCodeRules,
    pub style: StyleRules,
    pub architecture: ArchitectureRules,
}

#[derive(Debug, Serialize, Deserialize, Clone)]
pub struct SyntaxRules {
    pub enabled: bool,
    pub max_line_length: usize,
}

#[derive(Debug, Serialize, Deserialize, Clone)]
pub struct DeadCodeRules {
    pub enabled: bool,
    pub detect_unused_imports: bool,
    pub detect_unused_functions: bool,
    pub detect_unused_variables: bool,
}

#[derive(Debug, Serialize, Deserialize, Clone)]
pub struct StyleRules {
    pub enabled: bool,
    pub enforce_camel_case: bool,
    pub space_after_control_statements: bool,
    pub enforce_consistent_naming: bool,
}

#[derive(Debug, Serialize, Deserialize, Clone)]
pub struct ArchitectureRules {
    pub enabled: bool,
    pub enforce_package_boundaries: bool,
    pub detect_circular_dependencies: bool,
}

impl Default for Config {
    fn default() -> Self {
        Config {
            general: GeneralConfig {
                ignore_patterns: vec![
                    r"^\.git/".to_string(),
                    r"^vendor/".to_string(),
                    r"_test\.go$".to_string(),
                ],
                exclude_dirs: vec![
                    "vendor".to_string(),
                    "node_modules".to_string(),
                    "build".to_string(),
                    "dist".to_string(),
                ],
            },
            rules: Rules {
                syntax: SyntaxRules {
                    enabled: true,
                    max_line_length: 120,
                },
                dead_code: DeadCodeRules {
                    enabled: true,
                    detect_unused_imports: true,
                    detect_unused_functions: true,
                    detect_unused_variables: true,
                },
                style: StyleRules {
                    enabled: true,
                    enforce_camel_case: true,
                    space_after_control_statements: true,
                    enforce_consistent_naming: true,
                },
                architecture: ArchitectureRules {
                    enabled: true,
                    enforce_package_boundaries: true,
                    detect_circular_dependencies: true,
                },
            },
        }
    }
}

pub fn find_default_config() -> PathBuf {
    let cwd = std::env::current_dir().unwrap_or_else(|_| PathBuf::from("."));
    let config_paths = [
        cwd.join("dioxide.toml"),
        cwd.join(".dioxide.toml"),
        cwd.join(".config").join("dioxide.toml"),
    ];

    for path in &config_paths {
        if path.exists() {
            return path.clone();
        }
    }
    config_paths[0].clone()
}

pub fn load_config(path: &Path) -> Result<Config> {
    if !path.exists() {
        return Ok(Config::default());
    }

    let content = fs::read_to_string(path)
        .with_context(|| format!("Failed to read config file: {}", path.display()))?;

    let config = toml::from_str(&content)
        .with_context(|| format!("Failed to parse config file: {}", path.display()))?;

    Ok(config)
}

pub fn create_default_config(path: &Path) -> Result<()> {
    let config = Config::default();
    let content = toml::to_string_pretty(&config)
        .context("Failed to serialize default configuration")?;
    if let Some(parent) = path.parent() {
        if !parent.exists() {
            fs::create_dir_all(parent)
                .with_context(|| format!("Failed to create directory: {}", parent.display()))?;
        }
    }

    fs::write(path, content)
        .with_context(|| format!("Failed to write config file: {}", path.display()))?;

    Ok(())
} 