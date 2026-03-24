pub mod smells;
pub mod performance;
pub mod security;

use crate::models::Problem;

pub trait Detector {
    fn detect(&self, data: &crate::models::ParsedAst) -> Vec<Problem>;
}
