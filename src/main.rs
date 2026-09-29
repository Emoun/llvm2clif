//! `llvm2clif`: translate an LLVM IR (`.ll`) file into Cranelift IR and
//! compile it for the Scry ISA.

use anyhow::{bail, Context};
use clap::{Parser, ValueEnum};
use std::path::PathBuf;

#[derive(Clone, Copy, Debug, PartialEq, Eq, ValueEnum)]
enum Emit {
    /// A Scry ELF relocatable object file.
    Obj,
    /// Textual Cranelift IR.
    Clif,
    /// The Scry machine code produced by the backend, disassembled.
    Asm,
}

/// Translate LLVM IR into Cranelift IR for the Scry ISA.
#[derive(Parser, Debug)]
#[command(version, about, long_about = None)]
struct Cli {
    /// The LLVM IR text file (`.ll`) to translate. A `.clif` file is
    /// compiled directly with the Scry backend instead.
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

    /// Emit native signed comparisons and arithmetic shifts instead of
    /// rewriting them into unsigned operations (see README, "Known issues").
    #[arg(long)]
    native_signed_ops: bool,
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
    let translated = if is_clif {
        llvm2clif::emit::module_from_clif(&src)
            .map_err(|e| anyhow::anyhow!("{}: {e}", cli.input.display()))?
    } else {
        let module = llvm2clif::llvm::parse_module(&src)
            .map_err(|e| anyhow::anyhow!("{}: {e}", cli.input.display()))?;
        let options = llvm2clif::translate::Options {
            skip_unsupported: cli.skip_unsupported,
            verify: !cli.no_verify,
            signed_via_unsigned: !cli.native_signed_ops,
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
