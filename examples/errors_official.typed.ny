// Official application errors with explicit annotations.
// Runnable: nyra run examples/errors_official.typed.ny

import "stdlib/error.ny"
import "stdlib/json/mod.ny"

fn config_port_typed(json_text: string) -> Result<i32, Error> {
    let port: i32 = Result_i32_context(json_i32(json_text, "port"), "reading config")?
    return Result.Ok(port)
}

fn config_name_typed(json_text: string) -> Result<string, Error> {
    let name: string = Result_string_context(json_string(json_text, "name"), "reading config")?
    return Result.Ok(name)
}

fn main() {
    let loaded_name: string = match config_name_typed("{\"name\":\"nyra\"}") {
        Result.Ok(v) => v
        Result.Err(err) => Error_format(err)
    }
    print(loaded_name)
    let result: Result<i32, Error> = config_port_typed("{\"port\":8080}")
    let _: i32 = match result {
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
