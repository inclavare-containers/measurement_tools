// src/modules/mod.rs

pub mod file_measurer;
pub mod measurable;

// Re-export for easier access
pub use file_measurer::FileMeasurer;
pub use measurable::Measurable;
