import "file.ny"
import "dir.ny"
import "sugar.ny"

// Re-export fs helpers (import "stdlib/fs/mod.ny").
// Use read_file / write_file / append_file — not read/write, which collide with libc on Unix.
// Short aliases: slurp / spit / mkdir / mkdir_all / rm / ls / path(...).read()
