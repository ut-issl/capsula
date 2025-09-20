use capsula_config::CapsulaConfig;
use std::path::Path;

#[test]
fn test_parse_actual_config_file() {
    let config_path = Path::new("../../capsula.toml");
    if config_path.exists() {
        let config = CapsulaConfig::from_file(config_path).expect("Failed to parse capsula.toml");

        assert_eq!(config.vault.name, "capsula");

        assert_eq!(config.phase.pre.contexts.len(), 5);
        assert_eq!(config.phase.post.contexts.len(), 0);
        assert_eq!(config.phase.in_phase.watchers.len(), 0);

        println!("Successfully parsed capsula.toml:");
        println!("{:#?}", config);
    } else {
        eprintln!("capsula.toml not found at expected location");
    }
}
