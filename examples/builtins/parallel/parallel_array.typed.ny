// parallel for over a fixed array.
allow_extended

fn main() -> void {
    let nums: [i32; 3] = [10, 20, 30]
    parallel for n in nums {
        print(n)
    }
    print(999)
}
