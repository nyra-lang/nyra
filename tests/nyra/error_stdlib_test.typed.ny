import "stdlib/error.ny"
import "stdlib/fs/result.ny"
import "stdlib/json/mod.ny"

fn config_name_typed(json_text: string) -> Result<string, Error> {
    let name: string = Result_string_context(json_string(json_text, "name"), "loading config")?
    return Result.Ok(name)
}

test fn test_official_error_context_format_typed() {
    let err: Error = Error_context(Error_json("missing field: name"), "loading config")
    let text: string = Error_format(err)
    if strstr_pos(text, "loading config") < 0 {
        assert_eq(0, 1)
    }
    if strstr_pos(text, "caused by: json") < 0 {
        assert_eq(0, 1)
    }
}

test fn test_json_result_success_typed() {
    let value: string = match config_name_typed("{\"name\":\"nyra\"}") {
        Result.Ok(v) => v
        Result.Err(err) => Error_format(err)
    }
    assert_str_eq(value, "nyra")
}

test fn test_fs_result_error_typed() {
    let status: i32 = match read_text("target/nyra-missing-config-typed.json") {
        Result.Ok(_text) => 0
        Result.Err(_err) => 1
    }
    assert_eq(status, 1)
}
