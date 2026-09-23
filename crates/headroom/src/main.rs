use std::io::{self, Write};

fn main() -> anyhow::Result<()> {
    writeln!(io::stdout(), "headroom {}", env!("CARGO_PKG_VERSION"))?;
    Ok(())
}
