# QuantaTerm Performance Harness

This document describes the performance harness for QuantaTerm, which provides comprehensive performance testing capabilities for PTY operations, synthetic load generation, and regression testing.

## Overview

The performance harness consists of several components:

1. **Synthetic PTY Load Generator** - Generates configurable synthetic data patterns
2. **Performance Measurement** - Measures latency, throughput, and frame drops
3. **Benchmark Runner** - Automated benchmark execution and regression testing
4. **Results Analysis** - Performance reporting and baseline comparison

## Quick Start

### Running Basic Benchmarks

```bash
# Run minimal benchmark suite (fast, for development)
make bench

# Run full benchmark suite (comprehensive, for releases)
make bench-full

# Generate synthetic PTY load (demo)
make bench-generate
```

### Running Custom Benchmarks

```bash
# Custom benchmark with specific parameters
cd benchmarks
cargo run --bin pty-harness -- benchmark \
    --name "my_test" \
    --rate 100000 \
    --duration 10 \
    --iterations 5

# Generate synthetic data with specific patterns
cargo run --bin pty-harness -- generate \
    --rate 50000 \
    --duration 5 \
    --data-type ansi-colors \
    --burst-mode \
    --burst-frequency 2.0
```

## Performance Targets

Based on `PROJECT_PLAN.md`, the harness validates these performance targets:

| Scenario | Metric | Target |
|----------|--------|--------|
| Continuous output (cargo build) | Dropped frames | 0 until > 200k chars/sec |
| Large paste (100k chars) | Final render completion | < 250 ms |
| Interactive editing | p95 input→render | < 18 ms |
| Startup cold | Time to prompt | < 180 ms |

## Benchmark Suites

### Standard Suite

The standard suite includes comprehensive benchmarks for:
- **continuous_output**: Simulates `cargo build` style output
- **large_paste**: Tests 100k character paste operations
- **interactive_editing**: Simulates text editor interactions
- **burst_load**: Tests bursty load conditions

### Minimal Suite

A faster version of the standard suite with reduced iterations and duration, suitable for:
- Development testing
- CI/CD pipelines
- Quick regression checks

## Data Types

The synthetic generator supports several data patterns:

- **plain-text**: Simulates cargo build output
- **ansi-colors**: ANSI escape sequences with colors
- **large-paste**: Large text paste simulation
- **random**: Random printable ASCII data
- **scrolling**: Continuous scrolling content

## Usage Examples

### 1. Development Testing

```bash
# Quick performance check during development
make bench

# Test specific scenarios
cd benchmarks
cargo run --bin pty-harness -- benchmark \
    --name "dev_test" \
    --rate 10000 \
    --duration 2 \
    --iterations 3
```

### 2. Regression Testing

```bash
# Save current performance as baseline
make bench-baseline

# Later, test for regressions
make bench-regression
```

### 3. CI/CD Integration

```bash
# Run benchmarks with CI-friendly output
cd benchmarks
cargo run --bin benchmark-runner -- \
    --suite minimal \
    --fail-on-error \
    --json-output \
    --json-file ci_results.json
```

### 4. Performance Analysis

```bash
# Generate detailed reports
cd benchmarks
cargo run --bin benchmark-runner -- \
    --suite standard \
    --output-dir results/ \
    --json-output

# Analyze existing results
cargo run --bin pty-harness -- analyze \
    --input results/ \
    --report \
    --output performance_report.md
```

## Configuration

### Benchmark Configuration

Each benchmark can be configured with:

```rust
BenchmarkConfig {
    name: "my_benchmark",
    description: "Custom benchmark description",
    load_config: LoadConfig {
        bytes_per_second: 50000,
        duration: Duration::from_secs(10),
        chunk_size: 1024,
        data_type: DataType::PlainText,
        burst_mode: false,
        // ...
    },
    iterations: 5,
    warmup_iterations: 2,
    max_latency_ms: 50.0,
    max_frame_drop_percentage: 5.0,
    target_throughput: 50000,
}
```

### Load Generation Configuration

```rust
LoadConfig {
    bytes_per_second: 50000,     // Data rate
    duration: Duration::from_secs(10), // Test duration
    chunk_size: 1024,           // Chunk size
    data_type: DataType::PlainText, // Data pattern
    burst_mode: true,           // Enable bursts
    burst_frequency: 1.0,       // Bursts per second
    burst_multiplier: 5.0,      // Burst size multiplier
}
```

## Metrics Collected

### Latency Metrics
- Average, min, max latency
- 95th and 99th percentile latency
- Sample count

### Throughput Metrics
- Average bytes per second
- Peak throughput
- Total bytes processed

### Frame Drop Metrics (Simulated)
- Total frames
- Dropped frames
- Drop percentage
- Target FPS

### Memory Usage
- Peak memory usage
- Average memory usage
- Final memory usage

## Output Formats

### Console Output
Human-readable summaries with status indicators:

```
═══ BENCHMARK RESULTS ═══
Suite: QuantaTerm Minimal Performance Suite
Total benchmarks: 4
Passed: 4 ✅
Failed: 0 ❌
Pass rate: 100.0%

✅ continuous_output: 15.32ms latency, 198542 B/s throughput, 0.0% frame drops
✅ large_paste: 245.67ms latency, 408163 B/s throughput, 2.1% frame drops
✅ interactive_editing: 16.88ms latency, 1000 B/s throughput, 0.5% frame drops
✅ burst_load: 45.23ms latency, 48512 B/s throughput, 3.2% frame drops
```

### JSON Output
Machine-readable format for CI integration:

```json
{
  "success": true,
  "total_benchmarks": 4,
  "passed_benchmarks": 4,
  "failed_benchmarks": 0,
  "pass_rate": 100.0,
  "regressions": [],
  "summary": "4/4 benchmarks passed (100.0% pass rate)"
}
```

### Markdown Reports
Detailed analysis reports with charts and tables:

```markdown
# QuantaTerm Performance Benchmark Report

Generated: 2025-09-20 01:00:00 UTC

## Summary
- **Total Benchmarks:** 4
- **Passed:** 4
- **Failed:** 0
- **Pass Rate:** 100.0%

## Benchmark Results
...
```

## CI/CD Integration

### GitHub Actions Example

```yaml
- name: Run Performance Benchmarks
  run: |
    cd benchmarks
    cargo run --bin benchmark-runner -- \
      --suite minimal \
      --fail-on-error \
      --max-regression 10.0 \
      --json-output \
      --json-file benchmark_results.json

- name: Upload Results
  uses: actions/upload-artifact@v3
  with:
    name: benchmark-results
    path: benchmarks/benchmark_results/
```

### Regression Detection

The harness automatically detects performance regressions by comparing results against saved baselines:

- **Regression threshold**: 10% (configurable)
- **Baseline storage**: JSON files in baseline directory
- **Automatic comparison**: Against previous results
- **CI failure**: When regressions exceed threshold

## Extending the Harness

### Adding New Data Types

```rust
// In synthetic.rs
pub enum DataType {
    // ... existing types
    CustomPattern,
}

// Implement in SyntheticGenerator
fn generate_custom_pattern(&mut self) -> Vec<u8> {
    // Your custom data generation logic
}
```

### Adding New Metrics

```rust
// In harness.rs
pub struct CustomMetrics {
    // Your custom metrics
}

// Integrate into BenchmarkIteration
pub struct BenchmarkIteration {
    // ... existing fields
    pub custom_metrics: CustomMetrics,
}
```

### Custom Benchmark Suites

```rust
let custom_suite = BenchmarkSuite {
    name: "Custom Suite".to_string(),
    description: "Application-specific benchmarks".to_string(),
    benchmarks: vec![
        BenchmarkConfig::custom_config_1(),
        BenchmarkConfig::custom_config_2(),
        // ...
    ],
};
```

## Troubleshooting

### Common Issues

1. **High latency measurements**: Check system load and disable other applications
2. **Inconsistent results**: Increase warmup iterations and reduce system noise
3. **Memory issues**: Monitor system memory and adjust chunk sizes
4. **CI timeouts**: Use minimal suite or reduce test duration

### Performance Tips

1. **Baseline establishment**: Run benchmarks multiple times to establish stable baselines
2. **Environment consistency**: Use dedicated hardware or containers for consistent results
3. **System tuning**: Disable CPU frequency scaling and background processes
4. **Iteration tuning**: Balance between accuracy and execution time

## Future Enhancements

- Real PTY integration (currently simulated)
- GPU performance metrics
- Network latency simulation
- Plugin performance testing
- Visual performance dashboards