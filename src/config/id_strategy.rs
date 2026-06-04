use serde::{Deserialize, Serialize};

/// Work item ID generation strategy
///
/// - `Sequential`: `WI-YYYY-MM-DD-NNN` (default, for solo projects)
/// - `AuthorHash`: `WI-YYYY-MM-DD-{hash4}-NNN` (for multi-person teams)
/// - `Random`: `WI-YYYY-MM-DD-{rand4}` (simple uniqueness)
#[derive(Debug, Clone, Copy, PartialEq, Eq, Default, Serialize, Deserialize)]
#[serde(rename_all = "kebab-case")]
pub enum IdStrategy {
    /// Sequential numbering per day (default)
    #[default]
    Sequential,
    /// Hash of git user.email + sequential numbering per author
    AuthorHash,
    /// Random 4-char hex suffix (no sequence number)
    Random,
}

impl IdStrategy {
    /// Get the author hash (first 4 chars of sha256(git user.email))
    pub fn get_author_hash() -> Option<String> {
        let email = super::git_config_value("user.email")?;

        use sha2::{Digest, Sha256};
        let mut hasher = Sha256::new();
        hasher.update(email.as_bytes());
        let result = hasher.finalize();
        Some(format_hex4(result[0], result[1]))
    }

    /// Generate a random 4-char hex suffix
    pub fn generate_random_suffix() -> String {
        use rand::RngExt;
        let mut rng = rand::rng();
        let bytes: [u8; 2] = rng.random();
        format_hex4(bytes[0], bytes[1])
    }
}

fn format_hex4(first: u8, second: u8) -> String {
    format!("{first:02x}{second:02x}")
}
