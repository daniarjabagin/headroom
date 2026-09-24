use std::cmp::Ordering;

use super::*;

fn v(text: &str) -> Version {
    text.parse().unwrap()
}

#[test]
fn versions_compare_by_semver_precedence() {
    let cases = [
        ("0.4.0", "0.5.0", Ordering::Less),
        ("0.4.0", "0.4.0", Ordering::Equal),
        ("v0.4.0", "0.4.0", Ordering::Equal),
        ("0.4.1", "0.4.0", Ordering::Greater),
        ("0.10.0", "0.9.9", Ordering::Greater),
        ("1.0.0", "0.99.99", Ordering::Greater),
        ("0.5.0-rc.1", "0.5.0", Ordering::Less),
        ("0.5.0-alpha", "0.5.0-alpha.1", Ordering::Less),
        ("0.5.0-alpha.1", "0.5.0-alpha.beta", Ordering::Less),
        ("0.5.0-alpha.beta", "0.5.0-beta", Ordering::Less),
        ("0.5.0-beta.2", "0.5.0-beta.11", Ordering::Less),
        ("0.5.0-beta.11", "0.5.0-rc.1", Ordering::Less),
        ("0.5.0+build.7", "0.5.0", Ordering::Equal),
        ("0.4.9", "0.5.0-rc.1", Ordering::Less),
    ];
    for (left, right, expected) in cases {
        assert_eq!(v(left).cmp(&v(right)), expected, "{left} vs {right}");
    }
}

#[test]
fn malformed_versions_are_rejected() {
    for text in [
        "",
        "1",
        "1.2",
        "1.2.3.4",
        "01.2.3",
        "1.02.3",
        "a.b.c",
        "1.2.3-",
        "1.2.3-a..b",
        "1.2.3-01",
        "1.2.3-a_b",
        "v",
        "1.2.-3",
    ] {
        assert_eq!(
            text.parse::<Version>(),
            Err(VersionError(text.to_owned())),
            "{text}"
        );
    }
}

#[test]
fn prereleases_are_recognised_and_displayed() {
    assert!(v("0.5.0-rc.1").is_prerelease());
    assert!(!v("v0.5.0").is_prerelease());
    assert_eq!(v("v0.5.0").to_string(), "0.5.0");
    assert_eq!(v("0.5.0-rc.1+abc").to_string(), "0.5.0-rc.1");
}
