use std::io::{BufRead, BufReader};
use std::path::Path;
use std::process::{Child, Command, Stdio};

use tempfile::TempDir;

const HEADROOM_NAME: &str = "io.github.daniarjabagin.Headroom";

pub struct PrivateBus {
    child: Child,
    pub address: String,
    _dir: TempDir,
}

fn config(listen_dir: &Path) -> String {
    format!(
        r#"<!DOCTYPE busconfig PUBLIC "-//freedesktop//DTD D-Bus Bus Configuration 1.0//EN"
 "http://www.freedesktop.org/standards/dbus/1.0/busconfig.dtd">
<busconfig>
  <type>session</type>
  <keep_umask/>
  <listen>unix:dir={}</listen>
  <auth>EXTERNAL</auth>
  <policy context="default">
    <allow send_destination="*" eavesdrop="true"/>
    <allow eavesdrop="true"/>
    <allow own="*"/>
  </policy>
</busconfig>
"#,
        listen_dir.display()
    )
}

fn spawn(dir: &Path) -> Option<Child> {
    let config_file = dir.join("bus.conf");
    std::fs::write(&config_file, config(dir)).ok()?;
    Command::new("dbus-daemon")
        .arg(format!("--config-file={}", config_file.display()))
        .args(["--nofork", "--print-address"])
        .env_clear()
        .env("HOME", dir)
        .env("XDG_DATA_HOME", dir)
        .env("XDG_DATA_DIRS", dir)
        .env("XDG_CONFIG_HOME", dir)
        .env("XDG_RUNTIME_DIR", dir)
        .current_dir(dir)
        .stdout(Stdio::piped())
        .stderr(Stdio::null())
        .spawn()
        .ok()
}

impl PrivateBus {
    pub fn start() -> Option<PrivateBus> {
        let dir = tempfile::tempdir().ok()?;
        let mut child = spawn(dir.path())?;
        let mut line = String::new();
        let read = child
            .stdout
            .take()
            .map(|out| BufReader::new(out).read_line(&mut line));
        let address = line.trim().to_owned();
        let bus = PrivateBus {
            child,
            address,
            _dir: dir,
        };
        (matches!(read, Some(Ok(_))) && !bus.address.is_empty()).then_some(bus)
    }
}

impl Drop for PrivateBus {
    fn drop(&mut self) {
        self.child.kill().ok();
        self.child.wait().ok();
    }
}

pub async fn activation_is_impossible(address: &str) -> Result<bool, Box<dyn std::error::Error>> {
    let connection = zbus::connection::Builder::address(address)?.build().await?;
    let bus = zbus::fdo::DBusProxy::new(&connection).await?;
    let activatable = bus.list_activatable_names().await?;
    if activatable
        .iter()
        .any(|name| name.as_str() != "org.freedesktop.DBus")
    {
        return Ok(false);
    }
    let started = bus
        .start_service_by_name(HEADROOM_NAME.try_into()?, 0)
        .await;
    Ok(matches!(started, Err(zbus::fdo::Error::ServiceUnknown(_))))
}
