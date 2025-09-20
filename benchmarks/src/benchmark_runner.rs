//! Benchmark Runner
//!
//! Automated benchmark runner for CI/CD and regression testing.

use anyhow::Result;
use clap::Parser;
use quantaterm_benchmarks::{BenchmarkSuite, PerformanceHarness, ResultsManager};
use std::path::PathBuf;
use tracing::{error, info, Level};
use tracing_subscriber::{fmt, layer::SubscriberExt, util::SubscriberInitExt, EnvFilter};

#[derive(Parser)]
#[command(name = "benchmark-runner")]
#[command(about = "Automated benchmark runner for QuantaTerm")]
#[command(version)]
struct Cli {
    /// Suite to run [standard, minimal]
    #[arg(short, long, default_value = "minimal")]
    suite: String,

    /// Load custom suite from file
    #[arg(long)]
    suite_file: Option<PathBuf>,

    /// Load baseline results directory for regression testing
    #[arg(long)]
    baseline_dir: Option<PathBuf>,

    /// Output directory for results
    #[arg(short, long, default_value = "benchmark_results")]
    output_dir: PathBuf,

    /// Fail on any benchmark failure (for CI)
    #[arg(long)]
    fail_on_error: bool,

    /// Maximum regression percentage before failing (default 10%)
    #[arg(long, default_value = "10.0")]
    max_regression: f64,

    /// Enable verbose logging
    #[arg(short, long)]
    verbose: bool,

    /// Log level (trace, debug, info, warn, error)
    #[arg(long, default_value = "info")]
    log_level: String,

    /// Generate JSON output for CI integration
    #[arg(long)]
    json_output: bool,

    /// Output file for JSON results
    #[arg(long)]
    json_file: Option<PathBuf>,
}

fn setup_logging(verbose: bool, log_level: &str) -> Result<()> {
    let level = match log_level.to_lowercase().as_str() {
        "trace" => Level::TRACE,
        "debug" => Level::DEBUG,
        "info" => Level::INFO,
        "warn" => Level::WARN,
        "error" => Level::ERROR,
        _ => Level::INFO,
    };

    let env_filter = if verbose {
        EnvFilter::from_default_env()
            .add_directive("quantaterm_benchmarks=trace".parse()?)
            .add_directive("benchmark_runner=trace".parse()?)
    } else {
        EnvFilter::from_default_env()
            .add_directive(format!("quantaterm_benchmarks={}", level).parse()?)
            .add_directive(format!("benchmark_runner={}", level).parse()?)
    };

    tracing_subscriber::registry()
        .with(
            fmt::layer()
                .with_target(false)
                .with_ansi(!std::env::var("NO_COLOR").is_ok()),
        )
        .with(env_filter)
        .init();

    Ok(())
}

#[derive(serde::Serialize)]
struct CiOutput {
    success: bool,
    total_benchmarks: usize,
    passed_benchmarks: usize,
    failed_benchmarks: usize,
    pass_rate: f64,
    regressions: Vec<RegressionInfo>,
    summary: String,
}

#[derive(serde::Serialize)]
struct RegressionInfo {
    benchmark_name: String,
    regression_percentage: f64,
    exceeds_threshold: bool,
}

#[tokio::main]
async fn main() -> Result<()> {
    let cli = Cli::parse();

    setup_logging(cli.verbose, &cli.log_level)?;

    info!("Starting automated benchmark runner");

    let mut harness = PerformanceHarness::new();

    // Load baselines if provided
    if let Some(baseline_dir) = &cli.baseline_dir {
        if baseline_dir.exists() {
            info!("Loading baseline results from: {}", baseline_dir.display());
            for entry in std::fs::read_dir(baseline_dir)? {
                let entry = entry?;
                if entry.path().extension() == Some(std::ffi::OsStr::new("json")) {
                    if let Ok(baseline) = ResultsManager::load_results(entry.path()) {
                        info!("Loaded baseline for: {}", baseline.config.name);
                        harness.load_baseline(baseline);
                    }
                }
            }
        } else {
            info!("Baseline directory does not exist, running without regression testing");
        }
    }

    // Load and run benchmark suite
    let suite = if let Some(suite_file) = cli.suite_file {
        info!("Loading custom suite from: {}", suite_file.display());
        BenchmarkSuite::load_from_file(suite_file)?
    } else {
        info!("Running {} suite", cli.suite);
        match cli.suite.as_str() {
            "standard" => BenchmarkSuite::standard_suite(),
            "minimal" => BenchmarkSuite::minimal_suite(),
            _ => anyhow::bail!("Unknown suite: {}", cli.suite),
        }
    };

    info!("Suite: {}", suite.name);
    info!("Benchmarks: {}", suite.benchmarks.len());

    let results = suite.run(&mut harness).await?;

    // Save results
    std::fs::create_dir_all(&cli.output_dir)?;
    ResultsManager::save_suite_results(&results, &cli.output_dir)?;

    // Generate report
    let report = ResultsManager::generate_summary_report(&results);
    let report_path = cli.output_dir.join("benchmark_report.md");
    std::fs::write(&report_path, &report)?;

    // Analyze results
    let total_benchmarks = results.len();
    let passed_benchmarks = results.iter().filter(|r| r.passed).count();
    let failed_benchmarks = total_benchmarks - passed_benchmarks;
    let pass_rate = (passed_benchmarks as f64 / total_benchmarks as f64) * 100.0;

    // Check for regressions
    let mut regressions = Vec::new();
    for result in &results {
        if let Some(regression) = result.summary.regression_percentage {
            let exceeds_threshold = regression > cli.max_regression;
            regressions.push(RegressionInfo {
                benchmark_name: result.config.name.clone(),
                regression_percentage: regression,
                exceeds_threshold,
            });
        }
    }

    let has_significant_regressions = regressions.iter().any(|r| r.exceeds_threshold);

    // Print results
    println!("\n═══ BENCHMARK RESULTS ═══");
    println!("Suite: {}", suite.name);
    println!("Total benchmarks: {}", total_benchmarks);
    println!("Passed: {} ✅", passed_benchmarks);
    println!("Failed: {} ❌", failed_benchmarks);
    println!("Pass rate: {:.1}%", pass_rate);

    if !regressions.is_empty() {
        println!("\n═══ REGRESSION ANALYSIS ═══");
        for regression in &regressions {
            let status = if regression.exceeds_threshold {
                "⚠️ "
            } else {
                "ℹ️ "
            };
            println!(
                "{}{}: {:.1}% regression",
                status, regression.benchmark_name, regression.regression_percentage
            );
        }
    }

    println!("\n═══ INDIVIDUAL RESULTS ═══");
    for result in &results {
        let status = if result.passed { "✅" } else { "❌" };
        let regression_info = if let Some(reg) = result.summary.regression_percentage {
            format!(" ({:.1}% regression)", reg)
        } else {
            String::new()
        };

        println!(
            "{} {}: {:.2}ms latency, {:.0} B/s throughput, {:.1}% frame drops{}",
            status,
            result.config.name,
            result.summary.avg_latency_ms,
            result.summary.avg_throughput,
            result.summary.avg_frame_drop_percentage,
            regression_info
        );
    }

    println!("\nReport saved to: {}", report_path.display());

    // Generate CI output if requested
    if cli.json_output {
        let ci_output = CiOutput {
            success: failed_benchmarks == 0 && !has_significant_regressions,
            total_benchmarks,
            passed_benchmarks,
            failed_benchmarks,
            pass_rate,
            regressions,
            summary: format!(
                "{}/{} benchmarks passed ({:.1}% pass rate)",
                passed_benchmarks, total_benchmarks, pass_rate
            ),
        };

        let json = serde_json::to_string_pretty(&ci_output)?;

        if let Some(json_file) = cli.json_file {
            std::fs::write(&json_file, &json)?;
            info!("CI output saved to: {}", json_file.display());
        } else {
            println!("\n═══ CI OUTPUT ═══");
            println!("{}", json);
        }
    }

    // Exit with appropriate code for CI
    if cli.fail_on_error {
        if failed_benchmarks > 0 {
            error!("{} benchmark(s) failed", failed_benchmarks);
            std::process::exit(1);
        }

        if has_significant_regressions {
            error!(
                "Significant performance regressions detected (>{:.1}%)",
                cli.max_regression
            );
            std::process::exit(1);
        }
    }

    info!("Benchmark runner completed successfully");
    Ok(())
}
