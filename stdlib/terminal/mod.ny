import "../io.ny"
import "../strings.ny"

extern fn stdin_read_line(prompt: string) -> string

fn stdin_read(_max_bytes: i32) -> string {
    return stdin_read_line("")
}

fn stdout_write(msg: string) -> void {
    stdout_write_str(msg)
}

fn stdout_writeln(msg: string) -> void {
    stdout_writeln_str(msg)
}

fn stderr_write(msg: string) -> void {
    let line = strcat("[stderr] ", msg)
    stdout_writeln_str(line)
}

fn stderr_writeln(msg: string) -> void {
    stderr_write(msg)
}
