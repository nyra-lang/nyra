// Async state-machine with Future<string> — link test (rt_async + await poll).
// Promise is completed on the caller thread so this gate stays reliable on Windows CI
// (spawn:thread + cooperative await is covered by spawn/runtime tests separately).

import "stdlib/async/future.ny"

async fn greet() -> string {
    let h = async_promise_new()
    async_promise_complete_ptr(h, "nyra")
    let f = Future_from_handle_string(h)
    return await f
}

test fn test_state_machine_string_return() {
    let f = greet()
    let s = await f
    if s != "nyra" {
        assert_eq(1, 0)
    }
}
