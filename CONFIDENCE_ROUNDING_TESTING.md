# Issue #260: Confidence Threshold Rounding Tests

## Overview

This document describes the testing approach for Issue #260, addressing integer rounding bias in confidence threshold validation for oracle price data in PredictIQ.

## Problem Statement

The confidence validation formula in `oracles.rs` is:
```rust
let max_conf = (price_abs * config.max_confidence_bps as u64) / 10000;
```

### The Rounding Bias Issue

When using integer division (`/`), truncation occurs for small values. This creates a **downward bias** that makes it harder to accept prices with confidence intervals at very small valuations.

#### Examples of the Bias

| Price | BPS (%) | Calculation | Result | Issue |
|-------|---------|-------------|--------|-------|
| 1 | 500 (5%) | (1 × 500) / 10000 = 0.05 | **0** | Cannot accept any confidence > 0 |
| 10 | 100 (1%) | (10 × 100) / 10000 = 0.1 | **0** | Cannot accept any confidence > 0 |
| 50 | 200 (2%) | (50 × 200) / 10000 = 1.0 | **1** | Correct (boundary case) |
| 100 | 100 (1%) | (100 × 100) / 10000 = 1.0 | **1** | Correct |
| 1,000,000 | 100 (1%) | (1M × 100) / 10000 = 10,000 | **10,000** | Correct |

### Impact

This bias affects:
- **Small valuations**: Markets with very low prices (e.g., micropredictions, fractional shares)
- **Precision requirements**: Oracle data from price feeds with natural small numbers
- **Oracle acceptance**: Makes it harder to accept oracle data for small-valued markets

## Testing Strategy

The comprehensive test suite in `oracles_test.rs` covers:

### 1. Basic Validation Tests
- `test_validate_fresh_price`: Validates fresh price data is accepted
- `test_reject_stale_price`: Validates stale price data is rejected
- `test_reject_low_confidence`: Validates low-confidence prices are rejected

### 2. Small Price Range Tests (`test_confidence_rounding_small_prices`)
Tests prices from 1 to 10,000 with various BPS values to expose the truncation bias:
- Price = 1, BPS = 500: Truncates to 0
- Price = 10, BPS = 100: Truncates to 0
- Price = 99, BPS = 100: Truncates to 0
- Price = 100, BPS = 100: Correctly = 1
- Price = 1000, BPS = 100: Correctly = 10

### 3. Large Price Range Tests (`test_confidence_rounding_large_prices`)
Tests prices from 1M to 100M to verify minimal bias at larger scales:
- Ensures rounding bias diminishes as prices increase
- Validates expected confidence thresholds don't regress

### 4. Edge Case Tests (`test_confidence_rounding_edge_cases_low_prices`)
Specifically targets boundary conditions:
- (price=1, BPS=100%): Explores maximum confidence requirement
- (price=5, BPS=20%): Tests fractional boundaries
- (price=9, BPS=11.11%): Tests precision edge cases

### 5. Negative Price Tests (`test_confidence_rounding_negative_prices`)
Validates absolute value handling for negative prices:
- Ensures negative prices use correct absolute value
- Tests with small and large negative prices

### 6. Boundary Condition Tests (`test_confidence_rounding_boundary_conditions`)
Documents exact rounding transitions:
- Identifies where truncation starts to affect acceptance
- Provides reference points for rounding behavior

### 7. Bias Documentation Test (`test_confidence_rounding_documented_bias`)
Non-enforcement test that documents known bias cases:
- Lists all cases where truncation < ceiling
- Provides baseline for fix validation

## Table-Driven Test Format

All tests use a table-driven approach with test cases structured as:
```rust
(price, max_confidence_bps, confidence_value, should_accept, description)
```

Example:
```rust
(1, 500, 0, true, "price=1, 5% conf, conf=0 at boundary"),
(1, 500, 1, false, "price=1, 5% conf, conf=1 exceeds rounding result"),
```

## Potential Solutions

Three approaches to fix the rounding bias:

### 1. Ceiling Division (Recommended for Simplicity)
```rust
let max_conf = (price_abs * config.max_confidence_bps + 9999) / 10000;
// OR using a helper:
let max_conf = (price_abs * config.max_confidence_bps + 10000 - 1) / 10000;
```

**Pros**: Simple to implement, minimal performance impact
**Cons**: Slightly relaxes confidence requirements

### 2. Fixed-Point Math (Recommended for Precision)
```rust
// Scale up to preserve precision
let scaled_price = price_abs * 1_000_000;
let scaled_conf = price.conf * 10000;
let max_conf_scaled = (scaled_price * config.max_confidence_bps) / 10000;
if scaled_conf > max_conf_scaled {
    return Err(ErrorCode::ConfidenceTooLow);
}
```

**Pros**: Perfect precision, no rounding
**Cons**: Risk of overflow with very large values

### 3. Reverse Formula (Recommended for Correctness)
```rust
// Instead of: if conf > (price * bps) / 10000
// Use:        if (conf * 10000) > (price * bps)
if price.conf * 10000 > price_abs * config.max_confidence_bps {
    return Err(ErrorCode::ConfidenceTooLow);
}
```

**Pros**: Exact comparison, eliminates rounding entirely
**Cons**: Risk of multiplication overflow

## Test Execution

Run all confidence rounding tests:
```bash
cd contracts/predict-iq
cargo test --lib modules::oracles_test
```

Run specific test category:
```bash
cargo test --lib modules::oracles_test::test_confidence_rounding_small_prices -- --nocapture
```

## Fix Validation

After implementing a fix:

1. **Run test suite**: All existing tests should pass
2. **Verify fix improves coverage**: Pay special attention to:
   - `test_confidence_rounding_small_prices`
   - `test_confidence_rounding_edge_cases_low_prices`
3. **New test cases**: If adding fix-dependent tests, uncomment assertions
4. **Regression tests**: Ensure large prices still work correctly

## Related Files

- [oracles.rs](contracts/predict-iq/src/modules/oracles.rs) - Oracle validation logic
- [oracles_test.rs](contracts/predict-iq/src/modules/oracles_test.rs) - Test suite
- [types.rs](contracts/predict-iq/src/types.rs) - OracleConfig and PythPrice definitions

## Resolution Checklist

- [ ] Comprehensive table-driven tests written
- [ ] Small price range (1-100) tests covering truncation cases
- [ ] Large price range (1M+) tests ensuring no regression
- [ ] Edge case tests for boundary conditions
- [ ] Negative price handling verified
- [ ] Documentation complete
- [ ] Fix implemented and all tests passing
- [ ] Code review completed
- [ ] Merged to main branch

## Notes

- These tests document current behavior while exposing the bias
- The bias exists but is acceptable for larger prices (>1000)
- Priority should be given to solutions with minimal overflow risk
- Performance impact of any fix should be measured
