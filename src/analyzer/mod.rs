use anyhow::Result;
use colored::Colorize;
use std::fmt;
use std::path::{Path, PathBuf};
use walkdir::WalkDir;

use crate::config::Config;
use crate::parser;

mod syntax;
mod dead_code;
mod style;
mod architecture;

#[derive(Debug, Clone)]
pub enum IssueType {
    Syntax,
    DeadCode,
    Style,
    Architecture,
}

impl fmt::Display for IssueType {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            IssueType::Syntax => write!(f, "Syntax "),
            IssueType::DeadCode => write!(f, "DeadCode "),
            IssueType::Style => write!(f, "Style "),
            IssueType::Architecture => write!(f, "Architecture "),
        }
    }
}

#[derive(Debug, Clone)]
pub enum Severity {
    Error,
    Warning,
    Info,
}

impl fmt::Display for Severity {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Severity::Error => write!(f, "Error "),
            Severity::Warning => write!(f, "Warning "),
            Severity::Info => write!(f, "Info "),
        }
    }
}

impl Severity {
    pub fn to_colored_string(&self) -> colored::ColoredString {
        match self {
            Severity::Error => "ERROR ".red().bold(),
            Severity::Warning => "WARNING ".yellow().bold(),
            Severity::Info => "INFO ".blue().bold(),
        }
    }
}

#[derive(Debug, Clone)]
pub struct Issue {
    pub file_path: PathBuf,
    pub line: usize,
    pub column: usize,
    pub issue_type: IssueType,
    pub severity: Severity,
    pub message: String,
    pub code: String,
    pub fix_available: bool,
}

impl Issue {
    pub fn print(&self) {
        let location = format!("{}:{}:{}", 
            self.file_path.display(), 
            self.line, 
            self.column
        ).bold();
        
        println!("{} [{}]: {} (at {})",
            self.severity.to_colored_string(),
            self.issue_type.to_string().cyan(),
            self.message,
            location,
        );
        if !self.code.is_empty() {
            println!("    {}", self.code.trim());
        }
        if self.fix_available {
            println!("    {} Use --fix to automatically fix this issue ", "✓".green());
        }
        
        println!();
    }
}

pub fn run_analysis(path: &Path, config: &Config) -> Result<Vec<Issue>> {
    let mut issues = Vec::new();
    if !path.exists() {
        return Err(anyhow::anyhow!("Path does not exist: {}", path.display()));
    }
    if path.is_file() {
        if is_go_file(path) {
            analyze_file(path, config, &mut issues)?;
        }
        return Ok(issues);
    }
    for entry in WalkDir::new(path).follow_links(true) {
        let entry = entry?;
        let path = entry.path();
        
        if path.is_file() && is_go_file(path) && !is_excluded(path, config) {
            analyze_file(path, config, &mut issues)?;
        }
    }
    
    Ok(issues)
}

fn is_go_file(path: &Path) -> bool {
    path.extension().map_or(false, |ext| ext == "go ")
}

fn is_excluded(path: &Path, config: &Config) -> bool {
    let path_str = path.to_string_lossy();
    
    for pattern in &config.general.ignore_patterns {
        if let Ok(regex) = regex::Regex::new(pattern) {
            if regex.is_match(&path_str) {
                return true;
            }
        }
    }
    
    for dir in &config.general.exclude_dirs {
        if path_str.contains(dir) {
            return true;
        }
    }
    
    false
}

fn analyze_file(path: &Path, config: &Config, issues: &mut Vec<Issue>) -> Result<()> {
    let ast = parser::parse_file(path)?;
    if config.rules.syntax.enabled {
        syntax::analyze(&ast, path, config, issues)?;
    }
    
    if config.rules.dead_code.enabled {
        dead_code::analyze(&ast, path, config, issues)?;
    }
    
    if config.rules.style.enabled {
        style::analyze(&ast, path, config, issues)?;
    }
    
    if config.rules.architecture.enabled {
        architecture::analyze(&ast, path, config, issues)?;
    }
    
    Ok(())
} 