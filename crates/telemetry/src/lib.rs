//! QuantaTerm Telemetry and metrics collection
//!
//! Performance metrics, usage analytics, and structured logging support.

#![warn(missing_docs)]
#![deny(unsafe_code)]

use serde::{Deserialize, Serialize};
use std::collections::HashMap;
use std::time::{Duration, Instant};
use tracing::{debug, info, instrument, warn};

/// Performance metrics collector
#[derive(Debug)]
pub struct Telemetry {
    /// Performance counters
    counters: HashMap<String, u64>,
    /// Timing measurements  
    timings: HashMap<String, Vec<Duration>>,
    /// Start time for measurements
    start_times: HashMap<String, Instant>,
    /// Whether telemetry collection is enabled
    enabled: bool,
}

/// Telemetry event types
#[derive(Debug, Clone, Serialize, Deserialize)]
pub enum TelemetryEvent {
    /// Application lifecycle events
    AppStart {
        /// Application version
        version: String,
        /// Platform information
        platform: String,
    },
    /// Application shutdown
    AppShutdown {
        /// Uptime in seconds
        uptime_seconds: u64,
    },
    /// Rendering performance
    RenderFrame {
        /// Frame time in milliseconds
        frame_time_ms: f64,
        /// FPS
        fps: f32,
    },
    /// PTY operation
    PtyOperation {
        /// Operation type
        operation: String,
        /// Duration in milliseconds
        duration_ms: f64,
        /// Success status
        success: bool,
    },
    /// Terminal resize
    TerminalResize {
        /// New width in columns
        width: u16,
        /// New height in rows
        height: u16,
    },
    /// Memory usage snapshot
    MemoryUsage {
        /// RSS memory in bytes
        rss_bytes: u64,
        /// Virtual memory in bytes
        virtual_bytes: u64,
    },
}

impl Telemetry {
    /// Create a new telemetry instance
    pub fn new() -> Self {
        info!(subsystem = "telemetry", "Initializing telemetry collection");
        Self {
            counters: HashMap::new(),
            timings: HashMap::new(),
            start_times: HashMap::new(),
            enabled: true,
        }
    }

    /// Create a disabled telemetry instance
    pub fn disabled() -> Self {
        info!(subsystem = "telemetry", "Telemetry collection disabled");
        Self {
            counters: HashMap::new(),
            timings: HashMap::new(),
            start_times: HashMap::new(),
            enabled: false,
        }
    }

    /// Check if telemetry is enabled
    pub fn is_enabled(&self) -> bool {
        self.enabled
    }

    /// Enable or disable telemetry collection
    #[instrument(name = "telemetry_set_enabled", skip(self))]
    pub fn set_enabled(&mut self, enabled: bool) {
        info!(
            subsystem = "telemetry",
            enabled = enabled,
            "Telemetry collection toggled"
        );
        self.enabled = enabled;
    }

    /// Record a telemetry event
    #[instrument(name = "telemetry_record_event", skip(self))]
    pub fn record_event(&mut self, event: TelemetryEvent) {
        if !self.enabled {
            return;
        }

        match &event {
            TelemetryEvent::AppStart { version, platform } => {
                info!(
                    subsystem = "telemetry",
                    event_type = "app_start",
                    version = version,
                    platform = platform,
                    "Application started"
                );
            }
            TelemetryEvent::AppShutdown { uptime_seconds } => {
                info!(
                    subsystem = "telemetry",
                    event_type = "app_shutdown",
                    uptime_seconds = uptime_seconds,
                    "Application shutting down"
                );
            }
            TelemetryEvent::RenderFrame { frame_time_ms, fps } => {
                debug!(
                    subsystem = "telemetry",
                    event_type = "render_frame",
                    frame_time_ms = frame_time_ms,
                    fps = fps,
                    "Frame rendered"
                );
            }
            TelemetryEvent::PtyOperation {
                operation,
                duration_ms,
                success,
            } => {
                debug!(
                    subsystem = "telemetry",
                    event_type = "pty_operation",
                    operation = operation,
                    duration_ms = duration_ms,
                    success = success,
                    "PTY operation completed"
                );
            }
            TelemetryEvent::TerminalResize { width, height } => {
                info!(
                    subsystem = "telemetry",
                    event_type = "terminal_resize",
                    width = width,
                    height = height,
                    "Terminal resized"
                );
            }
            TelemetryEvent::MemoryUsage {
                rss_bytes,
                virtual_bytes,
            } => {
                debug!(
                    subsystem = "telemetry",
                    event_type = "memory_usage",
                    rss_bytes = rss_bytes,
                    virtual_bytes = virtual_bytes,
                    rss_mb = rss_bytes / 1024 / 1024,
                    virtual_mb = virtual_bytes / 1024 / 1024,
                    "Memory usage snapshot"
                );
            }
        }
    }

    /// Increment a counter
    #[instrument(name = "telemetry_increment_counter", skip(self))]
    pub fn increment_counter(&mut self, name: &str) {
        if !self.enabled {
            return;
        }

        let count = self.counters.entry(name.to_string()).or_insert(0);
        *count += 1;

        debug!(
            subsystem = "telemetry",
            counter_name = name,
            count = *count,
            "Counter incremented"
        );
    }

    /// Add a counter value
    #[instrument(name = "telemetry_add_counter", skip(self))]
    pub fn add_counter(&mut self, name: &str, value: u64) {
        if !self.enabled {
            return;
        }

        let count = self.counters.entry(name.to_string()).or_insert(0);
        *count += value;

        debug!(
            subsystem = "telemetry",
            counter_name = name,
            added_value = value,
            total_count = *count,
            "Counter added"
        );
    }

    /// Start timing an operation
    #[instrument(name = "telemetry_start_timing", skip(self))]
    pub fn start_timing(&mut self, name: &str) {
        if !self.enabled {
            return;
        }

        self.start_times.insert(name.to_string(), Instant::now());
        debug!(
            subsystem = "telemetry",
            timing_name = name,
            "Started timing operation"
        );
    }

    /// End timing an operation and record the duration
    #[instrument(name = "telemetry_end_timing", skip(self))]
    pub fn end_timing(&mut self, name: &str) -> Option<Duration> {
        if !self.enabled {
            return None;
        }

        if let Some(start_time) = self.start_times.remove(name) {
            let duration = start_time.elapsed();
            self.timings
                .entry(name.to_string())
                .or_default()
                .push(duration);

            debug!(
                subsystem = "telemetry",
                timing_name = name,
                duration_ms = duration.as_millis(),
                "Completed timing operation"
            );

            Some(duration)
        } else {
            warn!(
                subsystem = "telemetry",
                timing_name = name,
                "End timing called without corresponding start timing"
            );
            None
        }
    }

    /// Get counter value
    pub fn get_counter(&self, name: &str) -> u64 {
        self.counters.get(name).copied().unwrap_or(0)
    }

    /// Get timing statistics
    pub fn get_timing_stats(&self, name: &str) -> Option<TimingStats> {
        if let Some(durations) = self.timings.get(name) {
            if durations.is_empty() {
                return None;
            }

            let sum: Duration = durations.iter().sum();
            let count = durations.len();
            let avg = sum / count as u32;

            let mut sorted_durations = durations.clone();
            sorted_durations.sort();

            let min = sorted_durations[0];
            let max = sorted_durations[count - 1];
            let median = sorted_durations[count / 2];

            Some(TimingStats {
                count,
                min,
                max,
                avg,
                median,
                total: sum,
            })
        } else {
            None
        }
    }

    /// Get all metrics as a summary
    #[instrument(name = "telemetry_get_summary", skip(self))]
    pub fn get_summary(&self) -> TelemetrySummary {
        let counter_summary: HashMap<String, u64> = self.counters.clone();
        let timing_summary: HashMap<String, TimingStats> = self
            .timings
            .keys()
            .filter_map(|name| {
                self.get_timing_stats(name)
                    .map(|stats| (name.clone(), stats))
            })
            .collect();

        debug!(
            subsystem = "telemetry",
            counter_count = counter_summary.len(),
            timing_count = timing_summary.len(),
            "Generated telemetry summary"
        );

        TelemetrySummary {
            counters: counter_summary,
            timings: timing_summary,
        }
    }

    /// Clear all metrics
    #[instrument(name = "telemetry_clear", skip(self))]
    pub fn clear(&mut self) {
        info!(
            subsystem = "telemetry",
            cleared_counters = self.counters.len(),
            cleared_timings = self.timings.len(),
            "Cleared all telemetry data"
        );

        self.counters.clear();
        self.timings.clear();
        self.start_times.clear();
    }
}

/// Timing statistics
#[derive(Debug, Clone)]
pub struct TimingStats {
    /// Number of measurements
    pub count: usize,
    /// Minimum duration
    pub min: Duration,
    /// Maximum duration  
    pub max: Duration,
    /// Average duration
    pub avg: Duration,
    /// Median duration
    pub median: Duration,
    /// Total duration
    pub total: Duration,
}

/// Complete telemetry summary
#[derive(Debug, Clone)]
pub struct TelemetrySummary {
    /// Counter values
    pub counters: HashMap<String, u64>,
    /// Timing statistics
    pub timings: HashMap<String, TimingStats>,
}

impl Default for Telemetry {
    fn default() -> Self {
        Self::new()
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::thread;

    #[test]
    fn test_telemetry_creation() {
        let telemetry = Telemetry::new();
        assert!(telemetry.is_enabled());

        let disabled = Telemetry::disabled();
        assert!(!disabled.is_enabled());
    }

    #[test]
    fn test_counter_operations() {
        let mut telemetry = Telemetry::new();

        telemetry.increment_counter("test_counter");
        assert_eq!(telemetry.get_counter("test_counter"), 1);

        telemetry.add_counter("test_counter", 5);
        assert_eq!(telemetry.get_counter("test_counter"), 6);

        assert_eq!(telemetry.get_counter("nonexistent"), 0);
    }

    #[test]
    fn test_timing_operations() {
        let mut telemetry = Telemetry::new();

        telemetry.start_timing("test_operation");
        thread::sleep(Duration::from_millis(10));
        let duration = telemetry.end_timing("test_operation");

        assert!(duration.is_some());
        assert!(duration.unwrap() >= Duration::from_millis(10));

        let stats = telemetry.get_timing_stats("test_operation");
        assert!(stats.is_some());
        assert_eq!(stats.unwrap().count, 1);
    }

    #[test]
    fn test_disabled_telemetry() {
        let mut telemetry = Telemetry::disabled();

        telemetry.increment_counter("test");
        assert_eq!(telemetry.get_counter("test"), 0);

        telemetry.start_timing("test");
        let duration = telemetry.end_timing("test");
        assert!(duration.is_none());
    }

    #[test]
    fn test_event_recording() {
        let mut telemetry = Telemetry::new();

        // Test various events (mainly for compilation and no panics)
        telemetry.record_event(TelemetryEvent::AppStart {
            version: "0.1.0".to_string(),
            platform: "test".to_string(),
        });

        telemetry.record_event(TelemetryEvent::RenderFrame {
            frame_time_ms: 16.67,
            fps: 60.0,
        });

        telemetry.record_event(TelemetryEvent::TerminalResize {
            width: 80,
            height: 24,
        });
    }

    #[test]
    fn test_telemetry_summary() {
        let mut telemetry = Telemetry::new();

        telemetry.increment_counter("frames");
        telemetry.increment_counter("frames");
        telemetry.add_counter("bytes_transferred", 1024);

        telemetry.start_timing("render");
        thread::sleep(Duration::from_millis(1));
        telemetry.end_timing("render");

        let summary = telemetry.get_summary();
        assert_eq!(summary.counters.get("frames"), Some(&2));
        assert_eq!(summary.counters.get("bytes_transferred"), Some(&1024));
        assert!(summary.timings.contains_key("render"));
    }
}
