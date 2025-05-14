// src/modules/mod.rs

pub mod measurable;
pub mod file_measurer;

// Re-export for easier access
pub use measurable::Measurable;
pub use file_measurer::FileMeasurer; 