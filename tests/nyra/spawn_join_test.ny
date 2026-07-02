// spawn:task (default) vs spawn:thread — Extended
// nyra test tests/nyra/spawn_join_test.ny
allow_extended

test fn test_spawn_task_join_waits() {
    let h = spawn {
        print(99)
    }
    h.join()
    print(0)
}

test fn test_spawn_task_alias() {
    let h = spawn:task {
        print(11)
    }
    h.join()
}

test fn test_spawn_thread_join_waits() {
    let h = spawn:thread {
        print(88)
    }
    h.join()
    print(1)
}

test fn test_spawn_stmt_fire_and_forget() {
    spawn {
        print(7)
    }
    print(0)
}
