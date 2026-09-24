use anyhow::{Context, Result, bail};
use aws_lc_rs::signature::{ED25519, UnparsedPublicKey};
use base64::Engine;
use base64::engine::general_purpose::STANDARD;

pub type PublicKey = [u8; 32];

pub const RELEASE_KEY: PublicKey = [
    0x02, 0x79, 0xa4, 0xbe, 0x0a, 0x0b, 0x5c, 0xf0, 0x02, 0x58, 0xed, 0x9a, 0x2d, 0xca, 0xb5, 0x0e,
    0x45, 0x0f, 0x11, 0x01, 0x7e, 0x5e, 0x7b, 0x25, 0x8a, 0x65, 0x5d, 0x40, 0xb5, 0xe1, 0xe4, 0x9e,
];

const SIGNATURE_LEN: usize = 64;

pub fn verify_signature(key: &PublicKey, sums: &[u8], encoded: &[u8]) -> Result<()> {
    let text = std::str::from_utf8(encoded).context("SHA256SUMS.sig is not text")?;
    let signature = STANDARD
        .decode(text.trim())
        .context("SHA256SUMS.sig is not base64")?;
    if signature.len() != SIGNATURE_LEN {
        bail!("SHA256SUMS.sig is not an Ed25519 signature");
    }
    if UnparsedPublicKey::new(&ED25519, key)
        .verify(sums, &signature)
        .is_err()
    {
        bail!(
            "SHA256SUMS is not signed by the Headroom release key: the release was tampered with"
        );
    }
    Ok(())
}

#[cfg(test)]
pub mod testing {
    use aws_lc_rs::signature::{Ed25519KeyPair, KeyPair};
    use base64::Engine;
    use base64::engine::general_purpose::STANDARD;

    use super::PublicKey;

    pub struct TestSigner(Ed25519KeyPair);

    impl TestSigner {
        pub fn new(seed: u8) -> TestSigner {
            TestSigner(Ed25519KeyPair::from_seed_unchecked(&[seed; 32]).unwrap())
        }

        pub fn public_key(&self) -> PublicKey {
            self.0.public_key().as_ref().try_into().unwrap()
        }

        pub fn sign(&self, message: &[u8]) -> String {
            STANDARD.encode(self.0.sign(message).as_ref())
        }
    }
}

#[cfg(test)]
mod tests {
    use super::testing::TestSigner;
    use super::*;

    const PINNED_PEM: &str =
        include_str!("../../../../packaging/release/release-signing-key.pub.pem");
    const ED25519_SPKI_PREFIX: [u8; 12] = [
        0x30, 0x2a, 0x30, 0x05, 0x06, 0x03, 0x2b, 0x65, 0x70, 0x03, 0x21, 0x00,
    ];
    const SUMS: &[u8] = b"2cf2  headroom-0.5.0-x86_64-linux-musl.tar.gz\n";

    #[test]
    fn a_signature_by_the_key_passes() {
        let signer = TestSigner::new(7);
        let encoded = format!("{}\n", signer.sign(SUMS));
        assert!(verify_signature(&signer.public_key(), SUMS, encoded.as_bytes()).is_ok());
    }

    #[test]
    fn another_key_or_changed_sums_fail() {
        let signer = TestSigner::new(7);
        let encoded = signer.sign(SUMS);
        let other = TestSigner::new(8).public_key();
        let wrong_key = verify_signature(&other, SUMS, encoded.as_bytes()).unwrap_err();
        assert!(
            wrong_key
                .to_string()
                .contains("not signed by the Headroom release key")
        );
        let changed = verify_signature(&signer.public_key(), b"other", encoded.as_bytes());
        assert!(changed.is_err());
    }

    #[test]
    fn malformed_signatures_fail() {
        let key = TestSigner::new(7).public_key();
        for encoded in [&b""[..], b"not base64!", b"AAAA", b"\xff\xfe"] {
            assert!(
                verify_signature(&key, SUMS, encoded).is_err(),
                "{encoded:?}"
            );
        }
    }

    #[test]
    fn an_openssl_signature_by_the_release_key_verifies() {
        let message = include_bytes!("fixtures/interop.txt");
        let signature = include_bytes!("fixtures/interop.txt.sig");
        assert!(verify_signature(&RELEASE_KEY, message, signature).is_ok());
        assert!(verify_signature(&RELEASE_KEY, b"changed", signature).is_err());
    }

    #[test]
    fn the_pinned_key_matches_the_published_public_key_file() {
        let body: String = PINNED_PEM
            .lines()
            .filter(|line| !line.starts_with("-----"))
            .collect();
        let der = STANDARD.decode(body).unwrap();
        assert_eq!(der[..12], ED25519_SPKI_PREFIX);
        assert_eq!(der[12..], RELEASE_KEY);
    }
}
