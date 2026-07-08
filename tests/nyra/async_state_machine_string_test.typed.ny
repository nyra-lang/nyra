// Async state-machine with Future<string> (typed) — v1.31
import "stdlib/async/future.ny"

async fn greet() -> string {
    let h = async_promise_new()
    async_promise_complete_ptr(h, "nyra")
    let f: Future_string = Future_from_handle_string(h)
    return await f
}

test fn test_state_machine_string_return() {
    let f: Future_string = greet()
    let s: string = await f
    if s != "nyra" {
        assert_eq(1, 0)
    }
}
