//! Resource limits and monitoring for plugin execution
//! 
//! This module defines execution limits for WASM plugins to ensure they cannot
//! consume excessive resources or impact system performance.

use std::time::{Duration, Instant};
use thiserror::Error;

/// Resource limits for plugin execution
#[derive(Debug, Clone)]
pub struct ExecutionLimits {
    /// Maximum memory allocation (bytes)
    pub max_memory: u64,
    /// Maximum execution time per call
    pub max_time: Duration,
    /// Maximum fuel units (computational complexity)
    pub max_fuel: u64,
    /// Maximum file descriptor count
    pub max_file_handles: u32,
    /// Maximum network connections
    pub max_network_connections: u32,
}

impl Default for ExecutionLimits {
    fn default() -> Self {
        Self {
            max_memory: 16 * 1024 * 1024,           // 16MB
            max_time: Duration::from_millis(100),    // 100ms
            max_fuel: 1_000_000,                     // 1M instructions
            max_file_handles: 10,
            max_network_connections: 5,
        }
    }
}

impl ExecutionLimits {
    /// Create limits for development/testing (more generous)
    pub fn development() -> Self {
        Self {
            max_memory: 64 * 1024 * 1024,           // 64MB
            max_time: Duration::from_millis(1000),   // 1s
            max_fuel: 10_000_000,                    // 10M instructions
            max_file_handles: 50,
            max_network_connections: 20,
        }
    }
    
    /// Create strict limits for production
    pub fn production() -> Self {
        Self {
            max_memory: 8 * 1024 * 1024,            // 8MB
            max_time: Duration::from_millis(50),     // 50ms
            max_fuel: 500_000,                       // 500K instructions
            max_file_handles: 5,
            max_network_connections: 2,
        }
    }
}

/// Monitor resource usage during execution
#[derive(Debug)]
pub struct ResourceMonitor {
    start_time: Instant,
    limits: ExecutionLimits,
    memory_usage: u64,
    file_handles: u32,
    network_connections: u32,
}

/// Errors related to resource limits
#[derive(Debug, Error)]
pub enum LimitError {
    #[error("Memory limit exceeded: {used} bytes > {limit} bytes")]
    MemoryLimit { used: u64, limit: u64 },
    
    #[error("Execution timeout after {duration:?}")]
    Timeout { duration: Duration },
    
    #[error("Fuel exhausted: computation limit reached")]
    FuelExhausted,
    
    #[error("Too many file handles: {used} > {limit}")]
    FileHandleLimit { used: u32, limit: u32 },
    
    #[error("Too many network connections: {used} > {limit}")]
    NetworkConnectionLimit { used: u32, limit: u32 },
}

impl ResourceMonitor {
    /// Create a new resource monitor with the given limits
    pub fn new(limits: ExecutionLimits) -> Self {
        Self {
            start_time: Instant::now(),
            limits,
            memory_usage: 0,
            file_handles: 0,
            network_connections: 0,
        }
    }
    
    /// Check if execution should continue
    pub fn check_limits(&self) -> Result<(), LimitError> {
        // Check timeout
        let elapsed = self.start_time.elapsed();
        if elapsed > self.limits.max_time {
            return Err(LimitError::Timeout { duration: elapsed });
        }
        
        // Check memory usage
        if self.memory_usage > self.limits.max_memory {
            return Err(LimitError::MemoryLimit {
                used: self.memory_usage,
                limit: self.limits.max_memory,
            });
        }
        
        // Check file handles
        if self.file_handles > self.limits.max_file_handles {
            return Err(LimitError::FileHandleLimit {
                used: self.file_handles,
                limit: self.limits.max_file_handles,
            });
        }
        
        // Check network connections
        if self.network_connections > self.limits.max_network_connections {
            return Err(LimitError::NetworkConnectionLimit {
                used: self.network_connections,
                limit: self.limits.max_network_connections,
            });
        }
        
        Ok(())
    }
    
    /// Update memory usage
    pub fn update_memory_usage(&mut self, bytes: u64) {
        self.memory_usage = bytes;
    }
    
    /// Add a file handle
    pub fn add_file_handle(&mut self) -> Result<(), LimitError> {
        if self.file_handles + 1 > self.limits.max_file_handles {
            return Err(LimitError::FileHandleLimit {
                used: self.file_handles + 1,
                limit: self.limits.max_file_handles,
            });
        }
        self.file_handles += 1;
        Ok(())
    }
    
    /// Remove a file handle
    pub fn remove_file_handle(&mut self) {
        if self.file_handles > 0 {
            self.file_handles -= 1;
        }
    }
    
    /// Add a network connection
    pub fn add_network_connection(&mut self) -> Result<(), LimitError> {
        if self.network_connections + 1 > self.limits.max_network_connections {
            return Err(LimitError::NetworkConnectionLimit {
                used: self.network_connections + 1,
                limit: self.limits.max_network_connections,
            });
        }
        self.network_connections += 1;
        Ok(())
    }
    
    /// Remove a network connection
    pub fn remove_network_connection(&mut self) {
        if self.network_connections > 0 {
            self.network_connections -= 1;
        }
    }
    
    /// Get current execution time
    pub fn elapsed_time(&self) -> Duration {
        self.start_time.elapsed()
    }
    
    /// Get remaining time budget
    pub fn remaining_time(&self) -> Duration {
        self.limits.max_time.saturating_sub(self.elapsed_time())
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_default_limits() {
        let limits = ExecutionLimits::default();
        assert_eq!(limits.max_memory, 16 * 1024 * 1024);
        assert_eq!(limits.max_time, Duration::from_millis(100));
        assert_eq!(limits.max_fuel, 1_000_000);
    }

    #[test]
    fn test_development_limits() {
        let limits = ExecutionLimits::development();
        assert_eq!(limits.max_memory, 64 * 1024 * 1024);
        assert_eq!(limits.max_time, Duration::from_millis(1000));
        assert_eq!(limits.max_fuel, 10_000_000);
    }

    #[test]
    fn test_production_limits() {
        let limits = ExecutionLimits::production();
        assert_eq!(limits.max_memory, 8 * 1024 * 1024);
        assert_eq!(limits.max_time, Duration::from_millis(50));
        assert_eq!(limits.max_fuel, 500_000);
    }

    #[test]
    fn test_resource_monitor_creation() {
        let limits = ExecutionLimits::default();
        let monitor = ResourceMonitor::new(limits);
        assert!(monitor.check_limits().is_ok());
    }

    #[test]
    fn test_memory_limit_check() {
        let limits = ExecutionLimits::default();
        let mut monitor = ResourceMonitor::new(limits);
        
        // Should be ok initially
        assert!(monitor.check_limits().is_ok());
        
        // Exceed memory limit
        monitor.update_memory_usage(17 * 1024 * 1024); // > 16MB
        assert!(matches!(monitor.check_limits(), Err(LimitError::MemoryLimit { .. })));
    }

    #[test]
    fn test_file_handle_tracking() {
        let limits = ExecutionLimits::default();
        let mut monitor = ResourceMonitor::new(limits);
        
        // Add file handles up to limit
        for _ in 0..10 {
            assert!(monitor.add_file_handle().is_ok());
        }
        
        // Adding one more should fail
        assert!(matches!(monitor.add_file_handle(), Err(LimitError::FileHandleLimit { .. })));
        
        // Remove one and try again
        monitor.remove_file_handle();
        assert!(monitor.add_file_handle().is_ok());
    }

    #[test]
    fn test_timeout_detection() {
        let mut limits = ExecutionLimits::default();
        limits.max_time = Duration::from_millis(1); // Very short timeout
        
        let monitor = ResourceMonitor::new(limits);
        
        // Sleep longer than timeout
        std::thread::sleep(Duration::from_millis(5));
        
        assert!(matches!(monitor.check_limits(), Err(LimitError::Timeout { .. })));
    }

    #[test]
    fn test_remaining_time() {
        let limits = ExecutionLimits::default();
        let monitor = ResourceMonitor::new(limits.clone());
        
        let remaining = monitor.remaining_time();
        assert!(remaining <= limits.max_time);
        assert!(remaining > Duration::from_millis(90)); // Should be close to full time
    }
}