//! Tests for the `SlackNotifyHook` implementation.
#![cfg(test)]

use capsula_core::hook::{Hook, PreRun};
use capsula_notify_slack::SlackNotifyHook;
use serde_json::json;
use std::path::PathBuf;

#[test]
fn slack_hook_parses_config() {
    // Arrange
    let config = json!({
        "channel": "#test-channel",
        "token": "xoxb-test-token"
    });

    // Act - specify PreRun phase for type inference
    let hook = <SlackNotifyHook as Hook<PreRun>>::from_config(&config, &PathBuf::from("."))
        .expect("from_config should succeed");

    // Assert
    let hook_config = <SlackNotifyHook as Hook<PreRun>>::config(&hook);
    assert_eq!(
        serde_json::to_value(hook_config)
            .unwrap()
            .get("channel")
            .and_then(|v| v.as_str()),
        Some("#test-channel")
    );
    // The token is a secret and must be redacted when the config is serialized
    // (e.g. into the `__meta.config` block of pre-run.json / post-run.json and
    // into the server's `run_outputs.config` column).
    assert_eq!(
        serde_json::to_value(hook_config)
            .unwrap()
            .get("token")
            .and_then(|v| v.as_str()),
        Some("***")
    );
}

#[test]
fn slack_hook_requires_channel() {
    // Arrange
    let config = json!({
        "token": "xoxb-test-token"
    });

    // Act
    let result = <SlackNotifyHook as Hook<PreRun>>::from_config(&config, &PathBuf::from("."));

    // Assert
    assert!(result.is_err(), "Should fail without channel");
}

#[test]
fn slack_hook_requires_token() {
    // Arrange
    let config = json!({
        "channel": "#test-channel"
    });

    // Act
    let result = <SlackNotifyHook as Hook<PreRun>>::from_config(&config, &PathBuf::from("."));

    // Assert
    assert!(result.is_err(), "Should fail without token");
}

#[test]
fn slack_hook_has_correct_id() {
    assert_eq!(
        <SlackNotifyHook as Hook<PreRun>>::ID,
        "notify-slack",
        "Hook ID should be 'notify-slack'"
    );
}

#[test]
fn slack_hook_parses_config_with_attachments() {
    // Arrange
    let config = json!({
        "channel": "#test-channel",
        "token": "xoxb-test-token",
        "attachment_globs": ["*.png", "outputs/*.jpg"]
    });

    // Act
    let hook = <SlackNotifyHook as Hook<PreRun>>::from_config(&config, &PathBuf::from("."))
        .expect("from_config should succeed");

    // Assert
    let hook_config = <SlackNotifyHook as Hook<PreRun>>::config(&hook);
    let config_value = serde_json::to_value(hook_config).unwrap();
    let attachment_globs = config_value
        .get("attachment_globs")
        .and_then(|v| v.as_array())
        .map(|arr| arr.iter().filter_map(|v| v.as_str()).collect::<Vec<_>>());

    assert_eq!(attachment_globs, Some(vec!["*.png", "outputs/*.jpg"]));
}

#[test]
fn slack_hook_parses_config_without_attachments() {
    // Arrange - config without attachment_globs should default to empty vec
    let config = json!({
        "channel": "#test-channel",
        "token": "xoxb-test-token"
    });

    // Act
    let hook = <SlackNotifyHook as Hook<PreRun>>::from_config(&config, &PathBuf::from("."))
        .expect("from_config should succeed");

    // Assert
    let hook_config = <SlackNotifyHook as Hook<PreRun>>::config(&hook);
    let config_value = serde_json::to_value(hook_config).unwrap();
    let attachment_globs = config_value
        .get("attachment_globs")
        .and_then(|v| v.as_array())
        .map(Vec::len);

    assert_eq!(
        attachment_globs,
        Some(0),
        "attachment_globs should default to empty vec"
    );
}
