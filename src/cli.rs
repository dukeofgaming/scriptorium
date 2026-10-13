//! Command line for the three copying verbs, plus link inspection.

use std::ffi::OsString;
use std::path::PathBuf;

use clap::{Parser, Subcommand};

use crate::check;
use crate::drift::Drift;
use crate::links;
use crate::pull::{self, PullError, PullReport};
use crate::push::{self, PushError, PushReport};
use crate::remote;

#[derive(Parser)]
#[command(
    name = "scriptorium",
    version,
    about = "Copy a Markdown manuscript to and from Jira, Confluence, and Google Docs"
)]
struct Cli {
    #[command(subcommand)]
    command: Command,
}

#[derive(Subcommand)]
enum Command {
    /// Copy the remote exemplar onto the manuscript.
    Pull {
        /// Path to the Markdown manuscript.
        file: PathBuf,
        /// Overwrite a manuscript that has diverged.
        #[arg(long)]
        force: bool,
    },
    /// Copy the manuscript onto the remote exemplar.
    Push {
        /// Path to the Markdown manuscript.
        file: PathBuf,
        /// Overwrite a remote that has diverged.
        #[arg(long)]
        force: bool,
    },
    /// Compare the manuscript, the remote, and the last copy that matched.
    Check {
        /// Path to the Markdown manuscript.
        file: PathBuf,
    },
    /// List the classified `sync` links named by a manuscript.
    Links {
        /// Path to the Markdown manuscript.
        file: PathBuf,
    },
}

impl Cli {
    fn execute(self) -> i32 {
        match self.command {
            Command::Links { file } => match links::list(&file) {
                Ok(text) => {
                    print!("{text}");
                    0
                }
                Err(err) => {
                    eprintln!("{err}");
                    1
                }
            },
            Command::Pull { file, force } => match remote::from_env() {
                Ok(remote) => match pull::pull(&file, remote.as_ref(), force) {
                    Ok(PullReport::Pulled { url }) => {
                        println!("pulled {url}");
                        0
                    }
                    Ok(PullReport::InSync { url }) => {
                        println!("already in sync {url}");
                        0
                    }
                    Err(err) => {
                        eprintln!("{err}");
                        pull_code(&err)
                    }
                },
                Err(err) => {
                    eprintln!("{err}");
                    1
                }
            },
            Command::Push { file, force } => match remote::from_env() {
                Ok(remote) => match push::push(&file, remote.as_ref(), force) {
                    Ok(PushReport::Pushed { url }) => {
                        println!("pushed {url}");
                        0
                    }
                    Ok(PushReport::InSync { url }) => {
                        println!("already in sync {url}");
                        0
                    }
                    Err(err) => {
                        eprintln!("{err}");
                        push_code(&err)
                    }
                },
                Err(err) => {
                    eprintln!("{err}");
                    1
                }
            },
            Command::Check { file } => match remote::from_env() {
                Ok(remote) => match check::check(&file, remote.as_ref()) {
                    Ok(findings) => {
                        let mut diverged = false;
                        for finding in findings {
                            if finding.drift == Drift::Diverged {
                                diverged = true;
                            }
                            println!("{} {}", finding.drift.label(), finding.url);
                        }
                        if diverged {
                            3
                        } else {
                            0
                        }
                    }
                    Err(err) => {
                        eprintln!("{err}");
                        1
                    }
                },
                Err(err) => {
                    eprintln!("{err}");
                    1
                }
            },
        }
    }
}

fn pull_code(err: &PullError) -> i32 {
    match err {
        PullError::Conflict { .. } => 3,
        _ => 1,
    }
}

fn push_code(err: &PushError) -> i32 {
    match err {
        PushError::Conflict { .. } => 3,
        _ => 1,
    }
}

/// Parse `args` and run the selected command.
///
/// `--help` and `--version` print to the terminal and return `0`. A usage
/// error returns `2`.
pub fn execute(args: impl IntoIterator<Item = impl Into<OsString> + Clone>) -> i32 {
    match Cli::try_parse_from(args) {
        Ok(cli) => cli.execute(),
        Err(err) => {
            let code = err.exit_code();
            let _ = err.print();
            code
        }
    }
}
