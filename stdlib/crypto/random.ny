// Cryptographic random bytes — same ChaCha20 + HW seed as stdlib/random.ny.
extern fn random_hex(byte_count: i32) -> string

fn random_bytes(count: i32) -> string {
    return random_hex(count)
}
