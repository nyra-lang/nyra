fn main() {
    const MOD: i64 = 1000000007;
    let mut s = String::from("a");
    let mut acc: i64 = 0;
    for _ in 0..100000 {
        s.push('x');
        acc = (acc + s.len() as i64).rem_euclid(MOD);
    }
    println!("{}", acc);
}
