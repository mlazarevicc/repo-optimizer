use crate::models::{ClassInfo, CodeMetrics, FunctionInfo};
use crate::parsers::{calculate_cyclomatic_complexity, calculate_nesting_depth, count_lines, LanguageParser};
use tree_sitter::{Parser, Tree};

pub struct RustParser {
    parser: Parser,
}

impl RustParser {
    pub fn new() -> Self {
        let mut parser = Parser::new();
        parser
            .set_language(tree_sitter_rust::language())
            .expect("Failed to load Rust grammar");
        Self { parser }
    }
}

impl LanguageParser for RustParser {
    fn parse_code(&mut self, code: &str) -> Result<Tree, String> {
        self.parser
            .parse(code, None)
            .ok_or_else(|| "Failed to parse Rust code".to_string())
    }

    fn extract_functions(&self, tree: &Tree, code: &str) -> Vec<FunctionInfo> {
        let mut functions = Vec::new();
        let root = tree.root_node();
        let code_bytes = code.as_bytes();

        fn traverse(node: tree_sitter::Node, code_bytes: &[u8], functions: &mut Vec<FunctionInfo>) {
            if node.kind() == "function_item" {
                let name = node
                    .child_by_field_name("name")
                    .and_then(|n| n.utf8_text(code_bytes).ok())
                    .unwrap_or("unknown")
                    .to_string();

                let line_start = node.start_position().row + 1;
                let line_end = node.end_position().row + 1;
                let lines_of_code = line_end - line_start + 1;

                // "parameters" node u Rust-u sadrzi i eventualni "self"/"&self" kao posebno dete
                // (self_parameter), pa named_child_count ovde uracunava i njega - to je OK,
                // dosledno tome kako se ostali jezici tretiraju (uvek racunamo SVE parametre).
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
        // Rust nema klase. Najblizi analog je "impl" blok - grupa metoda vezana za
        // jedan tip - pa njega tretiramo kao "klasu" radi konzistentnosti sa
        // ostalim jezicima (god-class/large-class detektori rade na ClassInfo).
        let mut classes = Vec::new();
        let root = tree.root_node();
        let code_bytes = code.as_bytes();

        fn traverse(node: tree_sitter::Node, code_bytes: &[u8], classes: &mut Vec<ClassInfo>) {
            if node.kind() == "impl_item" {
                let name = node
                    .child_by_field_name("type")
                    .and_then(|n| n.utf8_text(code_bytes).ok())
                    .unwrap_or("unknown")
                    .to_string();

                let line_start = node.start_position().row + 1;
                let line_end = node.end_position().row + 1;
                let lines_of_code = line_end - line_start + 1;

                let mut method_count = 0;
                if let Some(body) = node.child_by_field_name("body") {
                    let mut cursor = body.walk();
                    for child in body.children(&mut cursor) {
                        if child.kind() == "function_item" {
                            method_count += 1;
                        }
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

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn extracts_simple_function() {
        let code = "fn add(a: i32, b: i32) -> i32 {\n    a + b\n}\n";
        let mut parser = RustParser::new();
        let tree = parser.parse_code(code).unwrap();
        let functions = parser.extract_functions(&tree, code);
        assert_eq!(functions.len(), 1);
        assert_eq!(functions[0].name, "add");
        assert_eq!(functions[0].params_count, 2);
    }

    #[test]
    fn self_parameter_is_counted() {
        let code = "struct Foo;\nimpl Foo {\n    fn bar(&self, x: i32) {\n        let _ = x;\n    }\n}\n";
        let mut parser = RustParser::new();
        let tree = parser.parse_code(code).unwrap();
        let functions = parser.extract_functions(&tree, code);
        let bar = functions.iter().find(|f| f.name == "bar").unwrap();
        // &self + x = 2 (dosledno: uvek brojimo SVE parametre, ukljucujuci self)
        assert_eq!(bar.params_count, 2);
    }

    #[test]
    fn impl_block_is_treated_as_class_with_method_count() {
        let code = "struct Foo;\nimpl Foo {\n    fn a(&self) {}\n    fn b(&self) {}\n}\n";
        let mut parser = RustParser::new();
        let tree = parser.parse_code(code).unwrap();
        let classes = parser.extract_classes(&tree, code);
        assert_eq!(classes.len(), 1);
        assert_eq!(classes[0].name, "Foo");
        assert_eq!(classes[0].method_count, 2);
    }

    #[test]
    fn no_impl_block_means_no_classes() {
        let code = "fn standalone() {}\n";
        let mut parser = RustParser::new();
        let tree = parser.parse_code(code).unwrap();
        let classes = parser.extract_classes(&tree, code);
        assert!(classes.is_empty());
    }
}
