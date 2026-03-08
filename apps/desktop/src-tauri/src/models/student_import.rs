use serde::{Deserialize, Serialize};
use specta::Type;

#[derive(Debug, Clone, Deserialize, Type)]
pub enum ImportDuplicateStrategy {
    Skip,
    Update,
    Add,
}

#[derive(Debug, Deserialize, Type)]
pub struct ImportStudentsInput {
    pub file_path: String,
    pub class_id: String,
    pub duplicate_strategy: ImportDuplicateStrategy,
}

#[derive(Debug, Clone, Serialize, Type)]
pub struct ImportRowError {
    pub row_number: usize,
    pub field: String,
    pub reason: String,
    pub suggestion: String,
}

#[derive(Debug, Clone, Serialize, Type)]
pub enum ImportRowResult {
    Created,
    Updated,
    Skipped,
    Error,
}

#[derive(Debug, Serialize, Type)]
pub struct ImportStudentsResult {
    pub total_rows: usize,
    pub created_count: usize,
    pub updated_count: usize,
    pub skipped_count: usize,
    pub error_count: usize,
    pub errors: Vec<ImportRowError>,
}
