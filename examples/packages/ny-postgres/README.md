# ny-postgres

NyraPkg binding for **libpq** — thin FFI shim over PostgreSQL's official C client.

## Install

```bash
nyra pkg install ny-postgres@^0.1.0
```

## Build deps

- macOS: `brew install libpq` (ensure `pkg-config --libs libpq` works)
- Debian: `libpq-dev`

## Connection spec

Pass a libpq connection string to `Postgres_connect`, e.g.:

```
host=127.0.0.1 port=5432 dbname=nyra user=postgres password=secret
```

Or set `DATABASE_URL` for the smoke test in `main.ny`.

## API

- `Postgres_connect(spec)` → connection handle
- `Postgres_exec(conn, sql)` → 0 on success
- `Postgres_query_scalar(conn, sql)` → first column of first row
- `Postgres_close(conn)`
