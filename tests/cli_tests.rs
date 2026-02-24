use assert_cmd::Command;
use predicates::prelude::*;

fn cmd() -> Command {
    Command::cargo_bin("context-surgeon").unwrap()
}

#[test]
fn test_help_flag() {
    cmd()
        .arg("--help")
        .assert()
        .success()
        .stdout(predicate::str::contains("context-surgeon"));
}

#[test]
fn test_version_flag() {
    cmd()
        .arg("--version")
        .assert()
        .success()
        .stdout(predicate::str::contains("context-surgeon"));
}

#[test]
fn test_no_args_error() {
    // Should error without budget or model
    cmd()
        .write_stdin("some input")
        .assert()
        .failure()
        .stderr(predicate::str::contains("budget"));
}

#[test]
fn test_basic_passthrough() {
    // Input smaller than budget should pass through unchanged
    let input = "This is a short test.";
    cmd()
        .arg("--budget")
        .arg("1000")
        .write_stdin(input)
        .assert()
        .success()
        .stdout(predicate::str::contains("short test"));
}

#[test]
fn test_compression_with_stats() {
    // Large input, small budget - should compress and show stats
    let input = "word ".repeat(500);
    cmd()
        .arg("--budget")
        .arg("100")
        .arg("--stats")
        .write_stdin(input)
        .assert()
        .success()
        .stderr(predicate::str::contains("Compression Statistics"));
}

#[test]
fn test_percentage_budget() {
    let input = "word ".repeat(100);
    cmd()
        .arg("--budget")
        .arg("50%")
        .arg("--stats")
        .write_stdin(input)
        .assert()
        .success()
        .stderr(predicate::str::contains("tokens"));
}

#[test]
fn test_model_budget() {
    let input = "short text";
    cmd()
        .arg("--model")
        .arg("gpt-4o")
        .arg("--reserve")
        .arg("4000")
        .write_stdin(input)
        .assert()
        .success();
}

#[test]
fn test_unknown_model_error() {
    cmd()
        .arg("--model")
        .arg("nonexistent-model-xyz")
        .write_stdin("test")
        .assert()
        .failure()
        .stderr(predicate::str::contains("unknown model"));
}

#[test]
fn test_empty_input_error() {
    cmd()
        .arg("--budget")
        .arg("1000")
        .write_stdin("")
        .assert()
        .failure()
        .code(1)
        .stderr(predicate::str::contains("empty"));
}

#[test]
fn test_file_not_found_error() {
    cmd()
        .arg("--budget")
        .arg("1000")
        .arg("/nonexistent/file/path.txt")
        .assert()
        .failure()
        .code(1)
        .stderr(predicate::str::contains("not found"));
}

#[test]
fn test_tokenizer_openai() {
    let input = "test input text";
    cmd()
        .arg("--budget")
        .arg("1000")
        .arg("--tokenizer")
        .arg("openai")
        .write_stdin(input)
        .assert()
        .success();
}

#[test]
fn test_tokenizer_anthropic() {
    let input = "test input text";
    cmd()
        .arg("--budget")
        .arg("1000")
        .arg("--tokenizer")
        .arg("anthropic")
        .write_stdin(input)
        .assert()
        .success();
}

#[test]
fn test_tokenizer_approximate() {
    let input = "test input text";
    cmd()
        .arg("--budget")
        .arg("1000")
        .arg("--tokenizer")
        .arg("approximate")
        .write_stdin(input)
        .assert()
        .success();
}

#[test]
fn test_conflicting_modes_error() {
    cmd()
        .arg("--budget")
        .arg("1000")
        .arg("--aggressive")
        .arg("--conservative")
        .write_stdin("test")
        .assert()
        .failure()
        .code(2);
}

#[test]
fn test_reserve_without_model_error() {
    cmd()
        .arg("--budget")
        .arg("1000")
        .arg("--reserve")
        .arg("500")
        .write_stdin("test")
        .assert()
        .failure()
        .code(2);
}

#[test]
fn test_strategy_disable_flags() {
    let input = "word ".repeat(200);
    cmd()
        .arg("--budget")
        .arg("50")
        .arg("--no-redundancy")
        .arg("--no-boilerplate")
        .write_stdin(input)
        .assert()
        .success();
}

#[test]
fn test_preserve_flags() {
    let input = "First paragraph here with content.\n\nSecond paragraph in the middle.\n\nThird paragraph at the end.";
    cmd()
        .arg("--budget")
        .arg("20")
        .arg("--preserve-head")
        .arg("10")
        .arg("--preserve-tail")
        .arg("10")
        .write_stdin(input)
        .assert()
        .success();
}

#[test]
fn test_output_contains_compressed_text() {
    // Generate text with multiple paragraphs
    let para1 = "First unique paragraph with distinctive content here.";
    let para2 = "Second different paragraph has other information inside.";
    let para3 = "Third separate paragraph contains more unique text.";
    let input = format!("{}\n\n{}\n\n{}", para1, para2, para3);
    
    let output = cmd()
        .arg("--budget")
        .arg("30")
        .write_stdin(input)
        .assert()
        .success()
        .get_output()
        .stdout
        .clone();
    
    let output_str = String::from_utf8_lossy(&output);
    // Should contain at least some text (not empty)
    assert!(!output_str.trim().is_empty());
}

#[test]
fn test_stats_shows_segments() {
    let input = "First paragraph text.\n\nSecond paragraph text.";
    cmd()
        .arg("--budget")
        .arg("1000")
        .arg("--stats")
        .write_stdin(input)
        .assert()
        .success()
        .stderr(predicate::str::contains("Segments:"));
}
