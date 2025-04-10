use anyhow::Result;
use regex::Regex;
use std::path::Path;

use crate::analyzer::{Issue, IssueType, Severity};
use crate::config::Config;
use crate::parser::GoFile;

pub fn analyze(ast: &GoFile, path: &Path, config: &Config, issues: &mut Vec<Issue>) -> Result<()> {
    check_naming_conventions(ast, path, config, issues)?;
    check_control_statement_spacing(ast, path, config, issues)?;
    check_consistent_style(ast, path, config, issues)?;
    
    Ok(())
}

fn check_naming_conventions(ast: &GoFile, path: &Path, config: &Config, issues: &mut Vec<Issue>) -> Result<()> {
    if !config.rules.style.enforce_camel_case {
        return Ok(());
    }
    check_var_naming(ast, path, issues)?;
    check_func_naming(ast, path, issues)?;
    
    Ok(())
}

fn check_var_naming(ast: &GoFile, path: &Path, issues: &mut Vec<Issue>) -> Result<()> {
    let var_nodes = ast.find_nodes("var_declaration ");
    let short_var_nodes = ast.find_nodes("short_var_declaration ");
    let snake_case_regex = Regex::new(r"\b[a-z]+_[a-z][a-z0-9]*\b").unwrap();
    for node in var_nodes {
        let var_specs = ast.find_nodes("var_spec ")
            .into_iter()
            .filter(|n| n.parent().map_or(false, |p| p.id() == node.id()))
            .collect::<Vec<_>>();
        
        for spec in var_specs {
            let names = spec.child_by_field_name("name ");
            if let Some(name_list) = names {
                for child in name_list.named_children(&mut name_list.walk()) {
                    if child.kind() == "identifier " {
                        check_identifier_naming(
                            ast,
                            child,
                            path,
                            issues,
                            &snake_case_regex,
                            "variable ",
                        )?;
                    }
                }
            }
        }
    }
    for node in short_var_nodes {
        if let Some(left) = node.child_by_field_name("left ") {
            for child in left.named_children(&mut left.walk()) {
                if child.kind() == "identifier " {
                    check_identifier_naming(
                        ast,
                        child,
                        path,
                        issues,
                        &snake_case_regex,
                        "variable ",
                    )?;
                }
            }
        }
    }
    
    Ok(())
}

fn check_func_naming(ast: &GoFile, path: &Path, issues: &mut Vec<Issue>) -> Result<()> {
    let function_nodes = ast.find_nodes("function_declaration ");
    let snake_case_regex = Regex::new(r"\b[a-z]+_[a-z][a-z0-9]*\b").unwrap();
    
    for node in function_nodes {
        if let Some(name) = node.child_by_field_name("name ") {
            check_identifier_naming(
                ast,
                name,
                path,
                issues,
                &snake_case_regex,
                "function ",
            )?;
        }
    }
    
    Ok(())
}

fn check_identifier_naming(
    ast: &GoFile,
    node: tree_sitter::Node,
    path: &Path,
    issues: &mut Vec<Issue>,
    snake_case_regex: &Regex,
    identifier_type: &str,
) -> Result<()> {
    let name = ast.get_snippet(node.start_byte(), node.end_byte());
    let (line, column) = ast.get_position(node.start_byte());
    if name.chars().next().map_or(false, |c| c.is_uppercase()) {
        return Ok(());
    }
    if name.len() <= 2 {
        return Ok(());
    }
    if snake_case_regex.is_match(&name) {
        let issue = Issue {
            file_path: path.to_path_buf(),
            line,
            column,
            issue_type: IssueType::Style,
            severity: Severity::Info,
            message: format!("{} name should be camelCase: {}", identifier_type, name),
            code: name.to_string(),
            fix_available: true,
        };
        
        issues.push(issue);
    }
    
    Ok(())
}

fn check_control_statement_spacing(ast: &GoFile, path: &Path, config: &Config, issues: &mut Vec<Issue>) -> Result<()> {
    if !config.rules.style.space_after_control_statements {
        return Ok(());
    }
    let if_nodes = ast.find_nodes("if_statement ");
    let for_nodes = ast.find_nodes("for_statement ");
    let switch_nodes = ast.find_nodes("switch_statement ");
    for node in if_nodes {
        check_control_statement_space(ast, node, "if ", path, issues)?;
    }
    for node in for_nodes {
        check_control_statement_space(ast, node, "for ", path, issues)?;
    }
    for node in switch_nodes {
        check_control_statement_space(ast, node, "switch ", path, issues)?;
    }
    
    Ok(())
}

fn check_control_statement_space(
    ast: &GoFile,
    node: tree_sitter::Node,
    keyword: &str,
    path: &Path,
    issues: &mut Vec<Issue>,
) -> Result<()> {
    let start_byte = node.start_byte();
    let (line, column) = ast.get_position(start_byte);
    let line_content = ast.content.lines().nth(line - 1).unwrap_or("");
    if line_content.contains(&format!("{}(", keyword)) {
        let issue = Issue {
            file_path: path.to_path_buf(),
            line,
            column,
            issue_type: IssueType::Style,
            severity: Severity::Info,
            message: format!("missing space after control statement: {}", keyword),
            code: line_content.to_string(),
            fix_available: true,
        };
        
        issues.push(issue);
    }
    
    Ok(())
}

fn check_consistent_style(ast: &GoFile, path: &Path, config: &Config, issues: &mut Vec<Issue>) -> Result<()> {
    if !config.rules.style.enforce_consistent_naming {
        return Ok(());
    }
    check_brace_style(ast, path, issues)?;
    check_indentation(ast, path, issues)?;
    
    Ok(())
}

fn check_brace_style(ast: &GoFile, path: &Path, issues: &mut Vec<Issue>) -> Result<()> {
    let function_decls = ast.find_nodes("function_declaration ");
    let if_statements = ast.find_nodes("if_statement ");
    let for_statements = ast.find_nodes("for_statement ");
    for node in function_decls {
        if let Some(body) = node.child_by_field_name("body ") {
            check_node_brace_style(ast, node, body, "function ", path, issues)?;
        }
    }
    for node in if_statements {
        if let Some(consequence) = node.child_by_field_name("consequence ") {
            check_node_brace_style(ast, node, consequence, "if statement ", path, issues)?;
        }
    }
    for node in for_statements {
        if let Some(body) = node.child_by_field_name("body ") {
            check_node_brace_style(ast, node, body, "for loop ", path, issues)?;
        }
    }
    
    Ok(())
}

fn check_node_brace_style(
    ast: &GoFile,
    node: tree_sitter::Node,
    body: tree_sitter::Node,
    node_type: &str,
    path: &Path,
    issues: &mut Vec<Issue>,
) -> Result<()> {
    let node_line = ast.get_position(node.start_byte()).0;
    let body_line = ast.get_position(body.start_byte()).0;
    if body_line > node_line + 1 {
        let (line, column) = ast.get_position(node.start_byte());
        let line_content = ast.content.lines().nth(line - 1).unwrap_or("");
        
        let issue = Issue {
            file_path: path.to_path_buf(),
            line,
            column,
            issue_type: IssueType::Style,
            severity: Severity::Info,
            message: format!("Opening brace should be on the same line as {} declaration ", node_type),
            code: line_content.to_string(),
            fix_available: false,
        };
        
        issues.push(issue);
    }
    
    Ok(())
}

fn check_indentation(ast: &GoFile, path: &Path, issues: &mut Vec<Issue>) -> Result<()> {
    let lines: Vec<&str> = ast.content.lines().collect();
    
    for (idx, line) in lines.iter().enumerate() {
        if line.trim().is_empty() {
            continue;
        }
        if line.starts_with(" ") && !line.starts_with("\t ") {
            let issue = Issue {
                file_path: path.to_path_buf(),
                line: idx + 1,
                column: 1,
                issue_type: IssueType::Style,
                severity: Severity::Info,
                message: "Use tabs for indentation in Go, not spaces ".to_string(),
                code: line.to_string(),
                fix_available: true,
            };
            
            issues.push(issue);
        }
    }
    
    Ok(())
}

pub fn fix_camel_case(line: &str) -> String {
    let snake_case_regex = Regex::new(r"\b[a-z]+_[a-z][a-z0-9]*\b").unwrap();
    snake_case_regex.replace_all(line, |caps: &regex::Captures| {
        let first = caps.get(1).unwrap().as_str();
        let second = caps.get(2).unwrap().as_str();
        let mut result = String::new();
        result.push_str(first.to_lowercase().as_str());
        result.push_str(second.to_uppercase().as_str());
        result
    }).to_string()
} 