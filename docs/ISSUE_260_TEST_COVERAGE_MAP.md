# Issue #260 - Test Coverage Map

## Visual Test Structure

```
test_confidence_rounding... Test Family
│
├── Basic Validation (Foundation Tests)
│   ├── test_validate_fresh_price()
│   │   └─ Type: Acceptance test
│   │   └─ Cases: 1  ✓ Valid fresh price accepted
│   │
│   ├── test_reject_stale_price()
│   │   └─ Type: Rejection test
│   │   └─ Cases: 1  ✓ Stale price rejected
│   │
│   └── test_reject_low_confidence()
│       └─ Type: Rejection test
│       └─ Cases: 1  ✓ Low confidence rejected
│
├── Small Price Range (1-10,000) - PRIMARY BIAS EXPOSURE
│   └── test_confidence_rounding_small_prices()
│       ├─ Cases: 12
│       ├─ Price range: 1 to 10,000
│       ├─ BPS tested: 50, 100, 200
│       ├─ KEY CASES:
│       │  • (1, 500): Rounds to 0 - TRUNCATION BIAS
│       │  • (10, 100): Rounds to 0 - TRUNCATION BIAS
│       │  • (100, 100): Rounds to 1 ✓ Correct
│       │  • (10000, 50): Rounds to 50 ✓ Correct
│       └─ Purpose: Expose rounding bias for low valuations
│
├── Large Price Range (1M-100M) - REGRESSION CHECK
│   └── test_confidence_rounding_large_prices()
│       ├─ Cases: 8
│       ├─ Price range: 1M to 100M
│       ├─ BPS tested: 50, 100, 200
│       ├─ KEY CASES:
│       │  • (1M, 100): Rounds to 10K ✓ Correct
│       │  • (100M, 50): Rounds to 5M ✓ Correct
│       └─ Purpose: Ensure large prices not affected by fix
│
├── Edge Cases (Boundary Tests) - PRECISION VERIFICATION
│   ├── test_confidence_rounding_edge_cases_low_prices()
│   │   ├─ Cases: 9
│   │   ├─ Price range: 1-50
│   │   ├─ BPS tested: 1111, 2000, 10000
│   │   ├─ Special focus:
│   │   │  • Fractional multiplication boundaries
│   │   │  • Maximum confidence requirements
│   │   │  • Price × BPS combinations at transition points
│   │   └─ Purpose: Target specific rounding boundaries
│   │
│   └── test_confidence_rounding_boundary_conditions()
│       ├─ Cases: 6
│       ├─ TRANSITION POINTS (when does truncation start?):
│       │  • price < 50 with 2% BPS: Truncation occurs
│       │  • price = 50 with 2% BPS: Boundary (1.0)
│       │  • price > 50 with 2% BPS: No truncation
│       └─ Purpose: Document exact rounding behavior
│
├── Special Cases (Sign Handling) - CORRECTNESS
│   └── test_confidence_rounding_negative_prices()
│       ├─ Cases: 6
│       ├─ Tests: Absolute value handling for negative prices
│       ├─ Ranges: -1 to -1M (mirrors positive tests)
│       └─ Purpose: Ensure sign handling doesn't affect rounding
│
└── Bias Documentation (Reference) - BASELINE
    └── test_confidence_rounding_documented_bias()
        ├─ Type: Non-enforcement validation
        ├─ Purpose: Document known bias cases
        ├─ Lists cases where: truncated < ceiling
        └─ Baseline for fix validation
```

## Test Execution Flow

```
cargo test --lib modules::oracles_test
    │
    ├─ Success Path (All pass):
    │   ├─ Basic tests: 3/3 ✓
    │   ├─ Small prices: 12/12 ✓ (documents bias)
    │   ├─ Large prices: 8/8 ✓
    │   ├─ Edge cases: 9/9 ✓ (if fix handles them)
    │   ├─ Negative: 6/6 ✓
    │   ├─ Boundary: 6/6 ✓
    │   └─ Bias doc: 1/1 ✓
    │   
    └─ Failure Path (Before fix):
        └─ Small prices may show inconsistencies
            with (1,500), (10,100), (99,100), (49,200)
```

## Coverage Matrix

| Scenario | Test Function | Cases | Status |
|----------|--------------|-------|--------|
| Small prices (bias prone) | `test_confidence_rounding_small_prices` | 12 | Documents current behavior |
| Large prices (baseline) | `test_confidence_rounding_large_prices` | 8 | Should pass, regression check |
| Negative prices | `test_confidence_rounding_negative_prices` | 6 | Should pass |
| Boundary transitions | `test_confidence_rounding_boundary_conditions` | 6 | Should pass |
| Edge cases | `test_confidence_rounding_edge_cases_low_prices` | 9 | May expose precision issues |
| **TOTAL** | **6 functions** | **54 cases** | **Comprehensive coverage** |

## Key Test Cases by Category

### 🔴 BIAS CASES (Truncation = 0)
```
(1, 500)     → (1×500)/10000 = 0  ← Cannot accept ANY confidence
(10, 100)    → (10×100)/10000 = 0 ← Cannot accept ANY confidence  
(99, 100)    → (99×100)/10000 = 0 ← Cannot accept ANY confidence
(49, 200)    → (49×200)/10000 = 0 ← Cannot accept ANY confidence
```

### 🟡 BOUNDARY CASES (Rounding Point)
```
(50, 200)    → (50×200)/10000 = 1  ← Just above boundary
(100, 100)   → (100×100)/10000 = 1 ← Clean boundary
(5, 2000)    → (5×2000)/10000 = 1  ← Ceiling would = 1
```

### 🟢 NORMAL CASES (No Bias)
```
(1000, 100)  → (1000×100)/10000 = 10 ✓
(1M, 100)    → (1M×100)/10000 = 10K ✓
(100M, 50)   → (100M×50)/10000 = 500K ✓
```

## How to Read Test Output

```bash
test_confidence_rounding_small_prices ... ok
```
Means: All 12 small price cases passed
- If this FAILS before fix: Rounding bias confirmed
- If this PASSES after fix: Fix successful

```bash
test_confidence_rounding_large_prices ... ok
```
Means: No regression in large prices ✓

## Validation After Fix

1. **All tests PASS**:
   ```
   test result: ok. 54 passed; 0 failed
   ```

2. **Check bias cases specifically**:
   ```bash
   cargo test test_confidence_rounding_small_prices -- --nocapture
   ```

3. **Verify no regression**:
   - Large prices test passes
   - Boundary conditions match expectations

## Notes

- Tests use **table-driven approach** for clarity and maintainability
- Each test case includes **descriptive comments** with expected behavior
- Coverage includes **positive and negative test cases**
- Edge cases specifically target **truncation boundaries**
- Documentation sufficient for **fix validation** and **code review**
