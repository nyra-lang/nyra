pub fn add(a, b) {
    return a + b
}

pub fn mul(a, b) {
    return a * b
}

pub fn unused_export() {
    return 0
}

priv fn double(x) {
    return mul(x, 2)
}

pub fn twice(x) {
    return double(x)
}
