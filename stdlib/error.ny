import "strings.ny"
import "option.ny"

extern fn error_stack_trace() -> string

struct Error {
    code: i32
    kind: string
    message: string
    cause: string
    trace: string
}

fn Error_new(code: i32, kind: string, message: string) -> Error {
    return Error {
        code: code
        kind: kind
        message: message
        cause: ""
        trace: error_stack_trace()
    }
}

fn Error_io(message: string) -> Error {
    return Error_new(1, "io", message)
}

fn Error_json(message: string) -> Error {
    return Error_new(2, "json", message)
}

fn Error_async(message: string) -> Error {
    return Error_new(3, "async", message)
}

fn Error_invalid(message: string) -> Error {
    return Error_new(4, "invalid", message)
}

fn Error_headline(err: Error) -> string {
    return strcat(strcat(err.kind, ": "), err.message)
}

fn Error_context(err: Error, context: string) -> Error {
    return Error {
        code: err.code
        kind: err.kind
        message: strcat(strcat(context, ": "), err.message)
        cause: Error_headline(err)
        trace: err.trace
    }
}

fn Error_format(err: Error) -> string {
    let mut out = Error_headline(err)
    if strlen(err.cause) > 0 {
        out = strcat(strcat(out, "\ncaused by: "), err.cause)
    }
    if strlen(err.trace) > 0 {
        out = strcat(strcat(out, "\nstack trace:\n"), err.trace)
    }
    return out
}

fn Error_print(err: Error) -> void {
    print(Error_format(err))
}

impl Error {
    fn context(self, context: string) -> Error {
        return Error_context(self, context)
    }

    fn format(self) -> string {
        return Error_format(self)
    }

    fn show(self) -> void {
        Error_print(self)
    }

    fn headline(self) -> string {
        return Error_headline(self)
    }
}

fn err_io(message: string) -> Error {
    return Error_io(message)
}

fn err_json(message: string) -> Error {
    return Error_json(message)
}

fn err_invalid(message: string) -> Error {
    return Error_invalid(message)
}

fn Result_string_context(result: Result<string, Error>, context: string) -> Result<string, Error> {
    return match result {
        Result.Ok(v) => Result__string_S_Error.Ok(v.clone())
        Result.Err(err) => Result__string_S_Error.Err(Error_context(err, context))
    }
}

fn Result_i32_context(result: Result<i32, Error>, context: string) -> Result<i32, Error> {
    return match result {
        Result.Ok(v) => Result__i32_S_Error.Ok(v)
        Result.Err(err) => Result__i32_S_Error.Err(Error_context(err, context))
    }
}

