use std::path::{Component, Path, PathBuf};

use crate::account::ProviderId;

#[derive(Debug, PartialEq, Eq)]
pub struct ProviderDescriptor {
    pub id: ProviderId,
    pub display_name: &'static str,
    /// Ways to add an account; the first one is the default.
    pub add_account: &'static [AddAccountMethod],
    pub multi_account: bool,
    pub local_usage: bool,
}

#[derive(Debug, PartialEq, Eq)]
pub enum AddAccountMethod {
    CliLogin(CliLogin),
    ApiKey(ApiKeyPrompt),
    AutoDetect { reason: &'static str },
}

#[derive(Debug, PartialEq, Eq)]
pub struct CliLogin {
    pub program: &'static str,
    pub args: &'static [&'static str],
    pub home_var: HomeVar,
    /// Written by a successful login, relative to the directory `home_var` names.
    pub credentials_file: &'static str,
    pub needs_pty: bool,
    /// Inherited variables removed before the login runs, because they would redirect it.
    pub scrub_env: &'static [&'static str],
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum HomeVar {
    Direct(&'static str),
    XdgBase {
        var: &'static str,
        subdir: &'static str,
    },
}

#[derive(Debug, PartialEq, Eq)]
pub struct ApiKeyPrompt {
    pub label: &'static str,
    pub console_url: &'static str,
    pub hint: &'static str,
}

#[derive(Debug, Clone, PartialEq, Eq, thiserror::Error)]
#[error("provider {provider}: {problem}")]
pub struct DescriptorError {
    pub provider: String,
    pub problem: &'static str,
}

impl ProviderDescriptor {
    #[must_use]
    pub fn default_method(&self) -> Option<&'static AddAccountMethod> {
        self.add_account.first()
    }

    #[must_use]
    pub fn accepts_api_key(&self) -> bool {
        self.add_account
            .iter()
            .any(|method| matches!(method, AddAccountMethod::ApiKey(_)))
    }

    pub fn validate(&self) -> Result<(), DescriptorError> {
        let fail = |problem| DescriptorError {
            provider: self.id.to_string(),
            problem,
        };
        if !self.id.is_valid() {
            return Err(fail("id must be lowercase letters, digits, '-' or '_'"));
        }
        if self.display_name.trim().is_empty() {
            return Err(fail("display name is empty"));
        }
        if self.add_account.is_empty() {
            return Err(fail("no way to add an account"));
        }
        self.add_account
            .iter()
            .try_for_each(AddAccountMethod::validate)
            .map_err(fail)
    }
}

impl AddAccountMethod {
    fn validate(&self) -> Result<(), &'static str> {
        match self {
            AddAccountMethod::CliLogin(login) => login.validate(),
            AddAccountMethod::ApiKey(prompt) => prompt.validate(),
            AddAccountMethod::AutoDetect { reason } if reason.trim().is_empty() => {
                Err("auto-detect reason is empty")
            }
            AddAccountMethod::AutoDetect { .. } => Ok(()),
        }
    }
}

impl CliLogin {
    #[must_use]
    pub fn credentials_path(&self, home: &Path) -> PathBuf {
        self.home_var.config_dir(home).join(self.credentials_file)
    }

    fn validate(&self) -> Result<(), &'static str> {
        if self.program.is_empty() || self.program.contains('/') {
            return Err("login program must be a bare command name");
        }
        if !is_relative_inside(self.credentials_file) {
            return Err("credentials file must be a relative path inside the home");
        }
        let scrubs_home = self.scrub_env.contains(&self.home_var.var());
        if scrubs_home || !self.scrub_env.iter().all(|name| is_env_name(name)) {
            return Err(
                "scrubbed variables must be environment variable names other than the home",
            );
        }
        self.home_var.validate()
    }
}

impl HomeVar {
    #[must_use]
    pub fn var(self) -> &'static str {
        match self {
            HomeVar::Direct(var) | HomeVar::XdgBase { var, .. } => var,
        }
    }

    /// The tool's own config directory when `var` points at `home`.
    #[must_use]
    pub fn config_dir(self, home: &Path) -> PathBuf {
        match self {
            HomeVar::Direct(_) => home.to_path_buf(),
            HomeVar::XdgBase { subdir, .. } => home.join(subdir),
        }
    }

    fn validate(self) -> Result<(), &'static str> {
        if !is_env_name(self.var()) {
            return Err("home variable must be an upper-case environment variable name");
        }
        match self {
            HomeVar::XdgBase { subdir, .. } if !is_relative_inside(subdir) => {
                Err("XDG subdirectory must be a relative path inside the base")
            }
            _ => Ok(()),
        }
    }
}

impl ApiKeyPrompt {
    fn validate(&self) -> Result<(), &'static str> {
        if self.label.trim().is_empty() {
            return Err("API key label is empty");
        }
        if !self.console_url.starts_with("https://") {
            return Err("API key console URL must use https");
        }
        Ok(())
    }
}

fn is_env_name(name: &str) -> bool {
    let mut bytes = name.bytes();
    bytes
        .next()
        .is_some_and(|first| first.is_ascii_uppercase() || first == b'_')
        && bytes.all(|b| b.is_ascii_uppercase() || b.is_ascii_digit() || b == b'_')
}

fn is_relative_inside(path: &str) -> bool {
    !path.is_empty()
        && Path::new(path)
            .components()
            .all(|part| matches!(part, Component::Normal(_)))
}

#[cfg(test)]
#[path = "descriptor_tests.rs"]
mod tests;
