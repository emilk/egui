//! Find "simple" numbers is some range. Used by sliders.

const NUM_DECIMALS: usize = 15;

/// Find the "simplest" number in a closed range [min, max], i.e. the one with the fewest decimal digits.
///
/// So in the range `[0.83, 1.354]` you will get `1.0`, and for `[0.37, 0.48]` you will get `0.4`.
/// This is used when dragging sliders etc to get the values that users are most likely to desire.
/// This assumes a decimal centric user.
pub fn best_in_range_f64(min: f64, max: f64) -> f64 {
    // Avoid NaN if we can:
    if min.is_nan() {
        return max;
    }
    if max.is_nan() {
        return min;
    }

    if max < min {
        return best_in_range_f64(max, min);
    }
    if min == max {
        return min;
    }
    if min <= 0.0 && 0.0 <= max {
        return 0.0; // always prefer zero
    }
    if min < 0.0 {
        return -best_in_range_f64(-max, -min);
    }

    // Prefer finite numbers:
    if !max.is_finite() {
        return min;
    }
    crate::emath_assert!(min.is_finite() && max.is_finite());

    let min_exponent = min.log10();
    let max_exponent = max.log10();

    if min_exponent.floor() != max_exponent.floor() {
        // pick the geometric center of the two:
        let exponent = (min_exponent + max_exponent) / 2.0;
        return 10.0_f64.powi(exponent.round() as i32);
    }

    if is_integer(min_exponent) {
        return 10.0_f64.powf(min_exponent);
    }
    if is_integer(max_exponent) {
        return 10.0_f64.powf(max_exponent);
    }

    let exp_factor = 10.0_f64.powi(max_exponent.floor() as i32);

    let min_str = to_decimal_string(min / exp_factor);
    let max_str = to_decimal_string(max / exp_factor);

    // eprintln!("min_str: {:?}", min_str);
    // eprintln!("max_str: {:?}", max_str);

    let mut ret_str = [0; NUM_DECIMALS];

    // Select the common prefix:
    let mut i = 0;
    while i < NUM_DECIMALS && max_str[i] == min_str[i] {
        ret_str[i] = max_str[i];
        i += 1;
    }

    if i < NUM_DECIMALS {
        // Pick the deciding digit.
        // Note that "to_decimal_string" rounds down, so we that's why we add 1 here
        ret_str[i] = simplest_digit_closed_range(min_str[i] + 1, max_str[i]);
    }

    from_decimal_string(&ret_str) * exp_factor
}

fn is_integer(f: f64) -> bool {
    f.round() == f
}

fn to_decimal_string(v: f64) -> [i32; NUM_DECIMALS] {
    crate::emath_assert!(v < 10.0, "{:?}", v);
    let mut digits = [0; NUM_DECIMALS];
    let mut v = v.abs();
    for r in &mut digits {
        let digit = v.floor();
        *r = digit as i32;
        v -= digit;
        v *= 10.0;
    }
    digits
}

fn from_decimal_string(s: &[i32]) -> f64 {
    let mut ret: f64 = 0.0;
    for (i, &digit) in s.iter().enumerate() {
        ret += (digit as f64) * 10.0_f64.powi(-(i as i32));
    }
    ret
}

/// Find the simplest integer in the range [min, max]
fn simplest_digit_closed_range(min: i32, max: i32) -> i32 {
    crate::emath_assert!(1 <= min && min <= max && max <= 9);
    if min <= 5 && 5 <= max {
        5
    } else {
        (min + max) / 2
    }
}

#[allow(clippy::approx_constant)]
#[test]
fn test_aim() {
    assert_eq!(best_in_range_f64(-0.2, 0.0), 0.0, "Prefer zero");
    assert_eq!(best_in_range_f64(-10_004.23, 3.14), 0.0, "Prefer zero");
    assert_eq!(best_in_range_f64(-0.2, 100.0), 0.0, "Prefer zero");
    assert_eq!(best_in_range_f64(0.2, 0.0), 0.0, "Prefer zero");
    assert_eq!(best_in_range_f64(7.8, 17.8), 10.0);
    assert_eq!(best_in_range_f64(99.0, 300.0), 100.0);
    assert_eq!(best_in_range_f64(-99.0, -300.0), -100.0);
    assert_eq!(best_in_range_f64(0.4, 0.9), 0.5, "Prefer ending on 5");
    assert_eq!(best_in_range_f64(14.1, 19.99), 15.0, "Prefer ending on 5");
    assert_eq!(best_in_range_f64(12.3, 65.9), 50.0, "Prefer leading 5");
    assert_eq!(best_in_range_f64(493.0, 879.0), 500.0, "Prefer leading 5");
    assert_eq!(best_in_range_f64(0.37, 0.48), 0.40);
    // assert_eq!(best_in_range_f64(123.71, 123.76), 123.75); // TODO(emilk): we get 123.74999999999999 here
    // assert_eq!(best_in_range_f32(123.71, 123.76), 123.75);
    assert_eq!(best_in_range_f64(7.5, 16.3), 10.0);
    assert_eq!(best_in_range_f64(7.5, 76.3), 10.0);
    assert_eq!(best_in_range_f64(7.5, 763.3), 100.0);
    assert_eq!(best_in_range_f64(7.5, 1_345.0), 100.0);
    assert_eq!(best_in_range_f64(7.5, 123_456.0), 1000.0, "Geometric mean");
    assert_eq!(best_in_range_f64(9.9999, 99.999), 10.0);
    assert_eq!(best_in_range_f64(10.000, 99.999), 10.0);
    assert_eq!(best_in_range_f64(10.001, 99.999), 50.0);
    assert_eq!(best_in_range_f64(10.001, 100.000), 100.0);
    assert_eq!(best_in_range_f64(99.999, 100.000), 100.0);
    assert_eq!(best_in_range_f64(10.001, 100.001), 100.0);

    use std::f64::{INFINITY, NAN, NEG_INFINITY};
    assert!(best_in_range_f64(NAN, NAN).is_nan());
    assert_eq!(best_in_range_f64(NAN, 1.2), 1.2);
    assert_eq!(best_in_range_f64(NAN, INFINITY), INFINITY);
    assert_eq!(best_in_range_f64(1.2, NAN), 1.2);
    assert_eq!(best_in_range_f64(1.2, INFINITY), 1.2);
    assert_eq!(best_in_range_f64(INFINITY, 1.2), 1.2);
    assert_eq!(best_in_range_f64(NEG_INFINITY, 1.2), 0.0);
    assert_eq!(best_in_range_f64(NEG_INFINITY, -2.7), -2.7);
    assert_eq!(best_in_range_f64(INFINITY, INFINITY), INFINITY);
    assert_eq!(best_in_range_f64(NEG_INFINITY, NEG_INFINITY), NEG_INFINITY);
    assert_eq!(best_in_range_f64(NEG_INFINITY, INFINITY), 0.0);
    assert_eq!(best_in_range_f64(INFINITY, NEG_INFINITY), 0.0);
}
