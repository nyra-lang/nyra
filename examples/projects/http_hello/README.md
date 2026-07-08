# HTTP hello (Nyra stdlib client)

Run from the **repository root** (`Nyra/`):

```bash
nyra run examples/projects/http_hello/
```

Prints `1` when `fetch("http://example.com/").text()` is non-empty (needs network).

Local one-shot server demo:

```bash
nyra run examples/projects/http_hello/server_main.ny
```

If you see `No such file or directory`, your shell is not in the repo root — run `cd /path/to/Nyra` first.
