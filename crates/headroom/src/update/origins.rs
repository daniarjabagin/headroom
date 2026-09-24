use std::sync::Arc;

use reqwest::Url;
use reqwest::redirect::{Attempt, Policy};

const HTTPS: &str = "https";
const GITHUB_HOSTS: [&str; 4] = [
    "api.github.com",
    "github.com",
    "objects.githubusercontent.com",
    "release-assets.githubusercontent.com",
];
const MAX_REDIRECTS: usize = 5;

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Origins {
    scheme: String,
    hosts: Arc<[String]>,
}

impl Origins {
    pub fn github() -> Origins {
        Origins {
            scheme: HTTPS.to_owned(),
            hosts: GITHUB_HOSTS.iter().map(|host| (*host).to_owned()).collect(),
        }
    }

    #[cfg(test)]
    pub fn plain_http(host: &str) -> Origins {
        Origins {
            scheme: "http".to_owned(),
            hosts: Arc::from([host.to_owned()]),
        }
    }

    pub fn https_only(&self) -> bool {
        self.scheme == HTTPS
    }

    pub fn allows(&self, url: &Url) -> bool {
        url.scheme() == self.scheme
            && url
                .host_str()
                .is_some_and(|host| self.hosts.iter().any(|allowed| allowed == host))
    }

    pub fn allows_text(&self, url: &str) -> bool {
        Url::parse(url).is_ok_and(|url| self.allows(&url))
    }

    pub fn redirect_policy(&self) -> Policy {
        let origins = self.clone();
        Policy::custom(move |attempt| origins.follow(attempt))
    }

    fn follow(&self, attempt: Attempt<'_>) -> reqwest::redirect::Action {
        if attempt.previous().len() >= MAX_REDIRECTS {
            attempt.error("too many redirects")
        } else if self.allows(attempt.url()) {
            attempt.follow()
        } else {
            let refused = format!("refused a redirect to {}", origin_of(attempt.url()));
            attempt.error(refused)
        }
    }
}

fn origin_of(url: &Url) -> String {
    format!("{}://{}", url.scheme(), url.host_str().unwrap_or_default())
}

#[cfg(test)]
mod tests {
    use super::*;

    fn allowed(url: &str) -> bool {
        Origins::github().allows_text(url)
    }

    #[test]
    fn only_github_release_hosts_over_https_are_allowed() {
        assert!(allowed(
            "https://github.com/daniarjabagin/headroom/releases/download/v0.5.0/SHA256SUMS"
        ));
        assert!(allowed("https://objects.githubusercontent.com/a/b"));
        assert!(allowed("https://release-assets.githubusercontent.com/a/b"));
        assert!(allowed("https://api.github.com/repos/a/b/releases/latest"));
        assert!(!allowed("http://github.com/a"));
        assert!(!allowed("https://github.com.evil.example/a"));
        assert!(!allowed("https://evil.example/github.com"));
        assert!(!allowed("https://user@evil.example/"));
        assert!(!allowed("file:///etc/passwd"));
        assert!(!allowed("not a url"));
    }

    #[test]
    fn plain_http_is_only_for_tests_against_one_host() {
        let local = Origins::plain_http("127.0.0.1");
        assert!(!local.https_only());
        assert!(Origins::github().https_only());
        assert!(local.allows_text("http://127.0.0.1:8080/x"));
        assert!(!local.allows_text("https://127.0.0.1/x"));
        assert!(!local.allows_text("http://localhost/x"));
    }
}
