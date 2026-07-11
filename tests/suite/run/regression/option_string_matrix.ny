// run-stdout: Anonymous
// run-stdout: Anonymous
// run-stdout: Anonymous
// run-stdout: Hamdy
// REG-OPTION-STRING-MATRIX: zero-types / explicit / generic / Some — one file, one link+run gate.
import "stdlib/option.ny"

fn unwrap_or_str(opt: Option<string>, fallback: string) -> string {
    return opt ?? fallback
}

fn main() {
    // 1) zero-types (inference)
    let a = Option.None
    print(a ?? "Anonymous")

    // 2) explicit Option<string>
    let b: Option<string> = Option.None
    print(b ?? "Anonymous")

    // 3) generics / helper
    let c: Option<string> = Option.None
    print(unwrap_or_str(c, "Anonymous"))

    // 4) Some payload
    let d: Option<string> = Option.Some("Hamdy")
    print(d ?? "Anonymous")
}
