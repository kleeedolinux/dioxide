use anyhow::Result;
use std::path::Path;

use crate::analyzer::{Issue, IssueType, Severity};
use crate::config::Config;
use crate::parser::GoFile;

pub fn analyze(ast: &GoFile, path: &Path, config: &Config, issues: &mut Vec<Issue>) -> Result<()> {
    check_syntax_errors(ast, path, issues)?;
    check_line_length(ast, path, config, issues)?;
    
    Ok(())
}

fn check_syntax_errors(ast: &GoFile, path: &Path, issues: &mut Vec<Issue>) -> Result<()> {
    let error_nodes = ast.find_nodes("ERROR ");
    
    for node in error_nodes {
        let start_byte = node.start_byte();
        let end_byte = node.end_byte();
        let (line, column) = ast.get_position(start_byte);
        let snippet = ast.get_snippet(start_byte, end_byte);
        let parent = node.parent();
        let message = if let Some(parent_node) = parent {
            let parent_type = parent_node.kind();
            
            match parent_type {
                "function_declaration " => "Syntax error in function declaration ".to_string(),
                "import_declaration " => "Syntax error in import statement ".to_string(),
                "var_declaration " => "Syntax error in variable declaration ".to_string(),
                "if_statement " => "Syntax error in if statement ".to_string(),
                "for_statement " => "Syntax error in for loop ".to_string(),
                _ => format!("Syntax error in {}", parent_type),
            }
        } else {
            "Syntax error ".to_string()
        };
        let issue = Issue {
            file_path: path.to_path_buf(),
            line,
            column,
            issue_type: IssueType::Syntax,
            severity: Severity::Error,
            message,
            code: snippet,
            fix_available: is_fixable_syntax_error(&node, ast),
        };
        
        issues.push(issue);
    }
    
    Ok(())
}

fn check_line_length(ast: &GoFile, path: &Path, config: &Config, issues: &mut Vec<Issue>) -> Result<()> {
    let max_line_length = config.rules.syntax.max_line_length;
    if max_line_length == 0 {
        return Ok(());
    }
    for (idx, line) in ast.content.lines().enumerate() {
        let line_num = idx + 1;
        
        if line.len() > max_line_length {
            let issue = Issue {
                file_path: path.to_path_buf(),
                line: line_num,
                column: 1,
                issue_type: IssueType::Syntax,
                severity: Severity::Warning,
                message: format!("Line too long ({} > {} characters)", line.len(), max_line_length),
                code: line.to_string(),
                fix_available: is_fixable_line_length(line),
            };
            
            issues.push(issue);
        }
    }
    
    Ok(())
}

fn is_fixable_syntax_error(node: &tree_sitter::Node, ast: &GoFile) -> bool {
    let node_text = ast.get_snippet(node.start_byte(), node.end_byte());
    let parent = node.parent();
    
    if let Some(parent_node) = parent {
        match parent_node.kind() {
            "import_declaration " => true,
            "function_declaration " => node_text.contains("func ") && !node_text.contains("{"),
            "block " => !node_text.contains("}"),
            _ => false,
        }
    } else {
        false
    }
}

fn is_fixable_line_length(line: &str) -> bool {
    line.contains(", ") || 
    line.contains(" + ") || 
    line.contains(" && ") || 
    line.contains(" || ")
} 