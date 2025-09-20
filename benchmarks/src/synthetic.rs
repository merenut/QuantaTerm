//! Synthetic PTY Load Generator
//!
//! This module provides utilities for generating synthetic PTY data for performance testing.

use anyhow::Result;
use rand::prelude::*;
use serde::{Deserialize, Serialize};
use std::time::{Duration, Instant};
use tokio::time;
use tracing::{debug, info, instrument};

/// Configuration for synthetic PTY load generation
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct LoadConfig {
    /// Data generation rate in bytes per second
    pub bytes_per_second: u64,
    /// Duration to generate data for
    pub duration: Duration,
    /// Chunk size for each data burst
    pub chunk_size: usize,
    /// Type of data to generate
    pub data_type: DataType,
    /// Enable burst mode (sudden spikes of data)
    pub burst_mode: bool,
    /// Burst frequency (bursts per second when enabled)
    pub burst_frequency: f64,
    /// Burst size multiplier
    pub burst_multiplier: f64,
}

/// Types of synthetic data to generate
#[derive(Debug, Clone, Serialize, Deserialize)]
pub enum DataType {
    /// Plain text output (like cargo build)
    PlainText,
    /// ANSI escape sequences with colors
    AnsiColors,
    /// Large data paste simulation
    LargePaste,
    /// Random terminal output
    Random,
    /// Continuous scrolling data
    Scrolling,
}

/// Statistics from a load generation session
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct LoadStats {
    /// Total bytes generated
    pub total_bytes: u64,
    /// Actual duration of generation
    pub actual_duration: Duration,
    /// Average bytes per second achieved
    pub avg_bytes_per_second: f64,
    /// Number of chunks sent
    pub chunks_sent: u64,
    /// Number of bursts sent (if burst mode enabled)
    pub bursts_sent: u64,
    /// Peak instantaneous rate (bytes/sec)
    pub peak_rate: f64,
}

impl Default for LoadConfig {
    fn default() -> Self {
        Self {
            bytes_per_second: 10_000, // 10KB/s
            duration: Duration::from_secs(10),
            chunk_size: 1024,
            data_type: DataType::PlainText,
            burst_mode: false,
            burst_frequency: 1.0,
            burst_multiplier: 5.0,
        }
    }
}

impl LoadConfig {
    /// Create a configuration for simulating continuous output (like cargo build)
    pub fn continuous_output(rate: u64, duration: Duration) -> Self {
        Self {
            bytes_per_second: rate,
            duration,
            chunk_size: 512,
            data_type: DataType::PlainText,
            burst_mode: false,
            ..Default::default()
        }
    }

    /// Create a configuration for simulating large paste operations
    pub fn large_paste(total_size: usize) -> Self {
        Self {
            bytes_per_second: total_size as u64, // Send all at once
            duration: Duration::from_millis(100),
            chunk_size: 4096,
            data_type: DataType::LargePaste,
            burst_mode: false,
            ..Default::default()
        }
    }

    /// Create a configuration for burst mode testing
    pub fn burst_mode(base_rate: u64, burst_freq: f64, burst_mult: f64) -> Self {
        Self {
            bytes_per_second: base_rate,
            duration: Duration::from_secs(30),
            chunk_size: 1024,
            data_type: DataType::Random,
            burst_mode: true,
            burst_frequency: burst_freq,
            burst_multiplier: burst_mult,
        }
    }
}

/// Synthetic PTY data generator
pub struct SyntheticGenerator {
    config: LoadConfig,
    rng: StdRng,
}

impl SyntheticGenerator {
    /// Create a new generator with the given configuration
    pub fn new(config: LoadConfig) -> Self {
        Self {
            config,
            rng: StdRng::from_entropy(),
        }
    }

    /// Generate a single chunk of data based on the configuration
    #[instrument(skip(self))]
    pub fn generate_chunk(&mut self) -> Vec<u8> {
        match self.config.data_type {
            DataType::PlainText => self.generate_plain_text(),
            DataType::AnsiColors => self.generate_ansi_colors(),
            DataType::LargePaste => self.generate_large_paste(),
            DataType::Random => self.generate_random_data(),
            DataType::Scrolling => self.generate_scrolling_data(),
        }
    }

    fn generate_plain_text(&mut self) -> Vec<u8> {
        let lines = [
            "   Compiling quantaterm-core v0.1.0\n",
            "   Compiling quantaterm-blocks v0.1.0\n",
            "   Compiling quantaterm-pty v0.1.0\n",
            "   Compiling quantaterm-renderer v0.1.0\n",
            "    Finished dev [unoptimized + debuginfo] target(s) in 2.34s\n",
            "Running tests...\n",
            "test result: ok. 57 passed; 0 failed; 0 ignored; 0 measured\n",
        ];

        let line = lines[self.rng.gen_range(0..lines.len())];
        let mut data = line.as_bytes().to_vec();

        // Fill up to chunk_size with repeated content
        while data.len() < self.config.chunk_size {
            data.extend_from_slice(line.as_bytes());
        }
        data.truncate(self.config.chunk_size);
        data
    }

    fn generate_ansi_colors(&mut self) -> Vec<u8> {
        let colors = [31, 32, 33, 34, 35, 36, 37]; // Red, Green, Yellow, Blue, Magenta, Cyan, White
        let styles = ["1", "4", "7"]; // Bold, Underline, Reverse

        let mut data = Vec::new();

        while data.len() < self.config.chunk_size {
            let color = colors[self.rng.gen_range(0..colors.len())];
            let style = styles[self.rng.gen_range(0..styles.len())];

            let text = format!(
                "\x1b[{};{}mColored text {} \x1b[0m",
                style,
                color,
                self.rng.gen::<u32>()
            );
            data.extend_from_slice(text.as_bytes());
        }

        data.truncate(self.config.chunk_size);
        data
    }

    fn generate_large_paste(&mut self) -> Vec<u8> {
        let sample_text = "This is a large paste operation with lots of text content. ";
        let mut data = Vec::new();

        while data.len() < self.config.chunk_size {
            data.extend_from_slice(sample_text.as_bytes());
        }

        data.truncate(self.config.chunk_size);
        data
    }

    fn generate_random_data(&mut self) -> Vec<u8> {
        let mut data = vec![0u8; self.config.chunk_size];

        // Generate printable ASCII characters
        for byte in &mut data {
            *byte = self.rng.gen_range(32..127); // Printable ASCII range
        }

        // Add some newlines for readability
        for i in (80..data.len()).step_by(80) {
            if i < data.len() {
                data[i] = b'\n';
            }
        }

        data
    }

    fn generate_scrolling_data(&mut self) -> Vec<u8> {
        let mut data = Vec::new();
        let line_count = self.config.chunk_size / 80; // Assume 80 char lines

        for i in 0..line_count {
            let line = format!(
                "Line {:06} - This is scrolling content that fills the terminal buffer\n",
                i
            );
            data.extend_from_slice(line.as_bytes());
        }

        data.truncate(self.config.chunk_size);
        data
    }

    /// Generate data according to the load configuration
    #[instrument(skip(self, callback))]
    pub async fn generate_load<F>(&mut self, mut callback: F) -> Result<LoadStats>
    where
        F: FnMut(Vec<u8>) -> Result<()>,
    {
        info!(
            config = ?self.config,
            "Starting synthetic load generation"
        );

        let start_time = Instant::now();
        let mut total_bytes = 0u64;
        let mut chunks_sent = 0u64;
        let mut bursts_sent = 0u64;
        let mut peak_rate = 0.0f64;

        let target_interval = Duration::from_millis(
            (self.config.chunk_size as f64 / self.config.bytes_per_second as f64 * 1000.0) as u64,
        );

        let mut next_burst = if self.config.burst_mode {
            start_time + Duration::from_millis((1000.0 / self.config.burst_frequency) as u64)
        } else {
            start_time + self.config.duration + Duration::from_secs(1) // Never
        };

        let mut last_chunk_time = start_time;

        while start_time.elapsed() < self.config.duration {
            let now = Instant::now();
            let is_burst = self.config.burst_mode && now >= next_burst;

            if is_burst {
                // Send burst
                let burst_size =
                    (self.config.chunk_size as f64 * self.config.burst_multiplier) as usize;
                let burst_data = vec![b'X'; burst_size]; // Simple burst data

                callback(burst_data)?;
                total_bytes += burst_size as u64;
                bursts_sent += 1;

                // Calculate instantaneous rate
                let time_since_last = now.duration_since(last_chunk_time).as_secs_f64();
                if time_since_last > 0.0 {
                    let rate = burst_size as f64 / time_since_last;
                    peak_rate = peak_rate.max(rate);
                }

                // Schedule next burst
                next_burst =
                    now + Duration::from_millis((1000.0 / self.config.burst_frequency) as u64);
                last_chunk_time = now;

                debug!(burst_size, "Sent burst data");
            } else {
                // Send regular chunk
                let chunk = self.generate_chunk();
                let chunk_size = chunk.len();

                callback(chunk)?;
                total_bytes += chunk_size as u64;
                chunks_sent += 1;

                // Calculate instantaneous rate
                let time_since_last = now.duration_since(last_chunk_time).as_secs_f64();
                if time_since_last > 0.0 {
                    let rate = chunk_size as f64 / time_since_last;
                    peak_rate = peak_rate.max(rate);
                }

                last_chunk_time = now;
            }

            // Wait for next interval (unless we're in burst mode)
            if !is_burst {
                time::sleep(target_interval).await;
            }
        }

        let actual_duration = start_time.elapsed();
        let avg_bytes_per_second = total_bytes as f64 / actual_duration.as_secs_f64();

        let stats = LoadStats {
            total_bytes,
            actual_duration,
            avg_bytes_per_second,
            chunks_sent,
            bursts_sent,
            peak_rate,
        };

        info!(
            stats = ?stats,
            "Completed synthetic load generation"
        );

        Ok(stats)
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::sync::{Arc, Mutex};

    #[tokio::test]
    async fn test_plain_text_generation() {
        let config = LoadConfig {
            bytes_per_second: 1000,
            duration: Duration::from_millis(100),
            chunk_size: 100,
            data_type: DataType::PlainText,
            ..Default::default()
        };

        let mut generator = SyntheticGenerator::new(config);
        let chunk = generator.generate_chunk();

        assert_eq!(chunk.len(), 100);
        assert!(chunk.windows(2).any(|w| w == b"\n ") || chunk.contains(&b'\n'));
    }

    #[tokio::test]
    async fn test_load_generation() {
        let config = LoadConfig {
            bytes_per_second: 5000,
            duration: Duration::from_millis(200),
            chunk_size: 1000,
            data_type: DataType::PlainText,
            ..Default::default()
        };

        let mut generator = SyntheticGenerator::new(config);
        let received_data = Arc::new(Mutex::new(Vec::new()));
        let received_data_clone = Arc::clone(&received_data);

        let stats = generator
            .generate_load(move |data| {
                received_data_clone.lock().unwrap().extend(data);
                Ok(())
            })
            .await
            .unwrap();

        assert!(stats.total_bytes > 0);
        assert!(stats.chunks_sent > 0);
        assert!(stats.avg_bytes_per_second > 0.0);

        let total_received = received_data.lock().unwrap().len();
        assert_eq!(total_received, stats.total_bytes as usize);
    }

    #[tokio::test]
    async fn test_burst_mode() {
        let config = LoadConfig::burst_mode(1000, 2.0, 3.0);
        let mut generator = SyntheticGenerator::new(config);

        let received_data = Arc::new(Mutex::new(Vec::new()));
        let received_data_clone = Arc::clone(&received_data);

        let stats = generator
            .generate_load(move |data| {
                received_data_clone.lock().unwrap().extend(data);
                Ok(())
            })
            .await
            .unwrap();

        assert!(stats.bursts_sent > 0);
        assert!(stats.peak_rate > stats.avg_bytes_per_second);
    }
}
