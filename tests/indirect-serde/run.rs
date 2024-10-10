use std::process::Command;

fn main() {
    let res = Command::new("cargo")
        .args(["check"])
        .current_dir("tests/indirect-serde")
        .status();

    assert!(res.is_ok_and(|exitcode| exitcode.success()));
}
