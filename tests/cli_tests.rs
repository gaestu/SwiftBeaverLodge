use std::process::Command;

#[test]
fn version_flag_prints_package_version_without_starting_gui() {
    let output = Command::new(env!("CARGO_BIN_EXE_swiftbeaverlodge"))
        .arg("--version")
        .output()
        .expect("failed to run swiftbeaverlodge --version");

    assert!(
        output.status.success(),
        "swiftbeaverlodge --version exited with {:?}",
        output.status.code()
    );

    let stdout = String::from_utf8(output.stdout).expect("stdout was not UTF-8");
    assert_eq!(
        stdout.trim(),
        format!("{} {}", env!("CARGO_PKG_NAME"), env!("CARGO_PKG_VERSION"))
    );
}

#[test]
fn version_flag_requires_single_argument_invocation() {
    let output = Command::new(env!("CARGO_BIN_EXE_swiftbeaverlodge"))
        .args(["--unexpected", "--version"])
        .output()
        .expect("failed to run swiftbeaverlodge with mixed arguments");

    assert!(
        !output.status.success(),
        "mixed arguments should be rejected"
    );
    assert!(String::from_utf8(output.stderr)
        .expect("stderr was not UTF-8")
        .contains("unsupported arguments"));
}
