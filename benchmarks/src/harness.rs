//! Performance Harness for QuantaTerm
//!
//! This module provides comprehensive performance testing capabilities for PTY operations,
//! rendering performance, and overall system throughput.

use crate::synthetic::{LoadConfig, LoadStats, SyntheticGenerator};
use anyhow::{Context, Result};
use quantaterm_telemetry::Telemetry;
use serde::{Deserialize, Serialize};
use std::collections::HashMap;
use std::time::{Duration, Instant};

use tracing::{debug, info, instrument, warn};

/// Configuration for performance benchmarking
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct BenchmarkConfig {
    /// Test name/identifier
    pub name: String,
    /// Description of what this benchmark tests
    pub description: String,
    /// Load generation configuration
    pub load_config: LoadConfig,
    /// Number of iterations to run
    pub iterations: u32,
    /// Warmup iterations (not included in results)
    pub warmup_iterations: u32,
    /// Maximum acceptable latency in milliseconds
    pub max_latency_ms: f64,
    /// Maximum acceptable frame drop percentage
    pub max_frame_drop_percentage: f64,
    /// Target throughput in bytes per second
    pub target_throughput: u64,
}

/// Results from a single benchmark iteration
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct BenchmarkIteration {
    /// Iteration number
    pub iteration: u32,
    /// Load generation statistics
    pub load_stats: LoadStats,
    /// Processing latency statistics
    pub latency_stats: LatencyStats,
    /// Throughput statistics
    pub throughput_stats: ThroughputStats,
    /// Frame drop statistics (simulated)
    pub frame_drop_stats: FrameDropStats,
    /// Memory usage during the test
    pub memory_usage: MemoryUsage,
    /// Duration of the entire iteration
    pub iteration_duration: Duration,
}

/// Aggregated results from multiple benchmark iterations
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct BenchmarkResults {
    /// Configuration used for this benchmark
    pub config: BenchmarkConfig,
    /// Individual iteration results
    pub iterations: Vec<BenchmarkIteration>,
    /// Aggregated statistics
    pub summary: BenchmarkSummary,
    /// Whether the benchmark passed all acceptance criteria
    pub passed: bool,
    /// Timestamp when the benchmark was run
    pub timestamp: chrono::DateTime<chrono::Utc>,
}

/// Latency measurement statistics
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct LatencyStats {
    /// Average latency in milliseconds
    pub avg_ms: f64,
    /// Minimum latency in milliseconds
    pub min_ms: f64,
    /// Maximum latency in milliseconds
    pub max_ms: f64,
    /// 95th percentile latency in milliseconds
    pub p95_ms: f64,
    /// 99th percentile latency in milliseconds
    pub p99_ms: f64,
    /// Number of latency measurements
    pub sample_count: usize,
}

/// Throughput measurement statistics
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ThroughputStats {
    /// Average bytes per second
    pub avg_bytes_per_sec: f64,
    /// Peak bytes per second
    pub peak_bytes_per_sec: f64,
    /// Minimum bytes per second
    pub min_bytes_per_sec: f64,
    /// Total bytes processed
    pub total_bytes: u64,
    /// Processing duration
    pub duration: Duration,
}

/// Frame drop statistics (simulated based on processing delays)
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct FrameDropStats {
    /// Total frames that would be rendered
    pub total_frames: u64,
    /// Number of frames that would be dropped
    pub dropped_frames: u64,
    /// Frame drop percentage
    pub drop_percentage: f64,
    /// Target frame rate (FPS)
    pub target_fps: f64,
}

/// Memory usage statistics
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct MemoryUsage {
    /// Peak memory usage in bytes
    pub peak_bytes: u64,
    /// Average memory usage in bytes
    pub avg_bytes: u64,
    /// Memory usage at end of test
    pub final_bytes: u64,
}

/// Summary statistics across all iterations
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct BenchmarkSummary {
    /// Average results across iterations
    pub avg_latency_ms: f64,
    /// Average throughput across iterations
    pub avg_throughput: f64,
    /// Average frame drop percentage
    pub avg_frame_drop_percentage: f64,
    /// Standard deviation of latency
    pub latency_std_dev: f64,
    /// Standard deviation of throughput
    pub throughput_std_dev: f64,
    /// Whether all iterations met the performance criteria
    pub all_passed: bool,
    /// Performance regression indicator (compared to baseline if available)
    pub regression_percentage: Option<f64>,
}

impl Default for BenchmarkConfig {
    fn default() -> Self {
        Self {
            name: "default_benchmark".to_string(),
            description: "Default performance benchmark".to_string(),
            load_config: LoadConfig::default(),
            iterations: 5,
            warmup_iterations: 2,
            max_latency_ms: 50.0,
            max_frame_drop_percentage: 5.0,
            target_throughput: 10_000,
        }
    }
}

impl BenchmarkConfig {
    /// Create a configuration for continuous output testing (like cargo build)
    pub fn continuous_output() -> Self {
        Self {
            name: "continuous_output".to_string(),
            description: "Simulates continuous output like cargo build with target of 0 dropped frames until >200k chars/sec".to_string(),
            load_config: LoadConfig::continuous_output(200_000, Duration::from_secs(10)),
            max_latency_ms: 20.0,
            max_frame_drop_percentage: 0.0, // 0 dropped frames until > 200k chars/sec
            target_throughput: 200_000,
            ..Default::default()
        }
    }

    /// Create a configuration for large paste testing
    pub fn large_paste() -> Self {
        Self {
            name: "large_paste".to_string(),
            description:
                "Simulates large paste operation (100k chars) with target completion < 250ms"
                    .to_string(),
            load_config: LoadConfig::large_paste(100_000),
            max_latency_ms: 250.0, // Final render completion < 250 ms
            max_frame_drop_percentage: 10.0,
            target_throughput: 400_000, // 100k chars in 250ms
            iterations: 10,
            ..Default::default()
        }
    }

    /// Create a configuration for interactive editing simulation
    pub fn interactive_editing() -> Self {
        Self {
            name: "interactive_editing".to_string(),
            description: "Simulates interactive editing with target p95 input→render < 18ms"
                .to_string(),
            load_config: LoadConfig {
                bytes_per_second: 1_000, // Low rate for interactive
                duration: Duration::from_secs(30),
                chunk_size: 10, // Small chunks like keystrokes
                data_type: crate::synthetic::DataType::PlainText,
                ..Default::default()
            },
            max_latency_ms: 18.0, // p95 input→render < 18 ms
            max_frame_drop_percentage: 1.0,
            target_throughput: 1_000,
            iterations: 20,
            ..Default::default()
        }
    }

    /// Create a configuration for burst load testing
    pub fn burst_load() -> Self {
        Self {
            name: "burst_load".to_string(),
            description: "Tests performance under bursty load conditions".to_string(),
            load_config: LoadConfig::burst_mode(50_000, 0.5, 4.0), // 50KB/s base, 0.5 bursts/sec, 4x multiplier
            max_latency_ms: 100.0,
            max_frame_drop_percentage: 15.0,
            target_throughput: 50_000,
            ..Default::default()
        }
    }
}

/// Performance harness for running benchmarks
pub struct PerformanceHarness {
    telemetry: Telemetry,
    baseline_results: HashMap<String, BenchmarkResults>,
}

impl PerformanceHarness {
    /// Create a new performance harness
    pub fn new() -> Self {
        Self {
            telemetry: Telemetry::new(),
            baseline_results: HashMap::new(),
        }
    }

    /// Load baseline results from previous runs for regression testing
    pub fn load_baseline(&mut self, results: BenchmarkResults) {
        info!(
            benchmark_name = %results.config.name,
            "Loading baseline results for regression testing"
        );
        self.baseline_results
            .insert(results.config.name.clone(), results);
    }

    /// Run a single benchmark iteration
    #[instrument(skip(self), fields(iteration = iteration, benchmark = %config.name))]
    async fn run_iteration(
        &mut self,
        config: &BenchmarkConfig,
        iteration: u32,
    ) -> Result<BenchmarkIteration> {
        debug!("Starting benchmark iteration");

        let iteration_start = Instant::now();
        self.telemetry.clear();

        // Create synthetic data generator
        let mut generator = SyntheticGenerator::new(config.load_config.clone());

        // Storage for performance measurements
        let mut latency_measurements = Vec::new();
        let mut throughput_measurements = Vec::new();
        let mut memory_measurements = Vec::new();

        // Simulate frame timing
        let target_fps = 60.0;
        let frame_duration = Duration::from_millis((1000.0 / target_fps) as u64);
        let mut last_frame_time = iteration_start;
        let mut frame_count = 0u64;
        let mut dropped_frames = 0u64;

        // Run the load generation
        let load_stats = generator
            .generate_load(|data| {
                let process_start = Instant::now();

                // Simulate PTY data processing
                self.simulate_pty_processing(&data)?;

                let process_duration = process_start.elapsed();
                latency_measurements.push(process_duration.as_secs_f64() * 1000.0);

                // Calculate throughput
                let throughput = data.len() as f64 / process_duration.as_secs_f64();
                throughput_measurements.push(throughput);

                // Simulate frame rendering timing
                let now = Instant::now();
                if now.duration_since(last_frame_time) >= frame_duration {
                    // Check if we would have dropped frames due to processing delay
                    let frames_that_should_have_passed =
                        now.duration_since(last_frame_time).as_secs_f64()
                            / frame_duration.as_secs_f64();

                    if frames_that_should_have_passed > 1.5 {
                        dropped_frames += (frames_that_should_have_passed - 1.0) as u64;
                    }

                    frame_count += 1;
                    last_frame_time = now;
                }

                // Simulate memory usage (this would be real memory measurement in practice)
                let memory_usage = self.simulate_memory_usage(data.len());
                memory_measurements.push(memory_usage);

                Ok(())
            })
            .await?;

        let iteration_duration = iteration_start.elapsed();

        // Calculate statistics
        let latency_stats = self.calculate_latency_stats(&latency_measurements);
        let throughput_stats =
            self.calculate_throughput_stats(&throughput_measurements, iteration_duration);
        let frame_drop_stats = FrameDropStats {
            total_frames: frame_count + dropped_frames,
            dropped_frames,
            drop_percentage: if frame_count + dropped_frames > 0 {
                dropped_frames as f64 / (frame_count + dropped_frames) as f64 * 100.0
            } else {
                0.0
            },
            target_fps,
        };
        let memory_usage = self.calculate_memory_usage(&memory_measurements);

        Ok(BenchmarkIteration {
            iteration,
            load_stats,
            latency_stats,
            throughput_stats,
            frame_drop_stats,
            memory_usage,
            iteration_duration,
        })
    }

    /// Simulate PTY data processing (placeholder for actual processing)
    fn simulate_pty_processing(&mut self, data: &[u8]) -> Result<()> {
        // Record telemetry
        self.telemetry.start_timing("pty_processing");

        // Simulate parsing and processing overhead
        let processing_time = Duration::from_micros(data.len() as u64 / 10); // Simulate some processing cost
        std::thread::sleep(processing_time);

        self.telemetry.end_timing("pty_processing");
        self.telemetry
            .add_counter("bytes_processed", data.len() as u64);

        Ok(())
    }

    /// Simulate memory usage calculation
    fn simulate_memory_usage(&self, data_size: usize) -> u64 {
        // Simulate memory usage based on data size and some base usage
        8_000_000 + (data_size * 2) as u64 // 8MB base + 2x data size
    }

    /// Calculate latency statistics from measurements
    fn calculate_latency_stats(&self, measurements: &[f64]) -> LatencyStats {
        if measurements.is_empty() {
            return LatencyStats {
                avg_ms: 0.0,
                min_ms: 0.0,
                max_ms: 0.0,
                p95_ms: 0.0,
                p99_ms: 0.0,
                sample_count: 0,
            };
        }

        let mut sorted = measurements.to_vec();
        sorted.sort_by(|a, b| a.partial_cmp(b).unwrap());

        let avg_ms = measurements.iter().sum::<f64>() / measurements.len() as f64;
        let min_ms = sorted[0];
        let max_ms = sorted[sorted.len() - 1];

        let p95_index = ((sorted.len() as f64 * 0.95) as usize).min(sorted.len() - 1);
        let p99_index = ((sorted.len() as f64 * 0.99) as usize).min(sorted.len() - 1);

        LatencyStats {
            avg_ms,
            min_ms,
            max_ms,
            p95_ms: sorted[p95_index],
            p99_ms: sorted[p99_index],
            sample_count: measurements.len(),
        }
    }

    /// Calculate throughput statistics
    fn calculate_throughput_stats(
        &self,
        measurements: &[f64],
        duration: Duration,
    ) -> ThroughputStats {
        if measurements.is_empty() {
            return ThroughputStats {
                avg_bytes_per_sec: 0.0,
                peak_bytes_per_sec: 0.0,
                min_bytes_per_sec: 0.0,
                total_bytes: 0,
                duration,
            };
        }

        let avg_bytes_per_sec = measurements.iter().sum::<f64>() / measurements.len() as f64;
        let peak_bytes_per_sec = measurements.iter().fold(0.0f64, |acc, &x| acc.max(x));
        let min_bytes_per_sec = measurements.iter().fold(f64::MAX, |acc, &x| acc.min(x));

        // Calculate total bytes from average throughput
        let total_bytes = (avg_bytes_per_sec * duration.as_secs_f64()) as u64;

        ThroughputStats {
            avg_bytes_per_sec,
            peak_bytes_per_sec,
            min_bytes_per_sec,
            total_bytes,
            duration,
        }
    }

    /// Calculate memory usage statistics
    fn calculate_memory_usage(&self, measurements: &[u64]) -> MemoryUsage {
        if measurements.is_empty() {
            return MemoryUsage {
                peak_bytes: 0,
                avg_bytes: 0,
                final_bytes: 0,
            };
        }

        let peak_bytes = *measurements.iter().max().unwrap();
        let avg_bytes = measurements.iter().sum::<u64>() / measurements.len() as u64;
        let final_bytes = *measurements.last().unwrap();

        MemoryUsage {
            peak_bytes,
            avg_bytes,
            final_bytes,
        }
    }

    /// Run a complete benchmark with multiple iterations
    #[instrument(skip(self), fields(benchmark = %config.name))]
    pub async fn run_benchmark(&mut self, config: BenchmarkConfig) -> Result<BenchmarkResults> {
        info!(
            benchmark = %config.name,
            iterations = config.iterations,
            warmup_iterations = config.warmup_iterations,
            "Starting benchmark"
        );

        let mut all_iterations = Vec::new();

        // Run warmup iterations
        for i in 0..config.warmup_iterations {
            info!(warmup_iteration = i + 1, "Running warmup iteration");
            let _ = self.run_iteration(&config, i + 1).await?;
        }

        // Run actual benchmark iterations
        for i in 0..config.iterations {
            info!(iteration = i + 1, "Running benchmark iteration");
            let iteration_result = self
                .run_iteration(&config, i + 1)
                .await
                .with_context(|| format!("Failed to run iteration {}", i + 1))?;
            all_iterations.push(iteration_result);
        }

        // Calculate summary statistics
        let summary = self.calculate_summary(&config, &all_iterations);

        // Check if benchmark passed
        let passed = self.check_acceptance_criteria(&config, &summary);

        let results = BenchmarkResults {
            config: config.clone(),
            iterations: all_iterations,
            summary,
            passed,
            timestamp: chrono::Utc::now(),
        };

        info!(
            benchmark = %config.name,
            passed = passed,
            avg_latency_ms = results.summary.avg_latency_ms,
            avg_throughput = results.summary.avg_throughput,
            "Benchmark completed"
        );

        Ok(results)
    }

    /// Calculate summary statistics across iterations
    fn calculate_summary(
        &self,
        config: &BenchmarkConfig,
        iterations: &[BenchmarkIteration],
    ) -> BenchmarkSummary {
        if iterations.is_empty() {
            return BenchmarkSummary {
                avg_latency_ms: 0.0,
                avg_throughput: 0.0,
                avg_frame_drop_percentage: 0.0,
                latency_std_dev: 0.0,
                throughput_std_dev: 0.0,
                all_passed: false,
                regression_percentage: None,
            };
        }

        let latencies: Vec<f64> = iterations.iter().map(|i| i.latency_stats.avg_ms).collect();
        let throughputs: Vec<f64> = iterations
            .iter()
            .map(|i| i.throughput_stats.avg_bytes_per_sec)
            .collect();
        let frame_drops: Vec<f64> = iterations
            .iter()
            .map(|i| i.frame_drop_stats.drop_percentage)
            .collect();

        let avg_latency_ms = latencies.iter().sum::<f64>() / latencies.len() as f64;
        let avg_throughput = throughputs.iter().sum::<f64>() / throughputs.len() as f64;
        let avg_frame_drop_percentage = frame_drops.iter().sum::<f64>() / frame_drops.len() as f64;

        // Calculate standard deviations
        let latency_variance = latencies
            .iter()
            .map(|&x| (x - avg_latency_ms).powi(2))
            .sum::<f64>()
            / latencies.len() as f64;
        let latency_std_dev = latency_variance.sqrt();

        let throughput_variance = throughputs
            .iter()
            .map(|&x| (x - avg_throughput).powi(2))
            .sum::<f64>()
            / throughputs.len() as f64;
        let throughput_std_dev = throughput_variance.sqrt();

        // Check if all iterations passed acceptance criteria
        let all_passed = iterations.iter().all(|iteration| {
            iteration.latency_stats.p95_ms <= config.max_latency_ms
                && iteration.frame_drop_stats.drop_percentage <= config.max_frame_drop_percentage
                && iteration.throughput_stats.avg_bytes_per_sec >= config.target_throughput as f64
        });

        // Calculate regression percentage if baseline is available
        let regression_percentage = self.baseline_results.get(&config.name).map(|baseline| {
            let baseline_latency = baseline.summary.avg_latency_ms;
            if baseline_latency > 0.0 {
                ((avg_latency_ms - baseline_latency) / baseline_latency) * 100.0
            } else {
                0.0
            }
        });

        BenchmarkSummary {
            avg_latency_ms,
            avg_throughput,
            avg_frame_drop_percentage,
            latency_std_dev,
            throughput_std_dev,
            all_passed,
            regression_percentage,
        }
    }

    /// Check if benchmark results meet acceptance criteria
    fn check_acceptance_criteria(
        &self,
        config: &BenchmarkConfig,
        summary: &BenchmarkSummary,
    ) -> bool {
        let latency_ok = summary.avg_latency_ms <= config.max_latency_ms;
        let frame_drop_ok = summary.avg_frame_drop_percentage <= config.max_frame_drop_percentage;
        let throughput_ok = summary.avg_throughput >= config.target_throughput as f64;

        // Check regression threshold (10% as mentioned in PROJECT_PLAN.md)
        let regression_ok = match summary.regression_percentage {
            Some(regression) => regression <= 10.0,
            None => true, // No baseline, so no regression
        };

        let passed = latency_ok && frame_drop_ok && throughput_ok && regression_ok;

        if !passed {
            warn!(
                latency_ok = latency_ok,
                frame_drop_ok = frame_drop_ok,
                throughput_ok = throughput_ok,
                regression_ok = regression_ok,
                "Benchmark failed acceptance criteria"
            );
        }

        passed
    }
}

impl Default for PerformanceHarness {
    fn default() -> Self {
        Self::new()
    }
}
