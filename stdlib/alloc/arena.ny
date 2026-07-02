// Bump-pointer arena allocator — thread-unsafe, O(1) reset.

extern fn arena_new(capacity: i64) -> ptr
extern fn arena_alloc(arena: ptr, nbytes: i64) -> ptr
extern fn arena_reset(arena: ptr) -> void
extern fn arena_free(arena: ptr) -> void

struct Arena {
    handle: ptr
}

fn Arena_new(capacity: i64) -> Arena {
    return Arena { handle: arena_new(capacity) }
}

fn Arena_alloc(arena: Arena, nbytes: i64) -> ptr {
    return arena_alloc(arena.handle, nbytes)
}

fn Arena_reset(arena: Arena) -> void {
    arena_reset(arena.handle)
}

fn Arena_free(arena: Arena) -> void {
    arena_free(arena.handle)
}
