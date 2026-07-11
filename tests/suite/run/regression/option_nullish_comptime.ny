// run-stdout: 7
// REG-OPTION-NULLISH-COMPTIME: Option.None ?? default must fold under #[comptime].
import "stdlib/option.ny"

#[comptime]
fn pick() -> i32 {
    let x = Option.None
    return x ?? 7
}

const N = pick()

fn main() {
    print(N)
}
