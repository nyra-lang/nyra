// Official application errors: Result<T, Error> + context + formatted trace.
// Runnable: nyra run examples/errors_official.ny

import "stdlib/error.ny"
import "stdlib/json/mod.ny"

fn config_port(json_text) -> Result<i32, Error> {
    let port = Result_i32_context(json_i32(json_text, "port"), "reading config")?
    return Result.Ok(port)
}

fn config_name(json_text) -> Result<string, Error> {
    let name = Result_string_context(json_string(json_text, "name"), "reading config")?
    return Result.Ok(name)
}

fn main() {
    let loaded_name = match config_name("{\"name\":\"nyra\"}") {
        Result.Ok(v) => v
        Result.Err(err) => Error_format(err)
    }
    print(loaded_name)
    let result = config_port("{\"port\":8080}")
    let _ = match result {
        Result.Ok(port) => {
            print(port)
            0
        }
        Result.Err(err) => {
            Error_print(err)
            1
        }
    }
}
