use assert_cmd::Command;

#[test]
fn version_matches_cargo() {
    Command::cargo_bin("hn-scored")
        .expect("binary exists")
        .arg("--version")
        .assert()
        .success()
        .stdout("hn-scored 0.1.0\n");
}
