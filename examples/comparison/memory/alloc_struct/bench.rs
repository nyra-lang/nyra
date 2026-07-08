fn main() {
    const N: i64 = 500000;
    const MOD: i64 = 1000000007;
    let mut acc: i64 = 0;
    for i in 0..N {
        let _p = vec![0u8; 8];
        let x = i % 997;
        let y = (i * 3) % 991;
        acc = (acc + x + y).rem_euclid(MOD);
    }
    println!("{}", acc);
}
