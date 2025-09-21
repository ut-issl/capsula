// tests/cwd_context.rs

use capsula_core::captured::Captured;
use capsula_core::context::{Context, ContextPhase, RuntimeParams};
use capsula_cwd_context::CwdContext;

#[test]
fn cwd_context_captures_current_dir_and_json() {
    // Arrange
    let expected = std::env::current_dir().expect("current_dir");
    let ctx = CwdContext::default();
    let params = RuntimeParams {
        phase: ContextPhase::Pre,
        run_dir: None,
        project_root: expected.clone(),
    };

    // Act
    let captured = ctx.run(&params).expect("CwdContext::run ok");
    let json = captured.to_json();
    let json_cwd = json
        .get("cwd")
        .and_then(|v| v.as_str())
        .expect("json has 'cwd' string");

    // Assert (captured struct)
    assert_eq!(
        captured.cwd_abs, expected,
        "cwd_abs should match current_dir"
    );

    // Assert (JSON view)
    assert_eq!(
        json_cwd,
        expected.to_string_lossy(),
        "JSON 'cwd' should match current_dir string"
    );
}
