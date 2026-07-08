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

