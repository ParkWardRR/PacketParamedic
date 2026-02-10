//! Smoke tests -- verify the binary runs and key modules load.

use assert_cmd::Command;

#[test]
fn test_cli_help() {
    Command::cargo_bin("packetparamedic")
        .unwrap()
        .arg("--help")
        .assert()
        .success()
        .stdout(predicates::str::contains("Appliance-grade network diagnostics"));
}

#[test]
fn test_cli_version() {
    Command::cargo_bin("packetparamedic")
        .unwrap()
        .arg("--version")
        .assert()
        .success()
        .stdout(predicates::str::contains("packetparamedic"));
}

#[test]
fn test_self_test_subcommand_exists() {
    Command::cargo_bin("packetparamedic")
        .unwrap()
        .arg("self-test")
        .arg("--help")
        .assert()
        .success();
}

#[test]
fn test_schedule_list_subcommand_exists() {
    Command::cargo_bin("packetparamedic")
        .unwrap()
        .args(["schedule", "list", "--help"])
        .assert()
        .success();
}

#[test]
fn test_speed_test_subcommand_exists() {
    Command::cargo_bin("packetparamedic")
        .unwrap()
        .arg("speed-test")
        .arg("--help")
        .assert()
        .success();
}
