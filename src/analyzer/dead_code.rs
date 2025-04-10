use anyhow::Result;
use std::collections::{HashMap, HashSet};
use std::path::Path;

use crate::analyzer::{Issue, IssueType, Severity};
use crate::config::Config;
use crate::parser::GoFile;

pub fn analyze(ast: &GoFile, path: &Path, config: &Config, issues: &mut Vec<Issue>) -> Result<()> {
    if config.rules.dead_code.detect_unused_imports {
        check_unused_imports(ast, path, issues)?;
    }
    
    if config.rules.dead_code.detect_unused_functions {
        check_unused_functions(ast, path, issues)?;
    }
    
    if config.rules.dead_code.detect_unused_variables {
        check_unused_variables(ast, path, issues)?;
    }
    
    Ok(())
}

fn check_unused_imports(ast: &GoFile, path: &Path, issues: &mut Vec<Issue>) -> Result<()> {
    let import_nodes = ast.find_nodes("import_spec ");
    let mut imports = HashMap::new();
    for node in &import_nodes {
        if let Some(path_node) = node.child_by_field_name("path ") {
            let import_path = ast.get_snippet(path_node.start_byte(), path_node.end_byte());
            let (line, column) = ast.get_position(node.start_byte());
            let import_alias = if let Some(name_node) = node.child_by_field_name("name ") {
                Some(ast.get_snippet(name_node.start_byte(), name_node.end_byte()))
            } else {
                None
            };
            if import_alias.as_ref().map_or(false, |a| a == "_") {
                continue;
            }
            if import_alias.as_ref().map_or(false, |a| a == ".") {
                continue;
            }
            let package_name = extract_package_name(&import_path);
            
            imports.insert(
                package_name.clone(),
                (import_path.clone(), line, column, import_alias.clone()),
            );
        }
    }
    let mut used_imports = HashSet::new();
    let selector_nodes = ast.find_nodes("selector_expression ");
    for node in selector_nodes {
        if let Some(operand) = node.child_by_field_name("operand ") {
            let package = ast.get_snippet(operand.start_byte(), operand.end_byte());
            used_imports.insert(package.trim().to_string());
        }
    }
    let type_nodes = ast.find_nodes("type_identifier ");
    for node in type_nodes {
        if let Some(package) = node.child_by_field_name("package ") {
            let pkg_name = ast.get_snippet(package.start_byte(), package.end_byte());
            used_imports.insert(pkg_name.trim().to_string());
        }
    }
    let qualified_nodes = ast.find_nodes("qualified_type ");
    for node in qualified_nodes {
        if let Some(package) = node.child_by_field_name("package ") {
            let pkg_name = ast.get_snippet(package.start_byte(), package.end_byte());
            used_imports.insert(pkg_name.trim().to_string());
        }
    }
    let type_assertion_nodes = ast.find_nodes("type_assertion_expression ");
    for node in type_assertion_nodes {
        if let Some(type_node) = node.child_by_field_name("type ") {
            if type_node.kind() == "qualified_type " {
                if let Some(package) = type_node.child_by_field_name("package ") {
                    let pkg_name = ast.get_snippet(package.start_byte(), package.end_byte());
                    used_imports.insert(pkg_name.trim().to_string());
                }
            }
        }
    }
    for (package, (import_path, line, column, alias)) in imports {
        let is_used = if let Some(alias_val) = alias {
            used_imports.contains(&alias_val.trim_matches('"').to_string())
        } else {
            used_imports.contains(&package.trim_matches('"').to_string())
        };
        
        if !is_used {
            let issue = Issue {
                file_path: path.to_path_buf(),
                line,
                column,
                issue_type: IssueType::DeadCode,
                severity: Severity::Warning,
                message: format!("Unused import: {}", import_path),
                code: import_path,
                fix_available: true,
            };
            
            issues.push(issue);
        }
    }
    
    Ok(())
}

fn check_unused_functions(ast: &GoFile, path: &Path, issues: &mut Vec<Issue>) -> Result<()> {
    let function_nodes = ast.find_nodes("function_declaration ");
    let mut functions = HashMap::new();
    for node in &function_nodes {
        if let Some(name_node) = node.child_by_field_name("name ") {
            let func_name = ast.get_snippet(name_node.start_byte(), name_node.end_byte());
            let (line, column) = ast.get_position(node.start_byte());
            if func_name == "main " || func_name == "init " {
                continue;
            }
            if func_name.chars().next().map_or(false, |c| c.is_uppercase()) {
                continue;
            }
            
            functions.insert(func_name.clone(), (line, column));
        }
    }
    let call_nodes = ast.find_nodes("call_expression ");
    let mut used_functions = HashSet::new();
    
    for node in call_nodes {
        if let Some(function_node) = node.child_by_field_name("function ") {
            if function_node.kind() == "identifier " {
                let func_name = ast.get_snippet(function_node.start_byte(), function_node.end_byte());
                used_functions.insert(func_name);
            }
        }
    }
    for (func_name, (line, column)) in functions {
        if !used_functions.contains(&func_name) {
            let (start, end) = find_function_range(ast, &func_name);
            let func_snippet = if start < end {
                ast.get_snippet(start, end)
            } else {
                func_name.clone()
            };
            let issue = Issue {
                file_path: path.to_path_buf(),
                line,
                column,
                issue_type: IssueType::DeadCode,
                severity: Severity::Warning,
                message: format!("Unused function: {}", func_name),
                code: if func_snippet.len() > 100 {
                    func_name.clone()
                } else {
                    func_snippet
                },
                fix_available: true,
            };
            
            issues.push(issue);
        }
    }
    
    Ok(())
}

fn check_unused_variables(ast: &GoFile, path: &Path, issues: &mut Vec<Issue>) -> Result<()> {
    let var_nodes = ast.find_nodes("var_declaration ");
    let short_var_nodes = ast.find_nodes("short_var_declaration ");
    let mut variables = HashMap::new();
    for node in &var_nodes {
        let var_specs = ast.find_nodes("var_spec ")
            .into_iter()
            .filter(|n| n.parent().map_or(false, |p| p.id() == node.id()))
            .collect::<Vec<_>>();
        
        for spec in var_specs {
            let names = spec.child_by_field_name("name ");
            if let Some(name_list) = names {
                for child in name_list.named_children(&mut name_list.walk()) {
                    if child.kind() == "identifier " {
                        let var_name = ast.get_snippet(child.start_byte(), child.end_byte());
                        let (line, column) = ast.get_position(child.start_byte());
                        if var_name != "_" {
                            variables.insert(var_name.clone(), (line, column));
                        }
                    }
                }
            }
        }
    }
    for node in &short_var_nodes {
        if let Some(left_node) = node.child_by_field_name("left ") {
            for child in left_node.named_children(&mut left_node.walk()) {
                if child.kind() == "identifier " {
                    let var_name = ast.get_snippet(child.start_byte(), child.end_byte());
                    let (line, column) = ast.get_position(child.start_byte());
                    if var_name != "_" {
                        variables.insert(var_name.clone(), (line, column));
                    }
                }
            }
        }
    }
    let expr_nodes = ast.find_nodes("identifier ");
    let mut used_vars = HashSet::new();
    
    for node in expr_nodes {
        let parent = node.parent();
        if let Some(parent_node) = parent {
            let parent_type = parent_node.kind();
            if parent_type == "var_spec " && parent_node.child_by_field_name("name ") == Some(node) {
                continue;
            }
            if parent_type == "short_var_declaration " && parent_node.child_by_field_name("left ") == Some(node) {
                continue;
            }
        }
        
        let var_name = ast.get_snippet(node.start_byte(), node.end_byte());
        used_vars.insert(var_name);
    }
    for (var_name, (line, column)) in variables {
        if !used_vars.contains(&var_name) {
            let issue = Issue {
                file_path: path.to_path_buf(),
                line,
                column,
                issue_type: IssueType::DeadCode,
                severity: Severity::Warning,
                message: format!("Unused variable: {}", var_name),
                code: var_name,
                fix_available: true,
            };
            
            issues.push(issue);
        }
    }
    
    Ok(())
}

fn extract_package_name(import_path: &str) -> String {
    let path = import_path.trim_matches('"');
    let parts: Vec<&str> = path.split('/').collect();
    parts.last().unwrap_or(&"").to_string()
}

fn find_function_range(ast: &GoFile, func_name: &str) -> (usize, usize) {
    let function_nodes = ast.find_nodes("function_declaration ");
    
    for node in function_nodes {
        if let Some(name_node) = node.child_by_field_name("name ") {
            let name = ast.get_snippet(name_node.start_byte(), name_node.end_byte());
            if name == func_name {
                return (node.start_byte(), node.end_byte());
            }
        }
    }
    
    (0, 0)
} 