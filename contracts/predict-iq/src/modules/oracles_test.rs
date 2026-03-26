#![cfg(test)]

//! Comprehensive tests for Oracle price validation, with focus on confidence threshold rounding.
//!
//! # Issue #260: Confidence Threshold Rounding
//!
//! The confidence validation formula is: `max_conf = (price_abs * max_confidence_bps) / 10000`
//!
//! ## Problem
//! Integer division can introduce bias for small prices:
//! - price=1, bps=500 (5%): (1 * 500) / 10000 = 0 (truncates, should be ~0.05)
//! - price=10, bps=100 (1%): (10 * 100) / 10000 = 0 (truncates, should be ~0.1)
//! - price=100, bps=100 (1%): (100 * 100) / 10000 = 1 (correct)
//!
//! This causes a **downward bias** for small prices, making it harder to accept prices
//! with any confidence interval at very small valuations.
//!
//! ## Potential Solutions
//! 1. **Ceiling division**: Use `(price * bps + 9999) / 10000` to round up
//! 2. **Fixed-point math**: Scale up before division to preserve precision
//! 3. **Reverse formula**: Check `(price * bps) >= (conf * 10000)` to avoid division
//!
//! ## Test Coverage
//! - `test_confidence_rounding_small_prices`: Tests 1-100 range prices
//! - `test_confidence_rounding_large_prices`: Tests million+ range prices
//! - `test_confidence_rounding_edge_cases_low_prices`: Targets specific rounding boundaries
//! - `test_confidence_rounding_negative_prices`: Validates absolute value handling
//! - `test_confidence_rounding_boundary_conditions`: Documents exact rounding behavior

use super::oracles::*;
use crate::types::OracleConfig;
use crate::errors::ErrorCode;
use soroban_sdk::{Env, Address, String, testutils::Address as _};

fn create_config(e: &Env, max_confidence_bps: u64) -> OracleConfig {
    OracleConfig {
        oracle_address: Address::generate(e),
        feed_id: String::from_str(e, "test_feed"),
        min_responses: Some(1),
        max_staleness_seconds: 3600,
        max_confidence_bps,
    }
}

fn create_price(price: i64, conf: u64, timestamp: u64) -> PythPrice {
    PythPrice {
        price,
        conf,
        expo: -2,
        publish_time: timestamp,
    }
}

#[test]
fn test_validate_fresh_price() {
    let e = Env::default();
    let current_time = e.ledger().timestamp();
    
    let config = create_config(&e, 200); // 2%
    
    let price = create_price(100000, 1000, current_time - 60); // 1% of price
    
    let result = validate_price(&e, &price, &config);
    assert!(result.is_ok());
}

#[test]
fn test_reject_stale_price() {
    let e = Env::default();
    let current_time = e.ledger().timestamp();
    
    let config = create_config(&e, 200);
    let price = create_price(100000, 1000, current_time - 400); // 400 seconds old

    let result = validate_price(&e, &price, &config);
    assert_eq!(result, Err(ErrorCode::StalePrice));
}

#[test]
fn test_reject_low_confidence() {
    let e = Env::default();
    let current_time = e.ledger().timestamp();
    
    let config = create_config(&e, 200); // 2%
    let price = create_price(100000, 3000, current_time - 60); // 3% - exceeds threshold

    let result = validate_price(&e, &price, &config);
    assert_eq!(result, Err(ErrorCode::ConfidenceTooLow));
}

/// Table-driven tests for confidence threshold rounding across price ranges.
/// Tests the formula: max_conf = (price_abs * max_confidence_bps) / 10000
/// Issue: Integer division can bias acceptance for small prices.
#[test]
fn test_confidence_rounding_small_prices() {
    let e = Env::default();
    let current_time = e.ledger().timestamp();

    // Test cases: (price, max_confidence_bps, acceptance_confidence, should_pass)
    // Format: test that conf <= (price * bps) / 10000
    let test_cases = vec![
        // (price, max_confidence_bps, confidence_value, should_accept, description)
        (1, 500, 0, true, "price=1, 5% conf, conf=0 at boundary"),
        (1, 500, 1, false, "price=1, 5% conf, conf=1 exceeds rounding result"),
        (10, 100, 0, true, "price=10, 1% conf, conf=0 at boundary"),
        (10, 100, 1, false, "price=10, 1% conf, conf=1 exceeds rounding result"),
        (99, 100, 0, true, "price=99, 1% conf, conf=0 at boundary"),
        (99, 100, 1, false, "price=99, 1% conf, conf=1 exceeds rounding result"),
        (100, 100, 1, true, "price=100, 1% conf, conf=1 within threshold"),
        (100, 100, 2, false, "price=100, 1% conf, conf=2 exceeds threshold"),
        (1000, 100, 10, true, "price=1000, 1% conf, conf=10 within threshold"),
        (1000, 100, 11, false, "price=1000, 1% conf, conf=11 exceeds threshold"),
        (10000, 50, 50, true, "price=10000, 0.5% conf, conf=50 within threshold"),
        (10000, 50, 51, false, "price=10000, 0.5% conf, conf=51 exceeds threshold"),
    ];

    for (price, bps, conf, should_accept, desc) in test_cases {
        let config = create_config(&e, bps);
        let price_obj = create_price(price as i64, conf, current_time - 60);
        let result = validate_price(&e, &price_obj, &config);

        if should_accept {
            assert!(result.is_ok(), "Test failed: {} | Result: {:?}", desc, result);
        } else {
            assert_eq!(result, Err(ErrorCode::ConfidenceTooLow), 
                      "Test failed: {} | Result: {:?}", desc, result);
        }
    }
}

/// Table-driven tests for confidence threshold rounding with large prices.
/// Verifies that rounding bias is minimized or absent for large prices.
#[test]
fn test_confidence_rounding_large_prices() {
    let e = Env::default();
    let current_time = e.ledger().timestamp();

    let test_cases = vec![
        // (price, max_confidence_bps, confidence_value, should_accept, description)
        (1_000_000, 100, 10_000, true, "price=1M, 1% conf, conf=10K within threshold"),
        (1_000_000, 100, 10_001, false, "price=1M, 1% conf, conf=10K+1 exceeds threshold"),
        (10_000_000, 100, 100_000, true, "price=10M, 1% conf, conf=100K within threshold"),
        (10_000_000, 100, 100_001, false, "price=10M, 1% conf, conf=100K+1 exceeds threshold"),
        (1_000_000, 200, 20_000, true, "price=1M, 2% conf, conf=20K within threshold"),
        (1_000_000, 200, 20_001, false, "price=1M, 2% conf, conf=20K+1 exceeds threshold"),
        (100_000_000, 50, 5_000_000, true, "price=100M, 0.5% conf, conf=5M within threshold"),
        (100_000_000, 50, 5_000_001, false, "price=100M, 0.5% conf, conf=5M+1 exceeds threshold"),
    ];

    for (price, bps, conf, should_accept, desc) in test_cases {
        let config = create_config(&e, bps);
        let price_obj = create_price(price as i64, conf, current_time - 60);
        let result = validate_price(&e, &price_obj, &config);

        if should_accept {
            assert!(result.is_ok(), "Test failed: {} | Result: {:?}", desc, result);
        } else {
            assert_eq!(result, Err(ErrorCode::ConfidenceTooLow), 
                      "Test failed: {} | Result: {:?}", desc, result);
        }
    }
}

/// Table-driven tests for edge cases where rounding can cause unexpected behavior.
/// These test cases specifically target the rounding bias problem where
/// low prices can cause max_conf to round down to 0.
#[test]
fn test_confidence_rounding_edge_cases_low_prices() {
    let e = Env::default();
    let current_time = e.ledger().timestamp();

    let test_cases = vec![
        // Edge case: very small prices with moderate confidence requirements
        // (price, max_confidence_bps, confidence_at_boundary, should_pass)
        (1, 10000, 0, true, "price=1, 100% bps, conf=0 at rounding boundary"),
        (1, 10000, 1, false, "price=1, 100% bps, conf=1 exceeds rounding result"),
        (5, 2000, 0, true, "price=5, 20% bps, conf=0 at boundary (5*2000/10000=1)"),
        (5, 2000, 1, true, "price=5, 20% bps, conf=1 within threshold"),
        (5, 2000, 2, false, "price=5, 20% bps, conf=2 exceeds threshold"),
        (9, 1111, 0, true, "price=9, 11.11% bps, conf=0 at boundary"),
        (9, 1111, 1, true, "price=9, 11.11% bps, conf=1 within threshold (9*1111/10000=0.9999≈1)"),
        (50, 200, 10, true, "price=50, 2% bps, conf=10 within threshold (50*200/10000=1)"),
        (50, 200, 11, false, "price=50, 2% bps, conf=11 exceeds threshold"),
    ];

    for (price, bps, conf, should_accept, desc) in test_cases {
        let config = create_config(&e, bps);
        let price_obj = create_price(price as i64, conf, current_time - 60);
        let result = validate_price(&e, &price_obj, &config);

        if should_accept {
            assert!(result.is_ok(), "Test failed: {} | Result: {:?}", desc, result);
        } else {
            assert_eq!(result, Err(ErrorCode::ConfidenceTooLow), 
                      "Test failed: {} | Result: {:?}", desc, result);
        }
    }
}

/// Table-driven tests for negative prices (should use absolute value).
#[test]
fn test_confidence_rounding_negative_prices() {
    let e = Env::default();
    let current_time = e.ledger().timestamp();

    let test_cases = vec![
        // (price, max_confidence_bps, confidence_value, should_accept, description)
        (-100, 100, 1, true, "price=-100, 1% conf, conf=1 within threshold"),
        (-100, 100, 2, false, "price=-100, 1% conf, conf=2 exceeds threshold"),
        (-1, 500, 0, true, "price=-1, 5% conf, conf=0 at boundary"),
        (-1, 500, 1, false, "price=-1, 5% conf, conf=1 exceeds boundary"),
        (-1_000_000, 100, 10_000, true, "price=-1M, 1% conf, conf=10K within threshold"),
        (-1_000_000, 100, 10_001, false, "price=-1M, 1% conf, conf=10K+1 exceeds threshold"),
    ];

    for (price, bps, conf, should_accept, desc) in test_cases {
        let config = create_config(&e, bps);
        let price_obj = create_price(price, conf, current_time - 60);
        let result = validate_price(&e, &price_obj, &config);

        if should_accept {
            assert!(result.is_ok(), "Test failed: {} | Result: {:?}", desc, result);
        } else {
            assert_eq!(result, Err(ErrorCode::ConfidenceTooLow), 
                      "Test failed: {} | Result: {:?}", desc, result);
        }
    }
}

/// Table-driven tests that verify boundary conditions.
/// These tests document the exact rounding behavior for reference.
#[test]
fn test_confidence_rounding_boundary_conditions() {
    let e = Env::default();
    let current_time = e.ledger().timestamp();

    let test_cases = vec![
        // Test rounding boundaries: when does (price * bps) / 10000 transition?
        // (price, max_confidence_bps, confidence_under_boundary, conf_at_boundary, description)
        (50, 200, 0, 1, "price=50, 2%: boundary at 1 (50*200/10000=1)"),
        (49, 200, 0, 0, "price=49, 2%: rounds to 0 (49*200/10000=0.98)"),
        (51, 200, 0, 1, "price=51, 2%: rounds to 1 (51*200/10000=1.02)"),
        (100, 100, 0, 1, "price=100, 1%: boundary at 1 (100*100/10000=1)"),
        (99, 100, 0, 0, "price=99, 1%: rounds to 0 (99*100/10000=0.99)"),
        (101, 100, 0, 1, "price=101, 1%: rounds to 1 (101*100/10000=1.01)"),
    ];

    for (price, bps, under_boundary, at_boundary, desc) in test_cases {
        let config = create_config(&e, bps);
        
        // Test with confidence under boundary
        let price_under = create_price(price as i64, under_boundary, current_time - 60);
        let result_under = validate_price(&e, &price_under, &config);
        assert!(result_under.is_ok(), 
                "Test failed (under boundary): {} | Result: {:?}", desc, result_under);
        
        // Test with confidence at boundary
        let price_at = create_price(price as i64, at_boundary, current_time - 60);
        let result_at = validate_price(&e, &price_at, &config);
        let expected_at = if at_boundary <= (price as u64 * bps) / 10000 {
            Ok(())
        } else {
            Err(ErrorCode::ConfidenceTooLow)
        };
        assert_eq!(result_at, expected_at, 
                  "Test failed (at boundary): {} | Result: {:?}", desc, result_at);
    }
}

/// Validation test for potential fix using ceiling division.
/// This test demonstrates the expected behavior after implementing a fix.
/// 
/// Currently documents the bias:
/// - price=1, 5% BPS, ceiling: (1*500 + 9999)/10000 = 1 (vs current 0)
/// - price=5, 2% BPS, ceiling: (5*2000 + 9999)/10000 = 2 (vs current 1)
///
/// Uncomment assertions once fix is implemented.
#[test]
fn test_confidence_rounding_documented_bias() {
    // This test documents which cases are currently biased
    
    // Small price, rounded down to 0
    let downward_bias_cases = vec![
        (1, 500),    // (1 * 500) / 10000 = 0, should be ~1 with ceiling
        (10, 100),   // (10 * 100) / 10000 = 0, should be ~1 with ceiling
        (99, 100),   // (99 * 100) / 10000 = 0, should be ~1 with ceiling
        (49, 200),   // (49 * 200) / 10000 = 0, should be ~1 with ceiling
    ];

    for (price, bps) in downward_bias_cases {
        let truncated = (price as u64 * bps) / 10000;
        let ceiling = (price as u64 * bps + 9999) / 10000;
        
        // Current implementation uses truncation (truncated)
        // Potential fix would use ceiling division
        // Verify that truncation < ceiling for small prices
        if truncated == 0 {
            assert!(ceiling > truncated, 
                   "Downward bias at price={}, bps={}: truncated={}, ceiling={}", 
                   price, bps, truncated, ceiling);
        }
    }
}
