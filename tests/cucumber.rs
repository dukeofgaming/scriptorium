//! End-to-end features. Each vertical slice has one file in `tests/features`.

use std::process::Command;

use cucumber::{gherkin::Step, given, then, when, World};
use scriptorium::remote::FakeRemote;
use scriptorium::sync_state::{self, CopyRecord, Ledger};
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

    fn remote_path(&self) -> std::path::PathBuf {
        self.dir.path().join("remote-copies.json")
    }
}

fn docstring(step: &Step) -> String {
    step.docstring
        .clone()
        .expect("docstring")
        .trim_start_matches(['\r', '\n'])
        .to_string()
}

#[given(expr = "the remote copy {string} at revision {string}:")]
fn seed_remote(world: &mut ScriptoriumWorld, url: String, revision: String, step: &Step) {
    let fake = FakeRemote::open(world.remote_path());
    fake.insert(&url, &revision, &docstring(step))
        .expect("seed remote");
}

#[given(expr = "a manuscript {string}:")]
fn write_manuscript(world: &mut ScriptoriumWorld, name: String, step: &Step) {
    let text = docstring(step);
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

#[given(expr = "the ledger for {string} records {string} at revision {string} for body:")]
fn seed_ledger(
    world: &mut ScriptoriumWorld,
    name: String,
    url: String,
    revision: String,
    step: &Step,
) {
    let path = sync_state::path_for(&world.dir.path().join(&name));
    let mut ledger = Ledger::load(&path).expect("load ledger");
    ledger.copies.insert(
        url,
        CopyRecord {
            revision,
            content_sha256: sync_state::content_sha256(&docstring(step)),
        },
    );
    ledger.save(&path).expect("save ledger");
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
        .env("SCRIPTORIUM_FAKE_REMOTE", world.remote_path())
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

#[then(expr = "the file {string} is:")]
fn file_is(world: &mut ScriptoriumWorld, name: String, step: &Step) {
    let actual = std::fs::read_to_string(world.dir.path().join(&name)).expect("read file");
    let expected = with_trailing_newline(&docstring(step));
    assert_eq!(actual, expected, "file {name} did not match");
}

#[then(
    expr = "the ledger for {string} records {string} at revision {string} for the manuscript body"
)]
fn ledger_records(world: &mut ScriptoriumWorld, name: String, url: String, revision: String) {
    let manuscript_path = world.dir.path().join(&name);
    let ledger = Ledger::load(&sync_state::path_for(&manuscript_path)).expect("load ledger");
    let record = ledger
        .copies
        .get(&url)
        .unwrap_or_else(|| panic!("no ledger entry for {url}"));
    assert_eq!(record.revision, revision);
    let text = std::fs::read_to_string(&manuscript_path).expect("read manuscript");
    let manuscript = scriptorium::document::parse(&text).expect("parse manuscript");
    assert_eq!(
        record.content_sha256,
        sync_state::content_sha256(&manuscript.body)
    );
}

#[then(expr = "the remote copy {string} is at revision {string}:")]
fn remote_is(world: &mut ScriptoriumWorld, url: String, revision: String, step: &Step) {
    let fake = FakeRemote::open(world.remote_path());
    let copy = fake
        .get(&url)
        .expect("read fake remote")
        .unwrap_or_else(|| panic!("no remote copy for {url}"));
    assert_eq!(copy.revision, revision);
    assert_eq!(copy.body, sync_state::normalize_body(&docstring(step)));
}

fn with_trailing_newline(text: &str) -> String {
    if text.ends_with('\n') {
        text.to_string()
    } else {
        format!("{text}\n")
    }
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
