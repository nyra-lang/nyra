import "stdlib/alloc/arena.ny"

test fn test_arena_bump_reset_free() {
    let arena = Arena_new(4096)
    let p = Arena_alloc(arena, 64)
    let _ = p
    Arena_reset(arena)
    let p2 = Arena_alloc(arena, 32)
    let _ = p2
    Arena_free(arena)
}

fn main() {
    print(0)
}
