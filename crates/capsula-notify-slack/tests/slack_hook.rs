use capsula_notify_slack::SlackNotifyHook;
use capsula_core::hook::{Hook, PreRun};
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
    assert_eq!(
        serde_json::to_value(hook_config)
            .unwrap()
            .get("token")
            .and_then(|v| v.as_str()),
        Some("xoxb-test-token")
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
