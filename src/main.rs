use std::env;
use std::path::PathBuf;
use std::process;

use typephp::codegen::{EmitMode, PhpVersion};
use typephp::CompileOptions;

fn print_usage() {
    eprintln!(
        "Usage: tphpc <input.tphp> [OPTIONS]\n\n\
         Options:\n  \
           -o, --out <path>          Output file (bundle/single) or directory (preserve)\n  \
           --target <8.1|8.2|8.3|8.4> PHP target version (default: 8.4)\n  \
           --mode <preserve|bundle>  Keep directory structure or bundle into 1 file (default: preserve)\n  \
           --bundle                  Shorthand for --mode bundle\n  \
           --minify                  Minify emitted PHP output\n  \
           --check                   Type-check only without emitting PHP"
    );
}

fn main() {
    let args: Vec<String> = env::args().skip(1).collect();
    if args.is_empty() || args.iter().any(|a| a == "--help" || a == "-h") {
        print_usage();
        process::exit(1);
    }

    let mut input: Option<PathBuf> = None;
    let mut opts = CompileOptions::default();
    let mut check_only = false;

    let mut i = 0;
    while i < args.len() {
        match args[i].as_str() {
            "--check" => check_only = true,
            "--bundle" => opts.mode = EmitMode::Bundle,
            "--minify" => opts.minify = true,
            "-o" | "--out" => {
                i += 1;
                if i >= args.len() {
                    eprintln!("Error: expected path after '-o/--out'");
                    process::exit(1);
                }
                opts.output = Some(PathBuf::from(&args[i]));
            }
            "--target" => {
                i += 1;
                if i >= args.len() {
                    eprintln!("Error: expected PHP version after '--target'");
                    process::exit(1);
                }
                match PhpVersion::from_str(&args[i]) {
                    Ok(v) => opts.target = v,
                    Err(e) => {
                        eprintln!("Error: {}", e);
                        process::exit(1);
                    }
                }
            }
            "--mode" => {
                i += 1;
                if i >= args.len() {
                    eprintln!("Error: expected 'preserve' or 'bundle' after '--mode'");
                    process::exit(1);
                }
                match args[i].as_str() {
                    "preserve" => opts.mode = EmitMode::Preserve,
                    "bundle" => opts.mode = EmitMode::Bundle,
                    other => {
                        eprintln!("Error: unknown mode '{}', expected 'preserve' or 'bundle'", other);
                        process::exit(1);
                    }
                }
            }
            arg if !arg.starts_with('-') => {
                input = Some(PathBuf::from(arg));
            }
            other => {
                eprintln!("Unknown argument: {}", other);
                process::exit(1);
            }
        }
        i += 1;
    }

    let Some(input_path) = input else {
        eprintln!("Error: missing input .tphp file");
        process::exit(1);
    };

    if check_only {
        match typephp::check_file(&input_path) {
            Ok(()) => {
                println!("OK: {}", input_path.display());
            }
            Err(e) => {
                eprintln!("{}: {}", input_path.display(), e);
                process::exit(1);
            }
        }
        return;
    }

    match typephp::compile_with_options(&input_path, &opts) {
        Ok(compiled) => {
            for (src, dst) in compiled {
                println!("Compiled {} -> {}", src.display(), dst.display());
            }
            println!(
                "PHPStan level 9: OK (target PHP {}, mode {:?})",
                opts.target.as_str(),
                opts.mode
            );
        }
        Err(e) => {
            eprintln!("{}: {}", input_path.display(), e);
            process::exit(1);
        }
    }
}
