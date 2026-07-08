// parallel for over a fixed array.
allow_extended

fn main() {
    let nums = [10, 20, 30]
    parallel for n in nums {
        print(n)
    }
    print(999)
}
