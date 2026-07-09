// Compiler intrinsics: abs_i32, min_i32, max_i32, clamp_i32 (and abs/min/max on f64)
// are lowered to LLVM intrinsics at call sites — these bodies are reference stubs only.

fn abs_i32(x: i32) -> i32 {
    if x < 0 {
        return 0 - x
    }
    return x
}

fn abs_f64(x: f64) -> f64 {
    if x < 0.0 {
        return 0.0 - x
    }
    return x
}

fn min_i32(a: i32, b: i32) -> i32 {
    if a < b {
        return a
    }
    return b
}

fn max_i32(a: i32, b: i32) -> i32 {
    if a > b {
        return a
    }
    return b
}

fn clamp_i32(x: i32, lo: i32, hi: i32) -> i32 {
    if x < lo {
        return lo
    }
    if x > hi {
        return hi
    }
    return x
}

fn min_f64(a: f64, b: f64) -> f64 {
    if a < b {
        return a
    }
    return b
}

fn max_f64(a: f64, b: f64) -> f64 {
    if a > b {
        return a
    }
    return b
}

fn pow_i32(base: i32, exp: i32) -> i32 {
    if exp < 0 {
        return 0
    }
    let mut result = 1
    let mut i = 0
    while i < exp {
        result = result * base
        i = i + 1
    }
    return result
}

// Integer sqrt (Newton) — no libm required.
fn sqrt_i32(n: i32) -> i32 {
    if n <= 0 {
        return 0
    }
    let mut x = n
    let mut y = (x + 1) / 2
    while y < x {
        x = y
        y = (x + n / x) / 2
    }
    return x
}

extern fn sin_f64(x: f64) -> f64
extern fn cos_f64(x: f64) -> f64
extern fn atan2_f64(y: f64, x: f64) -> f64
extern fn tan_f64(x: f64) -> f64

fn sin(x) {
    return sin_f64(x)
}

fn cos(x) {
    return cos_f64(x)
}

fn atan2(y, x) {
    return atan2_f64(y, x)
}

fn tan(x) {
    return tan_f64(x)
}

fn floor(x: f64) -> f64 {
    return floor_f64(x)
}

fn ceil(x: f64) -> f64 {
    return ceil_f64(x)
}

fn round(x: f64) -> f64 {
    return round_f64(x)
}

fn sqrt(x: f64) -> f64 {
    return sqrt_f64(x)
}

fn pow(x: f64, exp: f64) -> f64 {
    return pow_f64(x, exp)
}

fn log(x: f64) -> f64 {
    return log_f64(x)
}

fn exp(x: f64) -> f64 {
    return exp_f64(x)
}

fn clamp(x: f64, lo: f64, hi: f64) -> f64 {
    return clamp_f64(x, lo, hi)
}

fn trunc(x: f64) -> f64 {
    return trunc_f64(x)
}

fn hypot(x: f64, y: f64) -> f64 {
    return hypot_f64(x, y)
}

fn asin(x: f64) -> f64 {
    return asin_f64(x)
}

fn acos(x: f64) -> f64 {
    return acos_f64(x)
}

fn atan(x: f64) -> f64 {
    return atan_f64(x)
}

fn log10(x: f64) -> f64 {
    return log10_f64(x)
}

fn log2(x: f64) -> f64 {
    return log2_f64(x)
}
// [contrib-dev:ceil_f64:math]
extern fn ceil_f64(x: f64) -> f64
// [/contrib-dev:ceil_f64:math]
// [contrib-dev:clamp_f64:math]
extern fn clamp_f64(x: f64, lo: f64, hi: f64) -> f64
// [/contrib-dev:clamp_f64:math]
// [contrib-dev:exp_f64:math]
extern fn exp_f64(x: f64) -> f64
// [/contrib-dev:exp_f64:math]
// [contrib-dev:floor_f64:math]
extern fn floor_f64(x: f64) -> f64
// [/contrib-dev:floor_f64:math]
// [contrib-dev:log_f64:math]
extern fn log_f64(x: f64) -> f64
// [/contrib-dev:log_f64:math]
// [contrib-dev:pow_f64:math]
extern fn pow_f64(base: f64, exp: f64) -> f64
// [/contrib-dev:pow_f64:math]
// [contrib-dev:round_f64:math]
extern fn round_f64(x: f64) -> f64
// [/contrib-dev:round_f64:math]
// [contrib-dev:sqrt_f64:math]
extern fn sqrt_f64(x: f64) -> f64
// [/contrib-dev:sqrt_f64:math]
// [contrib-dev:acos_f64:math]
extern fn acos_f64(x: f64) -> f64
// [/contrib-dev:acos_f64:math]
// [contrib-dev:asin_f64:math]
extern fn asin_f64(x: f64) -> f64
// [/contrib-dev:asin_f64:math]
// [contrib-dev:atan_f64:math]
extern fn atan_f64(x: f64) -> f64
// [/contrib-dev:atan_f64:math]
// [contrib-dev:hypot_f64:math]
extern fn hypot_f64(x: f64, y: f64) -> f64
// [/contrib-dev:hypot_f64:math]
// [contrib-dev:log10_f64:math]
extern fn log10_f64(x: f64) -> f64
// [/contrib-dev:log10_f64:math]
// [contrib-dev:log2_f64:math]
extern fn log2_f64(x: f64) -> f64
// [/contrib-dev:log2_f64:math]
// [contrib-dev:trunc_f64:math]
extern fn trunc_f64(x: f64) -> f64
// [/contrib-dev:trunc_f64:math]

// [contrib-dev:ceil_i32:math]
extern fn ceil_i32(x: i32) -> i32
// [/contrib-dev:ceil_i32:math]

// [contrib-dev:floor_i32:math]
extern fn floor_i32(x: i32) -> i32
// [/contrib-dev:floor_i32:math]

// [contrib-dev:is_finite_f64:math]
extern fn is_finite_f64(x: f64) -> i32
// [/contrib-dev:is_finite_f64:math]
// [contrib-dev:is_finite_f64:math:alias]
fn is_finite(x: f64) -> i32 {
    return is_finite_f64(x)
}
// [/contrib-dev:is_finite_f64:math:alias]

// [contrib-dev:is_nan_f64:math]
extern fn is_nan_f64(x: f64) -> i32
// [/contrib-dev:is_nan_f64:math]
// [contrib-dev:is_nan_f64:math:alias]
fn is_nan(x: f64) -> i32 {
    return is_nan_f64(x)
}
// [/contrib-dev:is_nan_f64:math:alias]

// [contrib-dev:round_i32:math]
extern fn round_i32(x: i32) -> i32
// [/contrib-dev:round_i32:math]

// [contrib-dev:signum_f64:math]
extern fn signum_f64(x: f64) -> f64
// [/contrib-dev:signum_f64:math]
// [contrib-dev:signum_f64:math:alias]
fn signum(x: f64) -> f64 {
    return signum_f64(x)
}
// [/contrib-dev:signum_f64:math:alias]

// [contrib-dev:copysign_f64:math]
extern fn copysign_f64(x: f64, y: f64) -> f64
// [/contrib-dev:copysign_f64:math]
// [contrib-dev:fmod_f64:math]
extern fn fmod_f64(x: f64, y: f64) -> f64
// [/contrib-dev:fmod_f64:math]
// [contrib-dev:fmod_f64:math:alias]
fn fmod(x: f64, y: f64) -> f64 {
    return fmod_f64(x, y)
}
// [/contrib-dev:fmod_f64:math:alias]

// [contrib-dev:lerp_f64:math]
extern fn lerp_f64(a: f64, b: f64, t: f64) -> f64
// [/contrib-dev:lerp_f64:math]
// [contrib-dev:lerp_f64:math:alias]
fn lerp(a: f64, b: f64, t: f64) -> f64 {
    return lerp_f64(a, b, t)
}
// [/contrib-dev:lerp_f64:math:alias]

// [contrib-dev:signum_i32:math]
extern fn signum_i32(x: i32) -> i32
// [/contrib-dev:signum_i32:math]
// [contrib-dev:trunc_i32:math]
extern fn trunc_i32(x: i32) -> i32
// [/contrib-dev:trunc_i32:math]
// [contrib-dev:deg_to_rad_f64:math]
extern fn deg_to_rad_f64(deg: f64) -> f64
// [/contrib-dev:deg_to_rad_f64:math]
// [contrib-dev:deg_to_rad_f64:math:alias]
fn deg_to_rad(deg: f64) -> f64 {
    return deg_to_rad_f64(deg)
}
// [/contrib-dev:deg_to_rad_f64:math:alias]

// [contrib-dev:fract_f64:math]
extern fn fract_f64(x: f64) -> f64
// [/contrib-dev:fract_f64:math]
// [contrib-dev:fract_f64:math:alias]
fn fract(x: f64) -> f64 {
    return fract_f64(x)
}
// [/contrib-dev:fract_f64:math:alias]

// [contrib-dev:gcd_i32:math]
extern fn gcd_i32(a: i32, b: i32) -> i32
// [/contrib-dev:gcd_i32:math]
// [contrib-dev:lcm_i32:math]
extern fn lcm_i32(a: i32, b: i32) -> i32
// [/contrib-dev:lcm_i32:math]
// [contrib-dev:mod_i32:math]
extern fn mod_i32(a: i32, b: i32) -> i32
// [/contrib-dev:mod_i32:math]
// [contrib-dev:rad_to_deg_f64:math]
extern fn rad_to_deg_f64(rad: f64) -> f64
// [/contrib-dev:rad_to_deg_f64:math]
// [contrib-dev:rad_to_deg_f64:math:alias]
fn rad_to_deg(rad: f64) -> f64 {
    return rad_to_deg_f64(rad)
}
// [/contrib-dev:rad_to_deg_f64:math:alias]

// [contrib-dev:count_ones_i32:math]
extern fn count_ones_i32(n: i32) -> i32
// [/contrib-dev:count_ones_i32:math]
// [contrib-dev:count_ones_i32:math:alias]
fn count_ones(n: i32) -> i32 {
    return count_ones_i32(n)
}
// [/contrib-dev:count_ones_i32:math:alias]

// [contrib-dev:is_infinite_f64:math]
extern fn is_infinite_f64(x: f64) -> i32
// [/contrib-dev:is_infinite_f64:math]
// [contrib-dev:is_infinite_f64:math:alias]
fn is_infinite(x: f64) -> i32 {
    return is_infinite_f64(x)
}
// [/contrib-dev:is_infinite_f64:math:alias]

// [contrib-dev:leading_zeros_i32:math]
extern fn leading_zeros_i32(n: i32) -> i32
// [/contrib-dev:leading_zeros_i32:math]
// [contrib-dev:leading_zeros_i32:math:alias]
fn leading_zeros(n: i32) -> i32 {
    return leading_zeros_i32(n)
}
// [/contrib-dev:leading_zeros_i32:math:alias]

// [contrib-dev:saturating_add_i32:math]
extern fn saturating_add_i32(a: i32, b: i32) -> i32
// [/contrib-dev:saturating_add_i32:math]
// [contrib-dev:saturating_add_i32:math:alias]
fn saturating_add(a: i32, b: i32) -> i32 {
    return saturating_add_i32(a, b)
}
// [/contrib-dev:saturating_add_i32:math:alias]

// [contrib-dev:saturating_sub_i32:math]
extern fn saturating_sub_i32(a: i32, b: i32) -> i32
// [/contrib-dev:saturating_sub_i32:math]
// [contrib-dev:saturating_sub_i32:math:alias]
fn saturating_sub(a: i32, b: i32) -> i32 {
    return saturating_sub_i32(a, b)
}
// [/contrib-dev:saturating_sub_i32:math:alias]

// [contrib-dev:wrapping_add_i32:math]
extern fn wrapping_add_i32(a: i32, b: i32) -> i32
// [/contrib-dev:wrapping_add_i32:math]
// [contrib-dev:wrapping_add_i32:math:alias]
fn wrapping_add(a: i32, b: i32) -> i32 {
    return wrapping_add_i32(a, b)
}
// [/contrib-dev:wrapping_add_i32:math:alias]

// [contrib-dev:rem_euclid_i32:math]
extern fn rem_euclid_i32(a: i32, b: i32) -> i32
// [/contrib-dev:rem_euclid_i32:math]
// [contrib-dev:rem_euclid_i32:math:alias]
fn rem_euclid(a: i32, b: i32) -> i32 {
    return rem_euclid_i32(a, b)
}
// [/contrib-dev:rem_euclid_i32:math:alias]

// [contrib-dev:rotate_left_i32:math]
extern fn rotate_left_i32(n: i32, shift: i32) -> i32
// [/contrib-dev:rotate_left_i32:math]
// [contrib-dev:rotate_left_i32:math:alias]
fn rotate_left(n: i32, shift: i32) -> i32 {
    return rotate_left_i32(n, shift)
}
// [/contrib-dev:rotate_left_i32:math:alias]

// [contrib-dev:rotate_right_i32:math]
extern fn rotate_right_i32(n: i32, shift: i32) -> i32
// [/contrib-dev:rotate_right_i32:math]
// [contrib-dev:rotate_right_i32:math:alias]
fn rotate_right(n: i32, shift: i32) -> i32 {
    return rotate_right_i32(n, shift)
}
// [/contrib-dev:rotate_right_i32:math:alias]

// [contrib-dev:trailing_zeros_i32:math]
extern fn trailing_zeros_i32(n: i32) -> i32
// [/contrib-dev:trailing_zeros_i32:math]
// [contrib-dev:trailing_zeros_i32:math:alias]
fn trailing_zeros(n: i32) -> i32 {
    return trailing_zeros_i32(n)
}
// [/contrib-dev:trailing_zeros_i32:math:alias]

