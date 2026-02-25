use indicatif::{ProgressBar, ProgressStyle};
use std::io::IsTerminal;

const LARGE_INPUT_THRESHOLD: usize = 50_000;

pub struct ProgressTracker {
    bar: Option<ProgressBar>,
}

impl ProgressTracker {
    pub fn new(total_tokens: usize) -> Self {
        // Only show progress for large inputs and when stderr is a TTY
        let bar = if total_tokens >= LARGE_INPUT_THRESHOLD && std::io::stderr().is_terminal() {
            let pb = ProgressBar::new(100);
            pb.set_style(
                ProgressStyle::default_bar()
                    .template("Processing: {bar:40.cyan/blue} {pos}% ({msg})")
                    .unwrap_or_else(|_| ProgressStyle::default_bar())
                    .progress_chars("█▓░"),
            );
            pb.enable_steady_tick(std::time::Duration::from_secs(1));
            Some(pb)
        } else {
            None
        };
        
        Self { bar }
    }
    
    pub fn set_stage(&self, stage: &str, percent: u64) {
        if let Some(ref pb) = self.bar {
            pb.set_position(percent);
            pb.set_message(stage.to_string());
        }
    }
    
    pub fn segmenting(&self) {
        self.set_stage("segmenting", 10);
    }
    
    pub fn analyzing_redundancy(&self) {
        self.set_stage("analyzing redundancy", 30);
    }
    
    pub fn analyzing_importance(&self) {
        self.set_stage("analyzing importance", 50);
    }
    
    pub fn compressing(&self) {
        self.set_stage("compressing", 70);
    }
    
    pub fn reassembling(&self) {
        self.set_stage("reassembling", 90);
    }
    
    pub fn finish(&self) {
        if let Some(ref pb) = self.bar {
            pb.finish_and_clear();
        }
    }
}

impl Drop for ProgressTracker {
    fn drop(&mut self) {
        self.finish();
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_small_input_no_progress() {
        let tracker = ProgressTracker::new(1000);
        assert!(tracker.bar.is_none());
    }

    #[test]
    fn test_stages_dont_panic() {
        let tracker = ProgressTracker::new(1000);
        tracker.segmenting();
        tracker.analyzing_redundancy();
        tracker.analyzing_importance();
        tracker.compressing();
        tracker.reassembling();
        tracker.finish();
    }

    #[test]
    fn test_threshold_boundary() {
        // Just under threshold
        let tracker = ProgressTracker::new(49_999);
        assert!(tracker.bar.is_none());
    }
}
