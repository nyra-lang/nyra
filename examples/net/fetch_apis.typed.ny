import "stdlib/net/http/mod.ny"

fn main() -> void {
    let resp: HttpResponse = fetch("https://postman-echo.com/get?hello=nyra")
    print(resp.status)
    let posted: HttpResponse = req()
        .form(form().append("hello", "world"))
        .post("https://postman-echo.com/post")
    print(posted.status)
}
