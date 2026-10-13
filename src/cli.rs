//! Command line for the three copying verbs, plus link inspection.

use std::ffi::OsString;
use std::path::PathBuf;

use clap::{Parser, Subcommand};

use crate::links;

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
    Pull,
    /// Copy the manuscript onto the remote exemplar.
    Push,
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
            Command::Pull => unfinished("pull"),
            Command::Push => unfinished("push"),
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
