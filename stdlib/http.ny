import "net/http/client.ny"
import "net/http/response.ny"

fn fetch(url: string) -> string {
    return get(url)
}
