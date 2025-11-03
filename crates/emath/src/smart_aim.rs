//! Find "simple" numbers is some range. Used by sliders.

use crate::fast_midpoint;

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
    debug_assert!(
        min.is_finite() && max.is_finite(),
        "min: {min:?}, max: {max:?}"
    );

    let min_exponent = min.log10();
    let max_exponent = max.log10();

    if min_exponent.floor() != max_exponent.floor() {
        // Different orders of magnitude.
        // Pick the geometric center of the two:
        let exponent = fast_midpoint(min_exponent, max_exponent);
        return 10.0_f64.powi(exponent.round() as i32);
    }

    if is_integer(min_exponent) {
        return 10.0_f64.powf(min_exponent);
    }
    if is_integer(max_exponent) {
        return 10.0_f64.powf(max_exponent);
    }

    let exponent = max_exponent.floor() as i32;
    let exp_factor = 10.0_f64.powi(exponent);

    let min_str = to_decimal_string(min / exp_factor);
    let max_str = to_decimal_string(max / exp_factor);

    // We now have two positive integers of the same length.
    // We want to find the first non-matching digit,
    // which we will call the "deciding digit".
    // Everything before it will be the same,
    // everything after will be zero,
    // and the deciding digit itself will be picked as a "smart average"
    // min:    12345
    // max:    12780
    // output: 12500

    let mut ret_str = [0; NUM_DECIMALS];

    for i in 0..NUM_DECIMALS {
        if min_str[i] == max_str[i] {
            ret_str[i] = min_str[i];
        } else {
            // Found the deciding digit at index `i`
            let mut deciding_digit_min = min_str[i];
            let deciding_digit_max = max_str[i];

            debug_assert!(
                deciding_digit_min < deciding_digit_max,
                "Bug in smart aim code"
            );

            let rest_of_min_is_zeroes = min_str[i + 1..].iter().all(|&c| c == 0);

            if !rest_of_min_is_zeroes {
                // There are more digits coming after `deciding_digit_min`, so we cannot pick it.
                // So the true min of what we can pick is one greater:
                deciding_digit_min += 1;
            }

            let deciding_digit = if deciding_digit_min == 0 {
                0
            } else if deciding_digit_min <= 5 && 5 <= deciding_digit_max {
                5 // 5 is the roundest number in the range
            } else {
                deciding_digit_min.midpoint(deciding_digit_max)
            };

            ret_str[i] = deciding_digit;

            return from_decimal_string(&ret_str) * exp_factor;
        }
    }

    min // All digits are the same. Already handled earlier, but better safe than sorry
}

fn is_integer(f: f64) -> bool {
    f.round() == f
}

fn to_decimal_string(v: f64) -> [i32; NUM_DECIMALS] {
    debug_assert!(v < 10.0, "{v:?}");
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

#[expect(clippy::approx_constant)]
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

    const NAN: f64 = f64::NAN;
    const INFINITY: f64 = f64::INFINITY;
    const NEG_INFINITY: f64 = f64::NEG_INFINITY;
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

    #[track_caller]
    fn test_f64((min, max): (f64, f64), expected: f64) {
        let aimed = best_in_range_f64(min, max);
        assert!(
            aimed == expected,
            "smart_aim({min} – {max}) => {aimed}, but expected {expected}"
        );
    }
    #[track_caller]
    fn test_i64((min, max): (i64, i64), expected: i64) {
        let aimed = best_in_range_f64(min as _, max as _);
        assert!(
            aimed == expected as f64,
            "smart_aim({min} – {max}) => {aimed}, but expected {expected}"
        );
    }

    test_i64((99, 300), 100);
    test_i64((300, 99), 100);
    test_i64((-99, -300), -100);
    test_i64((-99, 123), 0); // Prefer zero
    test_i64((4, 9), 5); // Prefer ending on 5
    test_i64((14, 19), 15); // Prefer ending on 5
    test_i64((12, 65), 50); // Prefer leading 5
    test_i64((493, 879), 500); // Prefer leading 5
    test_i64((37, 48), 40);
    test_i64((100, 123), 100);
    test_i64((101, 1000), 1000);
    test_i64((999, 1000), 1000);
    test_i64((123, 500), 500);
    test_i64((500, 777), 500);
    test_i64((500, 999), 500);
    test_i64((12345, 12780), 12500);
    test_i64((12371, 12376), 12375);
    test_i64((12371, 12376), 12375);

    test_f64((7.5, 16.3), 10.0);
    test_f64((7.5, 76.3), 10.0);
    test_f64((7.5, 763.3), 100.0);
    test_f64((7.5, 1_345.0), 1_000.0);
    test_f64((7.5, 123_456.0), 100_000.0);
    test_f64((-0.2, 0.0), 0.0); // Prefer zero
    test_f64((-10_004.23, 4.14), 0.0); // Prefer zero
    test_f64((-0.2, 100.0), 0.0); // Prefer zero
    test_f64((0.2, 0.0), 0.0); // Prefer zero
    test_f64((7.8, 17.8), 10.0);
    test_f64((14.1, 19.1), 15.0); // Prefer ending on 5
    test_f64((12.3, 65.9), 50.0); // Prefer leading 5
}
