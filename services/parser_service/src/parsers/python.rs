use crate::models::{ClassInfo, CodeMetrics, FunctionInfo};
use crate::parsers::{calculate_cyclomatic_complexity, calculate_nesting_depth, count_lines, LanguageParser};
use tree_sitter::{Parser, Tree};

pub struct PythonParser {
    parser: Parser,
}

impl PythonParser {
    pub fn new() -> Self {
        let mut parser = Parser::new();
        parser
            .set_language(tree_sitter_python::language())
            .expect("Failed to load Python grammar");
        Self { parser }
    }
}

impl LanguageParser for PythonParser {
    fn parse_code(&mut self, code: &str) -> Result<Tree, String> {
        self.parser
            .parse(code, None)
            .ok_or_else(|| "Failed to parse Python code".to_string())
    }

    fn extract_functions(&self, tree: &Tree, code: &str) -> Vec<FunctionInfo> {
        let mut functions = Vec::new();
        let root = tree.root_node();
        let code_bytes = code.as_bytes();

        fn traverse(node: tree_sitter::Node, code_bytes: &[u8], functions: &mut Vec<FunctionInfo>) {
            if node.kind() == "function_definition" {
                let name = node
                    .child_by_field_name("name")
                    .and_then(|n| n.utf8_text(code_bytes).ok())
                    .unwrap_or("unknown")
                    .to_string();

                let line_start = node.start_position().row + 1;
                let line_end = node.end_position().row + 1;
                let lines_of_code = line_end - line_start + 1;

                // Count parameters
                let params_count = node
                    .child_by_field_name("parameters")
                    .map(|params| params.named_child_count())
                    .unwrap_or(0);

                let cyclomatic_complexity = calculate_cyclomatic_complexity(&node);
                let nesting_depth = calculate_nesting_depth(&node);

                functions.push(FunctionInfo {
                    name,
                    line_start,
                    line_end,
                    params_count,
                    lines_of_code,
                    cyclomatic_complexity,
                    nesting_depth,
                });
            }

            let mut cursor = node.walk();
            for child in node.children(&mut cursor) {
                traverse(child, code_bytes, functions);
            }
        }

        traverse(root, code_bytes, &mut functions);
        functions
    }

    fn extract_classes(&self, tree: &Tree, code: &str) -> Vec<ClassInfo> {
        let mut classes = Vec::new();
        let root = tree.root_node();
        let code_bytes = code.as_bytes();

        fn traverse(node: tree_sitter::Node, code_bytes: &[u8], classes: &mut Vec<ClassInfo>) {
            if node.kind() == "class_definition" {
                let name = node
                    .child_by_field_name("name")
                    .and_then(|n| n.utf8_text(code_bytes).ok())
                    .unwrap_or("unknown")
                    .to_string();

                let line_start = node.start_position().row + 1;
                let line_end = node.end_position().row + 1;
                let lines_of_code = line_end - line_start + 1;

                // Count methods
                let mut method_count = 0;
                let mut cursor = node.walk();
                for child in node.children(&mut cursor) {
                    if child.kind() == "function_definition" {
                        method_count += 1;
                    }
                }

                classes.push(ClassInfo {
                    name,
                    line_start,
                    line_end,
                    method_count,
                    lines_of_code,
                });
            }

            let mut cursor = node.walk();
            for child in node.children(&mut cursor) {
                traverse(child, code_bytes, classes);
            }
        }

        traverse(root, code_bytes, &mut classes);
        classes
    }

    fn calculate_metrics(&self, tree: &Tree, code: &str) -> CodeMetrics {
        let (total_lines, code_lines, blank_lines) = count_lines(code);
        let comment_lines = total_lines - code_lines - blank_lines;

        let functions = self.extract_functions(tree, code);
        let classes = self.extract_classes(tree, code);

        let max_nesting_depth = functions
            .iter()
            .map(|f| f.nesting_depth)
            .max()
            .unwrap_or(0);

        let cyclomatic_complexity = functions
            .iter()
            .map(|f| f.cyclomatic_complexity)
            .sum();

        CodeMetrics {
            total_lines,
            code_lines,
            comment_lines,
            blank_lines,
            total_functions: functions.len(),
            total_classes: classes.len(),
            max_nesting_depth,
            cyclomatic_complexity,
        }
    }
}
