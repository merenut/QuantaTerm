//! Integration tests for QuantaTerm

use quantaterm_core::logging::{ci_config, dev_config, prod_config, LogLevel};

pub mod sgr_integration;
pub mod vttest_subset;
pub mod color_rendering_demo;

#[test]
fn test_logging_configurations() {
    // Test different configurations compile and have expected properties
    let dev_config = dev_config();
    assert_eq!(dev_config.global_level, LogLevel::Debug);
    assert!(!dev_config.json_format);
    assert!(dev_config.use_colors);

    let prod_config = prod_config();
    assert_eq!(prod_config.global_level, LogLevel::Info);
    assert!(prod_config.json_format);
    assert!(!prod_config.use_colors);

    let ci_config = ci_config();
    assert_eq!(ci_config.global_level, LogLevel::Info);
    assert!(ci_config.json_format);
    assert!(!ci_config.use_colors);
    assert!(ci_config.include_timestamps);
}
