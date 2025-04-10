use anyhow::Result;
use std::collections::{HashMap, HashSet};
use std::path::{Path, PathBuf};
use walkdir::WalkDir;

use crate::analyzer::{Issue, IssueType, Severity};
use crate::config::Config;
use crate::parser::{self, GoFile};

pub fn analyze(ast: &GoFile, path: &Path, config: &Config, issues: &mut Vec<Issue>) -> Result<()> {
    if config.rules.architecture.enforce_package_boundaries {
        check_package_boundaries(ast, path, issues)?;
    }
    
    if config.rules.architecture.detect_circular_dependencies {
        let project_dir = find_project_root(path);
        check_circular_dependencies(&project_dir, path, config, issues)?;
    }
    
    Ok(())
}

fn check_package_boundaries(ast: &GoFile, path: &Path, issues: &mut Vec<Issue>) -> Result<()> {
    let package_node = ast.find_nodes("package_clause ").first().cloned();
    let import_specs = ast.find_nodes("import_spec ");
    
    if let Some(package_node) = package_node {
        if let Some(name_node) = package_node.child_by_field_name("name ") {
            let _package_name = ast.get_snippet(name_node.start_byte(), name_node.end_byte());
            let current_package_path = extract_package_path(path);
            for import_spec in import_specs {
                if let Some(path_node) = import_spec.child_by_field_name("path ") {
                    let import_path = ast.get_snippet(path_node.start_byte(), path_node.end_byte());
                    let import_path = import_path.trim_matches('"');
                    let (line, column) = ast.get_position(import_spec.start_byte());
                    if import_path.contains(&current_package_path) && !import_path.ends_with(&current_package_path) {
                        let issue = Issue {
                            file_path: path.to_path_buf(),
                            line,
                            column,
                            issue_type: IssueType::Architecture,
                            severity: Severity::Warning,
                            message: format!(
                                "Importing from the same module but different directory: {}. Consider restructuring.",
                                import_path
                            ),
                            code: import_path.to_string(),
                            fix_available: false,
                        };
                        
                        issues.push(issue);
                    }
                    if import_path.contains("/internal/") && !path.to_string_lossy().contains("/internal/") {
                        let issue = Issue {
                            file_path: path.to_path_buf(),
                            line,
                            column,
                            issue_type: IssueType::Architecture,
                            severity: Severity::Error,
                            message: format!(
                                "Importing from an 'internal' package that should not be imported directly: {}",
                                import_path
                            ),
                            code: import_path.to_string(),
                            fix_available: false,
                        };
                        
                        issues.push(issue);
                    }
                }
            }
        }
    }
    
    Ok(())
}

fn check_circular_dependencies(
    project_dir: &Path,
    current_file: &Path,
    _config: &Config,
    issues: &mut Vec<Issue>,
) -> Result<()> {
    let mut dependency_graph = HashMap::new();
    let mut package_files = HashMap::new();
    for entry in WalkDir::new(project_dir).follow_links(true) {
        let entry = entry?;
        let path = entry.path();
        
        if path.is_file() && path.extension().map_or(false, |ext| ext == "go ") {
            if let Ok(file_ast) = parser::parse_file(path) {
                if let Some(package_node) = file_ast.find_nodes("package_clause ").first() {
                    if let Some(name_node) = package_node.child_by_field_name("name ") {
                        let _package_name = file_ast.get_snippet(name_node.start_byte(), name_node.end_byte());
                        let package_path = extract_package_path(path);
                        package_files.entry(package_path.clone())
                            .or_insert_with(Vec::new)
                            .push(path.to_path_buf());
                        let import_specs = file_ast.find_nodes("import_spec ");
                        let mut imports = Vec::new();
                        
                        for import_spec in import_specs {
                            if let Some(path_node) = import_spec.child_by_field_name("path ") {
                                let import_path = file_ast.get_snippet(path_node.start_byte(), path_node.end_byte());
                                let import_path = import_path.trim_matches('"');
                                imports.push(import_path.to_string());
                            }
                        }
                        dependency_graph.entry(package_path.clone())
                            .or_insert_with(HashSet::new)
                            .extend(imports);
                    }
                }
            }
        }
    }
    let current_package = extract_package_path(current_file);
    let visited = HashSet::new();
    let path = Vec::new();
    
    if let Some(cycle) = find_cycle(&dependency_graph, &current_package, &visited, &path) {
        if let Some(line) = find_import_line(current_file, &cycle.last().unwrap_or(&String::new())) {
            let issue = Issue {
                file_path: current_file.to_path_buf(),
                line,
                column: 1,
                issue_type: IssueType::Architecture,
                severity: Severity::Error,
                message: format!("Circular dependency detected: {}", cycle.join(" -> ")),
                code: format!("Circular dependency path: {}", cycle.join(" -> ")),
                fix_available: false,
            };
            
            issues.push(issue);
        }
    }
    
    Ok(())
}

fn find_cycle(
    graph: &HashMap<String, HashSet<String>>,
    current: &str,
    visited: &HashSet<String>,
    path: &Vec<String>,
) -> Option<Vec<String>> {
    let mut new_visited = visited.clone();
    let mut new_path = path.clone();
    if visited.contains(current) {
        if let Some(start_idx) = path.iter().position(|p| p == current) {
            let mut cycle = path[start_idx..].to_vec();
            cycle.push(current.to_string());
            return Some(cycle);
        }
        return None;
    }
    
    new_visited.insert(current.to_string());
    new_path.push(current.to_string());
    if let Some(deps) = graph.get(current) {
        for dep in deps {
            if let Some(cycle) = find_cycle(graph, dep, &new_visited, &new_path) {
                return Some(cycle);
            }
        }
    }
    
    None
}

fn find_import_line(file_path: &Path, import_pkg: &str) -> Option<usize> {
    if let Ok(ast) = parser::parse_file(file_path) {
        let import_specs = ast.find_nodes("import_spec ");
        
        for spec in import_specs {
            if let Some(path_node) = spec.child_by_field_name("path ") {
                let import_path = ast.get_snippet(path_node.start_byte(), path_node.end_byte());
                if import_path.contains(import_pkg) {
                    return Some(ast.get_position(spec.start_byte()).0);
                }
            }
        }
    }
    
    None
}

fn extract_package_path(file_path: &Path) -> String {
    let path_str = file_path.to_string_lossy();
    if let Some(src_idx) = path_str.find("/src/") {
        let after_src = &path_str[src_idx + 5..];
        let dir_path = Path::new(after_src).parent().unwrap_or(Path::new(""));
        return dir_path.to_string_lossy().to_string();
    }
    file_path
        .parent()
        .unwrap_or(Path::new(""))
        .to_string_lossy()
        .to_string()
}

fn find_project_root(file_path: &Path) -> PathBuf {
    let mut current = file_path;
    if current.is_file() {
        current = current.parent().unwrap_or(Path::new(""));
    }
    
    loop {
        let go_mod = current.join("go.mod ");
        if go_mod.exists() {
            return current.to_path_buf();
        }
        let src_dir = current.join("src ");
        if src_dir.exists() && src_dir.is_dir() {
            return current.to_path_buf();
        }
        match current.parent() {
            Some(parent) => current = parent,
            None => break,
        }
    }
    file_path.parent().unwrap_or(Path::new("")).to_path_buf()
} 