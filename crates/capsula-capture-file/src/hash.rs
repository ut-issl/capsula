use capsula_core::util::hex_encode;
use sha2::Digest;
use sha2::Sha256;
use std::io::{Read, Write};

/// Copy everything from `reader` to `writer`, returning the hex-encoded
/// SHA-256 digest of the bytes written.
///
/// The bytes are read once, so the digest always describes exactly what was
/// written, even if the source changes while it is being read.
pub fn copy_with_digest_sha256(
    reader: &mut impl Read,
    writer: &mut impl Write,
) -> std::io::Result<String> {
    let mut hasher = Sha256::new();
    let mut buffer = [0; 8192];

    loop {
        let bytes_read = match reader.read(&mut buffer) {
            Ok(0) => break,
            Ok(n) => n,
            Err(error) if error.kind() == std::io::ErrorKind::Interrupted => continue,
            Err(error) => return Err(error),
        };
        writer.write_all(&buffer[..bytes_read])?;
        hasher.update(&buffer[..bytes_read]);
    }
    writer.flush()?;

    Ok(hex_encode(hasher.finalize().as_ref()))
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn copy_with_digest_writes_and_hashes_the_same_bytes() {
        let input = vec![7_u8; 20_000];
        let mut output = Vec::new();

        let digest = copy_with_digest_sha256(&mut input.as_slice(), &mut output).unwrap();

        assert_eq!(output, input);
        assert_eq!(digest, hex_encode(Sha256::digest(&input).as_ref()));
    }

    #[test]
    fn copy_with_digest_of_empty_input() {
        let mut output = Vec::new();

        let digest = copy_with_digest_sha256(&mut [].as_slice(), &mut output).unwrap();

        assert_eq!(output, [] as [u8; 0]);
        assert_eq!(
            digest,
            "e3b0c44298fc1c149afbf4c8996fb92427ae41e4649b934ca495991b7852b855"
        );
    }
}
