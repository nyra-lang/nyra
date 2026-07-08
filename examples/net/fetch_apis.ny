import "stdlib/net/http/mod.ny"

fn main() {
    // GET — same idea as JS fetch(url)
    let resp = fetch("https://postman-echo.com/get?hello=nyra")
    print(resp.status, resp.is_ok())
    print(jraw(resp.json(), "url"))

    // options on the request object, then verb + url
    let posted = req()
        .header("Accept", "application/json")
        .timeout(8000)
        .form(form().append("hello", "world"))
        .post("https://postman-echo.com/post")
    print(posted.status, jraw(posted.json(), "form"))
}
