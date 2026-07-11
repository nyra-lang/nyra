// run-stdout: Hamdy
// REG-OPTION-STRING-NULLISH-SOME: Option.Some(string) heap-clones literal; drop frees only Some.
import "stdlib/option.ny"

fn main() {
    let name: Option<string> = Option.Some("Hamdy")
    print(name ?? "Anonymous")
}
