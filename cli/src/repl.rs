//! Interactive Nyra REPL — compile-and-run session (no bytecode VM).

use std::io::{self, BufRead, Write};
use std::path::PathBuf;
use std::process::Command;
use std::time::{SystemTime, UNIX_EPOCH};

use compiler::NYRA_VERSION;

use crate::app::args::{OptFlags, StabilityFlags, TargetArgs};
use crate::app::session::compile_and_link;
use crate::target::TargetSpec;

pub fn repl() -> Result<(), String> {
    println!("Nyra {NYRA_VERSION} — interactive REPL");
    println!("  :help  commands   :quit  exit   Ctrl-D also quits");
    println!();

    let mut session = Session::default();
    let stdin = io::stdin();
    let mut stdout = io::stdout();
    let mut pending = String::new();

    loop {
        let prompt = if pending.is_empty() { "nyra> " } else { "....> " };
        write!(stdout, "{prompt}").map_err(|e| e.to_string())?;
        stdout.flush().map_err(|e| e.to_string())?;

        let mut line = String::new();
        let n = stdin.lock().read_line(&mut line).map_err(|e| e.to_string())?;
        if n == 0 {
            println!();
            break;
        }
        let trimmed = line.trim_end_matches(['\r', '\n']);

        if pending.is_empty() {
            let t = trimmed.trim();
            if t.is_empty() {
                continue;
            }
            if let Some(cmd) = t.strip_prefix(':') {
                if handle_meta(cmd, &mut session)? {
                    break;
                }
                continue;
            }
        }

        let cont = trimmed.trim_end();
        if let Some(head) = cont.strip_suffix('\\') {
            pending.push_str(head.trim_end());
            pending.push('\n');
            continue;
        }
        pending.push_str(trimmed);
        pending.push('\n');

        if !is_complete(&pending) {
            continue;
        }

        let chunk = std::mem::take(&mut pending);
        if let Err(e) = session.eval_chunk(chunk.trim()) {
            eprintln!("error: {e}");
        }
    }
    Ok(())
}

#[derive(Default)]
struct Session {
    /// Top-level items (`fn`, `struct`, `import`, …).
    items: Vec<String>,
    /// Statements kept inside `main` (`let`, assignments, …).
    locals: Vec<String>,
}

impl Session {
    fn eval_chunk(&mut self, chunk: &str) -> Result<(), String> {
        let chunk = chunk.trim();
        if chunk.is_empty() {
            return Ok(());
        }

        if is_toplevel_item(chunk) {
            let mut trial_items = self.items.clone();
            trial_items.push(chunk.to_string());
            compile_run(&build_program(&trial_items, &self.locals, None))?;
            self.items.push(chunk.to_string());
            println!("ok");
            return Ok(());
        }

        if is_local_stmt(chunk) {
            let mut trial_locals = self.locals.clone();
            trial_locals.push(chunk.to_string());
            compile_run(&build_program(&self.items, &trial_locals, None))?;
            self.locals.push(chunk.to_string());
            println!("ok");
            return Ok(());
        }

        // Expression — print result.
        let expr = strip_trailing_semi(chunk);
        compile_run(&build_program(
            &self.items,
            &self.locals,
            Some(expr),
        ))?;
        Ok(())
    }
}

fn handle_meta(cmd: &str, session: &mut Session) -> Result<bool, String> {
    let mut parts = cmd.split_whitespace();
    let name = parts.next().unwrap_or("").to_ascii_lowercase();
    match name.as_str() {
        "q" | "quit" | "exit" => Ok(true),
        "h" | "help" => {
            print_help();
            Ok(false)
        }
        "clear" | "reset" => {
            session.items.clear();
            session.locals.clear();
            println!("session cleared");
            Ok(false)
        }
        "items" => {
            if session.items.is_empty() && session.locals.is_empty() {
                println!("(empty session)");
            } else {
                for it in &session.items {
                    println!("{it}");
                }
                if !session.locals.is_empty() {
                    println!("// locals:");
                    for l in &session.locals {
                        println!("  {l}");
                    }
                }
            }
            Ok(false)
        }
        "load" => {
            let Some(path) = parts.next() else {
                return Err("usage: :load <file.ny>".into());
            };
            let src = std::fs::read_to_string(path).map_err(|e| format!("{path}: {e}"))?;
            session.items.push(src);
            println!("loaded {path}");
            Ok(false)
        }
        "type" => {
            let rest = cmd["type".len()..].trim();
            if rest.is_empty() {
                return Err("usage: :type <expression>".into());
            }
            // Best-effort: compile with a typed discard binding and report success.
            let prog = build_program(&session.items, &session.locals, Some(rest));
            match compile_check_only(&prog) {
                Ok(()) => println!("ok (compiles)"),
                Err(e) => eprintln!("{e}"),
            }
            Ok(false)
        }
        other => Err(format!("unknown command `:{other}` — try :help")),
    }
}

fn print_help() {
    println!(
        r#"Nyra REPL commands:
  :help           Show this help
  :quit / :exit   Leave the REPL
  :clear          Reset session definitions
  :items          Show accumulated definitions
  :load <file>    Append a .ny file to the session
  :type <expr>    Check that an expression typechecks

Enter declarations (fn, struct, …) to keep them in the session.
Enter statements (let x = …) to keep them as locals.
Enter an expression to compile, run, and print it.
Use a trailing `\`, or incomplete braces, for multi-line input."#
    );
}

fn is_toplevel_item(chunk: &str) -> bool {
    let t = chunk.trim_start();
    const PREFIXES: &[&str] = &[
        "fn ",
        "test fn ",
        "struct ",
        "enum ",
        "trait ",
        "impl ",
        "import ",
        "const ",
        "module ",
        "extern ",
        "macro ",
        "union ",
        "pub ",
        "priv ",
        "async fn ",
    ];
    PREFIXES.iter().any(|p| t.starts_with(p))
}

fn is_local_stmt(chunk: &str) -> bool {
    let t = chunk.trim_start();
    t.starts_with("let ")
        || t.starts_with("mut ")
        || t.starts_with("defer ")
        || t.starts_with("if ")
        || t.starts_with("while ")
        || t.starts_with("for ")
        || t.starts_with("match ")
        || t.starts_with("return ")
        || t.starts_with("print(")
        || t.starts_with("print ")
}

fn strip_trailing_semi(s: &str) -> &str {
    s.trim().trim_end_matches(';').trim()
}

fn is_complete(buf: &str) -> bool {
    let t = buf.trim_end();
    if t.ends_with('\\') {
        return false;
    }
    let mut depth = 0i32;
    let mut in_str = false;
    let mut escape = false;
    for ch in t.chars() {
        if in_str {
            if escape {
                escape = false;
            } else if ch == '\\' {
                escape = true;
            } else if ch == '"' {
                in_str = false;
            }
            continue;
        }
        match ch {
            '"' => in_str = true,
            '{' | '(' | '[' => depth += 1,
            '}' | ')' | ']' => depth -= 1,
            _ => {}
        }
    }
    depth <= 0
}

fn build_program(items: &[String], locals: &[String], expr: Option<&str>) -> String {
    let mut out = String::new();
    out.push_str("// nyra repl session\n");
    for it in items {
        out.push_str(it);
        if !it.ends_with('\n') {
            out.push('\n');
        }
        out.push('\n');
    }
    out.push_str("fn main() {\n");
    for loc in locals {
        out.push_str("    ");
        out.push_str(loc.trim());
        if !loc.trim().ends_with('}') && !loc.trim().ends_with(';') {
            // Nyra statements often omit semicolons — keep as-is.
        }
        out.push('\n');
    }
    if let Some(e) = expr {
        // Prefer printing; if the expression is already a print call, run raw.
        let e = e.trim();
        if e.starts_with("print(") || e.starts_with("print ") {
            out.push_str("    ");
            out.push_str(e);
            out.push('\n');
        } else {
            out.push_str("    print(");
            out.push_str(e);
            out.push_str(")\n");
        }
    }
    out.push_str("}\n");
    out
}

fn repl_workdir() -> Result<PathBuf, String> {
    let nanos = SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .map(|d| d.as_nanos())
        .unwrap_or(0);
    let dir = std::env::temp_dir().join(format!("nyra-repl-{nanos}"));
    std::fs::create_dir_all(&dir).map_err(|e| e.to_string())?;
    Ok(dir)
}

fn compile_run(source: &str) -> Result<(), String> {
    let dir = repl_workdir()?;
    let path = dir.join("repl.ny");
    std::fs::write(&path, source).map_err(|e| e.to_string())?;

    let spec = TargetSpec::host();
    let bin = compile_and_link(
        &path,
        &OptFlags::default(),
        false,
        false,
        false,
        &spec,
        None,
        &StabilityFlags::default(),
        false,
        false,
        false,
        None,
    )
    .map_err(|e| {
        let _ = std::fs::remove_dir_all(&dir);
        e
    })?;

    let output = Command::new(&bin)
        .output()
        .map_err(|e| format!("failed to run repl binary: {e}"))?;
    let _ = std::fs::remove_dir_all(&dir);

    io::stdout()
        .write_all(&output.stdout)
        .map_err(|e| e.to_string())?;
    io::stderr()
        .write_all(&output.stderr)
        .map_err(|e| e.to_string())?;

    if !output.status.success() {
        return Err(format!(
            "program exited with status {}",
            output.status.code().unwrap_or(-1)
        ));
    }
    Ok(())
}

fn compile_check_only(source: &str) -> Result<(), String> {
    let dir = repl_workdir()?;
    let path = dir.join("repl_type.ny");
    std::fs::write(&path, source).map_err(|e| e.to_string())?;
    let options = compiler::CompileOptions {
        stop_after: Some(compiler::CompileStage::Borrow),
        ..compiler::CompileOptions::default()
    };
    let out = compiler::Compiler::compile_source(source, path.to_str().unwrap_or("repl.ny"), &options)
        .map_err(|e| e.to_string())?;
    let _ = std::fs::remove_dir_all(&dir);
    if !out.lexer_errors.is_empty()
        || !out.parser_errors.is_empty()
        || !out.type_errors.is_empty()
        || !out.borrow_errors.is_empty()
    {
        let mut msgs = Vec::new();
        for e in out
            .lexer_errors
            .iter()
            .chain(&out.parser_errors)
            .chain(&out.type_errors)
            .chain(&out.borrow_errors)
        {
            msgs.push(e.message.clone());
        }
        return Err(msgs.join("\n"));
    }
    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn detects_items_and_stmts() {
        assert!(is_toplevel_item("fn add(a: i32) -> i32 { return a }"));
        assert!(is_local_stmt("let x = 1"));
        assert!(!is_toplevel_item("1 + 2"));
    }

    #[test]
    fn brace_completeness() {
        assert!(!is_complete("fn foo() {\n"));
        assert!(is_complete("fn foo() {\n  return 1\n}\n"));
        assert!(!is_complete("1 + \\\n"));
    }

    #[test]
    fn builds_print_wrapper() {
        let src = build_program(&[], &[], Some("1 + 2"));
        assert!(src.contains("print(1 + 2)"));
        assert!(src.contains("fn main()"));
    }
}
