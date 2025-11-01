use capsula_capture_cwd::CwdHook;
use capsula_core::captured::Captured;
use capsula_core::hook::{Hook, HookPhase, RuntimeParams};
use capsula_core::run::PreparedRun;
use ulid::Ulid;

#[test]
fn cwd_hook_captures_current_dir_and_json() {
    // Arrange
    let expected = std::env::current_dir().expect("current_dir");
    let hook = CwdHook;
    let run_metadata = PreparedRun {
        id: Ulid::new(),
        name: "test-run".to_string(),
        command: vec![],
        run_dir: expected.clone(),
        project_root: expected.clone(),
    };
    let params = RuntimeParams {
        phase: HookPhase::Pre,
        project_root: expected.clone(),
    };

    // Act
    let captured = hook.run(&run_metadata, &params).expect("CwdHook::run ok");
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
