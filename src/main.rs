//! Command-line entry for the Scriptorium copyist.

fn main() {
    let code = scriptorium::run(std::env::args_os());
    std::process::exit(code);
}
