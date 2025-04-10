use anyhow::{Context, Result};
use std::fs;
use std::path::Path;
use tree_sitter::{Parser, Tree};
pub struct GoFile {
    pub path: std::path::PathBuf,
    pub content: String,
    pub tree: Tree,
}

impl GoFile {
    pub fn get_position(&self, byte_offset: usize) -> (usize, usize) {
        let mut line = 1;
        let mut col = 1;
        
        for (i, c) in self.content.char_indices() {
            if i >= byte_offset {
                break;
            }
            
            if c == '\n' {
                line += 1;
                col = 1;
            } else {
                col += 1;
            }
        }
        
        (line, col)
    }
    pub fn get_snippet(&self, start_byte: usize, end_byte: usize) -> String {
        if start_byte >= self.content.len() || end_byte > self.content.len() {
            return String::new();
        }
        
        self.content[start_byte..end_byte].to_string()
    }
    pub fn find_nodes(&self, node_type: &str) -> Vec<tree_sitter::Node> {
        let mut cursor = tree_sitter::QueryCursor::new();
        let query = tree_sitter::Query::new(
            tree_sitter_go::language(),
            &format!("({}) @node ", node_type),
        ).unwrap_or_else(|_| tree_sitter::Query::new(tree_sitter_go::language(), "").unwrap());
        
        let matches = cursor.matches(&query, self.tree.root_node(), self.content.as_bytes());
        matches.map(|m| m.captures[0].node).collect()
    }
}
pub fn init_parser() -> Result<Parser> {
    let mut parser = Parser::new();
    parser.set_language(tree_sitter_go::language())
        .context("Failed to load Go grammar ")?;
    
    Ok(parser)
}
pub fn parse_file(path: &Path) -> Result<GoFile> {
    let content = fs::read_to_string(path)
        .with_context(|| format!("Failed to read file: {}", path.display()))?;
    
    let mut parser = init_parser()?;
    let tree = parser.parse(&content, None)
        .context("Failed to parse Go file ")?;
    
    Ok(GoFile {
        path: path.to_path_buf(),
        content,
        tree,
    })
} 