pub mod python;
pub mod javascript;

use crate::models::{ClassInfo, CodeMetrics, FunctionInfo, Language};
use tree_sitter::{Parser, Tree};

pub trait LanguageParser {
    fn parse_code(&mut self, code: &str) -> Result<Tree, String>;
    fn extract_functions(&self, tree: &Tree, code: &str) -> Vec<FunctionInfo>;
    fn extract_classes(&self, tree: &Tree, code: &str) -> Vec<ClassInfo>;
    fn calculate_metrics(&self, tree: &Tree, code: &str) -> CodeMetrics;
}

pub fn get_parser(language: &Language) -> Box<dyn LanguageParser + Send> {
    match language {
        Language::Python => Box::new(python::PythonParser::new()),
        Language::JavaScript | Language::TypeScript => Box::new(javascript::JavaScriptParser::new()),
        _ => Box::new(python::PythonParser::new()),
    }
}

pub fn count_lines(code: &str) -> (usize, usize, usize) {
    let total = code.lines().count();
    let mut blank = 0;
    let mut comments = 0;

    for line in code.lines() {
        let trimmed = line.trim();
        if trimmed.is_empty() {
            blank += 1;
        } else if trimmed.starts_with("//") || trimmed.starts_with('#') {
            comments += 1;
        }
    }

    let code_lines = total - blank - comments;
    (total, code_lines, blank)
}

pub fn calculate_nesting_depth(node: &tree_sitter::Node, code: &str) -> usize {
    let mut max_depth = 0;
    let mut current_depth = 0;

    fn traverse(node: &tree_sitter::Node, depth: usize, max: &mut usize) {
        if depth > *max {
            *max = depth;
        }

        let kind = node.kind();
        let new_depth = if kind.contains("statement") 
            || kind.contains("block") 
            || kind.contains("if") 
            || kind.contains("for") 
            || kind.contains("while") {
            depth + 1
        } else {
            depth
        };

        let mut cursor = node.walk();
        for child in node.children(&mut cursor) {
            traverse(&child, new_depth, max);
        }
    }

    traverse(node, 0, &mut max_depth);
    max_depth
}

pub fn calculate_cyclomatic_complexity(node: &tree_sitter::Node) -> usize {
    let mut complexity = 1; // Base complexity

    fn traverse(node: &tree_sitter::Node, complexity: &mut usize) {
        let kind = node.kind();
        
        // Increment for decision points
        if kind == "if_statement" 
            || kind == "for_statement"
            || kind == "while_statement"
            || kind == "case"
            || kind == "catch_clause"
            || kind == "conditional_expression"
            || kind == "binary_operator" && (node.kind() == "&&" || node.kind() == "||") {
            *complexity += 1;
        }

        let mut cursor = node.walk();
        for child in node.children(&mut cursor) {
            traverse(&child, complexity);
        }
    }

    traverse(node, &mut complexity);
    complexity
}
