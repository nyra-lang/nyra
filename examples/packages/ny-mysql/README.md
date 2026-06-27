# ny-mysql

NyraPkg binding for **libmysqlclient** — thin FFI shim.

## Install

```bash
nyra pkg install ny-mysql@^0.1.0
```

## Build deps

- macOS: `brew install mysql-client` (or `mysql`)
- Debian: `default-libmysqlclient-dev`

## Connection spec

`Mysql_connect` expects semicolon-separated DSN:

```
127.0.0.1;3306;nyra;root;secret
```

(host;port;database;user;password)

Set `MYSQL_SPEC` for the smoke test in `main.ny`.

## API

- `Mysql_connect(spec)` → connection handle
- `Mysql_exec(conn, sql)` → 0 on success
- `Mysql_query_scalar(conn, sql)` → first column of first row
- `Mysql_close(conn)`
