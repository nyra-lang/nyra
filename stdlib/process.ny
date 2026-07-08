import "vec_str.ny"

extern fn command_run_argv(program: string, args: ptr) -> i32
extern fn command_exec_capture_argv(program: string, args: ptr) -> string
extern fn json_get_string(json: string, key: string) -> string
extern fn json_get_i32(json: string, key: string) -> i32

struct ExecResult {
    code: i32
    stdout: string
    stderr: string
}

fn exec_result_from_json(raw: string) -> ExecResult {
    let stdout_text = json_get_string(raw, "stdout")
    let stderr_text = json_get_string(raw, "stderr")
    return ExecResult {
        code: json_get_i32(raw, "code"),
        stdout: if strlen(stdout_text) > 0 { stdout_text } else { "" },
        stderr: if strlen(stderr_text) > 0 { stderr_text } else { "" },
    }
}

fn exec(program: string, args: StrVec) -> ExecResult {
    let raw = command_exec_capture_argv(program, StrVec_raw(args))
    return exec_result_from_json(raw)
}

struct Command {
    program: string
    args: StrVec
}

fn Command_new(program: string) -> Command {
    return Command { program: program, args: StrVec_new() }
}

fn cmd(program: string) -> Command {
    return Command_new(program)
}

impl Command {
    fn arg(self, value: string) -> Command {
        return Command {
            program: self.program,
            args: self.args.push(value),
        }
    }

    fn run(self) -> i32 {
        return command_run_argv(self.program, StrVec_raw(self.args))
    }

    fn output(self) -> ExecResult {
        return exec(self.program, self.args)
    }
}

struct Process {
    pid: i32
}

fn Process_new(pid: i32) -> Process {
    return Process { pid: pid }
}

fn run_command(program: string) -> i32 {
    let args = StrVec_new()
    return command_run_argv(program, StrVec_raw(args))
}

fn command_run_args(program: string, args: StrVec) -> i32 {
    return command_run_argv(program, StrVec_raw(args))
}
