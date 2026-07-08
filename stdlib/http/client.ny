import "../net/http/client.ny"

// Legacy path: body-only. Prefer `stdlib/net/http/mod.ny` → `fetch(url) -> HttpResponse`.
fn fetch_text(url: string) -> string {
    return get(url)
}
