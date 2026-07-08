fn main() {
    use std::collections::HashMap;
    const MOD: i64 = 1000000007;
    let mut m: HashMap<i32, i32> = HashMap::with_capacity(10000);
    let mut acc: i64 = 0;
    for i in 0..200000 {
        let k = (i % 10000) as i32;
        m.insert(k, i as i32);
        acc = (acc + *m.get(&k).unwrap_or(&0) as i64).rem_euclid(MOD);
    }
    println!("{}", acc);
}
