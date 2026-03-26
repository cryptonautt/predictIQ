# Issue #260 Quick Implementation Guide

## Problem
Integer rounding in `(price_abs * max_confidence_bps) / 10000` bias acceptance for small prices.

## Test Files
- **Primary**: `contracts/predict-iq/src/modules/oracles_test.rs`
- **Code Under Test**: `contracts/predict-iq/src/modules/oracles.rs`
- **Documentation**: `CONFIDENCE_ROUNDING_TESTING.md`

## Current Behavior (Before Fix)

```
price=1,  BPS=500:  (1 × 500) / 10000 = 0     ← BIAS: Should accept some conf
price=10, BPS=100:  (10 × 100) / 10000 = 0    ← BIAS: Should accept some conf
price=50, BPS=200:  (50 × 200) / 10000 = 1    ✓ Correct
price=100, BPS=100: (100 × 100) / 10000 = 1   ✓ Correct
```

## Code Location to Modify

File: `contracts/predict-iq/src/modules/oracles.rs`

Function: `validate_price()` at line ~31

Current code:
```rust
let price_abs = if price.price < 0 {
    (-price.price) as u64
} else {
    price.price as u64
};
let max_conf = (price_abs * config.max_confidence_bps) / 10000;  // ← THIS LINE

if price.conf > max_conf {
    return Err(ErrorCode::ConfidenceTooLow);
}
```

## Fix Options (Choose One)

### Option A: Ceiling Division (SIMPLE)
```rust
let max_conf = (price_abs * config.max_confidence_bps + 9999) / 10000;
```
✓ Simple, one-line change
✗ Slightly relaxes confidence requirement

### Option B: Reverse Formula (BEST FOR CORRECTNESS)
```rust
// Instead of calculating max_conf and comparing:
if price.conf * 10000 > price_abs * config.max_confidence_bps {
    return Err(ErrorCode::ConfidenceTooLow);
}
```
✓ Exact comparison, no rounding
✓ Eliminates `max_conf` variable
⚠ Watch for multiplication overflow (u64 × u64 → u128)

### Option C: Fixed-Point Scaling (MOST PRECISE)
```rust
// For very high precision requirements
let numerator = price_abs * config.max_confidence_bps;
let max_conf = (numerator + 9999) / 10000;  // Ceiling division
```
✓ Combines ceiling division for precision
✗ Still uses division

## Test Verification After Fix

1. **All current tests pass**:
   ```bash
   cargo test --lib modules::oracles_test
   ```

2. **Check specific test cases**:
   ```bash
   cargo test --lib modules::oracles_test::test_confidence_rounding_small_prices -- --nocapture
   ```

3. **Verify edge cases still work**:
   ```bash
   cargo test --lib modules::oracles_test::test_confidence_rounding_boundary_conditions -- --nocapture
   ```

4. **Ensure large prices unchanged**:
   ```bash
   cargo test --lib modules::oracles_test::test_confidence_rounding_large_prices -- --nocapture
   ```

## Expected Results After Fix

With ceiling division fix:
```
price=1,  BPS=500:  (1 × 500 + 9999) / 10000 = 1    ✓ Fixed
price=10, BPS=100:  (10 × 100 + 9999) / 10000 = 1   ✓ Fixed
price=50, BPS=200:  (50 × 200 + 9999) / 10000 = 1   ✓ Fixed
price=100, BPS=100: (100 × 100 + 9999) / 10000 = 1  ✓ Correct
```

## Integration Test Checklist

- [ ] Modification compiles without errors
- [ ] All `oracles_test` tests pass
- [ ] No performance regression compared to original
- [ ] Related module tests still pass:
  - [ ] `modules::markets_test`
  - [ ] `modules::resolution_test` (if exists)
- [ ] Integration tests pass:
  - [ ] `integration_test.rs`
- Code review completed
- [ ] PR description references Issue #260

## Key Test Cases Exercised

The test suite includes **6 major test functions** with **50+ test cases** covering:

1. ✓ Small prices (1-100)
2. ✓ Medium prices (1,000-10,000)
3. ✓ Large prices (1M-100M)
4. ✓ Negative prices
5. ✓ Boundary conditions
6. ✓ Documented bias demonstration

## Measurement Points

To validate fix quality:
- **Before**: Document failing test cases
- **After**: Verify all tests pass
- **Performance**: Compare function execution time (should be negligible)
- **Safety**: Check for any potential overflow scenarios

## References

- Issue: #260 - Confidence threshold rounding tests
- Area: Oracle validation
- Algorithm: Price confidence interval validation
- Formula: `max_conf = (price_abs * max_confidence_bps) / 10000`

---

**Last Updated**: 2026-03-26
**Test Coverage**: 6 test functions, 50+ test cases
**Status**: ✓ Ready for fix implementation
