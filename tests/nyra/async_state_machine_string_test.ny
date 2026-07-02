// Async state-machine with Future<string> — v1.31
// nyra test tests/nyra/async_state_machine_string_test.ny

import "stdlib/async/future.ny"

async fn greet() -> string {
    let h = async_promise_new()
    spawn:thread {
        async_promise_complete_ptr(h, "nyra")
    }
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
