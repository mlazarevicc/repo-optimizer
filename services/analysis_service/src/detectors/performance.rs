use crate::detectors::Detector;
use crate::models::{ParsedAst, Problem, Severity};
use uuid::Uuid;

pub struct PerformanceDetector;

impl PerformanceDetector {
    pub fn new() -> Self { Self }
}

impl Detector for PerformanceDetector {
    fn detect(&self, data: &ParsedAst) -> Vec<Problem> {
        let mut problems = Vec::new();

        // 1. Nested Loops / High Complexity
        for func in &data.functions {
            if func.cyclomatic_complexity > 15 && func.nesting_depth > 3 {
                problems.push(Problem {
                    id: Uuid::new_v4(),
                    analysis_job_id: data.analysis_job_id,
                    problem_type: "performance.nested_loop".to_string(),
                    severity: Severity::Medium,
                    line_start: func.line_start,
                    line_end: func.line_end,
                    message: format!("Function '{}' has high complexity and deep nesting. Consider optimizing the algorithm.", func.name),
                    code_snippet: data.code.lines()
                        .skip(func.line_start.saturating_sub(1))
                        .take(func.line_end - func.line_start + 1)
                        .collect::<Vec<_>>()
                        .join("\n"),
                    created_at: chrono::Utc::now(),
                    file_path: data.file_path.clone(),
                    language: data.language.to_string(),
                    autofix: None,
                    rank_score: None,
                });
            }
        }

        // 2. Unoptimized Query (N+1) Check
        // Stari "naivni" pristup je samo provericmo da li CEO FAJL sadrzi rec "for" I rec
        // "query"/"select" BILO GDE (bez ikakve veze izmedju njih) - to je davalo lazne
        // pozitive (npr. "select" iz SQL stringa koji nije ni u kakvoj petlji) i, kad ne
        // nadje liniju sa OBE reci istovremeno, padalo na liniju 1 kao default.
        // Sad gledamo da li se DB/ORM poziv stvarno nalazi UNUTAR TELA petlje (telo = sledece
        // linije sa vecom uvucenoscu od "for"/"while" linije - radi tacno za Python, dovoljno
        // dobra aproksimacija i za C-stil jezike posto je kod skoro uvek konzistentno uvucen).
        let query_patterns = [
            "execute(", ".query(", ".filter(", ".get(", "select ", "insert into",
            "update ", "delete from", ".find(", ".findone(", ".fetchone(", ".fetchall(", ".objects.",
        ];

        let lines: Vec<&str> = data.code.lines().collect();
        for (i, line) in lines.iter().enumerate() {
            let trimmed = line.trim_start();
            let indent = line.len() - trimmed.len();
            let lower = trimmed.to_lowercase();
            let is_loop_start = lower.starts_with("for ") || lower.starts_with("for(")
                || lower.starts_with("while ") || lower.starts_with("while(");
            if !is_loop_start {
                continue;
            }

            for (j, body_line) in lines.iter().enumerate().skip(i + 1) {
                if body_line.trim().is_empty() {
                    continue;
                }
                let body_indent = body_line.len() - body_line.trim_start().len();
                if body_indent <= indent {
                    break; // izasli iz tela ove petlje
                }
                let body_lower = body_line.to_lowercase();
                if query_patterns.iter().any(|p| body_lower.contains(p)) {
                    let line_num = j + 1;
                    problems.push(Problem {
                        id: Uuid::new_v4(),
                        analysis_job_id: data.analysis_job_id,
                        problem_type: "performance.unoptimized_query".to_string(),
                        severity: Severity::High,
                        line_start: line_num,
                        line_end: line_num,
                        message: "Potential N+1 query detected in loop. Consider using batch queries or eager loading.".to_string(),
                        code_snippet: body_line.to_string(),
                        created_at: chrono::Utc::now(),
                        file_path: data.file_path.clone(),
                        language: data.language.to_string(),
                        autofix: None,
                        rank_score: None,
                    });
                    break; // jedan nalaz po petlji je dovoljan
                }
            }
        }


        // 3. String concatenation in loop (O(n²))
        // `result += part` kreira novi string na svaku iteraciju petlje.
        // Heuristika: trazi += unutar tela petlje, iskljucuje numericke akumulatore.
        {
            let numeric_re = regex::Regex::new(
                r"(?i)^\s*(i|j|k|n|m|x|y|count|total|sum|num|idx|index|cnt|acc|result_count)\s*\+="
            ).unwrap();

            let lines_str: Vec<&str> = data.code.lines().collect();
            for (i, line) in lines_str.iter().enumerate() {
                let trimmed = line.trim_start();
                let indent = line.len() - trimmed.len();
                let lower = trimmed.to_lowercase();
                let is_loop = lower.starts_with("for ") || lower.starts_with("for(")
                    || lower.starts_with("while ") || lower.starts_with("while(");
                if !is_loop { continue; }

                for (j, body_line) in lines_str.iter().enumerate().skip(i + 1) {
                    if body_line.trim().is_empty() { continue; }
                    let body_indent = body_line.len() - body_line.trim_start().len();
                    if body_indent <= indent { break; }

                    if body_line.contains("+=") && !numeric_re.is_match(body_line) {
                        problems.push(Problem {
                            id: Uuid::new_v4(),
                            analysis_job_id: data.analysis_job_id,
                            problem_type: "performance.string_concat_in_loop".to_string(),
                            severity: Severity::Medium,
                            line_start: j + 1,
                            line_end: j + 1,
                            message: "String concatenation with `+=` inside a loop is O(n²).                                       Collect parts into a list and join at the end.".to_string(),
                            code_snippet: body_line.to_string(),
                            created_at: chrono::Utc::now(),
                            file_path: data.file_path.clone(),
                            language: data.language.to_string(),
                            autofix: None,
                            rank_score: None,
                        });
                        break;
                    }
                }
            }
        }

        problems
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::models::{CodeMetrics, FunctionInfo, Language, ParsedAst};

    fn ast_with(code: &str, functions: Vec<FunctionInfo>) -> ParsedAst {
        ParsedAst {
            analysis_job_id: Uuid::new_v4(),
            language: Language::Python,
            code: code.to_string(),
            metrics: CodeMetrics {
                total_lines: code.lines().count(),
                code_lines: code.lines().count(),
                comment_lines: 0,
                blank_lines: 0,
                total_functions: functions.len(),
                total_classes: 0,
                max_nesting_depth: 0,
                cyclomatic_complexity: 0,
            },
            functions,
            classes: vec![],
            file_path: None,
        }
    }

    #[test]
    fn nested_loop_fires_above_thresholds() {
        let f = FunctionInfo {
            name: "f".to_string(),
            line_start: 1,
            line_end: 10,
            params_count: 1,
            lines_of_code: 10,
            cyclomatic_complexity: 16,
            nesting_depth: 4,
        };
        let ast = ast_with("def f(): pass\n", vec![f]);
        let problems = PerformanceDetector::new().detect(&ast);
        assert!(problems.iter().any(|p| p.problem_type == "performance.nested_loop"));
    }

    #[test]
    fn nested_loop_does_not_fire_below_thresholds() {
        let f = FunctionInfo {
            name: "f".to_string(),
            line_start: 1,
            line_end: 10,
            params_count: 1,
            lines_of_code: 10,
            cyclomatic_complexity: 5,
            nesting_depth: 2,
        };
        let ast = ast_with("def f(): pass\n", vec![f]);
        let problems = PerformanceDetector::new().detect(&ast);
        assert!(!problems.iter().any(|p| p.problem_type == "performance.nested_loop"));
    }

    #[test]
    fn n_plus_one_detected_when_query_is_inside_loop_body() {
        let code = "\
def f(users):
    for user in users:
        cursor.execute(\"SELECT * FROM orders WHERE user_id=\" + str(user.id))
";
        let ast = ast_with(code, vec![]);
        let problems = PerformanceDetector::new().detect(&ast);
        let hit = problems.iter().find(|p| p.problem_type == "performance.unoptimized_query");
        assert!(hit.is_some(), "treba da detektuje query poziv unutar tela petlje");
        assert_eq!(hit.unwrap().line_start, 3, "linija mora pokazivati na STVARNI query poziv");
    }

    // Regresioni test za bug iz produkcije: stari naivni check je gledao da li CEO FAJL
    // sadrzi "for" i "select"/"query" bilo gde, bez veze izmedju njih - SQL string van
    // svake petlje je davao lazan pozitivan nalaz na liniji 1.
    #[test]
    fn no_false_positive_when_select_is_outside_any_loop() {
        let code = "\
import sqlite3

def f(user_id, items):
    query = \"SELECT * FROM users WHERE id = '\" + str(user_id) + \"'\"
    cursor.execute(query)
    for item in items:
        print(item)
";
        let ast = ast_with(code, vec![]);
        let problems = PerformanceDetector::new().detect(&ast);
        assert!(
            problems.iter().all(|p| p.problem_type != "performance.unoptimized_query"),
            "ne treba da prijavi N+1 kad query NIJE unutar tela petlje (regresija na liniju 1 bug)"
        );
    }

    #[test]
    fn clean_code_triggers_nothing() {
        let code = "def add(a, b):\n    return a + b\n";
        let ast = ast_with(code, vec![]);
        let problems = PerformanceDetector::new().detect(&ast);
        assert!(problems.is_empty());
    }

    #[test]
    fn string_concat_in_loop_detected() {
        let code = "def build(items):\n    report = \"\"\n    for item in items:\n        report += str(item)\n    return report\n";
        let ast = ast_with(code, vec![]);
        let problems = PerformanceDetector::new().detect(&ast);
        assert!(problems.iter().any(|p| p.problem_type == "performance.string_concat_in_loop"));
    }

    #[test]
    fn numeric_accumulator_not_flagged() {
        let code = "def total(items):\n    t = 0\n    for item in items:\n        total += item.price\n    return total\n";
        let ast = ast_with(code, vec![]);
        let problems = PerformanceDetector::new().detect(&ast);
        assert!(problems.iter().all(|p| p.problem_type != "performance.string_concat_in_loop"));
    }

}
