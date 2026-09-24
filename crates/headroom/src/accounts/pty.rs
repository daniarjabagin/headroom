use std::ffi::CString;
use std::fs::File;
use std::os::fd::OwnedFd;
use std::process::Stdio;

use anyhow::{Context, Result};
use rustix::fs::{Mode, OFlags, open};
use rustix::pty::{OpenptFlags, grantpt, openpt, ptsname, unlockpt};
use rustix::termios::{LocalModes, OptionalActions, Winsize, tcgetattr, tcsetattr, tcsetwinsize};

const COLUMNS: u16 = 120;
const ROWS: u16 = 40;

#[cfg(target_os = "linux")]
fn open_master() -> rustix::io::Result<OwnedFd> {
    openpt(OpenptFlags::RDWR | OpenptFlags::NOCTTY | OpenptFlags::CLOEXEC)
}

#[cfg(not(target_os = "linux"))]
fn open_master() -> rustix::io::Result<OwnedFd> {
    let master = openpt(OpenptFlags::RDWR | OpenptFlags::NOCTTY)?;
    rustix::io::fcntl_setfd(&master, rustix::io::FdFlags::CLOEXEC)?;
    Ok(master)
}

pub struct Pty {
    master: OwnedFd,
    slave: OwnedFd,
}

impl Pty {
    pub fn open() -> Result<Pty> {
        let master = open_master().context("could not open a pseudo-terminal")?;
        grantpt(&master).context("could not grant the pseudo-terminal")?;
        unlockpt(&master).context("could not unlock the pseudo-terminal")?;
        let name = ptsname(&master, Vec::new()).context("could not name the pseudo-terminal")?;
        let slave = open_slave(&name)?;
        let pty = Pty { master, slave };
        pty.set_size(COLUMNS, ROWS)?;
        Ok(pty)
    }

    #[cfg(test)]
    fn echo(&self) -> Result<bool> {
        let modes = tcgetattr(&self.slave).context("could not read terminal modes")?;
        Ok(modes.local_modes.contains(LocalModes::ECHO))
    }

    pub fn set_echo(&self, on: bool) -> Result<()> {
        let mut modes = tcgetattr(&self.slave).context("could not read terminal modes")?;
        modes.local_modes.set(LocalModes::ECHO, on);
        tcsetattr(&self.slave, OptionalActions::Now, &modes).context("could not set terminal modes")
    }

    fn set_size(&self, columns: u16, rows: u16) -> Result<()> {
        let size = Winsize {
            ws_row: rows,
            ws_col: columns,
            ws_xpixel: 0,
            ws_ypixel: 0,
        };
        tcsetwinsize(&self.master, size).context("could not size the pseudo-terminal")
    }

    pub fn slave_stdio(&self) -> Result<Stdio> {
        let fd = self
            .slave
            .try_clone()
            .context("could not share the pseudo-terminal")?;
        Ok(Stdio::from(fd))
    }

    pub fn into_master(self) -> File {
        File::from(self.master)
    }
}

fn open_slave(name: &CString) -> Result<OwnedFd> {
    let flags = OFlags::RDWR | OFlags::NOCTTY | OFlags::CLOEXEC;
    open(name.as_c_str(), flags, Mode::empty()).context("could not open the pseudo-terminal")
}

#[cfg(test)]
mod tests {
    use std::io::{Read, Write};

    use rustix::termios::{isatty, tcgetwinsize};

    use super::*;

    fn read_some(master: &mut File) -> String {
        let mut text = Vec::new();
        let mut buffer = [0_u8; 256];
        while !text.ends_with(b"\n") {
            let read = master.read(&mut buffer).unwrap();
            text.extend_from_slice(&buffer[..read]);
        }
        String::from_utf8_lossy(&text).into_owned()
    }

    #[test]
    fn opens_a_sized_terminal_pair_and_closes_the_slave() {
        let pty = Pty::open().unwrap();
        assert!(isatty(&pty.slave));
        let size = tcgetwinsize(&pty.slave).unwrap();
        assert_eq!((size.ws_col, size.ws_row), (COLUMNS, ROWS));
        let mut slave = File::from(pty.slave.try_clone().unwrap());
        let mut master = pty.into_master();
        slave.write_all(b"hello\n").unwrap();
        assert_eq!(read_some(&mut master), "hello\r\n");
        drop(slave);
        let mut rest = [0_u8; 16];
        assert!(master.read(&mut rest).is_err());
    }

    #[test]
    fn echo_can_be_switched_off_and_on() {
        let pty = Pty::open().unwrap();
        assert!(pty.echo().unwrap());
        pty.set_echo(false).unwrap();
        assert!(!pty.echo().unwrap());
        let mut slave = File::from(pty.slave.try_clone().unwrap());
        let mut master = File::from(pty.master.try_clone().unwrap());
        master.write_all(b"quiet\n").unwrap();
        slave.write_all(b"marker\n").unwrap();
        assert_eq!(read_some(&mut master), "marker\r\n");
        let mut line = [0_u8; 6];
        slave.read_exact(&mut line).unwrap();
        assert_eq!(&line, b"quiet\n");
        pty.set_echo(true).unwrap();
        assert!(pty.echo().unwrap());
        master.write_all(b"loud\n").unwrap();
        assert_eq!(read_some(&mut master), "loud\r\n");
    }
}
