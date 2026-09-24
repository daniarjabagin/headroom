use std::cmp::Ordering;
use std::fmt;
use std::str::FromStr;

#[derive(Debug, Clone, PartialEq, Eq, thiserror::Error)]
#[error("{0:?} is not a semantic version")]
pub struct VersionError(pub String);

#[derive(Debug, Clone, PartialEq, Eq, Hash)]
pub struct Version {
    pub major: u64,
    pub minor: u64,
    pub patch: u64,
    pub pre: Vec<Identifier>,
}

#[derive(Debug, Clone, PartialEq, Eq, Hash)]
pub enum Identifier {
    Numeric(u64),
    Text(String),
}

impl Version {
    #[must_use]
    pub fn is_prerelease(&self) -> bool {
        !self.pre.is_empty()
    }
}

impl FromStr for Version {
    type Err = VersionError;

    fn from_str(text: &str) -> Result<Version, VersionError> {
        let invalid = || VersionError(text.to_owned());
        let bare = text.strip_prefix('v').unwrap_or(text);
        let without_build = bare.split_once('+').map_or(bare, |(core, _)| core);
        let (core, pre) = match without_build.split_once('-') {
            Some((core, pre)) => (core, Some(pre)),
            None => (without_build, None),
        };
        let mut numbers = core.split('.').map(number);
        let (Some(Some(major)), Some(Some(minor)), Some(Some(patch)), None) = (
            numbers.next(),
            numbers.next(),
            numbers.next(),
            numbers.next(),
        ) else {
            return Err(invalid());
        };
        let pre = pre
            .map_or(Some(Vec::new()), identifiers)
            .ok_or_else(invalid)?;
        Ok(Version {
            major,
            minor,
            patch,
            pre,
        })
    }
}

fn number(text: &str) -> Option<u64> {
    let canonical = !text.is_empty()
        && text.bytes().all(|b| b.is_ascii_digit())
        && (text == "0" || !text.starts_with('0'));
    canonical.then(|| text.parse().ok()).flatten()
}

fn identifiers(text: &str) -> Option<Vec<Identifier>> {
    text.split('.').map(identifier).collect()
}

fn identifier(text: &str) -> Option<Identifier> {
    let allowed = |b: u8| b.is_ascii_alphanumeric() || b == b'-';
    if text.is_empty() || !text.bytes().all(allowed) {
        return None;
    }
    if text.bytes().all(|b| b.is_ascii_digit()) {
        return number(text).map(Identifier::Numeric);
    }
    Some(Identifier::Text(text.to_owned()))
}

impl Ord for Version {
    fn cmp(&self, other: &Version) -> Ordering {
        (self.major, self.minor, self.patch)
            .cmp(&(other.major, other.minor, other.patch))
            .then_with(|| compare_pre(&self.pre, &other.pre))
    }
}

impl PartialOrd for Version {
    fn partial_cmp(&self, other: &Version) -> Option<Ordering> {
        Some(self.cmp(other))
    }
}

fn compare_pre(left: &[Identifier], right: &[Identifier]) -> Ordering {
    match (left.is_empty(), right.is_empty()) {
        (true, true) => Ordering::Equal,
        (true, false) => Ordering::Greater,
        (false, true) => Ordering::Less,
        (false, false) => left.cmp(right),
    }
}

impl Ord for Identifier {
    fn cmp(&self, other: &Identifier) -> Ordering {
        match (self, other) {
            (Identifier::Numeric(a), Identifier::Numeric(b)) => a.cmp(b),
            (Identifier::Numeric(_), Identifier::Text(_)) => Ordering::Less,
            (Identifier::Text(_), Identifier::Numeric(_)) => Ordering::Greater,
            (Identifier::Text(a), Identifier::Text(b)) => a.cmp(b),
        }
    }
}

impl PartialOrd for Identifier {
    fn partial_cmp(&self, other: &Identifier) -> Option<Ordering> {
        Some(self.cmp(other))
    }
}

impl fmt::Display for Version {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(f, "{}.{}.{}", self.major, self.minor, self.patch)?;
        for (index, part) in self.pre.iter().enumerate() {
            f.write_str(if index == 0 { "-" } else { "." })?;
            match part {
                Identifier::Numeric(value) => write!(f, "{value}")?,
                Identifier::Text(text) => f.write_str(text)?,
            }
        }
        Ok(())
    }
}

#[cfg(test)]
#[path = "version_tests.rs"]
mod tests;
