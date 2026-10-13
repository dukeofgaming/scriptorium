//! End-to-end features. Each vertical slice has one file in `tests/features`.

use std::process::Command;

use cucumber::{gherkin::Step, given, then, when, World};
use tempfile::TempDir;

#[derive(Debug, World)]
#[world(init = Self::new)]
pub struct ScriptoriumWorld {
    dir: TempDir,
    status: i32,
    stdout: String,
    stderr: String,
}

impl ScriptoriumWorld {
    fn new() -> Self {
        Self {
            dir: TempDir::new().expect("temp dir"),
            status: 0,
            stdout: String::new(),
            stderr: String::new(),
        }
    }
}

#[given(expr = "a manuscript {string}:")]
fn write_manuscript(world: &mut ScriptoriumWorld, name: String, step: &Step) {
    // The opening line of a docstring is the newline after `"""`.
    let text = step
        .docstring
        .clone()
        .expect("manuscript text")
        .trim_start_matches(['\r', '\n'])
        .to_string();
    let path = world.dir.path().join(&name);
    if let Some(parent) = path.parent() {
        std::fs::create_dir_all(parent).expect("parent directory");
    }
    let mut file = text;
    if !file.ends_with('\n') {
        file.push('\n');
    }
    std::fs::write(path, file).expect("write manuscript");
}

#[when(expr = "I run {string}")]
fn run_command(world: &mut ScriptoriumWorld, line: String) {
    let mut parts = line.split_whitespace();
    let program = parts.next().expect("a command");
    assert_eq!(
        program, "scriptorium",
        "features run the scriptorium binary"
    );
    let args: Vec<&str> = parts.collect();
    let output = Command::new(env!("CARGO_BIN_EXE_scriptorium"))
        .args(&args)
        .current_dir(world.dir.path())
        .output()
        .expect("run scriptorium");
    world.status = output.status.code().unwrap_or(1);
    world.stdout = String::from_utf8_lossy(&output.stdout).into_owned();
    world.stderr = String::from_utf8_lossy(&output.stderr).into_owned();
}

#[then(expr = "the exit code is {int}")]
fn exit_code(world: &mut ScriptoriumWorld, expected: i32) {
    assert_eq!(
        world.status, expected,
        "stdout:\n{}\nstderr:\n{}",
        world.stdout, world.stderr
    );
}

#[then(expr = "stdout contains {string}")]
fn stdout_contains(world: &mut ScriptoriumWorld, expected: String) {
    assert!(
        world.stdout.contains(&expected),
        "stdout did not contain {expected:?}:\n{}",
        world.stdout
    );
}

#[then(expr = "stderr contains {string}")]
fn stderr_contains(world: &mut ScriptoriumWorld, expected: String) {
    assert!(
        world.stderr.contains(&expected),
        "stderr did not contain {expected:?}:\n{}",
        world.stderr
    );
}

#[tokio::main]
async fn main() {
    ScriptoriumWorld::cucumber()
        .max_concurrent_scenarios(1)
        .run_and_exit("tests/features")
        .await;
}
