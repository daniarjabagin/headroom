use std::time::Duration;

use headroom_core::provider::ProviderError;
use reqwest::Client;

const CONNECT_TIMEOUT: Duration = Duration::from_secs(8);
const POOL_IDLE_TIMEOUT: Duration = Duration::from_secs(30);
const POOL_IDLE_PER_HOST: usize = 1;

/// Builds the one HTTP client a process shares: TLS roots and the connection pool are loaded once.
pub fn client() -> Result<Client, ProviderError> {
    Client::builder()
        .user_agent(concat!("headroom/", env!("CARGO_PKG_VERSION")))
        .connect_timeout(CONNECT_TIMEOUT)
        .pool_idle_timeout(POOL_IDLE_TIMEOUT)
        .pool_max_idle_per_host(POOL_IDLE_PER_HOST)
        .build()
        .map_err(|error| ProviderError::Network(error.without_url().to_string()))
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn the_shared_client_builds() {
        assert!(client().is_ok());
    }
}
