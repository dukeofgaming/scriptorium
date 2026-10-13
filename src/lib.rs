//! Scriptorium copies a Markdown manuscript to and from remote exemplars.
//!
//! The library is the domain. The binary in `main` is a thin command line.

/// Process command-line arguments and return an exit code.
pub fn run(args: impl IntoIterator<Item = impl Into<std::ffi::OsString> + Clone>) -> i32 {
    cli::execute(args)
}

pub mod document;
pub mod links;
pub mod pull;
pub mod remote;
pub mod sync_state;

mod cli;
