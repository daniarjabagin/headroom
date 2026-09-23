use aws_lc_rs::signature::{ED25519, UnparsedPublicKey};

use super::*;

const TEST_KEY: &str = include_str!("fixtures/test_id_ed25519");
const TEST_PUB: &str = include_str!("fixtures/test_id_ed25519.pub");

fn blob_of_pub_file() -> &'static str {
    TEST_PUB.split_whitespace().nth(1).unwrap()
}

#[test]
fn the_public_blob_matches_the_authorized_keys_form() {
    let key = SigningKey::parse(TEST_KEY).unwrap();
    assert_eq!(key.public_blob(), blob_of_pub_file());
}

#[test]
fn authorization_is_public_blob_colon_ed25519_signature() {
    let key = SigningKey::parse(TEST_KEY).unwrap();
    let challenge = "GET,/api/usage?ts=1790157600";
    let header = key.authorization(challenge);
    let (blob, signature) = header.split_once(':').unwrap();
    assert_eq!(blob, blob_of_pub_file());
    let signature = STANDARD.decode(signature).unwrap();
    assert_eq!(signature.len(), 64);
    let verifier = UnparsedPublicKey::new(&ED25519, key.public_raw());
    assert!(verifier.verify(challenge.as_bytes(), &signature).is_ok());
    assert!(verifier.verify(b"GET,/api/usage?ts=1", &signature).is_err());
}

#[test]
fn debug_output_hides_the_private_key() {
    let key = SigningKey::parse(TEST_KEY).unwrap();
    let debug = format!("{key:?}");
    assert!(debug.contains(blob_of_pub_file()));
    assert!(!debug.contains("pair"));
}

#[test]
fn garbage_and_truncated_keys_are_rejected() {
    let body: Vec<&str> = TEST_KEY.lines().collect();
    let truncated = format!("{}\n{}\n{}", body[0], &body[1][..40], body[body.len() - 1]);
    for pem in [
        "",
        "not a key",
        "-----BEGIN OPENSSH PRIVATE KEY-----\n",
        &truncated,
    ] {
        assert_eq!(SigningKey::parse(pem).unwrap_err(), InvalidKey, "{pem}");
    }
}

#[test]
fn encrypted_keys_are_rejected() {
    let mut blob = pem_body(TEST_KEY).unwrap();
    let cipher_at = MAGIC.len() + 4;
    blob[cipher_at..cipher_at + 4].copy_from_slice(b"aes2");
    let pem = format!(
        "-----BEGIN OPENSSH PRIVATE KEY-----\n{}\n-----END OPENSSH PRIVATE KEY-----\n",
        STANDARD.encode(blob)
    );
    assert_eq!(SigningKey::parse(&pem).unwrap_err(), InvalidKey);
}

#[test]
fn a_mismatched_public_half_is_rejected() {
    let mut blob = pem_body(TEST_KEY).unwrap();
    let public_at = blob
        .windows(KEY_TYPE.len())
        .position(|window| window == KEY_TYPE)
        .unwrap()
        + KEY_TYPE.len()
        + 4;
    blob[public_at] ^= 0xff;
    let pem = STANDARD.encode(blob);
    assert_eq!(SigningKey::parse(&pem).unwrap_err(), InvalidKey);
}
