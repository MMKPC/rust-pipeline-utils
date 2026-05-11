use serde::{Deserialize, Serialize};

/// Global pipeline configuration (loaded from pipeline.json or env).
#[derive(Debug, Serialize, Deserialize, Clone)]
pub struct PipelineConfig {
    /// Maximum parallel threads for batch operations.
    pub max_threads: usize,
    /// Default asset root directory.
    pub asset_root: String,
    /// Enable verbose logging.
    pub verbose: bool,
}

impl Default for PipelineConfig {
    fn default() -> Self {
        Self {
            max_threads: num_cpus(),
            asset_root: String::from("."),
            verbose: false,
        }
    }
}

fn num_cpus() -> usize {
    std::thread::available_parallelism()
        .map(|n| n.get())
        .unwrap_or(4)
}

impl PipelineConfig {
    /// Load from a JSON file, falling back to defaults on error.
    pub fn load(path: &str) -> Self {
        std::fs::read_to_string(path)
            .ok()
            .and_then(|s| serde_json::from_str(&s).ok())
            .unwrap_or_default()
    }
}
