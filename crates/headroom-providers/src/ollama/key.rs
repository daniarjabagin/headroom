use std::fmt;

use aws_lc_rs::signature::{Ed25519KeyPair, KeyPair};
use base64::Engine;
use base64::engine::general_purpose::STANDARD;

const MAGIC: &[u8] = b"openssh-key-v1\0";
const KEY_TYPE: &[u8] = b"ssh-ed25519";
const NONE: &[u8] = b"none";
const SEED_LEN: usize = 32;
const PRIVATE_LEN: usize = 64;

#[derive(Debug, Clone, Copy, PartialEq, Eq, thiserror::Error)]
#[error("not an unencrypted OpenSSH Ed25519 key")]
pub(super) struct InvalidKey;

pub(super) struct SigningKey {
    public_blob: String,
    pair: Ed25519KeyPair,
}

impl fmt::Debug for SigningKey {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.debug_struct("SigningKey")
            .field("public_blob", &self.public_blob)
            .finish_non_exhaustive()
    }
}

impl SigningKey {
    pub(super) fn parse(pem: &str) -> Result<SigningKey, InvalidKey> {
        let blob = pem_body(pem)?;
        let container = blob.strip_prefix(MAGIC).ok_or(InvalidKey)?;
        let mut reader = Wire(container);
        expect(reader.string()?, NONE)?;
        expect(reader.string()?, NONE)?;
        reader.string()?;
        if reader.u32()? != 1 {
            return Err(InvalidKey);
        }
        let public = reader.string()?;
        let private = reader.string()?;
        let public_raw = public_key(public)?;
        let pair = key_pair(private, public_raw)?;
        Ok(SigningKey {
            public_blob: STANDARD.encode(public),
            pair,
        })
    }

    pub(super) fn authorization(&self, challenge: &str) -> String {
        let signature = self.pair.sign(challenge.as_bytes());
        format!(
            "{}:{}",
            self.public_blob,
            STANDARD.encode(signature.as_ref())
        )
    }

    #[cfg(test)]
    pub(super) fn public_blob(&self) -> &str {
        &self.public_blob
    }

    #[cfg(test)]
    pub(super) fn public_raw(&self) -> Vec<u8> {
        self.pair.public_key().as_ref().to_vec()
    }
}

fn pem_body(pem: &str) -> Result<Vec<u8>, InvalidKey> {
    let body: String = pem
        .lines()
        .map(str::trim)
        .filter(|line| !line.is_empty() && !line.starts_with("-----"))
        .collect();
    STANDARD.decode(body).map_err(|_| InvalidKey)
}

fn public_key(blob: &[u8]) -> Result<&[u8], InvalidKey> {
    let mut reader = Wire(blob);
    expect(reader.string()?, KEY_TYPE)?;
    let raw = reader.string()?;
    if raw.len() == SEED_LEN {
        Ok(raw)
    } else {
        Err(InvalidKey)
    }
}

fn key_pair(section: &[u8], public_raw: &[u8]) -> Result<Ed25519KeyPair, InvalidKey> {
    let mut reader = Wire(section);
    if reader.u32()? != reader.u32()? {
        return Err(InvalidKey);
    }
    expect(reader.string()?, KEY_TYPE)?;
    expect(reader.string()?, public_raw)?;
    let private = reader.string()?;
    if private.len() != PRIVATE_LEN {
        return Err(InvalidKey);
    }
    let (seed, embedded_public) = private.split_at(SEED_LEN);
    expect(embedded_public, public_raw)?;
    let pair =
        Ed25519KeyPair::from_seed_and_public_key(seed, public_raw).map_err(|_| InvalidKey)?;
    expect(pair.public_key().as_ref(), public_raw)?;
    Ok(pair)
}

fn expect(actual: &[u8], wanted: &[u8]) -> Result<(), InvalidKey> {
    if actual == wanted {
        Ok(())
    } else {
        Err(InvalidKey)
    }
}

struct Wire<'a>(&'a [u8]);

impl<'a> Wire<'a> {
    fn take(&mut self, count: usize) -> Result<&'a [u8], InvalidKey> {
        if count > self.0.len() {
            return Err(InvalidKey);
        }
        let (head, rest) = self.0.split_at(count);
        self.0 = rest;
        Ok(head)
    }

    fn u32(&mut self) -> Result<u32, InvalidKey> {
        let bytes: [u8; 4] = self.take(4)?.try_into().map_err(|_| InvalidKey)?;
        Ok(u32::from_be_bytes(bytes))
    }

    fn string(&mut self) -> Result<&'a [u8], InvalidKey> {
        let len = usize::try_from(self.u32()?).map_err(|_| InvalidKey)?;
        self.take(len)
    }
}

#[cfg(test)]
#[path = "key_tests.rs"]
mod tests;
