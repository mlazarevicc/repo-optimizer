use crate::models::{Problem, ProblemFeatures, RankedProblem, ParsedAst};

#[derive(Debug, Clone)]
pub struct MLEngine {
    feature_importance: Vec<&'static str>,
}

impl MLEngine {
    pub fn new() -> Self {
        Self {
            feature_importance: vec![
                "is_security", "severity_base", "loc", "cyclomatic_complexity",
                "nesting_depth", "params_count", "file_size", "position"
            ],
        }
    }

    fn get_problem_weight(&self, problem_type: &str) -> f64 {
        let pt = problem_type.to_lowercase();
        if pt.contains("secret") || pt.contains("password") || pt.contains("credential") { return 0.95; }
        if pt.contains("sql") || pt.contains("injection") { return 0.93; }
        if pt.contains("xss") || pt.contains("cross-site") { return 0.90; }
        if pt.contains("security") || pt.contains("vuln") { return 0.85; }
        if pt.contains("query") || pt.contains("performance") { return 0.75; }
        if pt.contains("loop") || pt.contains("nesting") { return 0.70; }
        if pt.contains("long") || pt.contains("complex") || pt.contains("large") { return 0.60; }
        0.50 // Default weight
    }

    pub fn extract_features(&self, problem: &Problem, ast_data: &ParsedAst) -> ProblemFeatures {
        let severity_base = problem.severity.to_score() as f64 / 100.0;
        let type_weight = self.get_problem_weight(&problem.problem_type);

        let pt = problem.problem_type.to_lowercase();
        let is_security = if pt.contains("security") || pt.contains("injection") || pt.contains("secret") || pt.contains("xss") {
            1.0
        } else {
            0.0
        };

        let mut loc = 0.0;
        let mut cyclomatic_complexity = 0.0;
        let mut nesting_depth = 0.0;
        let mut params_count = 0.0;

        for func in &ast_data.functions {
            if problem.line_start >= func.line_start && problem.line_end <= func.line_end {
                loc = (func.lines_of_code as f64).min(200.0) / 200.0;
                cyclomatic_complexity = (func.cyclomatic_complexity as f64).min(50.0) / 50.0;
                nesting_depth = (func.nesting_depth as f64).min(10.0) / 10.0;
                params_count = (func.params_count as f64).min(10.0) / 10.0;
                break;
            }
        }

        let file_size = (ast_data.metrics.total_lines as f64).min(5000.0) / 5000.0;
        let position = (problem.line_start as f64) / (ast_data.metrics.total_lines.max(1) as f64);

        ProblemFeatures {
            severity_base,
            type_weight,
            loc,
            cyclomatic_complexity,
            nesting_depth,
            params_count,
            file_size,
            is_security,
            position,
        }
    }

    pub fn predict_score(&self, features: &ProblemFeatures) -> f32 {
        let score = (features.is_security * 0.30)
            + (features.severity_base * 0.25)
            + (features.type_weight * 0.20)
            + (features.cyclomatic_complexity * 0.10)
            + (features.nesting_depth * 0.05)
            + (features.loc * 0.05)
            + ((1.0 - features.position) * 0.05);

        score.clamp(0.0, 1.0) as f32
    }

    pub fn get_feature_importance(&self) -> Vec<(&'static str, f64)> {
        vec![
            ("is_security", 0.30),
            ("severity_base", 0.25),
            ("type_weight", 0.20),
            ("cyclomatic_complexity", 0.10),
            ("loc", 0.05),
            ("nesting_depth", 0.05),
            ("position", 0.05),
        ]
    }

    pub fn rank_problems(&self, problems: Vec<Problem>, ast_data: &ParsedAst) -> Vec<RankedProblem> {
        let mut ranked: Vec<_> = problems
            .into_iter()
            .map(|problem| {
                let features = self.extract_features(&problem, ast_data);
                let rank_score = self.predict_score(&features);

                RankedProblem {
                    id: problem.id,
                    problem_type: problem.problem_type.clone(),
                    severity: problem.severity.clone(),
                    rank_score,
                    line_start: problem.line_start,
                    line_end: problem.line_end,
                    message: problem.message,
                    code_snippet: problem.code_snippet,
                    feature_importance: self.get_feature_importance()
                        .iter()
                        .take(5)
                        .map(|(_, v)| *v)
                        .collect(),
                }
            })
            .collect();

        ranked.sort_by(|a, b| b.rank_score.partial_cmp(&a.rank_score).unwrap());
        ranked
    }
}