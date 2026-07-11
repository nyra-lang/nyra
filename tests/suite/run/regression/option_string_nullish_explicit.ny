// run-stdout: Anonymous
// REG-OPTION-STRING-NULLISH-EXPLICIT: Option<string> = None + ?? must not LLVM-crash or free garbage on drop.
import "stdlib/option.ny"

fn main() {
    let name: Option<string> = Option.None
    let default = "Anonymous"
    print(name ?? default)
}
