// Use compiler builtins so we never resolve Nyra's @sin/@cos wrappers at link time.
double sin_f64(double x) { return __builtin_sin(x); }
double cos_f64(double x) { return __builtin_cos(x); }
double atan2_f64(double y, double x) { return __builtin_atan2(y, x); }
double tan_f64(double x) { return __builtin_tan(x); }
// [contrib-dev:ceil_f64:math]
double ceil_f64(double x) {
    return __builtin_ceil(x);
}
// [/contrib-dev:ceil_f64:math]

// [contrib-dev:clamp_f64:math]
double clamp_f64(double x, double lo, double hi) {
    if (x < lo) {
        return lo;
    }
    if (x > hi) {
        return hi;
    }
    return x;
}
// [/contrib-dev:clamp_f64:math]

// [contrib-dev:exp_f64:math]
double exp_f64(double x) {
    return __builtin_exp(x);
}
// [/contrib-dev:exp_f64:math]

// [contrib-dev:floor_f64:math]
double floor_f64(double x) {
    return __builtin_floor(x);
}
// [/contrib-dev:floor_f64:math]

// [contrib-dev:log_f64:math]
double log_f64(double x) {
    return __builtin_log(x);
}
// [/contrib-dev:log_f64:math]

// [contrib-dev:pow_f64:math]
double pow_f64(double base, double exp) {
    return __builtin_pow(base, exp);
}
// [/contrib-dev:pow_f64:math]

// [contrib-dev:round_f64:math]
double round_f64(double x) {
    return __builtin_round(x);
}
// [/contrib-dev:round_f64:math]

// [contrib-dev:sqrt_f64:math]
double sqrt_f64(double x) {
    return __builtin_sqrt(x);
}
// [/contrib-dev:sqrt_f64:math]
// [contrib-dev:acos_f64:math]
double acos_f64(double x) {
    return __builtin_acos(x);
}
// [/contrib-dev:acos_f64:math]

// [contrib-dev:asin_f64:math]
double asin_f64(double x) {
    return __builtin_asin(x);
}
// [/contrib-dev:asin_f64:math]

// [contrib-dev:atan_f64:math]
double atan_f64(double x) {
    return __builtin_atan(x);
}
// [/contrib-dev:atan_f64:math]

// [contrib-dev:hypot_f64:math]
double hypot_f64(double x, double y) {
    return __builtin_sqrt((x * x) + (y * y));
}
// [/contrib-dev:hypot_f64:math]

// [contrib-dev:log10_f64:math]
double log10_f64(double x) {
    return __builtin_log10(x);
}
// [/contrib-dev:log10_f64:math]

// [contrib-dev:log2_f64:math]
double log2_f64(double x) {
    return __builtin_log2(x);
}
// [/contrib-dev:log2_f64:math]

// [contrib-dev:trunc_f64:math]
double trunc_f64(double x) {
    return __builtin_trunc(x);
}
// [/contrib-dev:trunc_f64:math]

// [contrib-dev:ceil_i32:math]
int ceil_i32(int x) {
    return (int)__builtin_ceil((double)x);
}
// [/contrib-dev:ceil_i32:math]

// [contrib-dev:floor_i32:math]
int floor_i32(int x) {
    return (int)__builtin_floor((double)x);
}
// [/contrib-dev:floor_i32:math]

// [contrib-dev:is_finite_f64:math]
int is_finite_f64(double x) {
    return __builtin_isfinite(x) ? 1 : 0;
}
// [/contrib-dev:is_finite_f64:math]

// [contrib-dev:is_nan_f64:math]
int is_nan_f64(double x) {
    return __builtin_isnan(x) ? 1 : 0;
}
// [/contrib-dev:is_nan_f64:math]

// [contrib-dev:round_i32:math]
int round_i32(int x) {
    return (int)__builtin_round((double)x);
}
// [/contrib-dev:round_i32:math]

// [contrib-dev:signum_f64:math]
double signum_f64(double x) {
    if (x > 0.0) return 1.0;
    if (x < 0.0) return -1.0;
    return 0.0;
}
// [/contrib-dev:signum_f64:math]

// [contrib-dev:copysign_f64:math]
double copysign_f64(double x, double y) {
    return __builtin_copysign(x, y);
}
// [/contrib-dev:copysign_f64:math]

// [contrib-dev:fmod_f64:math]
double fmod_f64(double x, double y) {
    if (y == 0.0) return 0.0;
    int n = (int)(x / y);
    double r = x - (double)n * y;
    if ((r > 0.0) != (x > 0.0)) r += (x > 0.0 ? y : -y);
    return r;
}
// [/contrib-dev:fmod_f64:math]

// [contrib-dev:lerp_f64:math]
double lerp_f64(double a, double b, double t) {
    return a + (b - a) * t;
}
// [/contrib-dev:lerp_f64:math]

// [contrib-dev:signum_i32:math]
int signum_i32(int x) {
    if (x > 0) return 1;
    if (x < 0) return -1;
    return 0;
}
// [/contrib-dev:signum_i32:math]

// [contrib-dev:trunc_i32:math]
int trunc_i32(int x) {
    return (int)__builtin_trunc((double)x);
}
// [/contrib-dev:trunc_i32:math]

// [contrib-dev:deg_to_rad_f64:math]
double deg_to_rad_f64(double deg) {
    return deg * (3.141592653589793 / 180.0);
}
// [/contrib-dev:deg_to_rad_f64:math]

// [contrib-dev:fract_f64:math]
double fract_f64(double x) {
    return x - __builtin_trunc(x);
}
// [/contrib-dev:fract_f64:math]

// [contrib-dev:gcd_i32:math]
int gcd_i32(int a, int b) {
    if (a < 0) a = -a;
    if (b < 0) b = -b;
    while (b != 0) { int t = a % b; a = b; b = t; }
    return a;
}
// [/contrib-dev:gcd_i32:math]

// [contrib-dev:lcm_i32:math]
int lcm_i32(int a, int b) {
    if (a == 0 || b == 0) return 0;
    int x = a < 0 ? -a : a;
    int y = b < 0 ? -b : b;
    int g = x;
    int h = y;
    while (h != 0) { int t = g % h; g = h; h = t; }
    if (g == 0) return 0;
    return (x / g) * y;
}
// [/contrib-dev:lcm_i32:math]

// [contrib-dev:mod_i32:math]
int mod_i32(int a, int b) {
    if (b == 0) return 0;
    int r = a % b;
    if (r < 0) r += (b < 0 ? -b : b);
    return r;
}
// [/contrib-dev:mod_i32:math]

// [contrib-dev:rad_to_deg_f64:math]
double rad_to_deg_f64(double rad) {
    return rad * (180.0 / 3.141592653589793);
}
// [/contrib-dev:rad_to_deg_f64:math]

// [contrib-dev:count_ones_i32:math]
int count_ones_i32(int n) {
    unsigned u = (unsigned)n;
    int c = 0;
    while (u) { c += (int)(u & 1u); u >>= 1; }
    return c;
}
// [/contrib-dev:count_ones_i32:math]

// [contrib-dev:is_infinite_f64:math]
int is_infinite_f64(double x) {
    return (x == x && (x > 1e308 || x < -1e308)) ? 1 : 0;
}
// [/contrib-dev:is_infinite_f64:math]

// [contrib-dev:leading_zeros_i32:math]
int leading_zeros_i32(int n) {
    if (n == 0) return 32;
    unsigned u = (unsigned)n;
    int c = 0;
    if ((u & 0xFFFF0000u) == 0) { c += 16; u <<= 16; }
    if ((u & 0xFF000000u) == 0) { c += 8; u <<= 8; }
    if ((u & 0xF0000000u) == 0) { c += 4; u <<= 4; }
    if ((u & 0xC0000000u) == 0) { c += 2; u <<= 2; }
    if ((u & 0x80000000u) == 0) { c += 1; }
    return c;
}
// [/contrib-dev:leading_zeros_i32:math]

// [contrib-dev:saturating_add_i32:math]
int saturating_add_i32(int a, int b) {
    long long r = (long long)a + (long long)b;
    if (r > 2147483647LL) return 2147483647;
    if (r < -2147483648LL) return (int)-2147483648LL;
    return (int)r;
}
// [/contrib-dev:saturating_add_i32:math]

// [contrib-dev:saturating_sub_i32:math]
int saturating_sub_i32(int a, int b) {
    long long r = (long long)a - (long long)b;
    if (r > 2147483647LL) return 2147483647;
    if (r < -2147483648LL) return (int)-2147483648LL;
    return (int)r;
}
// [/contrib-dev:saturating_sub_i32:math]

// [contrib-dev:wrapping_add_i32:math]
int wrapping_add_i32(int a, int b) {
    return (int)((unsigned int)a + (unsigned int)b);
}
// [/contrib-dev:wrapping_add_i32:math]

// [contrib-dev:rem_euclid_i32:math]
int rem_euclid_i32(int a, int b) {
    if (b == 0) return 0;
    int r = a % b;
    if (r < 0) r += (b < 0 ? -b : b);
    return r;
}
// [/contrib-dev:rem_euclid_i32:math]

// [contrib-dev:rotate_left_i32:math]
int rotate_left_i32(int n, int shift) {
    unsigned u = (unsigned)n;
    int s = shift & 31;
    return (int)((u << s) | (u >> (32 - s)));
}
// [/contrib-dev:rotate_left_i32:math]

// [contrib-dev:rotate_right_i32:math]
int rotate_right_i32(int n, int shift) {
    unsigned u = (unsigned)n;
    int s = shift & 31;
    return (int)((u >> s) | (u << (32 - s)));
}
// [/contrib-dev:rotate_right_i32:math]

// [contrib-dev:trailing_zeros_i32:math]
int trailing_zeros_i32(int n) {
    if (n == 0) return 32;
    unsigned u = (unsigned)n;
    int c = 0;
    while ((u & 1u) == 0u) { c++; u >>= 1; }
    return c;
}
// [/contrib-dev:trailing_zeros_i32:math]

