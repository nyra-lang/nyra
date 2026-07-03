//! Persistent compiler daemon (Unix socket) for warm parse caches between edits.

use std::io::{BufRead, BufReader, Write};
use std::path::{Path, PathBuf};
use std::process::Command;

use serde::{Deserialize, Serialize};

use crate::app::args::{OptFlags, StabilityFlags, TargetArgs};
use crate::app::session::{build, compile_and_link};
use crate::commands::check;
use crate::target::{TargetSpec, validate_native_cpu};

#[derive(Debug, Serialize, Deserialize)]
struct DaemonRequest {
    cmd: String,
    path: String,
    #[serde(default)]
    release: bool,
    #[serde(default)]
    timings: bool,
}

#[derive(Debug, Serialize, Deserialize)]
struct DaemonResponse {
    ok: bool,
    #[serde(skip_serializing_if = "Option::is_none")]
    exit_code: Option<i32>,
    #[serde(default, skip_serializing_if = "String::is_empty")]
    stderr: String,
    #[serde(default, skip_serializing_if = "String::is_empty")]
    stdout: String,
    #[serde(skip_serializing_if = "Option::is_none")]
    error: Option<String>,
}

pub fn socket_path() -> PathBuf {
    if let Ok(dir) = std::env::var("NYRA_DAEMON_SOCK") {
        return PathBuf::from(dir);
    }
    dirs::home_dir()
        .unwrap_or_else(|| PathBuf::from("."))
        .join(".nyra")
        .join("run")
        .join("daemon.sock")
}

pub fn wants_daemon(opt: &OptFlags) -> bool {
    if opt.no_daemon {
        return false;
    }
    if opt.use_daemon {
        return true;
    }
    socket_path().exists()
}

pub fn try_dispatch_run(
    path: &Path,
    opt: &OptFlags,
    target_args: &TargetArgs,
    stability: &StabilityFlags,
    no_std: bool,
    freestanding: bool,
    no_prelude: bool,
) -> Result<Option<Result<(), String>>, String> {
    if !wants_daemon(opt) {
        return Ok(None);
    }
    let _ = (stability, no_std, freestanding, no_prelude, target_args);
    match send_request(DaemonRequest {
        cmd: "run".into(),
        path: path.to_string_lossy().into_owned(),
        release: opt.release,
        timings: opt.timings,
    }) {
        Ok(resp) => Ok(Some(handle_run_response(resp))),
        Err(e) if daemon_unavailable(&e) => Ok(None),
        Err(e) => Err(e),
    }
}

pub fn try_dispatch_check(
    path: &Path,
    opt: &OptFlags,
    stability: &StabilityFlags,
) -> Result<Option<Result<(), String>>, String> {
    if !wants_daemon(opt) {
        return Ok(None);
    }
    let _ = stability;
    match send_request(DaemonRequest {
        cmd: "check".into(),
        path: path.to_string_lossy().into_owned(),
        release: false,
        timings: opt.timings,
    }) {
        Ok(resp) => Ok(Some(handle_simple_response(resp))),
        Err(e) if daemon_unavailable(&e) => Ok(None),
        Err(e) => Err(e),
    }
}

fn daemon_unavailable(err: &str) -> bool {
    err.contains("Connection refused")
        || err.contains("No such file")
        || err.contains("Connection reset")
}

fn handle_run_response(resp: DaemonResponse) -> Result<(), String> {
    relay_output(&resp);
    if resp.ok {
        if let Some(code) = resp.exit_code {
            if code != 0 {
                return Err(format!("program exited with status {code}"));
            }
        }
        Ok(())
    } else {
        Err(resp.error.unwrap_or_else(|| "daemon run failed".into()))
    }
}

fn handle_simple_response(resp: DaemonResponse) -> Result<(), String> {
    relay_output(&resp);
    if resp.ok {
        Ok(())
    } else {
        Err(resp.error.unwrap_or_else(|| "daemon command failed".into()))
    }
}

fn relay_output(resp: &DaemonResponse) {
    if !resp.stderr.is_empty() {
        eprint!("{}", resp.stderr);
    }
    if !resp.stdout.is_empty() {
        print!("{}", resp.stdout);
    }
}

fn send_request(req: DaemonRequest) -> Result<DaemonResponse, String> {
    #[cfg(unix)]
    {
        use std::os::unix::net::UnixStream;
        let mut stream = UnixStream::connect(socket_path()).map_err(|e| e.to_string())?;
        let line = serde_json::to_string(&req).map_err(|e| e.to_string())?;
        stream.write_all(line.as_bytes()).map_err(|e| e.to_string())?;
        stream.write_all(b"\n").map_err(|e| e.to_string())?;
        let mut reader = BufReader::new(stream);
        let mut out = String::new();
        reader.read_line(&mut out).map_err(|e| e.to_string())?;
        serde_json::from_str(out.trim()).map_err(|e| format!("invalid daemon response: {e}"))
    }
    #[cfg(not(unix))]
    {
        let _ = req;
        Err("compiler daemon requires Unix".into())
    }
}

pub fn serve(background: bool) -> Result<(), String> {
    #[cfg(unix)]
    {
        let sock = socket_path();
        if let Some(parent) = sock.parent() {
            std::fs::create_dir_all(parent).map_err(|e| e.to_string())?;
        }
        if sock.exists() {
            let _ = std::fs::remove_file(&sock);
        }
        if background {
            return spawn_background();
        }
        use std::os::unix::net::UnixListener;
        let listener = UnixListener::bind(&sock).map_err(|e| e.to_string())?;
        eprintln!(
            "nyra daemon: listening on {} (Ctrl+C to stop)",
            sock.display()
        );
        for stream in listener.incoming() {
            let mut stream = stream.map_err(|e| e.to_string())?;
            let mut reader = BufReader::new(stream.try_clone().map_err(|e| e.to_string())?);
            let mut line = String::new();
            if reader.read_line(&mut line).is_err() {
                continue;
            }
            let req: DaemonRequest = match serde_json::from_str(line.trim()) {
                Ok(r) => r,
                Err(e) => {
                    let resp = DaemonResponse {
                        ok: false,
                        exit_code: None,
                        stderr: String::new(),
                        stdout: String::new(),
                        error: Some(format!("bad request: {e}")),
                    };
                    write_response(&mut stream, &resp);
                    continue;
                }
            };
            let resp = handle_request(&req);
            write_response(&mut stream, &resp);
        }
        Ok(())
    }
    #[cfg(not(unix))]
    {
        let _ = background;
        Err("compiler daemon requires Unix".into())
    }
}

#[cfg(unix)]
fn spawn_background() -> Result<(), String> {
    let exe = std::env::current_exe().map_err(|e| e.to_string())?;
    Command::new(exe)
        .args(["internal", "daemon"])
        .stdin(std::process::Stdio::null())
        .stdout(std::process::Stdio::null())
        .spawn()
        .map_err(|e| e.to_string())?;
    eprintln!(
        "nyra daemon: started in background ({})",
        socket_path().display()
    );
    Ok(())
}

#[cfg(unix)]
fn write_response(stream: &mut std::os::unix::net::UnixStream, resp: &DaemonResponse) {
    if let Ok(line) = serde_json::to_string(resp) {
        let _ = stream.write_all(line.as_bytes());
        let _ = stream.write_all(b"\n");
    }
}

#[cfg(unix)]
fn handle_request(req: &DaemonRequest) -> DaemonResponse {
    let path = PathBuf::from(&req.path);
    let opt = OptFlags {
        release: req.release,
        timings: req.timings,
        no_daemon: true,
        ..OptFlags::default()
    };
    let stability = StabilityFlags::default();
    let target_args = TargetArgs::default();

    match req.cmd.as_str() {
        "ping" => DaemonResponse {
            ok: true,
            exit_code: None,
            stderr: String::new(),
            stdout: "pong\n".into(),
            error: None,
        },
        "check" => {
            let (result, stderr) = capture_stderr(|| check::check(&path, &stability));
            daemon_result(result, stderr, String::new())
        }
        "run" => daemon_run(&path, &opt, &target_args, &stability),
        "build" => {
            let (result, stderr) = capture_stderr(|| {
                build(
                    &path,
                    None,
                    &opt,
                    false,
                    false,
                    false,
                    &target_args,
                    &stability,
                    false,
                    false,
                    false,
                )
            });
            daemon_result(result, stderr, String::new())
        }
        other => DaemonResponse {
            ok: false,
            exit_code: None,
            stderr: String::new(),
            stdout: String::new(),
            error: Some(format!("unknown daemon command '{other}'")),
        },
    }
}

#[cfg(unix)]
fn daemon_run(
    path: &Path,
    opt: &OptFlags,
    target_args: &TargetArgs,
    stability: &StabilityFlags,
) -> DaemonResponse {
    let spec = match target_args.resolve() {
        Ok(s) => s,
        Err(e) => {
            return DaemonResponse {
                ok: false,
                exit_code: None,
                stderr: String::new(),
                stdout: String::new(),
                error: Some(e),
            };
        }
    };
    if spec.is_wasm {
        return DaemonResponse {
            ok: false,
            exit_code: None,
            stderr: String::new(),
            stdout: String::new(),
            error: Some("daemon run does not support wasm".into()),
        };
    }
    if let Err(e) = validate_native_cpu(
        &spec,
        opt.native_cpu || (opt.release && !spec.is_cross && !opt.no_native_cpu),
    ) {
        return DaemonResponse {
            ok: false,
            exit_code: None,
            stderr: String::new(),
            stdout: String::new(),
            error: Some(e),
        };
    }
    if spec.is_cross {
        return DaemonResponse {
            ok: false,
            exit_code: None,
            stderr: String::new(),
            stdout: String::new(),
            error: Some("daemon run does not support cross targets".into()),
        };
    }
    let (compile_result, stderr) = capture_stderr(|| {
        compile_and_link(
            path,
            opt,
            false,
            false,
            false,
            &spec,
            None,
            stability,
            false,
            false,
            false,
            None,
        )
    });
    let bin_path = match compile_result {
        Ok(p) => p,
        Err(e) => {
            return DaemonResponse {
                ok: false,
                exit_code: None,
                stderr,
                stdout: String::new(),
                error: Some(e),
            };
        }
    };
    match Command::new(&bin_path).output() {
        Ok(output) => {
            let stdout = String::from_utf8_lossy(&output.stdout).into_owned();
            let code = output.status.code().unwrap_or(-1);
            DaemonResponse {
                ok: output.status.success(),
                exit_code: Some(code),
                stderr,
                stdout,
                error: if output.status.success() {
                    None
                } else {
                    Some(format!("program exited with status {code}"))
                },
            }
        }
        Err(e) => DaemonResponse {
            ok: false,
            exit_code: None,
            stderr,
            stdout: String::new(),
            error: Some(format!("Failed to run {}: {e}", bin_path.display())),
        },
    }
}

#[cfg(unix)]
fn daemon_result(
    result: Result<(), String>,
    stderr: String,
    stdout: String,
) -> DaemonResponse {
    match result {
        Ok(()) => DaemonResponse {
            ok: true,
            exit_code: None,
            stderr,
            stdout,
            error: None,
        },
        Err(e) => DaemonResponse {
            ok: false,
            exit_code: None,
            stderr,
            stdout,
            error: Some(e),
        },
    }
}

#[cfg(unix)]
fn capture_stderr<F: FnOnce() -> T, T>(f: F) -> (T, String) {
    use std::io::Read;
    use std::os::unix::io::FromRawFd;

    unsafe {
        let mut pipe_fds = [0i32; 2];
        if libc::pipe(pipe_fds.as_mut_ptr()) != 0 {
            return (f(), String::new());
        }
        let stderr_fd = libc::STDERR_FILENO;
        let saved = libc::dup(stderr_fd);
        libc::dup2(pipe_fds[1], stderr_fd);
        libc::close(pipe_fds[1]);
        let result = f();
        libc::fflush(std::ptr::null_mut());
        libc::dup2(saved, stderr_fd);
        libc::close(saved);
        let mut file = std::fs::File::from_raw_fd(pipe_fds[0]);
        let mut captured = String::new();
        let _ = file.read_to_string(&mut captured);
        (result, captured)
    }
}

#[cfg(not(unix))]
pub fn serve(_background: bool) -> Result<(), String> {
    Err("compiler daemon requires Unix".into())
}
