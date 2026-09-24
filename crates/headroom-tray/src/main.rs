use std::process::ExitCode;

use headroom_tray::app::{TOGGLE_FLAG, run};
use tracing_subscriber::EnvFilter;

const USAGE: &str = "Usage: headroom-tray [--toggle]

Shows Headroom's usage limits in the system tray.

  --toggle   open or close the popup of the running headroom-tray
  --version  print the version
  --help     print this help";

fn main() -> ExitCode {
    tracing_subscriber::fmt()
        .with_env_filter(
            EnvFilter::try_from_default_env().unwrap_or_else(|_| EnvFilter::new("warn")),
        )
        .with_writer(std::io::stderr)
        .init();
    let args: Vec<String> = std::env::args().collect();
    let flags = &args[1.min(args.len())..];
    if flags.iter().any(|arg| arg == "--help" || arg == "-h") {
        println!("{USAGE}");
        return ExitCode::SUCCESS;
    }
    if flags.iter().any(|arg| arg == "--version") {
        println!("headroom-tray {}", env!("CARGO_PKG_VERSION"));
        return ExitCode::SUCCESS;
    }
    if let Some(unknown) = flags.iter().find(|arg| *arg != TOGGLE_FLAG) {
        eprintln!("headroom-tray: unknown option {unknown}\n\n{USAGE}");
        return ExitCode::from(2);
    }
    if run(&args) == gtk::glib::ExitCode::SUCCESS {
        ExitCode::SUCCESS
    } else {
        ExitCode::FAILURE
    }
}
