mod app;
mod c_cache;
mod c_lib;
mod llvm_obj;
mod prebuilt_rt;
mod prebuilt_tls;
mod ui;
mod cc;
mod llvm_tools;
mod wasm_toolchain;
mod artifacts;
mod bind;
mod commands;
mod debug;
mod fmt;
mod link;
mod nyrapkg;
mod pgo;
mod target;
mod toolchain;
mod watch;
mod timings;
mod daemon;

use clap::Parser as ClapParser;

use app::{apply_color_choice, run};
use app::args::Cli;

fn main() {
    let cli = Cli::parse();
    apply_color_choice(&cli.color);
    if let Err(e) = run(cli) {
        eprintln!("{e}");
        std::process::exit(1);
    }
}
