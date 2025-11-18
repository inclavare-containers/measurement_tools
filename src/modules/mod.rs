// src/modules/mod.rs

pub mod file_config_watcher;
pub mod file_measurer;
pub mod measurable;
pub mod watcher;

// Re-export for easier access
pub use file_measurer::FileMeasurer;
pub use measurable::Measurable;
pub use watcher::ConfigWatcher;
