use crate::models::{Problem, ProblemFeatures, RankedProblem, ParsedAst};

#[derive(Debug, Clone)]
pub struct MLEngine {
    feature_importance: Vec<(&'static str, f64)>,
}

impl MLEngine {
    pub fn new() -> Self {
        Self {
            feature_importance: vec![
                ("is_security",         0.28),
                ("category_weight",     0.22),
                ("severity_calibrated", 0.20),
                ("file_exposure",       0.10),
                ("cyclomatic",          0.08),
                ("nesting",             0.06),
                ("position",            0.04),
                ("loc",                 0.02),
            ],
        }
    }

    fn category_weight(&self, problem_type: &str) -> f64 {
        let pt = problem_type.to_lowercase();
        if pt.contains("secret") || pt.contains("credential")
            || pt.contains("hardcoded") || pt.contains("password") { return 1.00; }
        if pt.contains("sql") || pt.contains("injection") { return 0.97; }
        if pt.contains("xss") || pt.contains("cross-site") { return 0.93; }
        if pt.contains("insecure_random") { return 0.88; }
        if pt.contains("security") || pt.contains("vuln") { return 0.85; }
        if pt.contains("n_plus_one") || pt.contains("unoptimized_query") { return 0.78; }
        if pt.contains("string_concat_in_loop") || pt.contains("string_concat") { return 0.70; }
        if pt.contains("nested_loop") || pt.contains("loop") { return 0.68; }
        if pt.contains("performance") { return 0.65; }
        if pt.contains("duplicate") || pt.contains("duplication") { return 0.60; }
        if pt.contains("long_method") || pt.contains("complex_method") { return 0.52; }
        if pt.contains("large_class") || pt.contains("long_file") { return 0.50; }
        if pt.contains("deep_nesting") { return 0.48; }
        if pt.contains("long_parameter") { return 0.44; }
        if pt.contains("magic_number") { return 0.28; }
        0.40
    }

    fn file_exposure_multiplier(file_path: Option<&str>) -> f64 {
        let path = match file_path {
            Some(p) => p.to_lowercase(),
            None => return 1.0,
        };
        let file_name = path.split('/').last().unwrap_or(&path);
        let high = ["main", "app", "server", "index", "handler",
                    "controller", "route", "router", "api", "endpoint",
                    "gateway", "auth", "login", "register"];
        let low  = ["test", "spec", "mock", "fixture", "stub", "fake",
                    "generated", "gen_", "_gen", "migration", "seed",
                    "script", "util", "helper", "common"];
        for kw in &high { if file_name.contains(kw) { return 1.25; } }
        for kw in &low  { if path.contains(kw)      { return 0.60; } }
        1.0
    }

    fn severity_calibrated(severity_score: f64) -> f64 {
        if severity_score >= 0.95 {
            0.80 + (severity_score - 0.95) * 2.4
        } else if severity_score >= 0.70 {
            0.58 + (severity_score - 0.70) * 0.56
        } else if severity_score >= 0.45 {
            0.30 + (severity_score - 0.45) * 0.64
        } else {
            0.08 + severity_score * 0.31
        }
    }

    pub fn extract_features(&self, problem: &Problem, ast_data: &ParsedAst) -> ProblemFeatures {
        let severity_raw = problem.severity.to_score() as f64 / 100.0;
        let type_weight  = self.category_weight(&problem.problem_type);
        let pt = problem.problem_type.to_lowercase();
        let is_security  = if pt.contains("security") || pt.contains("injection")
            || pt.contains("secret") || pt.contains("xss") || pt.contains("credential")
            || pt.contains("hardcoded") || pt.contains("insecure_random") { 1.0 } else { 0.0 };

        let mut loc = 0.0;
        let mut cyclomatic_complexity = 0.0;
        let mut nesting_depth = 0.0;
        let mut params_count  = 0.0;

        for func in &ast_data.functions {
            if problem.line_start >= func.line_start && problem.line_end <= func.line_end {
                loc = (func.lines_of_code as f64).min(200.0) / 200.0;
                cyclomatic_complexity = (func.cyclomatic_complexity as f64).min(50.0) / 50.0;
                nesting_depth = (func.nesting_depth as f64).min(10.0) / 10.0;
                params_count  = (func.params_count as f64).min(10.0) / 10.0;
                break;
            }
        }

        let file_size = (ast_data.metrics.total_lines as f64).min(5000.0) / 5000.0;
        let position  = 1.0 - (problem.line_start as f64 / ast_data.metrics.total_lines.max(1) as f64);

        ProblemFeatures { severity_base: severity_raw, type_weight, loc,
                          cyclomatic_complexity, nesting_depth, params_count,
                          file_size, is_security, position }
    }

    pub fn predict_score(&self, features: &ProblemFeatures, file_path: Option<&str>) -> f32 {
        let cat_w    = features.type_weight;
        let sev_cal  = Self::severity_calibrated(features.severity_base);
        let exposure = Self::file_exposure_multiplier(file_path);

        let raw = (features.is_security              * 0.28)
            + (cat_w                                 * 0.22)
            + (sev_cal                               * 0.20)
            + (features.cyclomatic_complexity        * 0.08)
            + (features.nesting_depth                * 0.06)
            + ((1.0 - features.position)             * 0.04)
            + (features.loc                          * 0.02);

        (raw.clamp(0.0, 1.0) * exposure).clamp(0.0, 1.0) as f32
    }

    pub fn get_feature_importance(&self) -> Vec<(&'static str, f64)> {
        self.feature_importance.clone()
    }

    pub fn rank_problems(&self, problems: Vec<Problem>, ast_data: &ParsedAst) -> Vec<RankedProblem> {
        let file_path = ast_data.file_path.as_deref();
        let mut ranked: Vec<_> = problems
            .into_iter()
            .map(|problem| {
                let features   = self.extract_features(&problem, ast_data);
                let rank_score = self.predict_score(&features, file_path);
                RankedProblem {
                    id: problem.id,
                    problem_type: problem.problem_type.clone(),
                    severity: problem.severity.clone(),
                    rank_score,
                    line_start: problem.line_start,
                    line_end:   problem.line_end,
                    message: problem.message,
                    code_snippet: problem.code_snippet,
                    feature_importance: self.get_feature_importance()
                        .iter().map(|(_, v)| *v).collect(),
                }
            })
            .collect();

        ranked.sort_by(|a, b| b.rank_score.partial_cmp(&a.rank_score)
            .unwrap_or(std::cmp::Ordering::Equal));
        ranked
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::models::{Language, ParsedAst, CodeMetrics, Problem, Severity};
    use uuid::Uuid;

    fn empty_ast(file_path: Option<&str>) -> ParsedAst {
        ParsedAst {
            analysis_job_id: Uuid::new_v4(),
            language: Language::Python,
            code: "x = 1\n".to_string(),
            metrics: CodeMetrics { total_lines: 50, code_lines: 40, comment_lines: 5,
                                   blank_lines: 5, total_functions: 0, total_classes: 0,
                                   max_nesting_depth: 0, cyclomatic_complexity: 0 },
            functions: vec![], classes: vec![],
            file_path: file_path.map(|s| s.to_string()),
        }
    }

    fn make_problem(problem_type: &str, severity: Severity) -> Problem {
        Problem { id: Uuid::new_v4(), analysis_job_id: Uuid::new_v4(),
                  problem_type: problem_type.to_string(), severity,
                  line_start: 5, line_end: 5,
                  message: "test".to_string(), code_snippet: "test".to_string(),
                  created_at: chrono::Utc::now() }
    }

    #[test]
    fn hardcoded_secret_scores_highest() {
        let engine = MLEngine::new();
        let ast = empty_ast(Some("main.py"));
        let secret = make_problem("security.hardcoded_secret", Severity::Critical);
        let magic  = make_problem("code_smell.magic_number", Severity::Low);
        let s1 = engine.predict_score(&engine.extract_features(&secret, &ast), Some("main.py"));
        let s2 = engine.predict_score(&engine.extract_features(&magic,  &ast), Some("main.py"));
        assert!(s1 > s2, "hardcoded_secret ({:.3}) > magic_number ({:.3})", s1, s2);
    }

    #[test]
    fn security_outranks_style() {
        let engine = MLEngine::new();
        let ast = empty_ast(None);
        let sec   = make_problem("security.insecure_random", Severity::High);
        let smell = make_problem("code_smell.long_method", Severity::Medium);
        let s1 = engine.predict_score(&engine.extract_features(&sec,   &ast), None);
        let s2 = engine.predict_score(&engine.extract_features(&smell, &ast), None);
        assert!(s1 > s2, "insecure_random ({:.3}) > long_method ({:.3})", s1, s2);
    }

    #[test]
    fn main_file_higher_than_test_file() {
        let engine = MLEngine::new();
        let ast_main = empty_ast(Some("src/main.py"));
        let ast_test = empty_ast(Some("tests/test_utils.py"));
        let prob = make_problem("security.hardcoded_secret", Severity::High);
        let s1 = engine.predict_score(&engine.extract_features(&prob, &ast_main), Some("src/main.py"));
        let s2 = engine.predict_score(&engine.extract_features(&prob, &ast_test), Some("tests/test_utils.py"));
        assert!(s1 > s2, "main ({:.3}) > test ({:.3})", s1, s2);
    }

    #[test]
    fn magic_number_in_helper_is_low() {
        let engine = MLEngine::new();
        let ast  = empty_ast(Some("utils/helper.py"));
        let prob = make_problem("code_smell.magic_number", Severity::Low);
        let score = engine.predict_score(&engine.extract_features(&prob, &ast), Some("utils/helper.py"));
        assert!(score < 0.45, "magic_number/helper score treba biti nizak, dobili {:.3}", score);
    }

    #[test]
    fn scores_in_unit_range() {
        let engine = MLEngine::new();
        let ast = empty_ast(Some("app.py"));
        for (t, s) in [
            ("security.hardcoded_secret", Severity::Critical),
            ("performance.n_plus_one", Severity::High),
            ("code_smell.deep_nesting", Severity::Medium),
            ("code_smell.magic_number", Severity::Low),
        ] {
            let prob  = make_problem(t, s);
            let score = engine.predict_score(&engine.extract_features(&prob, &ast), Some("app.py"));
            assert!(score >= 0.0 && score <= 1.0, "{} score={:.3}", t, score);
        }
    }

    #[test]
    fn category_hierarchy() {
        let engine = MLEngine::new();
        assert!(engine.category_weight("security.hardcoded_secret")  > engine.category_weight("security.sql_injection"));
        assert!(engine.category_weight("security.sql_injection")      > engine.category_weight("security.insecure_random"));
        assert!(engine.category_weight("security.insecure_random")    > engine.category_weight("performance.n_plus_one"));
        assert!(engine.category_weight("performance.n_plus_one")      > engine.category_weight("code_smell.duplicate_code"));
        assert!(engine.category_weight("code_smell.duplicate_code")   > engine.category_weight("code_smell.magic_number"));
    }

    #[test]
    fn severity_calibration_bands() {
        let c = MLEngine::severity_calibrated(1.00);
        let h = MLEngine::severity_calibrated(0.75);
        let m = MLEngine::severity_calibrated(0.50);
        let l = MLEngine::severity_calibrated(0.25);
        assert!(c >= 0.80,           "Critical: {:.3}", c);
        assert!(h >= 0.58 && h < 0.80, "High: {:.3}", h);
        assert!(m >= 0.30 && m < 0.58, "Medium: {:.3}", m);
        assert!(l >= 0.08 && l < 0.30, "Low: {:.3}", l);
        assert!(c > h && h > m && m > l, "monotonicnost");
    }
}
