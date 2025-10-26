use sha2::Digest;
use sha2::Sha256;
use std::fs::File;
use std::io::{BufReader, Read};
use std::path::Path;

pub fn file_digest_sha256(path: impl AsRef<Path>) -> std::io::Result<String> {
    let file = File::open(path.as_ref())?;
    let mut reader = BufReader::new(file);
    let mut hasher = Sha256::new();
    let mut buffer = [0; 8192];

    loop {
        let bytes_read = reader.read(&mut buffer)?;
        if bytes_read == 0 {
            break;
        }
        hasher.update(&buffer[..bytes_read]);
    }

    Ok(format!("{:x}", hasher.finalize()))
}
