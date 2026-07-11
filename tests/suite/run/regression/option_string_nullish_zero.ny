// run-stdout: Anonymous
// REG-OPTION-STRING-NULLISH-ZERO: inferred Option + ?? must link, run, and exit cleanly.
import "stdlib/option.ny"

fn main() {
    let name = Option.None
    let default = "Anonymous"
    print(name ?? default)
}
