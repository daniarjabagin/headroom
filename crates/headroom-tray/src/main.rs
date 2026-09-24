use std::process::ExitCode;

use headroom_tray::app::{SETTINGS_FLAG, TOGGLE_FLAG, run};
use headroom_tray::desktop::has_own_shell;
use tracing_subscriber::EnvFilter;

const AUTOSTART_FLAG: &str = "--autostart";
const FLAGS: [&str; 3] = [TOGGLE_FLAG, SETTINGS_FLAG, AUTOSTART_FLAG];
const USAGE: &str = "Usage: headroom-tray [--toggle | --settings] [--autostart]

Shows Headroom's usage limits in the system tray.

  --toggle     open or close the popup of the running headroom-tray
  --settings   open the settings window
  --autostart  exit quietly on GNOME Shell and KDE Plasma, which have their own Headroom
  --version    print the version
  --help       print this help";

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
    if let Some(unknown) = flags.iter().find(|arg| !FLAGS.contains(&arg.as_str())) {
        eprintln!("headroom-tray: unknown option {unknown}\n\n{USAGE}");
        return ExitCode::from(2);
    }
    let desktop = std::env::var("XDG_CURRENT_DESKTOP").ok();
    if flags.iter().any(|arg| arg == AUTOSTART_FLAG) && has_own_shell(desktop.as_deref()) {
        tracing::info!(?desktop, "this desktop shows Headroom in its own shell");
        return ExitCode::SUCCESS;
    }
    if run(&args) == gtk::glib::ExitCode::SUCCESS {
        ExitCode::SUCCESS
    } else {
        ExitCode::FAILURE
    }
}
