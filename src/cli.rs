//! Command line for the three copying verbs, plus link inspection.

use std::ffi::OsString;
use std::path::PathBuf;

use clap::{Parser, Subcommand};

use crate::links;
use crate::pull;
use crate::push::{self, PushReport};
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
    },
    /// Copy the manuscript onto the remote exemplar.
    Push {
        /// Path to the Markdown manuscript.
        file: PathBuf,
    },
    /// Compare the manuscript, the remote, and the last copy that matched.
    Check,
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
            Command::Pull { file } => match remote::from_env() {
                Ok(remote) => match pull::pull(&file, remote.as_ref()) {
                    Ok(report) => {
                        println!("pulled {}", report.url);
                        0
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
            Command::Push { file } => match remote::from_env() {
                Ok(remote) => match push::push(&file, remote.as_ref()) {
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
                        1
                    }
                },
                Err(err) => {
                    eprintln!("{err}");
                    1
                }
            },
            Command::Check => unfinished("check"),
        }
    }
}

fn unfinished(name: &str) -> i32 {
    eprintln!("{name} is not copied yet");
    2
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
