use crate::models::{ClassInfo, CodeMetrics, FunctionInfo};
use crate::parsers::{calculate_cyclomatic_complexity, calculate_nesting_depth, count_lines, LanguageParser};
use tree_sitter::{Parser, Tree};

pub struct GoParser {
    parser: Parser,
}

impl GoParser {
    pub fn new() -> Self {
        let mut parser = Parser::new();
        parser
            .set_language(tree_sitter_go::language())
            .expect("Failed to load Go grammar");
        Self { parser }
    }
}

/// Izvlaci receiver tip iz method_declaration cvora.
/// Npr. `func (s *Server) Start()` => vraca "Server".
/// Odvojeno u helper funkciju da bismo izbegli nested closure/borrow
/// probleme sa tree-sitter walker-om (cursor mora da se drop-uje pre
/// nego sto se funkcija vrati).
fn get_receiver_type(node: tree_sitter::Node, code_bytes: &[u8]) -> String {
    let recv = match node.child_by_field_name("receiver") {
        Some(r) => r,
        None => return "unknown".to_string(),
    };

    // Nadjemo parameter_declaration unutar receiver-a (npr. "(s *Server)").
    let mut cur = recv.walk();
    let mut param_decl = None;
    for child in recv.children(&mut cur) {
        if child.kind() == "parameter_declaration" {
            param_decl = Some(child);
            break;
        }
    }
    drop(cur);

    let pd = match param_decl {
        Some(p) => p,
        None => return "unknown".to_string(),
    };

    // Unutar parameter_declaration nadjemo tip: pointer_type (*Server) ili
    // type_identifier (Server ako nema pointera).
    let mut cur2 = pd.walk();
    let mut type_node_opt = None;
    for child in pd.children(&mut cur2) {
        if child.kind() == "pointer_type" || child.kind() == "type_identifier" {
            type_node_opt = Some(child);
            break;
        }
    }
    drop(cur2);

    let type_node = match type_node_opt {
        Some(t) => t,
        None => return "unknown".to_string(),
    };

    // Ako je pointer_type (*Server), uzimamo prvo named_child (Server).
    let actual = if type_node.kind() == "pointer_type" {
        match type_node.named_child(0) {
            Some(c) => c,
            None => type_node,
        }
    } else {
        type_node
    };

    actual.utf8_text(code_bytes).unwrap_or("unknown").to_string()
}

impl LanguageParser for GoParser {
    fn parse_code(&mut self, code: &str) -> Result<Tree, String> {
        self.parser
            .parse(code, None)
            .ok_or_else(|| "Failed to parse Go code".to_string())
    }

    fn extract_functions(&self, tree: &Tree, code: &str) -> Vec<FunctionInfo> {
        let mut functions = Vec::new();
        let root = tree.root_node();
        let code_bytes = code.as_bytes();

        fn traverse(node: tree_sitter::Node, code_bytes: &[u8], functions: &mut Vec<FunctionInfo>) {
            if node.kind() == "function_declaration" || node.kind() == "method_declaration" {
                let name = node
                    .child_by_field_name("name")
                    .and_then(|n| n.utf8_text(code_bytes).ok())
                    .unwrap_or("unknown")
                    .to_string();

                let line_start = node.start_position().row + 1;
                let line_end = node.end_position().row + 1;
                let lines_of_code = line_end - line_start + 1;

                // Racunamo identifier-e unutar "parameters" node-a.
                // Receiver ("(s *Server)") se NE racuna - konzistentno sa
                // Python/Java/Rust gde self/this ne ulaze u params_count.
                let params_count = node
                    .child_by_field_name("parameters")
                    .map(|params| {
                        let mut count = 0;
                        let mut cur = params.walk();
                        for child in params.children(&mut cur) {
                            if child.kind() == "parameter_declaration"
                                || child.kind() == "variadic_parameter_declaration"
                            {
                                let mut cur2 = child.walk();
                                let mut id_count = 0;
                                for inner in child.children(&mut cur2) {
                                    if inner.kind() == "identifier" {
                                        id_count += 1;
                                    }
                                }
                                // Anonimni parametar (samo tip, bez imena) = 1.
                                count += if id_count > 0 { id_count } else { 1 };
                            }
                        }
                        count
                    })
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
        // Go nema klase. Grupicemo method_declaration-e po receiver tipu -
        // to je ekvivalent "klase" u Go-u (slicno impl bloku u Rustu).
        let mut class_map: std::collections::HashMap<String, (usize, usize, usize)> =
            std::collections::HashMap::new();
        let root = tree.root_node();
        let code_bytes = code.as_bytes();

        fn traverse(
            node: tree_sitter::Node,
            code_bytes: &[u8],
            class_map: &mut std::collections::HashMap<String, (usize, usize, usize)>,
        ) {
            if node.kind() == "method_declaration" {
                let receiver_type = get_receiver_type(node, code_bytes);
                let line_start = node.start_position().row + 1;
                let line_end = node.end_position().row + 1;

                let entry = class_map
                    .entry(receiver_type)
                    .or_insert((line_start, line_end, 0));
                if line_start < entry.0 { entry.0 = line_start; }
                if line_end > entry.1 { entry.1 = line_end; }
                entry.2 += 1;
            }

            let mut cursor = node.walk();
            for child in node.children(&mut cursor) {
                traverse(child, code_bytes, class_map);
            }
        }

        traverse(root, code_bytes, &mut class_map);

        class_map
            .into_iter()
            .filter(|(name, _)| name != "unknown")
            .map(|(name, (line_start, line_end, method_count))| ClassInfo {
                name,
                line_start,
                line_end,
                method_count,
                lines_of_code: line_end - line_start + 1,
            })
            .collect()
    }

    fn calculate_metrics(&self, tree: &Tree, code: &str) -> CodeMetrics {
        let (total_lines, code_lines, blank_lines) = count_lines(code);
        let comment_lines = total_lines - code_lines - blank_lines;

        let functions = self.extract_functions(tree, code);
        let classes = self.extract_classes(tree, code);

        let max_nesting_depth = functions.iter().map(|f| f.nesting_depth).max().unwrap_or(0);
        let cyclomatic_complexity = functions.iter().map(|f| f.cyclomatic_complexity).sum();

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
    fn extracts_free_function() {
        let code = "package main\n\nfunc Add(a int, b int) int {\n    return a + b\n}\n";
        let mut parser = GoParser::new();
        let tree = parser.parse_code(code).unwrap();
        let functions = parser.extract_functions(&tree, code);
        assert_eq!(functions.len(), 1);
        assert_eq!(functions[0].name, "Add");
        assert_eq!(functions[0].params_count, 2);
    }

    #[test]
    fn extracts_method_and_groups_by_receiver_type() {
        let code = "package main\n\ntype Server struct{}\n\nfunc (s *Server) Start() error {\n    return nil\n}\n\nfunc (s *Server) Stop() {\n}\n";
        let mut parser = GoParser::new();
        let tree = parser.parse_code(code).unwrap();
        let classes = parser.extract_classes(&tree, code);
        assert_eq!(classes.len(), 1);
        assert_eq!(classes[0].name, "Server");
        assert_eq!(classes[0].method_count, 2);
    }

    #[test]
    fn receiver_not_counted_in_params() {
        let code = "package main\n\ntype S struct{}\n\nfunc (s *S) Greet(name string) string {\n    return name\n}\n";
        let mut parser = GoParser::new();
        let tree = parser.parse_code(code).unwrap();
        let functions = parser.extract_functions(&tree, code);
        let greet = functions.iter().find(|f| f.name == "Greet").unwrap();
        assert_eq!(greet.params_count, 1);
    }

    #[test]
    fn no_classes_without_methods() {
        let code = "package main\n\nfunc standalone() {}\n";
        let mut parser = GoParser::new();
        let tree = parser.parse_code(code).unwrap();
        let classes = parser.extract_classes(&tree, code);
        assert!(classes.is_empty());
    }
}
