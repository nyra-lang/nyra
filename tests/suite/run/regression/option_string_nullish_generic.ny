// run-stdout: Anonymous
// REG-OPTION-STRING-NULLISH-GENERIC: same ?? logic through a generic helper must link+run cleanly.
import "stdlib/option.ny"

fn unwrap_or<T>(opt: Option<T>, fallback: T) -> T {
    return opt ?? fallback
}

fn main() {
    let name: Option<string> = Option.None
    print(unwrap_or(name, "Anonymous"))
}
