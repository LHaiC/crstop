use std::process::Command;

fn fixture_path() -> &'static str {
    concat!(
        env!("CARGO_MANIFEST_DIR"),
        "/tests/fixtures/sample_snapshot.json"
    )
}

#[test]
fn fixture_output_defaults_to_one_second_refresh() {
    let output = Command::new(env!("CARGO_BIN_EXE_crstop"))
        .args(["--fixture", fixture_path()])
        .output()
        .unwrap();

    assert!(output.status.success());
    let stdout = String::from_utf8(output.stdout).unwrap();
    assert!(stdout.contains("Refresh"));
    assert!(stdout.contains("1s"));
}

#[test]
fn fixture_output_accepts_fractional_refresh_interval() {
    let output = Command::new(env!("CARGO_BIN_EXE_crstop"))
        .args(["--fixture", fixture_path(), "--refresh", "0.5"])
        .output()
        .unwrap();

    assert!(output.status.success());
    let stdout = String::from_utf8(output.stdout).unwrap();
    assert!(stdout.contains("0.5s"));
}
