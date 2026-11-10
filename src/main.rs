//! `llvm2clif`: translate an LLVM IR (`.ll`) file into Cranelift IR and
//! compile it for the Scry ISA (or for riscv64, to compare the ISAs through
//! the same translation).

use anyhow::{bail, Context};
use clap::{Parser, ValueEnum};
use llvm2clif::translate::{OptLevel, Target};
use std::path::PathBuf;

#[derive(Clone, Copy, Debug, PartialEq, Eq, ValueEnum)]
enum Emit {
    /// An ELF relocatable object file.
    Obj,
    /// Textual Cranelift IR.
    Clif,
    /// The machine code produced by the backend, disassembled.
    Asm,
}

#[derive(Clone, Copy, Debug, PartialEq, Eq, ValueEnum)]
enum TargetArg {
    /// Scry (32-bit), through the Scry backend.
    Scry32,
    /// RISC-V RV64IMC, through Cranelift's riscv64 backend; the IR must come
    /// from a 64-bit target (`--target=riscv64-unknown-none-elf`).
    Riscv64,
}

#[derive(Clone, Copy, Debug, PartialEq, Eq, ValueEnum)]
enum OptLevelArg {
    /// No Cranelift mid-end optimization (the IR comes optimized from `opt`).
    None,
    /// Cranelift's own optimizations as well.
    Speed,
}

/// Translate LLVM IR into Cranelift IR and compile it for the Scry ISA.
#[derive(Parser, Debug)]
#[command(version, about, long_about = None)]
struct Cli {
    /// The LLVM IR text file (`.ll`) to translate. A `.clif` file is
    /// compiled directly with the backend instead.
    input: PathBuf,

    /// Output file. Defaults to the input name with a `.o` (or `.clif`)
    /// extension; `-` writes to standard output.
    #[arg(short, long)]
    output: Option<PathBuf>,

    /// What to produce.
    #[arg(long, value_enum, default_value_t = Emit::Obj)]
    emit: Emit,

    /// Skip functions that use unsupported constructs (with a warning)
    /// instead of failing.
    #[arg(long)]
    skip_unsupported: bool,

    /// Do not run the Cranelift IR verifier on the translated functions.
    #[arg(long)]
    no_verify: bool,

    /// The machine to compile for.
    #[arg(long, value_enum, default_value_t = TargetArg::Scry32)]
    target: TargetArg,

    /// Cranelift's optimization level.
    #[arg(long, value_enum, default_value_t = OptLevelArg::None)]
    opt_level: OptLevelArg,
}

fn main() {
    // `RUST_LOG=cranelift_codegen=trace` exposes the backend's logging.
    env_logger::init();
    if let Err(e) = run() {
        eprintln!("llvm2clif: error: {e:#}");
        std::process::exit(1);
    }
}

fn run() -> anyhow::Result<()> {
    let cli = Cli::parse();
    let src = std::fs::read_to_string(&cli.input)
        .with_context(|| format!("reading {}", cli.input.display()))?;
    let is_clif = cli
        .input
        .extension()
        .is_some_and(|e| e.eq_ignore_ascii_case("clif"));
    let target = match cli.target {
        TargetArg::Scry32 => Target::Scry32,
        TargetArg::Riscv64 => Target::Riscv64,
    };
    let opt_level = match cli.opt_level {
        OptLevelArg::None => OptLevel::None,
        OptLevelArg::Speed => OptLevel::Speed,
    };
    let translated = if is_clif {
        let mut m = llvm2clif::emit::module_from_clif(&src)
            .map_err(|e| anyhow::anyhow!("{}: {e}", cli.input.display()))?;
        m.target = target;
        m.opt_level = opt_level;
        m
    } else {
        let module = llvm2clif::llvm::parse_module(&src)
            .map_err(|e| anyhow::anyhow!("{}: {e}", cli.input.display()))?;
        let options = llvm2clif::translate::Options {
            skip_unsupported: cli.skip_unsupported,
            verify: !cli.no_verify,
            target,
            opt_level,
        };
        let translated = llvm2clif::translate::translate_module(&module, &options)
            .map_err(|e| anyhow::anyhow!("{}: {e}", cli.input.display()))?;
        for (name, reason) in &translated.skipped {
            eprintln!("llvm2clif: warning: skipped function @{name}: {reason}");
        }
        translated
    };

    let output = match &cli.output {
        Some(p) => p.clone(),
        None => cli.input.with_extension(match cli.emit {
            Emit::Obj => "o",
            Emit::Clif => "clif",
            Emit::Asm => "s",
        }),
    };
    let bytes = match cli.emit {
        Emit::Clif => llvm2clif::emit::clif_text(&translated).into_bytes(),
        Emit::Asm => llvm2clif::emit::assembly_text(&translated)?.into_bytes(),
        Emit::Obj => {
            let name = cli
                .input
                .file_stem()
                .and_then(|s| s.to_str())
                .unwrap_or("module");
            llvm2clif::emit::object_file(&translated, name)?
        }
    };
    if output.as_os_str() == "-" {
        use std::io::Write;
        std::io::stdout().write_all(&bytes)?;
    } else {
        if output == cli.input {
            bail!("refusing to overwrite the input file");
        }
        std::fs::write(&output, bytes).with_context(|| format!("writing {}", output.display()))?;
    }
    Ok(())
}
