import "stdlib/testing.ny"

test fn conf_spawn_003_fire_and_forget() {
    spawn {
        let _ = 1 + 1
    }
    assert_eq(1, 1)
}
