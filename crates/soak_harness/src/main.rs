//! Headless soak test CLI for Lattice simulation.
//!
//! Run deterministic simulation for overnight testing without GPU/windowing.
//!
//! # Usage
//!
//! ```text
//! soak-harness [OPTIONS]
//! soak-harness --preset overnight
//! soak-harness --config soak.json
//! soak-harness --seed 42 --ticks 10000 --regions medium
//! soak-harness --determinism --ticks 1000
//! ```

use std::env;
use std::fs;
use std::process::ExitCode;

use soak_harness::{OutputFormat, RegionSetup, SoakConfig, SoakRunner};

fn main() -> ExitCode {
    let args: Vec<String> = env::args().collect();

    match parse_args(&args) {
        Ok(command) => match command {
            Command::Run(config, format) => run_soak(config, format),
            Command::Determinism(config, format) => run_determinism(config, format),
            Command::Help => {
                print_help();
                ExitCode::from(0)
            }
            Command::Version => {
                println!("soak-harness 0.1.0");
                ExitCode::from(0)
            }
        },
        Err(e) => {
            eprintln!("Error: {e}");
            eprintln!();
            print_help();
            ExitCode::from(1)
        }
    }
}

enum Command {
    Run(SoakConfig, OutputFormat),
    Determinism(SoakConfig, OutputFormat),
    Help,
    Version,
}

#[allow(clippy::too_many_lines)]
fn parse_args(args: &[String]) -> Result<Command, String> {
    let mut config = SoakConfig::default();
    let mut format = OutputFormat::Text;
    let mut determinism = false;
    let mut config_file: Option<String> = None;
    let mut preset: Option<String> = None;

    let mut i = 1;
    while i < args.len() {
        match args[i].as_str() {
            "-h" | "--help" => return Ok(Command::Help),
            "-V" | "--version" => return Ok(Command::Version),
            "-c" | "--config" => {
                i += 1;
                config_file = Some(args.get(i).ok_or("--config requires a file path")?.clone());
            }
            "-p" | "--preset" => {
                i += 1;
                preset = Some(args.get(i).ok_or("--preset requires a name")?.clone());
            }
            "-s" | "--seed" => {
                i += 1;
                config.seed = args
                    .get(i)
                    .ok_or("--seed requires a value")?
                    .parse()
                    .map_err(|_| "invalid seed value")?;
            }
            "-t" | "--ticks" => {
                i += 1;
                config.tick_count = args
                    .get(i)
                    .ok_or("--ticks requires a value")?
                    .parse()
                    .map_err(|_| "invalid tick count")?;
            }
            "-d" | "--duration" => {
                i += 1;
                config.max_duration_secs = args
                    .get(i)
                    .ok_or("--duration requires a value")?
                    .parse()
                    .map_err(|_| "invalid duration value")?;
            }
            "-r" | "--regions" => {
                i += 1;
                let region_str = args.get(i).ok_or("--regions requires a value")?;
                config.regions = match region_str.as_str() {
                    "single" => RegionSetup::single(),
                    "small" => RegionSetup::small(),
                    "medium" => RegionSetup::medium(),
                    "large" => RegionSetup::large(),
                    _ => return Err(format!("unknown region preset: {region_str}")),
                };
            }
            "-i" | "--checkpoint-interval" => {
                i += 1;
                config.checkpoint_interval = args
                    .get(i)
                    .ok_or("--checkpoint-interval requires a value")?
                    .parse()
                    .map_err(|_| "invalid checkpoint interval")?;
            }
            "-f" | "--format" => {
                i += 1;
                let fmt_str = args.get(i).ok_or("--format requires a value")?;
                format = OutputFormat::from_str(fmt_str)
                    .ok_or_else(|| format!("unknown format: {fmt_str} (use text or json)"))?;
            }
            "--determinism" => {
                determinism = true;
            }
            "--fail-fast" => {
                config.fail_fast = true;
            }
            "--no-invariants" => {
                config.check_invariants = false;
            }
            "-v" | "--verbose" => {
                config.verbose = true;
            }
            arg => {
                return Err(format!("unknown argument: {arg}"));
            }
        }
        i += 1;
    }

    if let Some(file) = config_file {
        let contents = fs::read_to_string(&file)
            .map_err(|e| format!("failed to read config file '{file}': {e}"))?;
        config = SoakConfig::from_json(&contents)
            .map_err(|e| format!("failed to parse config file: {e}"))?;
    } else if let Some(preset_name) = preset {
        config = match preset_name.as_str() {
            "smoke" => SoakConfig::smoke(),
            "short" => SoakConfig::short(),
            "medium" => SoakConfig::medium(),
            "overnight" => SoakConfig::overnight(),
            _ => return Err(format!("unknown preset: {preset_name}")),
        };
    }

    config
        .validate()
        .map_err(|e| format!("invalid config: {e}"))?;

    if determinism {
        Ok(Command::Determinism(config, format))
    } else {
        Ok(Command::Run(config, format))
    }
}

fn run_soak(config: SoakConfig, format: OutputFormat) -> ExitCode {
    let mut runner = SoakRunner::new(config);
    runner.set_output_format(format);
    let report = runner.run();

    let output = report.format(format);
    println!("{output}");

    if report.passed {
        ExitCode::from(0)
    } else {
        ExitCode::from(1)
    }
}

fn run_determinism(config: SoakConfig, format: OutputFormat) -> ExitCode {
    let mut runner = SoakRunner::new(config);
    runner.set_output_format(format);
    let (report, violation) = runner.run_determinism_check();

    let output = report.format(format);
    println!("{output}");

    if let Some(v) = violation {
        eprintln!("Determinism violation: {v}");
        return ExitCode::from(1);
    }

    if report.passed {
        ExitCode::from(0)
    } else {
        ExitCode::from(1)
    }
}

fn print_help() {
    println!(
        r"soak-harness - Headless simulation harness for overnight soak tests

USAGE:
    soak-harness [OPTIONS]

OPTIONS:
    -h, --help                  Show this help message
    -V, --version               Show version
    -c, --config <FILE>         Load config from JSON file
    -p, --preset <NAME>         Use preset config (smoke, short, medium, overnight)
    -s, --seed <N>              Set random seed (default: 42)
    -t, --ticks <N>             Number of ticks to simulate (default: 1000)
    -d, --duration <SECS>       Max wall-clock duration in seconds (0 = unlimited)
    -r, --regions <PRESET>      Region setup (single, small, medium, large)
    -i, --checkpoint-interval <N>  Ticks between checkpoints (0 = none)
    -f, --format <FMT>          Output format (text, json)
    -v, --verbose               Print checkpoint reports during run
    --determinism               Run determinism check (execute twice, compare checksums)
    --fail-fast                 Stop on first critical invariant violation
    --no-invariants             Disable invariant checking

EXAMPLES:
    soak-harness --preset smoke
    soak-harness --seed 42 --ticks 10000 --regions medium
    soak-harness --preset overnight --format json > report.json
    soak-harness --determinism --ticks 1000
    soak-harness --config my-soak.json --verbose"
    );
}
