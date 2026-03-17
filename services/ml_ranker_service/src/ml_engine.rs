use crate::models::{Problem, ProblemFeatures, RankedProblem, ProblemType, ParsedAst};
use std::collections::HashMap;

#[derive(Debug, Clone)]
pub struct MLEngine {
    problem_weights: HashMap<ProblemType, f64>,
    feature_importance: Vec<&'static str>,
}

impl MLEngine {
    pub fn new() -> Self {
        let mut weights = HashMap::new();
        weights.insert(ProblemType::HardcodedSecret, 0.95);
        weights.insert(ProblemType::SqlInjection, 0.93);
        weights.insert(ProblemType::XssVulnerability, 0.90);
        weights.insert(ProblemType::UnoptimizedQuery, 0.75);
        weights.insert(ProblemType::NestedLoop, 0.70);
        weights.insert(ProblemType::LongMethod, 0.65);
        weights.insert(ProblemType::ComplexMethod, 0.60);
        weights.insert(ProblemType::DeepNesting, 0.55);
        weights.insert(ProblemType::LongParameterList, 0.50);
        weights.insert(ProblemType::LargeClass, 0.45);
        weights.insert(ProblemType::LongFile, 0.40);

        Self {
            problem_weights: weights,
            feature_importance: vec![
                "is_security", "severity_base", "loc", "cyclomatic_complexity",
                "nesting_depth", "params_count", "file_size", "position"
            ],
        }
    }

    pub fn extract_features(&self, problem: &Problem, ast_data: &ParsedAst) -> ProblemFeatures {
        let loc = (problem.line_end - problem.line_start + 1) as f64;
        let is_security = matches!(
            problem.problem_type,
            ProblemType::HardcodedSecret | ProblemType::SqlInjection | ProblemType::XssVulnerability
        ) as u8 as f64;
        let is_performance = matches!(
            problem.problem_type,
            ProblemType::NestedLoop | ProblemType::UnoptimizedQuery
        ) as u8 as f64;

        let position = (problem.line_start as f64) / (ast_data.metrics.total_lines.max(1) as f64);

        ProblemFeatures {
            loc,
            cyclomatic_complexity: 5.0, // dummy za sada
            nesting_depth: 3.0,
            params_count: 2.0,
            is_security,
            is_performance,
            file_size: ast_data.metrics.total_lines as f64,
            position,
            severity_base: problem.severity.to_score() as f64 / 100.0,
        }
    }

    pub fn predict_score(&self, features: &ProblemFeatures) -> f64 {
        // Problem type weight
        let base_weight = self
            .problem_weights
            .get(&ProblemType::LongMethod) // fallback
            .copied()
            .unwrap_or(0.5);

        let mut score = 0.5 * base_weight;

        // Feature multipliers
        score *= 1.0 + features.is_security * 0.3;
        score *= 1.0 + features.is_performance * 0.2;
        score *= 1.0 + features.severity_base * 0.5;
        score *= 1.0 + (features.loc / 100.0).min(2.0);
        score *= 1.0 + (features.cyclomatic_complexity / 10.0).min(1.5);
        score *= 1.0 + (features.nesting_depth / 10.0).min(1.2);
        score *= 1.0 + (features.params_count / 10.0).min(1.2);

        // Position penalty
        score *= 1.0 - (features.position * 0.2);

        score.clamp(0.0, 1.0)
    }

    pub fn get_feature_importance(&self) -> Vec<(&'static str, f64)> {
        vec![
            ("is_security", 0.28),
            ("severity_base", 0.22),
            ("loc", 0.15),
            ("cyclomatic_complexity", 0.12),
            ("nesting_depth", 0.08),
            ("is_performance", 0.07),
            ("params_count", 0.05),
            ("position", 0.03),
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
