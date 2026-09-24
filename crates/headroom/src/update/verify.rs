use anyhow::{Result, bail};
use sha2::{Digest, Sha256};

#[must_use]
pub fn listed_digest<'a>(sums: &'a str, name: &str) -> Option<&'a str> {
    sums.lines().find_map(|line| {
        let (digest, file) = line.split_once(char::is_whitespace)?;
        let file = file.trim_start();
        let file = file.strip_prefix('*').unwrap_or(file);
        (file == name).then_some(digest)
    })
}

pub fn verify(sums: &str, name: &str, data: &[u8]) -> Result<()> {
    let Some(expected) = listed_digest(sums, name) else {
        bail!("{name} is not listed in SHA256SUMS");
    };
    let actual = hex::encode(Sha256::digest(data));
    if !actual.eq_ignore_ascii_case(expected) {
        bail!("checksum mismatch for {name}: the download is corrupt or was tampered with");
    }
    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;

    const HELLO_SHA256: &str = "2cf24dba5fb0a30e26e83b2ac5b9e29e1b161e5c1fa7425e73043362938b9824";

    fn sums() -> String {
        format!(
            "{HELLO_SHA256}  headroom-0.5.0-x86_64-linux-musl.tar.gz\n\
             0000000000000000000000000000000000000000000000000000000000000000 *SHA256SUMS.sig\n"
        )
    }

    #[test]
    fn digests_are_found_in_text_and_binary_mode_lines() {
        let sums = sums();
        assert_eq!(
            listed_digest(&sums, "headroom-0.5.0-x86_64-linux-musl.tar.gz"),
            Some(HELLO_SHA256)
        );
        assert!(listed_digest(&sums, "SHA256SUMS.sig").is_some());
        assert_eq!(listed_digest(&sums, "headroom-0.5.0-x86_64"), None);
    }

    #[test]
    fn only_the_listed_digest_passes() {
        let sums = sums();
        let name = "headroom-0.5.0-x86_64-linux-musl.tar.gz";
        assert!(verify(&sums, name, b"hello").is_ok());
        let mismatch = verify(&sums, name, b"hello!").unwrap_err();
        assert!(mismatch.to_string().starts_with("checksum mismatch"));
        let unlisted = verify(&sums, "other.tar.gz", b"hello").unwrap_err();
        assert_eq!(
            unlisted.to_string(),
            "other.tar.gz is not listed in SHA256SUMS"
        );
    }
}
