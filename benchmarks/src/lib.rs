//! QuantaTerm Performance Benchmarks
//!
//! This crate provides comprehensive performance testing and benchmarking capabilities
//! for QuantaTerm terminal emulator, including synthetic PTY load generation,
//! latency measurement, throughput analysis, and regression testing.

#![warn(missing_docs)]
#![deny(unsafe_code)]

pub mod harness;
pub mod synthetic;

pub use harness::{
    BenchmarkConfig, BenchmarkIteration, BenchmarkResults, BenchmarkSummary, FrameDropStats,
    LatencyStats, MemoryUsage, PerformanceHarness, ThroughputStats,
};
pub use synthetic::{DataType, LoadConfig, LoadStats, SyntheticGenerator};

use anyhow::Result;
use serde::{Deserialize, Serialize};
use std::path::Path;
use tracing::info;

/// Predefined benchmark suite for comprehensive testing
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct BenchmarkSuite {
    /// Name of the benchmark suite
    pub name: String,
    /// Description of the suite
    pub description: String,
    /// List of benchmarks to run
    pub benchmarks: Vec<BenchmarkConfig>,
}

impl BenchmarkSuite {
    /// Create the standard QuantaTerm benchmark suite based on PROJECT_PLAN.md targets
    pub fn standard_suite() -> Self {
        Self {
            name: "QuantaTerm Standard Performance Suite".to_string(),
            description: "Standard performance benchmarks based on PROJECT_PLAN.md targets"
                .to_string(),
            benchmarks: vec![
                BenchmarkConfig::continuous_output(),
                BenchmarkConfig::large_paste(),
                BenchmarkConfig::interactive_editing(),
                BenchmarkConfig::burst_load(),
            ],
        }
    }

    /// Create a minimal test suite for CI/development
    pub fn minimal_suite() -> Self {
        let mut suite = Self::standard_suite();
        suite.name = "QuantaTerm Minimal Performance Suite".to_string();
        suite.description = "Minimal performance benchmarks for CI and development".to_string();

        // Reduce iterations for faster execution
        for benchmark in &mut suite.benchmarks {
            benchmark.iterations = 3;
            benchmark.warmup_iterations = 1;
            benchmark.load_config.duration = std::cmp::min(
                benchmark.load_config.duration,
                std::time::Duration::from_secs(5),
            );
        }

        suite
    }

    /// Run the entire benchmark suite
    pub async fn run(&self, harness: &mut PerformanceHarness) -> Result<Vec<BenchmarkResults>> {
        info!(
            suite = %self.name,
            benchmark_count = self.benchmarks.len(),
            "Starting benchmark suite"
        );

        let mut results = Vec::new();

        for benchmark_config in &self.benchmarks {
            let result = harness.run_benchmark(benchmark_config.clone()).await?;
            results.push(result);
        }

        info!(
            suite = %self.name,
            completed_benchmarks = results.len(),
            "Benchmark suite completed"
        );

        Ok(results)
    }

    /// Save suite configuration to file
    pub fn save_to_file<P: AsRef<Path>>(&self, path: P) -> Result<()> {
        let json = serde_json::to_string_pretty(self)?;
        std::fs::write(path, json)?;
        Ok(())
    }

    /// Load suite configuration from file
    pub fn load_from_file<P: AsRef<Path>>(path: P) -> Result<Self> {
        let json = std::fs::read_to_string(path)?;
        let suite: Self = serde_json::from_str(&json)?;
        Ok(suite)
    }
}

/// Utilities for saving and loading benchmark results
pub struct ResultsManager;

impl ResultsManager {
    /// Save benchmark results to JSON file
    pub fn save_results<P: AsRef<Path>>(results: &BenchmarkResults, path: P) -> Result<()> {
        let json = serde_json::to_string_pretty(results)?;
        std::fs::write(&path, json)?;
        info!(
            benchmark = %results.config.name,
            path = %path.as_ref().display(),
            "Saved benchmark results"
        );
        Ok(())
    }

    /// Load benchmark results from JSON file
    pub fn load_results<P: AsRef<Path>>(path: P) -> Result<BenchmarkResults> {
        let json = std::fs::read_to_string(&path)?;
        let results: BenchmarkResults = serde_json::from_str(&json)?;
        info!(
            benchmark = %results.config.name,
            path = %path.as_ref().display(),
            "Loaded benchmark results"
        );
        Ok(results)
    }

    /// Save suite results to directory (one file per benchmark)
    pub fn save_suite_results<P: AsRef<Path>>(
        results: &[BenchmarkResults],
        dir_path: P,
    ) -> Result<()> {
        let dir = dir_path.as_ref();
        std::fs::create_dir_all(dir)?;

        for result in results {
            let filename = format!("{}_results.json", result.config.name);
            let file_path = dir.join(filename);
            Self::save_results(result, file_path)?;
        }

        info!(
            benchmark_count = results.len(),
            directory = %dir.display(),
            "Saved suite results"
        );

        Ok(())
    }

    /// Generate a summary report of benchmark results
    pub fn generate_summary_report(results: &[BenchmarkResults]) -> String {
        let mut report = String::new();

        report.push_str("# QuantaTerm Performance Benchmark Report\n\n");
        report.push_str(&format!(
            "Generated: {}\n\n",
            chrono::Utc::now().format("%Y-%m-%d %H:%M:%S UTC")
        ));

        let total_benchmarks = results.len();
        let passed_benchmarks = results.iter().filter(|r| r.passed).count();

        report.push_str("## Summary\n\n");
        report.push_str(&format!("- **Total Benchmarks:** {}\n", total_benchmarks));
        report.push_str(&format!("- **Passed:** {}\n", passed_benchmarks));
        report.push_str(&format!(
            "- **Failed:** {}\n",
            total_benchmarks - passed_benchmarks
        ));
        report.push_str(&format!(
            "- **Pass Rate:** {:.1}%\n\n",
            (passed_benchmarks as f64 / total_benchmarks as f64) * 100.0
        ));

        report.push_str("## Benchmark Results\n\n");

        for result in results {
            let status = if result.passed {
                "✅ PASS"
            } else {
                "❌ FAIL"
            };
            report.push_str(&format!("### {} - {}\n\n", result.config.name, status));
            report.push_str(&format!(
                "**Description:** {}\n\n",
                result.config.description
            ));

            report.push_str("**Performance Metrics:**\n");
            report.push_str(&format!(
                "- Average Latency: {:.2} ms\n",
                result.summary.avg_latency_ms
            ));
            report.push_str(&format!(
                "- Average Throughput: {:.0} bytes/sec\n",
                result.summary.avg_throughput
            ));
            report.push_str(&format!(
                "- Frame Drop Rate: {:.2}%\n",
                result.summary.avg_frame_drop_percentage
            ));

            if let Some(regression) = result.summary.regression_percentage {
                let regression_symbol = if regression > 0.0 { "📈" } else { "📉" };
                report.push_str(&format!(
                    "- Regression vs Baseline: {:.1}% {}\n",
                    regression, regression_symbol
                ));
            }

            report.push_str("\n**Acceptance Criteria:**\n");
            report.push_str(&format!(
                "- Max Latency: {:.1} ms ({})\n",
                result.config.max_latency_ms,
                if result.summary.avg_latency_ms <= result.config.max_latency_ms {
                    "✅"
                } else {
                    "❌"
                }
            ));
            report.push_str(&format!(
                "- Max Frame Drop: {:.1}% ({})\n",
                result.config.max_frame_drop_percentage,
                if result.summary.avg_frame_drop_percentage
                    <= result.config.max_frame_drop_percentage
                {
                    "✅"
                } else {
                    "❌"
                }
            ));
            report.push_str(&format!(
                "- Min Throughput: {} bytes/sec ({})\n",
                result.config.target_throughput,
                if result.summary.avg_throughput >= result.config.target_throughput as f64 {
                    "✅"
                } else {
                    "❌"
                }
            ));

            report.push('\n');
        }

        report
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_standard_suite_creation() {
        let suite = BenchmarkSuite::standard_suite();
        assert_eq!(suite.benchmarks.len(), 4);
        assert!(suite
            .benchmarks
            .iter()
            .any(|b| b.name == "continuous_output"));
        assert!(suite.benchmarks.iter().any(|b| b.name == "large_paste"));
        assert!(suite
            .benchmarks
            .iter()
            .any(|b| b.name == "interactive_editing"));
        assert!(suite.benchmarks.iter().any(|b| b.name == "burst_load"));
    }

    #[test]
    fn test_minimal_suite_creation() {
        let suite = BenchmarkSuite::minimal_suite();
        assert_eq!(suite.benchmarks.len(), 4);

        // Check that iterations are reduced
        for benchmark in &suite.benchmarks {
            assert_eq!(benchmark.iterations, 3);
            assert_eq!(benchmark.warmup_iterations, 1);
        }
    }

    #[tokio::test]
    async fn test_single_benchmark() {
        let mut harness = PerformanceHarness::new();
        let config = BenchmarkConfig {
            name: "test_benchmark".to_string(),
            description: "Test benchmark".to_string(),
            load_config: LoadConfig {
                bytes_per_second: 1000,
                duration: std::time::Duration::from_millis(100),
                chunk_size: 100,
                ..Default::default()
            },
            iterations: 2,
            warmup_iterations: 1,
            ..Default::default()
        };

        let result = harness.run_benchmark(config).await.unwrap();
        assert_eq!(result.iterations.len(), 2);
        assert!(result.summary.avg_latency_ms >= 0.0);
        assert!(result.summary.avg_throughput > 0.0);
    }
}
