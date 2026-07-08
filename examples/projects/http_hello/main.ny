import "../../../stdlib/net/http/mod.ny"

fn main() {
    let resp = fetch("http://example.com/")
    if strlen(resp.text()) > 0 {
        print(1)
    } else {
        print(0)
    }
}
