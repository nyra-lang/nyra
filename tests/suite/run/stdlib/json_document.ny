// run-stdout: ok
import "stdlib/json/mod.ny"

fn main() {
    let spaced = "{\n  \"a\": 1,\n  \"b\": true\n}"
    let compact = parse_json(spaced)
    if strcmp(compact, "{\"a\":1,\"b\":true}") != 0 {
        print("fail normalize")
        return
    }
    let round = stringify_json(compact)
    if strcmp(round, "{\"a\":1,\"b\":true}") != 0 {
        print("fail stringify")
        return
    }
    let bad = parse_json("{not json")
    if strlen(bad) != 0 {
        print("fail invalid")
        return
    }
    print("ok")
}
