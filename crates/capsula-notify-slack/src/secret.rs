use serde::{Deserialize, Serialize, Serializer};

/// A string whose value is redacted in `Debug` and `Serialize` output.
///
/// Used to hold sensitive values (API tokens, credentials) so they do not
/// leak into logs, captured hook outputs on disk, or database rows.
///
/// Deserialization accepts a plain string so existing config files keep
/// working. Serialization always emits the literal `"***"` — this means the
/// hook's `__meta.config` block will carry the placeholder rather than the
/// real token.
#[derive(Clone, Default, Deserialize)]
#[serde(transparent)]
pub struct SecretString(String);

impl SecretString {
    pub const fn new(value: String) -> Self {
        Self(value)
    }

    pub fn expose(&self) -> &str {
        &self.0
    }

    pub const fn is_empty(&self) -> bool {
        self.0.is_empty()
    }
}

impl std::fmt::Debug for SecretString {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.write_str("SecretString(***)")
    }
}

impl Serialize for SecretString {
    fn serialize<S: Serializer>(&self, serializer: S) -> Result<S::Ok, S::Error> {
        serializer.serialize_str("***")
    }
}

#[cfg(test)]
#[expect(clippy::unwrap_used, reason = "tests may use unwrap for brevity")]
mod tests {
    use super::SecretString;

    #[test]
    fn debug_redacts_value() {
        let s = SecretString::new("xoxb-super-secret".to_owned());
        let rendered = format!("{s:?}");
        assert_eq!(rendered, "SecretString(***)");
        assert!(!rendered.contains("xoxb-super-secret"));
    }

    #[test]
    fn serialize_emits_placeholder() {
        let s = SecretString::new("xoxb-super-secret".to_owned());
        let json = serde_json::to_string(&s).unwrap();
        assert_eq!(json, "\"***\"");
    }

    #[test]
    fn deserialize_reads_plain_string() {
        let s: SecretString = serde_json::from_str("\"xoxb-super-secret\"").unwrap();
        assert_eq!(s.expose(), "xoxb-super-secret");
    }

    #[test]
    fn expose_returns_inner_value() {
        let s = SecretString::new("abc".to_owned());
        assert_eq!(s.expose(), "abc");
        assert!(!s.is_empty());
        assert!(SecretString::default().is_empty());
    }
}
