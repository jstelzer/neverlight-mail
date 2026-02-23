use serde::{Deserialize, Serialize};

#[derive(Debug, Clone, Default, Serialize, Deserialize, PartialEq)]
pub struct Config {
    pub imap_server: String,
    pub imap_port: u16,
    pub username: String,
    pub password: String,
    pub use_starttls: bool,
}

impl Config {
    /// Load configuration from environment variables.
    ///
    /// Required: NEVERMAIL_SERVER, NEVERMAIL_USER, NEVERMAIL_PASSWORD
    /// Optional: NEVERMAIL_PORT (default 993), NEVERMAIL_STARTTLS (default false)
    pub fn from_env() -> Self {
        let imap_server = std::env::var("NEVERMAIL_SERVER")
            .expect("NEVERMAIL_SERVER must be set (e.g. mail.runbox.com)");
        let username = std::env::var("NEVERMAIL_USER")
            .expect("NEVERMAIL_USER must be set (e.g. you@runbox.com)");
        let password = std::env::var("NEVERMAIL_PASSWORD")
            .expect("NEVERMAIL_PASSWORD must be set");
        let imap_port = std::env::var("NEVERMAIL_PORT")
            .ok()
            .and_then(|p| p.parse().ok())
            .unwrap_or(993);
        let use_starttls = std::env::var("NEVERMAIL_STARTTLS")
            .map(|v| v == "true" || v == "1")
            .unwrap_or(false);

        Config {
            imap_server,
            imap_port,
            username,
            password,
            use_starttls,
        }
    }
}
