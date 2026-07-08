fn main() -> void {
    print(env_or("__NYRA_DOCS_MISSING__", "fallback"))
    env_set("__NYRA_DOCS_TMP__", "ok")
    print(env("__NYRA_DOCS_TMP__"))
    print(env_has("__NYRA_DOCS_TMP__"))
}
