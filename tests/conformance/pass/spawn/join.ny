import "stdlib/testing.ny"

test fn conf_spawn_001_task_join_waits() {
    let h = spawn {
        let _ = 99 + 1
    }
    h.join()
    assert_eq(1, 1)
}

test fn conf_spawn_002_thread_join_waits() {
    let h = spawn:thread {
        let _ = 88 + 2
    }
    h.join()
    assert_eq(1, 1)
}
