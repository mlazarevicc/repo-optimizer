use crate::detectors::Detector;
use crate::models::{ParsedAst, Problem, Severity};
use std::collections::HashMap;
use uuid::Uuid;

/// Detektuje doslovno (Type-1) duplikovane blokove koda - isti niz linija
/// koji se ponavlja na 2+ mesta u fajlu. Radi na "sliding window" principu:
/// uzima prozor od MIN_BLOCK_LINES uzastopnih linija, normalizuje ih (trim),
/// i trazi gde se IDENTICAN blok ponavlja. Kad nadje poklapanje, pokusava da
/// "produzi" ga (greedy) da uhvati ceo duplikovani segment, ne samo prvih
/// nekoliko linija - tako se izbegava da se jedan veliki duplikat prijavi kao
/// gomila preklapajucih sitnih nalaza.
pub struct DuplicationDetector;

const MIN_BLOCK_LINES: usize = 6;
const MIN_BLOCK_CHARS: usize = 40; // izbegava lazne pozitive na npr. ponovljenim "}" linijama

impl DuplicationDetector {
    pub fn new() -> Self {
        Self
    }

    fn block_key(lines: &[&str], start: usize, len: usize) -> Option<String> {
        let slice = &lines[start..start + len];
        let trimmed: Vec<&str> = slice.iter().map(|l| l.trim()).collect();
        let non_empty = trimmed.iter().filter(|l| !l.is_empty()).count();
        let total_chars: usize = trimmed.iter().map(|l| l.len()).sum();
        if non_empty < (len.min(MIN_BLOCK_LINES) - 1) || total_chars < MIN_BLOCK_CHARS {
            // Previse praznih/sitnih linija u prozoru - nije pouzdan signal duplikata.
            return None;
        }
        Some(trimmed.join("\n"))
    }

    fn lines_match(lines: &[&str], a: usize, b: usize) -> bool {
        a < lines.len() && b < lines.len() && lines[a].trim() == lines[b].trim()
    }
}

impl Detector for DuplicationDetector {
    fn detect(&self, data: &ParsedAst) -> Vec<Problem> {
        let mut problems = Vec::new();
        let lines: Vec<&str> = data.code.lines().collect();
        if lines.len() < MIN_BLOCK_LINES * 2 {
            return problems; // fajl je premali da bi imalo smisla trazi duplikate
        }

        let mut seen: HashMap<String, usize> = HashMap::new();
        let mut i = 0usize;

        while i + MIN_BLOCK_LINES <= lines.len() {
            let key = match Self::block_key(&lines, i, MIN_BLOCK_LINES) {
                Some(k) => k,
                None => {
                    i += 1;
                    continue;
                }
            };

            if let Some(&first_start) = seen.get(&key) {
                // Nasli smo poklapanje - produzi ga greedy-jem dok linije i dalje
                // odgovaraju jedna drugoj (i nismo izasli iz fajla).
                let mut extra = 0usize;
                while Self::lines_match(&lines, first_start + MIN_BLOCK_LINES + extra, i + MIN_BLOCK_LINES + extra) {
                    extra += 1;
                }
                let total_len = MIN_BLOCK_LINES + extra;

                problems.push(Problem {
                    id: Uuid::new_v4(),
                    analysis_job_id: data.analysis_job_id,
                    problem_type: "code_smell.duplicate_code".to_string(),
                    severity: if total_len > 15 { Severity::High } else { Severity::Medium },
                    line_start: i + 1,
                    line_end: i + total_len,
                    message: format!(
                        "Duplicated block of {} lines - identical to lines {}-{}. Consider extracting a shared function.",
                        total_len, first_start + 1, first_start + total_len
                    ),
                    code_snippet: lines[i..i + total_len].join("\n"),
                    created_at: chrono::Utc::now(),
                    file_path: data.file_path.clone(),
                    language: data.language.to_string(),
                    autofix: None,
                    rank_score: None,
                });

                // Preskoci ceo prijavljeni duplikat da ne bismo isti segment
                // prijavili jos N puta kroz preklapajuce prozore.
                i += total_len;
                continue;
            }

            seen.entry(key).or_insert(i);
            i += 1;
        }

        problems
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::models::{CodeMetrics, Language};

    fn ast_with(code: &str) -> ParsedAst {
        ParsedAst {
            analysis_job_id: Uuid::new_v4(),
            language: Language::Python,
            code: code.to_string(),
            metrics: CodeMetrics {
                total_lines: code.lines().count(),
                code_lines: code.lines().count(),
                comment_lines: 0,
                blank_lines: 0,
                total_functions: 0,
                total_classes: 0,
                max_nesting_depth: 0,
                cyclomatic_complexity: 0,
            },
            functions: vec![],
            classes: vec![],
            file_path: None,
        }
    }

    #[test]
    fn detects_exact_duplicate_block() {
        let block = "    total = price * quantity\n    discount = total * 0.1\n    final_price = total - discount\n    tax = final_price * 0.08\n    grand_total = final_price + tax\n    print(grand_total)\n";
        let code = format!("def order_a():\n{}\ndef order_b():\n{}", block, block);
        let problems = DuplicationDetector::new().detect(&ast_with(&code));
        assert!(problems.iter().any(|p| p.problem_type == "code_smell.duplicate_code"));
    }

    #[test]
    fn extends_match_beyond_minimum_window() {
        // Kratke promenljive ("a = 1", 5 znakova) padaju ispod MIN_BLOCK_CHARS(40)
        // strazara koji postoji da bi sprečio lazne pozitive na ponavljajucim
        // zatvarajucim zagradama/trivijalnim linijama. Koristimo realisticniji
        // kod sa linijama kakve se zaista javljaju u produkciji.
        let block = "\
    result = calculate_price(quantity, unit_price)\n\
    discount = apply_discount(result, customer_tier)\n\
    final_price = result - discount\n\
    tax_amount = compute_tax(final_price, region)\n\
    grand_total = final_price + tax_amount\n\
    invoice_id = generate_invoice_number()\n\
    customer = fetch_customer_by_id(user_id)\n\
    payment_ok = verify_payment_status(invoice_id)\n\
    log_transaction(invoice_id, grand_total)\n\
    send_confirmation_email(customer, invoice_id)\n";
        let code = format!("def process_order_a():\n{}\ndef process_order_b():\n{}", block, block);
        let problems = DuplicationDetector::new().detect(&ast_with(&code));
        let hit = problems
            .iter()
            .find(|p| p.problem_type == "code_smell.duplicate_code")
            .unwrap();
        // Blok ima 10 linija, ne samo MIN_BLOCK_LINES(6) - greedy extension mora da uhvati sve.
        assert_eq!(hit.line_end - hit.line_start + 1, 10);
    }

    #[test]
    fn no_duplicate_in_unique_code() {
        let code = "def a():\n    x = 1\n    y = 2\n    return x + y\n\ndef b():\n    p = 5\n    q = 9\n    return p * q\n";
        let problems = DuplicationDetector::new().detect(&ast_with(code));
        assert!(problems.is_empty());
    }

    #[test]
    fn repeated_braces_alone_not_flagged() {
        // Cesto ponavljanje kratkih linija (npr. zatvarajuce zagrade) ne treba
        // da se tretira kao "duplikovan kod" - to bi bilo previse lazno-pozitivno.
        let code = "}\n}\n}\n}\n}\n}\n}\n}\n}\n}\n}\n}\n}\n}\n";
        let problems = DuplicationDetector::new().detect(&ast_with(code));
        assert!(problems.is_empty());
    }
}
