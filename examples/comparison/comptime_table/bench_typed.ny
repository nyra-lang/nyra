extern fn blackbox_i32(x: i32) -> i32

const TABLE_LEN: i32 = 64
const BUILD_ITERS: i32 = 8000
const SUM_ROUNDS: i32 = 8
const MOD: i32 = 1000000007

fn mix(n: i32) -> i32 {
    let a: i32 = n * 100003
    let b: i32 = n * n
    return (a + b * 31 + 997) % MOD
}

fn main() {
    let mut table: [i32; 64] = [0; 64]
    let mut i: i32 = 0
    while i < TABLE_LEN {
        let mut k: i32 = 0
        let mut v: i32 = 0
        while k < BUILD_ITERS {
            v = (v + mix(i + k)) % MOD
            k = k + 1
        }
        table[i] = v
        i = i + 1
    }
    let mut acc: i32 = 0
    let mut r: i32 = 0
    while r < SUM_ROUNDS {
        let mut j: i32 = 0
        while j < TABLE_LEN {
            acc = (acc + table[j]) % MOD
            j = j + 1
        }
        r = r + 1
    }
    print(blackbox_i32(acc))
}
