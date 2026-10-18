//! The binary run as a process: the diagram in, the SVG out, and the exit codes. The exit code is 1
//! for a diagram or configuration that does not render, and 2 for a command line that does not parse.

use std::fs;
use std::io::Write;
use std::path::PathBuf;
use std::process::{Command, Output, Stdio};

const PIE: &str = "pie title Pets\n  \"Dogs\" : 386\n  \"Cats\" : 85\n";

fn run(args: &[&str], input: &str) -> Output {
    let mut child = Command::new(env!("CARGO_BIN_EXE_ferrigura"))
        .args(args)
        .stdin(Stdio::piped())
        .stdout(Stdio::piped())
        .stderr(Stdio::piped())
        .spawn()
        .expect("the binary starts");
    child
        .stdin
        .take()
        .expect("standard input is piped")
        .write_all(input.as_bytes())
        .expect("the input is written");
    child.wait_with_output().expect("the binary finishes")
}

/// A file in the directory cargo gives the integration tests, named for the test that writes it.
fn scratch(name: &str, contents: &str) -> PathBuf {
    let path = PathBuf::from(env!("CARGO_TARGET_TMPDIR")).join(name);
    fs::write(&path, contents).expect("the scratch file is written");
    path
}

fn stdout(output: &Output) -> String {
    String::from_utf8(output.stdout.clone()).expect("the output is UTF-8")
}

fn stderr(output: &Output) -> String {
    String::from_utf8(output.stderr.clone()).expect("the messages are UTF-8")
}

#[test]
fn standard_input_renders_to_standard_output() {
    let output = run(&[], PIE);
    assert!(output.status.success(), "{}", stderr(&output));
    let svg = stdout(&output);
    assert!(svg.starts_with("<svg "), "{svg}");
    assert!(svg.ends_with("</svg>\n"), "{svg}");
    assert!(svg.contains("id=\"ferrigura-"), "the default id is derived");
}

#[test]
fn the_id_option_names_the_diagram() {
    let output = run(&["--id", "pie-one"], PIE);
    assert!(output.status.success(), "{}", stderr(&output));
    assert!(stdout(&output).contains("id=\"pie-one\""));
}

#[test]
fn a_file_is_the_input_and_the_output_option_writes_the_svg() {
    let input = scratch("file-in-file-out.mmd", PIE);
    let output_path = PathBuf::from(env!("CARGO_TARGET_TMPDIR")).join("file-in-file-out.svg");
    let output = run(
        &[
            input.to_str().expect("UTF-8 path"),
            "-o",
            output_path.to_str().expect("UTF-8 path"),
        ],
        "",
    );
    assert!(output.status.success(), "{}", stderr(&output));
    assert!(
        stdout(&output).is_empty(),
        "nothing goes to standard output"
    );
    let svg = fs::read_to_string(&output_path).expect("the SVG is written");
    assert!(svg.starts_with("<svg "), "{svg}");
}

#[test]
fn the_configuration_file_applies_its_theme_variables() {
    let config = scratch(
        "pie-colour.json",
        r##"{"themeVariables": {"pie1": "#00ff00"}}"##,
    );
    let output = run(
        &[
            "--config",
            config.to_str().expect("UTF-8 path"),
            "--scheme",
            "light",
        ],
        PIE,
    );
    assert!(output.status.success(), "{}", stderr(&output));
    assert!(stdout(&output).contains("--pie1: #00ff00;"));
}

#[test]
fn the_scheme_option_writes_both_sets_for_auto() {
    let output = run(&["--scheme", "auto"], PIE);
    assert!(output.status.success(), "{}", stderr(&output));
    assert!(stdout(&output).contains("@media (prefers-color-scheme: dark)"));
}

#[test]
fn text_that_is_not_a_diagram_is_an_error_of_the_diagram() {
    let output = run(&[], "not a diagram\n");
    assert_eq!(output.status.code(), Some(1));
    assert!(
        stderr(&output).contains("no diagram type detected"),
        "{}",
        stderr(&output)
    );
}

#[test]
fn a_configuration_that_is_not_an_object_is_refused() {
    let config = scratch("config-array.json", "[1, 2]");
    let output = run(&["--config", config.to_str().expect("UTF-8 path")], PIE);
    assert_eq!(output.status.code(), Some(1));
    assert!(
        stderr(&output).contains("must be a JSON object"),
        "{}",
        stderr(&output)
    );
}

#[test]
fn a_missing_input_file_is_an_error_of_the_diagram() {
    let output = run(&["/nonexistent/ferrigura-missing.mmd"], "");
    assert_eq!(output.status.code(), Some(1));
    assert!(
        stderr(&output).contains("cannot read"),
        "{}",
        stderr(&output)
    );
}

#[test]
fn an_unknown_option_is_a_usage_error() {
    let output = run(&["--bogus"], "");
    assert_eq!(output.status.code(), Some(2));
    assert!(
        stderr(&output).contains("unknown option --bogus"),
        "{}",
        stderr(&output)
    );
    assert!(stderr(&output).contains("--help"));
}

#[test]
fn an_unknown_colour_scheme_is_a_usage_error() {
    let output = run(&["--scheme", "blue"], PIE);
    assert_eq!(output.status.code(), Some(2));
}

#[test]
fn help_and_version_print_and_succeed() {
    let help = run(&["--help"], "");
    assert!(help.status.success());
    assert!(stdout(&help).contains("Usage: ferrigura [OPTIONS] [FILE]"));

    let version = run(&["--version"], "");
    assert!(version.status.success());
    assert!(
        stdout(&version).starts_with("ferrigura "),
        "{}",
        stdout(&version)
    );
}

#[test]
fn several_files_render_through_one_renderer_to_standard_output_in_order() {
    let first = scratch("several-first.mmd", PIE);
    let second = scratch("several-second.mmd", "pie title Sports\n  \"Bandy\" : 40\n");
    let single = |path: &PathBuf| {
        let output = run(&[path.to_str().expect("a UTF-8 path")], "");
        assert!(output.status.success(), "{}", stderr(&output));
        stdout(&output)
    };
    let output = run(
        &[
            first.to_str().expect("a UTF-8 path"),
            second.to_str().expect("a UTF-8 path"),
        ],
        "",
    );
    assert!(output.status.success(), "{}", stderr(&output));
    assert_eq!(
        stdout(&output),
        format!("{}{}", single(&first), single(&second))
    );
}

#[test]
fn output_dir_writes_each_svg_named_after_its_input() {
    let dir = PathBuf::from(env!("CARGO_TARGET_TMPDIR")).join("output-dir");
    let input = scratch("output-dir-pets.mmd", PIE);
    let path = input.to_str().expect("a UTF-8 path");
    let output = run(
        &["--output-dir", dir.to_str().expect("a UTF-8 path"), path],
        "",
    );
    assert!(output.status.success(), "{}", stderr(&output));
    let written = fs::read_to_string(dir.join("output-dir-pets.svg")).expect("the SVG is written");
    let printed = stdout(&run(&[path], ""));
    assert_eq!(written, printed);
}

#[test]
fn output_takes_one_input_and_output_dir_takes_files() {
    let input = scratch("output-two.mmd", PIE);
    let path = input.to_str().expect("a UTF-8 path");
    let output = run(&["-o", "unused.svg", path, path], "");
    assert_eq!(output.status.code(), Some(2));
    assert!(
        stderr(&output).contains("--output takes one input"),
        "{}",
        stderr(&output)
    );
    let output = run(&["--output-dir", "unused", "-"], PIE);
    assert_eq!(output.status.code(), Some(2));
}
