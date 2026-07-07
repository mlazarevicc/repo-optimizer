use crate::models::Problem;

/// Da li je problem nasao NAS sopstveni detektor (security.rs / smells.rs / performance.rs)
/// ili semgrep ("p/default" ruleset, check_id tipa "python.lang.security.audit...").
/// Nasi detektori uvek koriste fiksni mali skup prefiksa - sve sto ne pocinje
/// jednim od njih je po definiciji semgrep nalaz.
fn is_custom_detector_finding(problem_type: &str) -> bool {
    problem_type.starts_with("code_smell.")
        || problem_type.starts_with("security.")
        || problem_type.starts_with("performance.")
}

/// Gruba kategorizacija po znacenju, ista logika kao u suggestion_generator_service/suggestions.rs
/// (namerno - zelimo da "isti problem" prepoznamo bez obzira da li ga je nasao semgrep
/// (npr. "python.lang.security.audit.hardcoded-password...") ili nas detektor
/// (npr. "security.hardcoded_secret").
fn category_bucket(problem_type: &str) -> &'static str {
    let p = problem_type.to_lowercase();
    if p.contains("secret") || p.contains("password") || p.contains("key") || p.contains("credential") {
        "secret"
    } else if p.contains("sql") || p.contains("injection") {
        "sql_injection"
    } else if p.contains("xss") || p.contains("cross-site") {
        "xss"
    } else if p.contains("long") || p.contains("complex") {
        "long_complex"
    } else if p.contains("insecure_random") {
        "insecure_random"
    } else if p.contains("string_concat") {
        "string_concat_loop"
    } else if p.contains("loop") || p.contains("nesting") {
        "loop_nesting"
    } else if p.contains("query") || p.contains("unoptimized") {
        "unoptimized_query"
    } else {
        "other"
    }
}

fn ranges_overlap(a_start: usize, a_end: usize, b_start: usize, b_end: usize) -> bool {
    a_start <= b_end && b_start <= a_end
}

/// Uklanja duplikate kad semgrep i nasi detektori prijave ISTU vrstu problema
/// na istom (ili preklapajucem) opsegu linija u istom fajlu.
/// Kad dodje do duplikata, prioritet ima NAS detektor (jer mu se problem_type/severity/poruka
/// bolje slazu sa ostatkom pipeline-a), a semgrep nalaz se odbacuje.
pub fn dedupe_problems(problems: Vec<Problem>) -> Vec<Problem> {
    let mut kept: Vec<Problem> = Vec::with_capacity(problems.len());

    'outer: for candidate in problems {
        let candidate_bucket = category_bucket(&candidate.problem_type);
        let candidate_is_custom = is_custom_detector_finding(&candidate.problem_type);

        for existing in kept.iter_mut() {
            let same_file = existing.file_path == candidate.file_path;
            let same_bucket = category_bucket(&existing.problem_type) == candidate_bucket;
            let overlapping = ranges_overlap(
                existing.line_start, existing.line_end,
                candidate.line_start, candidate.line_end,
            );

            if same_file && same_bucket && overlapping && candidate_bucket != "other" {
                let existing_is_custom = is_custom_detector_finding(&existing.problem_type);
                // Ako je postojeci semgrep nalaz a kandidat je nas detektor, nas detektor pobedjuje
                if !existing_is_custom && candidate_is_custom {
                    *existing = candidate;
                }
                // U svakom slucaju, ovo je duplikat - ne dodajemo kandidata kao novi unos
                continue 'outer;
            }
        }

        kept.push(candidate);
    }

    kept
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::models::Severity;
    use uuid::Uuid;

    fn problem(problem_type: &str, file: &str, line_start: usize, line_end: usize) -> Problem {
        Problem {
            id: Uuid::new_v4(),
            analysis_job_id: Uuid::new_v4(),
            problem_type: problem_type.to_string(),
            severity: Severity::High,
            line_start,
            line_end,
            message: "test".to_string(),
            code_snippet: "test".to_string(),
            created_at: chrono::Utc::now(),
            file_path: Some(file.to_string()),
            language: "python".to_string(),
            autofix: None,
            rank_score: None,
        }
    }

    #[test]
    fn semgrep_and_custom_overlap_keeps_custom() {
        // semgrep check_id stil (vise tacaka, jezik na pocetku), nas detektor "security.*"
        let semgrep_finding = problem(
            "python.lang.security.audit.hardcoded-password.hardcoded-password",
            "app.py", 8, 8,
        );
        let custom_finding = problem("security.hardcoded_secret", "app.py", 8, 8);

        let result = dedupe_problems(vec![semgrep_finding, custom_finding.clone()]);
        assert_eq!(result.len(), 1, "preklapajuci nalazi iste kategorije moraju se spojiti u jedan");
        assert_eq!(result[0].problem_type, "security.hardcoded_secret", "nas detektor mora pobediti");
    }

    #[test]
    fn custom_finding_first_then_semgrep_still_keeps_custom() {
        // Redosled obrnut u odnosu na test iznad - rezultat mora biti isti.
        let custom_finding = problem("security.hardcoded_secret", "app.py", 8, 8);
        let semgrep_finding = problem(
            "python.lang.security.audit.hardcoded-password.hardcoded-password",
            "app.py", 8, 8,
        );

        let result = dedupe_problems(vec![custom_finding, semgrep_finding]);
        assert_eq!(result.len(), 1);
        assert_eq!(result[0].problem_type, "security.hardcoded_secret");
    }

    #[test]
    fn different_buckets_are_not_merged() {
        let sql_injection = problem("security.sql_injection_test", "app.py", 10, 10);
        let nesting = problem("code_smell.deep_nesting", "app.py", 10, 10);

        let result = dedupe_problems(vec![sql_injection, nesting]);
        assert_eq!(result.len(), 2, "razlicite kategorije problema ne treba spajati");
    }

    #[test]
    fn different_files_are_not_merged() {
        let a = problem("security.hardcoded_secret", "app.py", 8, 8);
        let b = problem("security.hardcoded_secret", "other.py", 8, 8);

        let result = dedupe_problems(vec![a, b]);
        assert_eq!(result.len(), 2, "isti tip problema u RAZLICITIM fajlovima nije duplikat");
    }

    #[test]
    fn non_overlapping_lines_are_not_merged() {
        let a = problem("security.hardcoded_secret", "app.py", 8, 8);
        let b = problem("security.hardcoded_secret", "app.py", 50, 50);

        let result = dedupe_problems(vec![a, b]);
        assert_eq!(result.len(), 2, "isti tip problema na razlicitim linijama nije duplikat");
    }

    #[test]
    fn overlapping_ranges_not_just_exact_match_are_merged() {
        // Funkcija na linijama 1-20 (npr. deep_nesting na celoj funkciji) i druga
        // na 15-15 (npr. complex_method) - ako bi neki drugi detektor prijavio
        // ISTU kategoriju sa preklapajucim (ne identicnim) opsegom, i to se spaja.
        let a = problem("code_smell.long_complex", "app.py", 1, 20);
        let b = problem("code_smell.long_complex", "app.py", 15, 15);

        let result = dedupe_problems(vec![a, b]);
        assert_eq!(result.len(), 1);
    }
}
