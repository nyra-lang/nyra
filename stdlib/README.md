# Nyra standard library

Nyra ships a **strong, batteries-included** standard library with the compiler. Application developers should get production-grade building blocks **in-tree** — not install separate packages for everyday crypto, databases, serialization, or networking.

## Design goals

These constraints come from the Nyra project guidelines and apply to every stdlib module:

| Goal | How stdlib honors it |
|------|----------------------|
| **High performance, low memory** | Native implementations in `stdlib/rt/`; demand-driven linking — only linked code you use |
| **Types optional** | APIs work with inference; annotations never required for the six value kinds (strings, numbers, arrays, objects, booleans, and optional type annotations) |
| **Modular layout** | Small focused `.ny` files under `stdlib/` — no monolithic mega-modules |
| **Stable import paths** | `import "stdlib/crypto/mod.ny"` stays stable as implementations mature |

## What belongs in stdlib

| Domain | Modules | Target |
|--------|---------|--------|
| **Crypto** | `crypto/mod.ny` | SHA-256/512, HMAC, AES, secure random |
| **Databases** | `db/sql.ny`, `db/sqlite.ny`, `db/postgres.ny`, `db/mysql.ny` | Generic driver, SQLite, PG/MySQL stubs |
| **Serialization** | `json/mod.ny`, `serialize/mod.ny` | Full JSON, TOML, YAML, binary serde |
| **Networking** | `net/tcp.ny`, `net/http/`, `net/websocket.ny`, `net/udp.ny` | TCP, HTTP, WebSocket, UDP |
| **Compression** | `compress/mod.ny` | gzip, zip |
| **Encoding** | `encoding/mod.ny`, `encoding/csv.ny` | base64, hex, URL, CSV |
| **CLI / parsing** | `strconv/mod.ny`, `flag/mod.ny`, `bufio/mod.ny` | atoi/itoa, CLI flags, line scanner |
| **Concurrency** | `sync/mod.ny`, `context/mod.ny` | Mutex, RWMutex, WaitGroup, Atomic, cancellation |
| **HTTP bodies** | `mime/mod.ny` | multipart/form-data |
| **Crypto (OpenSSL)** | `crypto/rsa.ny`, `crypto/x509.ny` | RSA sign/encrypt, X.509 PEM parse |
| **TLS server** | `tls.ny` | `tls_listen`, `tls_accept` (OpenSSL) |
| **Mail** | `net/smtp.ny`, `net/mail.ny` | SMTP client, message builder |
| **RPC** | `net/rpc.ny` | JSON-RPC 2.0 encode/decode |
| **Templates** | `text/template`, `html/template` | `{{key}}` substitution, HTML escape |
| **Logging** | `slog/mod.ny` | Structured JSON logs |
| **XML** | `encoding/xml.ny` | Element encode, tag text decode |
| **Compress** | `compress/flate.ny`, `compress/bzip2.ny` | zlib flate; bzip2 stub |
| **Unicode** | `unicode/utf8.ny` | UTF-8 validation |
| **Embed** | `embed/mod.ny` | Runtime embed FS + manifest |
| **Testing+** | `testing/fstest.ny`, `testing/quick.ny` | File assertions, property checks |
| **Archive** | `archive/tar.ny`, `archive/zip.ny` | tar, real ZIP (store method) |
| **Core** | `vec.ny`, `map.ny`, `fs/`, `strings/`, `time/` | Collections, FS, strings, time |

Framework stacks (e.g. Sonic microservices, Socket.io hubs) stay **outside** stdlib in `sonic/` — stdlib provides primitives; frameworks compose them.

## NyraPkg role

**NyraPkg** complements stdlib — it does **not** replace core domains:

- Community libraries, game engines, niche drivers
- Faster iteration before a module graduates into stdlib
- Optional alternatives (e.g. a third-party ORM)

Patterns proven in `examples/packages/` can be **upstreamed** into `stdlib/` + `stdlib/rt/`.

## Implementation status

Some modules are still **stubs or MVP** while native code lands. They compile and keep stable APIs; they will gain real implementations without import-path changes.

| Status | Examples |
|--------|----------|
| **Shipped** | `vec`, `vec_str`, `map`, `collections/*`, `fs`, `strings`, `crypto` (SHA/HMAC/AES), `encoding/base64`, `net/tcp`, `net/http`, `net/udp`, `net/websocket`, `compress`, `serialize`, `json`, `db/sqlite`, `strconv`, `flag`, `bufio`, `sync`, `context`, `process` (POSIX + Windows), `env_set`, `bridge`, `terminal/pty`, `encoding/csv`, `archive/zip`, `mime`, `time`, `random_bytes`, `embed`, `slog`, `testing/fstest`, `testing/quick` |
| **MVP / partial** | `json`/`serialize` (not full schema serde — use NyraPkg `ny-serde`), `uuid`, `url` encoding |
| **Native when linked** | `db/postgres` (`link pq`), `db/mysql` (`link mysqlclient`), `compress/bzip2` (`link bz2`) |

See `webDocs/stdlib.html#status` for the live inventory.

## Auto-prelude (lazy, on-demand)

Nyra does **not** merge the entire stdlib like Go’s global import. The compiler keeps a **virtual symbol table** (`StdlibVirtualIndex`) of every public stdlib name → source file, then **lazy-resolves** only what your program references:

1. Parse your entry file (+ explicit `import`s).
2. Collect used identifiers (`collect_program_uses`).
3. Look up missing names in the virtual table.
4. Load and merge **only** those `.ny` modules (plus their `import` chains).
5. Repeat until stable (fixed-point).

**Developer experience:** call `Vec_i32_new()`, `read_file()`, `sha256()` without imports.  
**Compiler/linker:** smaller AST and LLVM IR; C runtime was already demand-driven via `runtime_map.rs`.

Opt out: `# no_std` or `nyra build --no-prelude` (explicit `import "stdlib/…"` required).

**Compiler intrinsics** (`abs_i32`, `min_i32`, …) are always available — they lower to LLVM intrinsics and do not require the prelude. See `examples/builtins/math_intrinsics.ny`.

Implementation: `compiler/resolve/src/prelude.rs` · `compiler/resolve/src/symbols.rs` · `compiler/types/src/intrinsics.rs`.

