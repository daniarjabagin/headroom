use std::fmt::Write;
use std::fs;
use std::os::unix::fs::symlink;
use std::path::Path;

const TCP_HEADER: &str = "  sl  local_address rem_address   st tx_queue rx_queue tr tm->when retrnsmt   uid  timeout inode\n";

pub(super) fn fake_process(root: &Path, pid: u32, argv: &[&str], listen: &[(u16, u64)]) {
    let dir = root.join(pid.to_string());
    fs::create_dir_all(dir.join("fd")).unwrap();
    fs::create_dir_all(dir.join("net")).unwrap();
    fs::write(dir.join("cmdline"), argv.join("\0") + "\0").unwrap();
    let mut table = TCP_HEADER.to_owned();
    for (index, (port, inode)) in listen.iter().enumerate() {
        symlink(
            format!("socket:[{inode}]"),
            dir.join("fd").join(format!("{}", index + 3)),
        )
        .unwrap();
        writeln!(
            table,
            "   {index}: 0100007F:{port:04X} 00000000:0000 0A 00000000:00000000 00:00000000 00000000  1000        0 {inode} 1"
        )
        .unwrap();
    }
    symlink("/dev/null", dir.join("fd/0")).unwrap();
    fs::write(dir.join("net/tcp"), table).unwrap();
}
