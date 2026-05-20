// This integration test crate is only compiled for test targets.
#![cfg(test)]

use capsula_config::CapsulaConfig;
use std::path::Path;

#[test]
fn test_parse_actual_config_file() {
    let config_path = Path::new("../../capsula.toml");
    if config_path.exists() {
        let config = CapsulaConfig::from_file(config_path).expect("Failed to parse capsula.toml");

        assert_eq!(config.vault.name, "capsula");

        println!("Successfully parsed capsula.toml:");
        println!("{config:#?}");
    } else {
        eprintln!("capsula.toml not found at expected location");
    }
}
