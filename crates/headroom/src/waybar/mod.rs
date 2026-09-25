#[cfg(target_os = "linux")]
mod bus;
mod socket;

use std::io::{self, Write};
use std::time::Duration;

use anyhow::Result;

use crate::cli::WaybarArgs;
use crate::client::{self, Transport};
use crate::paths::Globals;
use crate::render::waybar::WaybarLine;

const RERENDER_EVERY: Duration = Duration::from_secs(60);
const RETRY_AFTER: Duration = Duration::from_secs(5);

struct Printer {
    last: Option<WaybarLine>,
}

impl Printer {
    fn print(&mut self, line: WaybarLine) -> Result<()> {
        if self.last.as_ref() == Some(&line) {
            return Ok(());
        }
        let mut stdout = io::stdout().lock();
        writeln!(stdout, "{}", serde_json::to_string(&line)?)?;
        stdout.flush()?;
        self.last = Some(line);
        Ok(())
    }
}

pub async fn run(globals: &Globals, args: &WaybarArgs) -> Result<()> {
    let mut printer = Printer { last: None };
    match client::transport(globals)? {
        #[cfg(target_os = "linux")]
        Transport::Bus(target) => bus::run(&target, args, &mut printer).await,
        Transport::Socket(path) => socket::run(&path, args, &mut printer).await,
    }
}
