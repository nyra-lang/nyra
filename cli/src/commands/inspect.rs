use std::path::{Path, PathBuf};

use compiler::{
    BindingInspectReport, BindingStatus, CompileOptions, CompileStage, Compiler, InspectQuery,
    InspectRole,
};

use crate::app::args::StabilityFlags;
use crate::ui::Ui;

/// Parse `file:line` for `--at`.
pub(crate) fn parse_at(at: &str) -> Result<(String, usize), String> {
    let (file, line_s) = at
        .rsplit_once(':')
        .ok_or_else(|| format!("expected file:line, got `{at}`"))?;
    if file.is_empty() {
        return Err(format!("expected file:line, got `{at}`"));
    }
    let line: usize = line_s
        .parse()
        .map_err(|_| format!("invalid line number in `{at}`"))?;
    if line == 0 {
        return Err("line number must be >= 1".into());
    }
    Ok((file.to_string(), line))
}

pub(crate) fn inspect(
    project: &Path,
    name: &str,
    at: &str,
    stability: &StabilityFlags,
) -> Result<(), String> {
    let (file, line) = parse_at(at)?;
    let query = InspectQuery {
        file: normalize_query_file(project, &file),
        line,
        name: name.to_string(),
    };
    let query_display = format!("{}:{}", query_file_display(&file), line);

    let options = CompileOptions {
        stop_after: Some(CompileStage::Borrow),
        deny_extended: stability.deny_extended,
        deny_warnings: stability.deny_warnings,
        inspect_query: Some(query),
        ..CompileOptions::default()
    };

    let output = if project.is_dir() {
        Compiler::compile_project(project, &options)?
    } else {
        Compiler::compile_file(project, &options)?
    };

    if Compiler::report_errors(&output) {
        return Err("inspect failed (compile errors)".into());
    }

    let Some(report) = output.inspect_report else {
        return Err(format!(
            "binding `{name}` not in scope at {query_display}",
        ));
    };

    print_report(&report, &Ui::new());
    Ok(())
}

fn print_report(report: &BindingInspectReport, ui: &Ui) {
    let file_display = Path::new(&report.file)
        .file_name()
        .and_then(|s| s.to_str())
        .unwrap_or(&report.file);

    println!(
        "\n{} {} {} {}:{} {} {}",
        ui.bold(&report.name),
        ui.dim(&format!("({})", report.ty)),
        ui.dim("@"),
        ui.path(file_display),
        ui.bold(&report.line.to_string()),
        ui.dim("in"),
        ui.cyan(&report.func),
    );

    if report.role == InspectRole::Borrower {
        println!(
            "  {}  {} {}",
            ui.dim("you inspect:"),
            ui.yellow(&format!("`{}`", report.name)),
            ui.dim("(borrower)")
        );
        println!(
            "  {}  {} {}",
            ui.dim("heap owner:"),
            ui.green(&format!("`{}`", report.heap_owner)),
            ui.dim("(owns Move value)")
        );
        if report.borrow_chain.len() > 1 {
            println!("  {}", ui.bold("borrow chain:"));
            println!("    {}", format_borrow_chain_colored(&report.borrow_chain, ui));
        }
    } else {
        if report.kind.is_move() && report.ownership_chain.len() > 1 {
            println!("  {}", ui.bold("ownership chain:"));
            println!(
                "    {}",
                format_move_chain_colored(&report.ownership_chain, ui)
            );
        }

        print!("  {}  {}", ui.dim("you inspect:"), ui.bold(&format!("`{}`", report.name)));
        match report.role {
            InspectRole::Owner => {
                println!();
                println!(
                    "  {}  {} {}",
                    ui.dim("current owner:"),
                    ui.green(&format!("`{}`", report.current_owner)),
                    ui.dim("(owns value)")
                );
            }
            InspectRole::MovedAway => {
                println!();
                println!(
                    "  {}  {} {}",
                    ui.dim("current owner:"),
                    ui.green(&format!("`{}`", report.current_owner)),
                    ui.dim(&format!("(`{}` moved away)", report.name))
                );
            }
            InspectRole::CopyBinding => {
                println!();
                println!(
                    "  {}  {} {}",
                    ui.dim("current owner:"),
                    ui.bold(&format!("`{}`", report.name)),
                    ui.dim("(Copy)")
                );
            }
            _ => println!(),
        }
    }

    let base_kind = if report.kind.is_move() { "Move" } else { "Copy" };
    let kind_text = if report.role == InspectRole::Borrower {
        format!("{base_kind} (reference)")
    } else {
        base_kind.to_string()
    };
    println!("  {}  {}", ui.dim("kind:"), ui.magenta(&kind_text));

    let binding_text = match report.binding_status {
        BindingStatus::Valid if report.role == InspectRole::Borrower => "valid (borrow)",
        BindingStatus::Valid if report.kind.is_move() => "owned (valid)",
        BindingStatus::Valid => "valid (Copy)",
        BindingStatus::Moved => "moved (invalid)",
        BindingStatus::NotInScope => "not in scope",
    };
    println!(
        "  {}  {}",
        ui.dim("binding:"),
        if binding_text.contains("moved") {
            ui.yellow(binding_text)
        } else {
            ui.green(binding_text)
        }
    );

    if report.binding_status == BindingStatus::Moved {
        if let Some(dest) = &report.moved_to {
            println!(
                "  {}  {}",
                ui.dim("moved to:"),
                ui.yellow(&format!("`{dest}`"))
            );
        } else {
            println!("  {}  {}", ui.dim("moved:"), ui.yellow("yes"));
        }
    } else {
        println!("  {}  {}", ui.dim("moved:"), ui.green("no"));
    }

    if let Some(src) = &report.moved_from {
        println!(
            "  {}  {}",
            ui.dim("moved from:"),
            ui.yellow(&format!("`{src}`"))
        );
    }
    if let Some(src) = &report.borrows_from {
        let mutability = if src.mutable { "&mut" } else { "&" };
        println!(
            "  {}  {} {}",
            ui.dim("borrows from:"),
            ui.cyan(&format!("`{}`", src.name)),
            ui.dim(&format!("({mutability})"))
        );
    }

    if report.borrowed_by.is_empty() {
        println!("  {}  {}", ui.dim("borrowed by:"), ui.dim("(none)"));
    } else {
        for b in &report.borrowed_by {
            let mutability = if b.mutable { "&mut" } else { "&" };
            println!(
                "  {}  {} {} {}",
                ui.dim("borrowed by:"),
                ui.cyan(&format!("`{}`", b.name)),
                ui.dim(&format!("({mutability}{})", b.ty)),
                ui.dim(&format!("expires after line {}", b.expires_after_line))
            );
        }
    }

    if let Some(drop) = &report.drop {
        println!(
            "  {}  {} {}",
            ui.dim("drop:"),
            ui.dim(drop.kind),
            ui.dim(&format!("at scope exit (line {})", drop.at_scope_exit_line))
        );
    }

    println!();
}

fn format_move_chain_colored(chain: &[String], ui: &Ui) -> String {
    chain
        .iter()
        .enumerate()
        .map(|(i, name)| {
            if i == 0 {
                ui.bold(name)
            } else {
                format!("{} {}", ui.yellow("──move──►"), ui.bold(name))
            }
        })
        .collect::<Vec<_>>()
        .join(" ")
}

fn format_borrow_chain_colored(chain: &[String], ui: &Ui) -> String {
    chain
        .iter()
        .enumerate()
        .map(|(i, name)| {
            if i == 0 {
                ui.green(name)
            } else {
                format!("{} {}", ui.cyan("◄──borrow──"), ui.yellow(name))
            }
        })
        .collect::<Vec<_>>()
        .join(" ")
}

fn normalize_query_file(project: &Path, file: &str) -> String {
    let path = PathBuf::from(file);
    if path.is_absolute() {
        return file.to_string();
    }
    if project.is_dir() {
        return project.join(file).to_string_lossy().into_owned();
    }
    if let Some(parent) = project.parent() {
        return parent.join(file).to_string_lossy().into_owned();
    }
    file.to_string()
}

fn query_file_display(file: &str) -> &str {
    Path::new(file)
        .file_name()
        .and_then(|s| s.to_str())
        .unwrap_or(file)
}
