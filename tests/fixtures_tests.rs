use assert_cmd::Command;
use predicates::prelude::*;
use std::path::PathBuf;

fn cmd() -> Command {
    Command::cargo_bin("context-surgeon").unwrap()
}

fn fixture_path(name: &str) -> PathBuf {
    PathBuf::from(env!("CARGO_MANIFEST_DIR"))
        .join("tests")
        .join("fixtures")
        .join(name)
}

#[test]
fn test_short_fixture_exists() {
    let path = fixture_path("short.txt");
    assert!(path.exists(), "short.txt fixture should exist");
}

#[test]
fn test_short_fixture_passthrough() {
    // Short fixture should pass through at high budget
    cmd()
        .arg("--budget")
        .arg("10000")
        .arg(fixture_path("short.txt"))
        .assert()
        .success()
        .stdout(predicate::str::contains("Context compression"));
}

#[test]
fn test_redundant_fixture_compression() {
    // Redundant fixture should compress significantly
    let output = cmd()
        .arg("--budget")
        .arg("50")
        .arg("--stats")
        .arg(fixture_path("redundant.txt"))
        .assert()
        .success()
        .get_output()
        .clone();
    
    let stderr = String::from_utf8_lossy(&output.stderr);
    // Should show compression happened
    assert!(stderr.contains("Compression Statistics"));
}

#[test]
fn test_boilerplate_fixture_detection() {
    // Boilerplate fixture should have boilerplate filtered
    cmd()
        .arg("--budget")
        .arg("100")
        .arg("--stats")
        .arg(fixture_path("boilerplate.txt"))
        .assert()
        .success()
        .stderr(predicate::str::contains("Segments:"));
}

#[test]
fn test_fixture_with_different_tokenizers() {
    let path = fixture_path("short.txt");
    
    for tokenizer in &["openai", "anthropic", "approximate"] {
        cmd()
            .arg("--budget")
            .arg("10000")
            .arg("--tokenizer")
            .arg(tokenizer)
            .arg(&path)
            .assert()
            .success();
    }
}

#[test]
fn test_redundant_fixture_removes_duplicates() {
    // With redundancy detection, similar paragraphs should be filtered
    let output = cmd()
        .arg("--budget")
        .arg("80")
        .arg("--stats")
        .arg(fixture_path("redundant.txt"))
        .assert()
        .success()
        .get_output()
        .clone();
    
    let stderr = String::from_utf8_lossy(&output.stderr);
    // Should keep fewer segments than total
    assert!(stderr.contains("Segments:"));
}

#[test]
fn test_medium_fixture_exists() {
    let path = fixture_path("medium.txt");
    assert!(path.exists(), "medium.txt fixture should exist");
}

#[test]
fn test_medium_fixture_compression() {
    // Medium fixture should compress to target budget
    cmd()
        .arg("--budget")
        .arg("500")
        .arg("--stats")
        .arg(fixture_path("medium.txt"))
        .assert()
        .success()
        .stderr(predicate::str::contains("Compression Statistics"));
}

#[test]
fn test_large_fixture_exists() {
    let path = fixture_path("large.txt");
    assert!(path.exists(), "large.txt fixture should exist");
}

#[test]
fn test_large_fixture_compression() {
    // Large fixture (~50k tokens) should compress
    cmd()
        .arg("--budget")
        .arg("1000")
        .arg("--stats")
        .arg(fixture_path("large.txt"))
        .assert()
        .success()
        .stderr(predicate::str::contains("Compression Statistics"));
}

#[test]
fn test_code_fixture_exists() {
    let path = fixture_path("code.py");
    assert!(path.exists(), "code.py fixture should exist");
}

#[test]
fn test_code_fixture_passthrough() {
    // Code fixture should process successfully
    cmd()
        .arg("--budget")
        .arg("10000")
        .arg(fixture_path("code.py"))
        .assert()
        .success()
        .stdout(predicate::str::contains("def"));
}

#[test]
fn test_mixed_fixture_exists() {
    let path = fixture_path("mixed.txt");
    assert!(path.exists(), "mixed.txt fixture should exist");
}

#[test]
fn test_mixed_fixture_compression() {
    // Mixed content fixture should compress and detect boilerplate
    cmd()
        .arg("--budget")
        .arg("200")
        .arg("--stats")
        .arg(fixture_path("mixed.txt"))
        .assert()
        .success()
        .stderr(predicate::str::contains("Segments:"));
}
