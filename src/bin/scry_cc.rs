//! `scry-cc`: a C compiler driver for the Scry ISA.
//!
//! Pipeline: `clang` (C -> LLVM IR) -> `opt` (optimization) -> llvm2clif
//! (LLVM IR -> Cranelift IR -> Scry object) -> `wild` (linking).

use llvm2clif::driver::{self, Config, Mode};
use std::ffi::OsString;
use std::path::PathBuf;

const USAGE: &str = "\
scry-cc: compile C for the Scry ISA (clang -> opt -> llvm2clif -> wild)

Usage: scry-cc [OPTIONS] <INPUT>... [-- <SIMULATOR ARGS>...]

Inputs may be C sources (.c), LLVM IR text (.ll) or Scry objects (.o).

Options:
  -o <FILE>            Output file (default: <first input>.elf, .o, .clif or .ll)
  -c                   Compile to object files, do not link
  --emit-clif          Stop after translation and write textual Cranelift IR
  --emit-llvm          Stop after optimization and write LLVM IR
  -O<LEVEL>            Optimization level (0, 1, 2, 3, s, z; default 2)
  --target <TRIPLE>    Clang target triple (default riscv32-unknown-none-elf)
  --entry <SYMBOL>     Entry point symbol (default main)
  --no-runtime         Do not link the bundled runtime library (memcpy, malloc, ...)
  --heap-size <BYTES>  Size of the runtime's heap (default 8192)
  --no-gc-sections     Keep unreferenced sections when linking
  --skip-unsupported   Skip functions using unsupported constructs (warning)
  --native-signed-ops  Do not rewrite signed compares/shifts into unsigned ones
  -Wl,<ARG>            Pass <ARG> to the linker
  --run                Run the linked program on the simulator; arguments
                       after `--` are passed to scryer (e.g. -i 5i32)
  --clang <PATH>       clang executable (or env SCRY_CLANG)
  --opt <PATH>         opt executable (or env SCRY_OPT)
  --wild <PATH>        wild linker executable (or env SCRY_WILD)
  --scryer <PATH>      scryer simulator executable (or env SCRY_SCRYER)
  --keep-temps         Keep intermediate files
  -v, --verbose        Print the commands that are run
  -h, --help           Show this help
  --version            Show the version

Any other option starting with `-` (e.g. -I, -D, -std=, -W...) is passed
through to clang.
";

fn main() {
    let cfg = match parse_args(std::env::args_os().skip(1).collect()) {
        Ok(Some(cfg)) => cfg,
        Ok(None) => return,
        Err(e) => {
            eprintln!("scry-cc: error: {e}");
            eprintln!("Run `scry-cc --help` for usage.");
            std::process::exit(2);
        }
    };
    if let Err(e) = driver::run(&cfg) {
        eprintln!("scry-cc: error: {e:#}");
        std::process::exit(1);
    }
}

fn parse_args(args: Vec<OsString>) -> Result<Option<Config>, String> {
    let mut cfg = Config::default();
    let mut it = args.into_iter();
    let mut run_args = None;
    while let Some(arg) = it.next() {
        let s = arg.to_string_lossy().into_owned();
        let mut value = |name: &str| -> Result<OsString, String> {
            it.next()
                .ok_or_else(|| format!("option `{name}` needs a value"))
        };
        match s.as_str() {
            "-h" | "--help" => {
                print!("{USAGE}");
                return Ok(None);
            }
            "--version" => {
                println!("scry-cc {}", env!("CARGO_PKG_VERSION"));
                return Ok(None);
            }
            "-o" => cfg.output = Some(PathBuf::from(value("-o")?)),
            "-c" => cfg.mode = Mode::Object,
            "--emit-clif" => cfg.mode = Mode::Clif,
            "--emit-llvm" => cfg.mode = Mode::Llvm,
            "--target" => cfg.target = value("--target")?.to_string_lossy().into_owned(),
            "--entry" => cfg.entry = value("--entry")?.to_string_lossy().into_owned(),
            "--no-runtime" | "-nostdlib" => cfg.link_runtime = false,
            "--heap-size" => {
                cfg.heap_size = value("--heap-size")?
                    .to_string_lossy()
                    .parse()
                    .map_err(|_| "invalid --heap-size".to_string())?
            }
            "--no-gc-sections" => cfg.gc_sections = false,
            "--skip-unsupported" => cfg.skip_unsupported = true,
            "--native-signed-ops" => cfg.native_signed_ops = true,
            "--run" => run_args = Some(Vec::new()),
            "--clang" => cfg.clang = Some(PathBuf::from(value("--clang")?)),
            "--opt" => cfg.opt = Some(PathBuf::from(value("--opt")?)),
            "--wild" => cfg.wild = Some(PathBuf::from(value("--wild")?)),
            "--scryer" => cfg.scryer = Some(PathBuf::from(value("--scryer")?)),
            "--keep-temps" | "-save-temps" | "--save-temps" => cfg.keep_temps = true,
            "-v" | "--verbose" => cfg.verbose = true,
            "--" => {
                let rest: Vec<OsString> = it.by_ref().collect();
                match &mut run_args {
                    Some(r) => r.extend(rest),
                    None => return Err("arguments after `--` are only used with --run".into()),
                }
            }
            _ => {
                if let Some(level) = s.strip_prefix("-O") {
                    match level {
                        "0" | "1" | "2" | "3" | "s" | "z" => cfg.opt_level = level.to_string(),
                        "" => cfg.opt_level = "2".to_string(),
                        _ => return Err(format!("unknown optimization level `{s}`")),
                    }
                } else if let Some(rest) = s.strip_prefix("-Wl,") {
                    for part in rest.split(',') {
                        cfg.linker_args.push(OsString::from(part));
                    }
                } else if let Some(p) = s.strip_prefix("--target=") {
                    cfg.target = p.to_string();
                } else if let Some(p) = s.strip_prefix("--entry=") {
                    cfg.entry = p.to_string();
                } else if s.starts_with('-') && s.len() > 1 {
                    // Pass-through to clang.
                    cfg.clang_args.push(arg);
                } else {
                    cfg.inputs.push(PathBuf::from(arg));
                }
            }
        }
    }
    cfg.run = run_args;
    if cfg.inputs.is_empty() {
        return Err("no input files".into());
    }
    Ok(Some(cfg))
}
