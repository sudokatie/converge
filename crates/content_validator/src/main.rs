//! Content validation CLI for Lattice game data files.

use std::env;
use std::path::PathBuf;
use std::process::ExitCode;

use content_validator::ContentValidator;

fn main() -> ExitCode {
    let args: Vec<String> = env::args().collect();

    let content_root = if args.len() > 1 {
        PathBuf::from(&args[1])
    } else {
        PathBuf::from("assets/data")
    };

    if !content_root.exists() {
        eprintln!(
            "Error: Content root '{}' does not exist",
            content_root.display()
        );
        return ExitCode::from(1);
    }

    if !content_root.is_dir() {
        eprintln!(
            "Error: Content root '{}' is not a directory",
            content_root.display()
        );
        return ExitCode::from(1);
    }

    let validator = ContentValidator::new(&content_root);
    let report = validator.validate();

    println!("{report}");

    if report.has_errors() {
        ExitCode::from(1)
    } else {
        ExitCode::from(0)
    }
}
