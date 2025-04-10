use anyhow::Result;
use std::collections::HashMap;
use std::fs;
use std::path::Path;

use crate::analyzer::Issue;
use crate::config::Config;

pub fn apply_fixes(_path: &Path, issues: &[Issue], config: &Config) -> Result<usize> {
    let mut fixed_count = 0;
    let mut modified_files = HashMap::new();
    for issue in issues {
        if !issue.fix_available {
            continue;
        }
        
        let file_path = &issue.file_path;
        if !modified_files.contains_key(file_path) {
            let content = match fs::read_to_string(file_path) {
                Ok(content) => content,
                Err(e) => {
                    eprintln!("Failed to read file for fixing: {}: {}", file_path.display(), e);
                    continue;
                }
            };
            modified_files.insert(file_path.clone(), content);
        }
    }
    for issue in issues {
        if !issue.fix_available {
            continue;
        }
        
        if let Some(file_content) = modified_files.get_mut(&issue.file_path) {
            println!("Attempting to fix: {} in {}", issue.message, issue.file_path.display());
            
            let fixed = match issue.issue_type {
                crate::analyzer::IssueType::Syntax => fix_syntax_issue(issue, file_content, config),
                crate::analyzer::IssueType::DeadCode => fix_dead_code_issue(issue, file_content, config),
                crate::analyzer::IssueType::Style => fix_style_issue(issue, file_content, config),
                crate::analyzer::IssueType::Architecture => false,
            };
            
            if fixed {
                println!("  ✓ Successfully fixed issue ");
                fixed_count += 1;
            } else {
                println!("  ✗ Could not fix issue automatically ");
            }
        }
    }
    for (file_path, content) in modified_files {
        println!("Writing changes to file: {}", file_path.display());
        match fs::write(&file_path, content) {
            Ok(_) => println!("  ✓ Successfully wrote changes "),
            Err(e) => {
                eprintln!("Failed to write fixes to file {}: {}", file_path.display(), e);
                let issue_count_in_file = issues.iter()
                    .filter(|i| i.fix_available && i.file_path == file_path)
                    .count();
                if issue_count_in_file <= fixed_count {
                    fixed_count -= issue_count_in_file;
                }
            }
        }
    }
    
    Ok(fixed_count)
}

fn fix_syntax_issue(issue: &Issue, content: &mut String, _config: &Config) -> bool {
    let lines: Vec<&str> = content.lines().collect();
    
    if issue.line > lines.len() {
        return false;
    }
    
    let line_idx = issue.line - 1;
    let line = lines[line_idx];
    let mut fixed = false;
    let mut fixed_line = line.to_string();
    if issue.message.contains("missing semicolon ") {
        fixed_line.push(';');
        fixed = true;
    } else if issue.message.contains("unmatched parenthesis ") || issue.message.contains("unclosed parenthesis ") {
        fixed_line.push(')');
        fixed = true;
    } else if issue.message.contains("missing closing brace ") || issue.message.contains("unclosed brace ") {
        fixed_line.push('}');
        fixed = true;
    } else if issue.message.contains("missing closing bracket ") || issue.message.contains("unclosed bracket ") {
        fixed_line.push(']');
        fixed = true;
    } else if issue.message.contains("import ") && issue.message.contains("syntax error ") {
        if !fixed_line.contains("\"") && !fixed_line.contains("(") {
            fixed_line = format!("import \"{}\"", fixed_line.trim().trim_start_matches("import ").trim());
            fixed = true;
        } else if fixed_line.contains("\"") && fixed_line.contains("(") && !fixed_line.contains(")") {
            fixed_line.push(')');
            fixed = true;
        }
    }
    if fixed {
        let mut result = String::new();
        if line_idx > 0 {
            result.push_str(&lines[..line_idx].join("\n "));
            result.push_str("\n ");
        }
        result.push_str(&fixed_line);
        if line_idx < lines.len() - 1 {
            result.push_str("\n ");
            result.push_str(&lines[(line_idx + 1)..].join("\n "));
        }
        
        *content = result;
    }
    
    fixed
}

fn fix_dead_code_issue(issue: &Issue, content: &mut String, _config: &Config) -> bool {
    let lines: Vec<&str> = content.lines().collect();
    
    if issue.line > lines.len() {
        return false;
    }
    if issue.message.contains("unused import ") {
        println!("  Fixing unused import: {}", issue.code);
        let import_text = issue.code.trim_matches('"');
        println!("  Import text to remove: \"{}\"", import_text);
        let block_import_regex = regex::Regex::new(r"import\s*\(\s*((?:.|\n)*?)\s*\)").unwrap();
        let _single_import_regex = regex::Regex::new(r#"import\s+"([^"]+)""#).unwrap();
        println!("  Checking for block imports...");
        if let Some(caps) = block_import_regex.captures(content) {
            println!("  Found block imports ");
            let imports_block = caps.get(1).unwrap().as_str();
            let mut import_lines: Vec<&str> = imports_block.lines().collect();
            
            println!("  Original import block:");
            for line in &import_lines {
                println!("    \"{}\"", line);
            }
            let before_count = import_lines.len();
            import_lines.retain(|line| {
                let trimmed = line.trim();
                let contains_import = trimmed.contains(import_text) || trimmed == format!("\"{}\"", import_text);
                let keep = !contains_import || trimmed.starts_with("//");
                if !keep {
                    println!("  Removing line: \"{}\"", line);
                }
                keep
            });
            
            println!("  Import lines after filtering: {}", import_lines.len());
            if import_lines.len() < before_count {
                let new_imports = import_lines.join("\n");
                let replacement = if new_imports.trim().is_empty() {
                    println!("  No imports left, removing entire block ");
                    String::from("")
                } else {
                    println!("  Creating new import block ");
                    format!("import (\n{}\n)", new_imports)
                };
                
                *content = block_import_regex.replace(content, replacement).to_string();
                return true;
            } else {
                println!("  No imports removed from block ");
            }
        } else {
            println!("  No block imports found ");
        }
        println!("  Checking for single line imports...");
        let import_with_package = format!("import \"{}\"", import_text);
        println!("  Looking for: \"{}\"", import_with_package);
        
        if content.contains(&import_with_package) {
            println!("  Found single line import to remove ");
            let mut new_content = String::new();
            let mut removed = false;
            
            for line in content.lines() {
                if line.trim() == import_with_package {
                    println!("  Removing line: \"{}\"", line);
                    removed = true;
                    continue;
                }
                new_content.push_str(line);
                new_content.push('\n');
            }
            
            if removed {
                *content = new_content;
                return true;
            } else {
                println!("  Couldn't remove single line import ");
            }
        } else {
            println!("  No matching single line import found ");
        }
        println!("  Attempting line-by-line search for import...");
        let original_line = lines[issue.line - 1].trim();
        println!("  Original line ({}): \"{}\"", issue.line, original_line);
        
        if original_line.contains(import_text) {
            println!("  Found import in line, removing...");
            let mut result = String::new();
            if issue.line > 1 {
                result.push_str(&lines[..issue.line-1].join("\n "));
                result.push_str("\n ");
            }
            if issue.line < lines.len() {
                result.push_str(&lines[issue.line..].join("\n "));
            }
            
            *content = result;
            return true;
        }
        
        println!("  Could not find and remove the import ");
        return false;
    } else if issue.message.contains("unused variable ") || issue.message.contains("unused function ") {
        let line_idx = issue.line - 1;
        if line_idx < lines.len() {
            let mut result = String::new();
            if line_idx > 0 {
                result.push_str(&lines[..line_idx].join("\n "));
                result.push_str("\n ");
            }
            result.push_str("// Commented out unused code\n");
            result.push_str(lines[line_idx]);
            result.push_str("\n// End of commented code\n");
            if line_idx < lines.len() - 1 {
                result.push_str("\n ");
                result.push_str(&lines[(line_idx + 1)..].join("\n "));
            }
            
            *content = result;
            return true;
        }
    }
    
    false
}

fn fix_style_issue(issue: &Issue, content: &mut String, config: &Config) -> bool {
    let lines: Vec<&str> = content.lines().collect();
    
    if issue.line > lines.len() {
        return false;
    }
    
    let line_idx = issue.line - 1;
    let line = lines[line_idx];
    let mut fixed = false;
    let mut fixed_line = line.to_string();
    if issue.message.contains("line too long ") && config.rules.syntax.max_line_length > 0 {
        let max_len = config.rules.syntax.max_line_length;
        if fixed_line.len() > max_len {
            if let Some(pos) = fixed_line[..max_len].rfind(", ") {
                fixed_line.insert(pos + 1, '\n');
                fixed_line.insert(pos + 2, '\t');
                fixed = true;
            } else if let Some(pos) = fixed_line[..max_len].rfind(" ") {
                fixed_line.insert(pos + 1, '\n');
                fixed_line.insert(pos + 2, '\t');
                fixed = true;
            }
        }
    }
    else if issue.message.contains("missing space after control statement ") && config.rules.style.space_after_control_statements {
        let space_fix_regex = regex::Regex::new(r"(if|for|switch|select)\(").unwrap();
        if space_fix_regex.is_match(&fixed_line) {
            fixed_line = space_fix_regex.replace_all(&fixed_line, "$1 (").to_string();
            fixed = true;
        }
    }
    else if issue.message.contains("should be camelCase ") && config.rules.style.enforce_camel_case {
        let snake_case_regex = regex::Regex::new(r"\b([a-z]+)_([a-z][a-z0-9]*)\b").unwrap();
        if snake_case_regex.is_match(&fixed_line) {
            fixed_line = snake_case_regex.replace_all(&fixed_line, |caps: &regex::Captures| {
                let first = caps.get(1).unwrap().as_str();
                let second = caps.get(2).unwrap().as_str();
                let second_capitalized = second.chars().enumerate()
                    .map(|(i, c)| if i == 0 { c.to_uppercase().next().unwrap() } else { c })
                    .collect::<String>();
                format!("{}{}", first, second_capitalized)
            }).to_string();
            fixed = true;
        }
    }
    else if issue.message.contains("Use tabs for indentation ") {
        let leading_spaces_regex = regex::Regex::new(r"^( +)").unwrap();
        if let Some(captures) = leading_spaces_regex.captures(&fixed_line) {
            if let Some(spaces) = captures.get(1) {
                let num_spaces = spaces.as_str().len();
                let num_tabs = (num_spaces + 3) / 4;
                let tabs = "\t".repeat(num_tabs);
                fixed_line = leading_spaces_regex.replace(&fixed_line, tabs.as_str()).to_string();
                fixed = true;
            }
        }
    }
    if fixed {
        let mut result = String::new();
        if line_idx > 0 {
            result.push_str(&lines[..line_idx].join("\n"));
            result.push_str("\n");
        }
        result.push_str(&fixed_line);
        if line_idx < lines.len() - 1 {
            result.push_str("\n");
            result.push_str(&lines[(line_idx + 1)..].join("\n"));
        }
        
        *content = result;
    }
    
    fixed
} 