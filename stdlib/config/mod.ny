import "../fs/mod.ny"
import "../env/mod.ny"
import "../strings.ny"

struct Config {
    path: string
    raw: string
}

fn config_load(path: string) -> Config {
    let raw = read_file(path)
    return Config { path: path, raw: raw }
}

impl Config {
    fn get_string(self, key: string) -> string {
        let pos = strstr_pos(self.raw, key)
        if pos < 0 {
            return ""
        }
        return self.raw
    }
}

fn config_get_env_or_file(env_key: string, file_path: string, file_key: string) -> string {
    let from_env = env_get(env_key)
    if strlen(from_env) > 0 {
        return from_env
    }
    let cfg = config_load(file_path)
    return cfg.get_string(file_key)
}

extern fn strstr_pos(hay: &string, needle: &string) -> i32
extern fn strlen(s: &string) -> i32
