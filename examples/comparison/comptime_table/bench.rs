fn mix(n: i64) -> i64 {
    const MOD: i64 = 1_000_000_007;
    let a = n * 100_003;
    let b = n * n;
    (a + b * 31 + 997).rem_euclid(MOD)
}

fn main() {
    const TABLE_LEN: usize = 64;
    const BUILD_ITERS: i32 = 8000;
    const SUM_ROUNDS: i32 = 8;
    const MOD: i64 = 1_000_000_007;

    let mut table = [0i64; TABLE_LEN];
    for (i, slot) in table.iter_mut().enumerate() {
        let mut v: i64 = 0;
        for k in 0..BUILD_ITERS {
            v = (v + mix((i as i64) + (k as i64))).rem_euclid(MOD);
        }
        *slot = v;
    }
    let mut acc: i64 = 0;
    for _ in 0..SUM_ROUNDS {
        for &x in &table {
            acc = (acc + x).rem_euclid(MOD);
        }
    }
    println!("{}", acc);
}
