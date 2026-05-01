//! Content validation CLI for Lattice game data files.

use std::env;
use std::fs;
use std::path::Path;
use std::process::ExitCode;

use content_validator::ContentValidator;
use content_validator::manifest::{
    AssetManifest, ManifestGenerator, ManifestVersion, check_compatibility,
};

fn print_usage() {
    eprintln!("Usage: content-validator [COMMAND] [OPTIONS]");
    eprintln!();
    eprintln!("Commands:");
    eprintln!("  validate [PATH]              Validate content files (default)");
    eprintln!("  manifest [PATH] [VERSION]    Generate asset manifest as JSON");
    eprintln!("  diff <OLD> <NEW>             Diff two manifest files");
    eprintln!("  check <OLD> <NEW>            Check compatibility between manifests");
    eprintln!();
    eprintln!("Arguments:");
    eprintln!("  PATH      Content root directory (default: assets/data)");
    eprintln!("  VERSION   Pack version string (default: 0.1.0)");
    eprintln!("  OLD       Path to old manifest JSON file");
    eprintln!("  NEW       Path to new manifest JSON file");
}

fn run_validate(content_root: &Path) -> ExitCode {
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

    let validator = ContentValidator::new(content_root);
    let report = validator.validate();

    println!("{report}");

    if report.has_errors() {
        ExitCode::from(1)
    } else {
        ExitCode::from(0)
    }
}

fn run_manifest(content_root: &Path, pack_version: &str) -> ExitCode {
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

    let version = ManifestVersion::new(pack_version);
    let generator = ManifestGenerator::new(content_root, version);

    match generator.generate() {
        Ok(manifest) => match manifest.to_json() {
            Ok(json) => {
                println!("{json}");
                ExitCode::from(0)
            }
            Err(e) => {
                eprintln!("Error serializing manifest: {e}");
                ExitCode::from(1)
            }
        },
        Err(e) => {
            eprintln!("Error generating manifest: {e}");
            ExitCode::from(1)
        }
    }
}

fn load_manifest(path: &Path) -> Result<AssetManifest, String> {
    let content = fs::read_to_string(path)
        .map_err(|e| format!("Failed to read '{}': {}", path.display(), e))?;
    AssetManifest::from_json(&content)
        .map_err(|e| format!("Failed to parse '{}': {}", path.display(), e))
}

fn run_diff(old_path: &Path, new_path: &Path) -> ExitCode {
    let old_manifest = match load_manifest(old_path) {
        Ok(m) => m,
        Err(e) => {
            eprintln!("Error: {e}");
            return ExitCode::from(1);
        }
    };

    let new_manifest = match load_manifest(new_path) {
        Ok(m) => m,
        Err(e) => {
            eprintln!("Error: {e}");
            return ExitCode::from(1);
        }
    };

    let diff = old_manifest.diff(&new_manifest);
    println!("{diff}");

    ExitCode::from(0)
}

fn run_check(old_path: &Path, new_path: &Path) -> ExitCode {
    let old_manifest = match load_manifest(old_path) {
        Ok(m) => m,
        Err(e) => {
            eprintln!("Error: {e}");
            return ExitCode::from(1);
        }
    };

    let new_manifest = match load_manifest(new_path) {
        Ok(m) => m,
        Err(e) => {
            eprintln!("Error: {e}");
            return ExitCode::from(1);
        }
    };

    let result = check_compatibility(&old_manifest, &new_manifest);

    if result.compatible {
        println!("Manifests are compatible.");
    } else {
        println!("Manifests are NOT compatible.");
    }

    if !result.issues.is_empty() {
        println!();
        println!("Issues:");
        for issue in &result.issues {
            println!("  - {issue}");
        }
    }

    if result.compatible {
        ExitCode::from(0)
    } else {
        ExitCode::from(1)
    }
}

fn main() -> ExitCode {
    let args: Vec<String> = env::args().collect();

    if args.len() < 2 {
        return run_validate(Path::new("assets/data"));
    }

    let command = args[1].as_str();

    if command == "--help" || command == "-h" {
        print_usage();
        return ExitCode::from(0);
    }

    if !command.starts_with('-') && !["validate", "manifest", "diff", "check"].contains(&command) {
        return run_validate(Path::new(&args[1]));
    }

    match command {
        "validate" => {
            let content_root = if args.len() > 2 {
                Path::new(&args[2])
            } else {
                Path::new("assets/data")
            };
            run_validate(content_root)
        }
        "manifest" => {
            let content_root = if args.len() > 2 {
                Path::new(&args[2])
            } else {
                Path::new("assets/data")
            };
            let pack_version = if args.len() > 3 {
                args[3].as_str()
            } else {
                "0.1.0"
            };
            run_manifest(content_root, pack_version)
        }
        "diff" => {
            if args.len() < 4 {
                eprintln!("Error: diff requires two manifest file paths");
                print_usage();
                return ExitCode::from(1);
            }
            run_diff(Path::new(&args[2]), Path::new(&args[3]))
        }
        "check" => {
            if args.len() < 4 {
                eprintln!("Error: check requires two manifest file paths");
                print_usage();
                return ExitCode::from(1);
            }
            run_check(Path::new(&args[2]), Path::new(&args[3]))
        }
        _ => {
            eprintln!("Unknown command: {command}");
            print_usage();
            ExitCode::from(1)
        }
    }
}
