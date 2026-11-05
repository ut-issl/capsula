//! Small pure-helper utilities shared across Capsula crates.

/// Encode bytes as a lowercase hexadecimal string.
#[must_use]
pub fn hex_encode(bytes: &[u8]) -> String {
    bytes
        .iter()
        .fold(String::with_capacity(bytes.len() * 2), |mut output, b| {
            use std::fmt::Write;
            let _ = write!(output, "{b:02x}");
            output
        })
}

#[cfg(test)]
mod tests {
    use super::hex_encode;

    #[test]
    fn hex_encode_empty() {
        assert_eq!(hex_encode(&[]), "");
    }

    #[test]
    fn hex_encode_single_bytes() {
        assert_eq!(hex_encode(&[0x00]), "00");
        assert_eq!(hex_encode(&[0xff]), "ff");
        assert_eq!(hex_encode(&[0xab, 0xcd, 0xef]), "abcdef");
    }
}
