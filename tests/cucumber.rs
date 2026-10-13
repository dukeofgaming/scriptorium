//! End-to-end features. Each vertical slice has one file in `tests/features`.

use std::process::Command;

use cucumber::{then, when, World};

#[derive(Debug, Default, World)]
pub struct ScriptoriumWorld {
    status: i32,
    stdout: String,
    stderr: String,
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

#[tokio::main]
async fn main() {
    ScriptoriumWorld::cucumber()
        .max_concurrent_scenarios(1)
        .run_and_exit("tests/features")
        .await;
}
