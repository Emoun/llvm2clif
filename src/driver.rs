//! The `scry-cc` compiler driver: runs clang, opt, the translator and the
//! wild linker to turn C sources into Scry executables.

use crate::translate::Options;
use anyhow::{anyhow, bail, Context};
use std::ffi::OsString;
use std::path::{Path, PathBuf};
use std::process::Command;

/// Source of the runtime library and its headers, embedded so that the
/// driver is self-contained.
pub const RUNTIME_SOURCE: &str = include_str!("../runtime/scryrt.c");
pub const RUNTIME_STRING_H: &str = include_str!("../runtime/include/string.h");
pub const RUNTIME_STDLIB_H: &str = include_str!("../runtime/include/stdlib.h");

/// The default clang target: a 32-bit little-endian target whose data layout
/// matches Scry (ILP32, no native 64-bit integers).
pub const DEFAULT_TARGET: &str = "riscv32-unknown-none-elf";

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum Mode {
    /// Link an executable (the default).
    Link,
    /// Stop after producing object files (`-c`).
    Object,
    /// Stop after producing textual Cranelift IR (`--emit-clif`).
    Clif,
    /// Stop after producing optimized LLVM IR (`--emit-llvm`).
    Llvm,
}

#[derive(Clone, Debug)]
pub struct Config {
    pub inputs: Vec<PathBuf>,
    pub output: Option<PathBuf>,
    pub mode: Mode,
    /// Optimization level: "0", "1", "2", "3", "s" or "z".
    pub opt_level: String,
    pub target: String,
    pub entry: String,
    pub clang_args: Vec<OsString>,
    pub linker_args: Vec<OsString>,
    pub link_runtime: bool,
    pub gc_sections: bool,
    pub keep_temps: bool,
    pub verbose: bool,
    pub skip_unsupported: bool,
    pub clang: Option<PathBuf>,
    pub opt: Option<PathBuf>,
    pub wild: Option<PathBuf>,
    pub scryer: Option<PathBuf>,
    /// Run the linked program on the simulator with these arguments.
    pub run: Option<Vec<OsString>>,
    /// Heap size for the runtime's allocator, in bytes.
    pub heap_size: u32,
}

impl Default for Config {
    fn default() -> Self {
        Config {
            inputs: Vec::new(),
            output: None,
            mode: Mode::Link,
            opt_level: "2".into(),
            target: DEFAULT_TARGET.into(),
            entry: "main".into(),
            clang_args: Vec::new(),
            linker_args: Vec::new(),
            link_runtime: true,
            gc_sections: true,
            keep_temps: false,
            verbose: false,
            skip_unsupported: false,
            clang: None,
            opt: None,
            wild: None,
            scryer: None,
            run: None,
            heap_size: 8192,
        }
    }
}

/// Locates an external tool: an explicit path, the environment variable,
/// or the bare name (resolved through `PATH` by the OS).
pub fn tool_path(explicit: &Option<PathBuf>, env_var: &str, default: &str) -> PathBuf {
    if let Some(p) = explicit {
        return p.clone();
    }
    if let Some(p) = std::env::var_os(env_var) {
        if !p.is_empty() {
            return PathBuf::from(p);
        }
    }
    PathBuf::from(default)
}

fn run_tool(cmd: &mut Command, what: &str, verbose: bool) -> anyhow::Result<()> {
    if verbose {
        eprintln!("scry-cc: {}", format_command(cmd));
    }
    let status = cmd.status().map_err(|e| {
        if e.kind() == std::io::ErrorKind::NotFound {
            anyhow!(
                "could not run `{}` ({what}): {e}. Install it, put it on PATH, or point the corresponding option/environment variable at it (see `scry-cc --help`).",
                cmd.get_program().to_string_lossy()
            )
        } else {
            anyhow!("could not run `{}` ({what}): {e}", cmd.get_program().to_string_lossy())
        }
    })?;
    if !status.success() {
        bail!("{what} failed ({status}); command: {}", format_command(cmd));
    }
    Ok(())
}

fn format_command(cmd: &Command) -> String {
    let mut s = cmd.get_program().to_string_lossy().into_owned();
    for a in cmd.get_args() {
        s.push(' ');
        let a = a.to_string_lossy();
        if a.contains(' ') {
            s.push('"');
            s.push_str(&a);
            s.push('"');
        } else {
            s.push_str(&a);
        }
    }
    s
}

/// A temporary directory that is removed on drop unless kept.
struct TempDir {
    path: PathBuf,
    keep: bool,
}

impl TempDir {
    fn new(keep: bool) -> anyhow::Result<TempDir> {
        let nanos = std::time::SystemTime::now()
            .duration_since(std::time::UNIX_EPOCH)
            .map(|d| d.as_nanos())
            .unwrap_or(0);
        let path = std::env::temp_dir().join(format!("scry-cc-{}-{}", std::process::id(), nanos));
        std::fs::create_dir_all(&path)
            .with_context(|| format!("creating temporary directory {}", path.display()))?;
        Ok(TempDir { path, keep })
    }
}

impl Drop for TempDir {
    fn drop(&mut self) {
        if !self.keep {
            let _ = std::fs::remove_dir_all(&self.path);
        }
    }
}

fn stem(p: &Path) -> String {
    p.file_stem()
        .map(|s| s.to_string_lossy().into_owned())
        .unwrap_or_else(|| "out".into())
}

/// Runs the whole pipeline described by `cfg`.
pub fn run(cfg: &Config) -> anyhow::Result<()> {
    if cfg.inputs.is_empty() {
        bail!("no input files");
    }
    if cfg.mode != Mode::Link && cfg.output.is_some() && cfg.inputs.len() > 1 {
        bail!("cannot use -o with multiple inputs unless linking");
    }
    let temps = TempDir::new(cfg.keep_temps)?;
    if cfg.keep_temps {
        eprintln!(
            "scry-cc: keeping intermediate files in {}",
            temps.path.display()
        );
    }
    let clang = tool_path(&cfg.clang, "SCRY_CLANG", "clang");
    let opt = tool_path(&cfg.opt, "SCRY_OPT", "opt");
    let wild = tool_path(&cfg.wild, "SCRY_WILD", "wild");

    // Materialize the runtime headers so that `#include <string.h>` works
    // without a system C library.
    let include_dir = temps.path.join("include");
    std::fs::create_dir_all(&include_dir)?;
    std::fs::write(include_dir.join("string.h"), RUNTIME_STRING_H)?;
    std::fs::write(include_dir.join("stdlib.h"), RUNTIME_STDLIB_H)?;

    let mut objects: Vec<PathBuf> = Vec::new();
    let mut extra_link_inputs: Vec<PathBuf> = Vec::new();

    for input in &cfg.inputs {
        let ext = input
            .extension()
            .and_then(|e| e.to_str())
            .unwrap_or("")
            .to_ascii_lowercase();
        match ext.as_str() {
            "c" | "i" => {
                let ll = compile_c(cfg, &clang, &opt, &include_dir, &temps.path, input, &[])?;
                if cfg.mode == Mode::Llvm {
                    finish_single(cfg, &ll, input, "ll")?;
                    continue;
                }
                let obj = translate(cfg, &temps.path, &ll, input)?;
                if let Some(obj) = obj {
                    objects.push(obj);
                }
            }
            "ll" => {
                if cfg.mode == Mode::Llvm {
                    bail!("{} is already LLVM IR", input.display());
                }
                let obj = translate(cfg, &temps.path, input, input)?;
                if let Some(obj) = obj {
                    objects.push(obj);
                }
            }
            "o" | "a" | "obj" => {
                if cfg.mode != Mode::Link {
                    bail!("{} is already compiled", input.display());
                }
                extra_link_inputs.push(input.clone());
            }
            _ => bail!("unknown input file type: {}", input.display()),
        }
    }

    if cfg.mode != Mode::Link {
        return Ok(());
    }

    if cfg.link_runtime {
        let rt_src = temps.path.join("scryrt.c");
        std::fs::write(&rt_src, RUNTIME_SOURCE)?;
        let heap = OsString::from(format!("-DSCRY_HEAP_SIZE={}", cfg.heap_size));
        let ll = compile_c(
            cfg,
            &clang,
            &opt,
            &include_dir,
            &temps.path,
            &rt_src,
            &[OsString::from("-fno-builtin"), heap],
        )?;
        let obj = translate_to(cfg, &ll, &temps.path.join("scryrt.o"))?;
        objects.push(obj);
    }

    let output = cfg
        .output
        .clone()
        .unwrap_or_else(|| PathBuf::from(format!("{}.elf", stem(&cfg.inputs[0]))));
    let mut cmd = Command::new(&wild);
    // wild guesses its command-line flavor from the host (ld64-style on
    // macOS); the Scry ELF options need the GNU ld flavor, and the flag
    // forcing it has to come first.
    cmd.arg("-flavor")
        .arg("gnu")
        .arg("-m")
        .arg("elf32scry")
        .arg("-e")
        .arg(&cfg.entry)
        .arg("-z")
        .arg("noexecstack");
    if cfg.gc_sections {
        cmd.arg("--gc-sections");
    }
    cmd.arg("-o").arg(&output);
    for o in &objects {
        cmd.arg(o);
    }
    for o in &extra_link_inputs {
        cmd.arg(o);
    }
    for a in &cfg.linker_args {
        cmd.arg(a);
    }
    run_tool(&mut cmd, "linking with wild", cfg.verbose)?;

    if let Some(args) = &cfg.run {
        let scryer = tool_path(&cfg.scryer, "SCRY_SCRYER", "scryer");
        let mut cmd = Command::new(&scryer);
        cmd.arg(&output).arg("--target=scry32-unknown-none-elf");
        for a in args {
            cmd.arg(a);
        }
        if cfg.verbose {
            eprintln!("scry-cc: {}", format_command(&cmd));
        }
        let status = cmd
            .status()
            .map_err(|e| anyhow!("could not run `{}` (the simulator): {e}", scryer.display()))?;
        if let Some(code) = status.code() {
            std::process::exit(code);
        }
        bail!("the simulator was terminated by a signal");
    }
    Ok(())
}

/// Compiles a C file to optimized LLVM IR; returns the `.ll` path.
fn compile_c(
    cfg: &Config,
    clang: &Path,
    opt: &Path,
    include_dir: &Path,
    tmp: &Path,
    input: &Path,
    extra: &[OsString],
) -> anyhow::Result<PathBuf> {
    let base = stem(input);
    let pre = tmp.join(format!("{base}.pre.ll"));
    let mut cmd = Command::new(clang);
    cmd.arg(format!("--target={}", cfg.target));
    if cfg.target.starts_with("riscv32") {
        cmd.arg("-march=rv32im").arg("-mabi=ilp32");
    }
    cmd.arg(format!("-O{}", cfg.opt_level));
    cmd.arg("-S").arg("-emit-llvm");
    if cfg.opt_level != "0" {
        // Let `opt` do the optimizing so that the pipeline is explicit.
        cmd.arg("-Xclang").arg("-disable-llvm-passes");
    }
    // Plain `char` is unsigned in the RISC-V ABI; make it signed as on x86
    // so that programs behave as most people expect (`-funsigned-char`
    // can be passed to override).
    cmd.arg("-fsigned-char");
    cmd.arg("-fno-vectorize")
        .arg("-fno-slp-vectorize")
        .arg("-g0");
    cmd.arg("-isystem").arg(include_dir);
    for a in extra {
        cmd.arg(a);
    }
    for a in &cfg.clang_args {
        cmd.arg(a);
    }
    cmd.arg("-o").arg(&pre).arg(input);
    run_tool(
        &mut cmd,
        &format!("compiling {} with clang", input.display()),
        cfg.verbose,
    )?;

    if cfg.opt_level == "0" {
        return Ok(pre);
    }
    let ll = tmp.join(format!("{base}.ll"));
    let mut cmd = Command::new(opt);
    cmd.arg(format!("-O{}", cfg.opt_level))
        .arg("-S")
        .arg("-o")
        .arg(&ll)
        .arg(&pre);
    run_tool(
        &mut cmd,
        &format!("optimizing {} with opt", input.display()),
        cfg.verbose,
    )?;
    Ok(ll)
}

/// Translates an `.ll` file according to the mode; returns the object path
/// when one was produced.
fn translate(
    cfg: &Config,
    tmp: &Path,
    ll: &Path,
    original: &Path,
) -> anyhow::Result<Option<PathBuf>> {
    match cfg.mode {
        Mode::Clif => {
            let text = crate::emit::clif_text(&translate_ll(cfg, ll)?);
            let out = output_for(cfg, original, "clif");
            std::fs::write(&out, text).with_context(|| format!("writing {}", out.display()))?;
            Ok(None)
        }
        Mode::Object => {
            let out = output_for(cfg, original, "o");
            translate_to(cfg, ll, &out)?;
            Ok(None)
        }
        Mode::Link => {
            let out = tmp.join(format!("{}.o", stem(original)));
            Ok(Some(translate_to(cfg, ll, &out)?))
        }
        Mode::Llvm => unreachable!(),
    }
}

fn output_for(cfg: &Config, original: &Path, ext: &str) -> PathBuf {
    match &cfg.output {
        Some(o) => o.clone(),
        None => PathBuf::from(format!("{}.{ext}", stem(original))),
    }
}

fn finish_single(cfg: &Config, produced: &Path, original: &Path, ext: &str) -> anyhow::Result<()> {
    let out = output_for(cfg, original, ext);
    std::fs::copy(produced, &out).with_context(|| format!("writing {}", out.display()))?;
    Ok(())
}

/// Parses and translates an `.ll` file; returns the CLIF text and the
/// object bytes.
/// Parses and translates an `.ll` file.
fn translate_ll(cfg: &Config, ll: &Path) -> anyhow::Result<crate::translate::ClifModule> {
    let src = std::fs::read_to_string(ll).with_context(|| format!("reading {}", ll.display()))?;
    let module = crate::llvm::parse_module(&src).map_err(|e| anyhow!("{}: {e}", ll.display()))?;
    let options = Options {
        skip_unsupported: cfg.skip_unsupported,
        verify: true,
    };
    let translated = crate::translate::translate_module(&module, &options)
        .map_err(|e| anyhow!("{}: {e}", ll.display()))?;
    for (name, reason) in &translated.skipped {
        eprintln!("scry-cc: warning: skipped function @{name}: {reason}");
    }
    Ok(translated)
}

fn translate_to(cfg: &Config, ll: &Path, out: &Path) -> anyhow::Result<PathBuf> {
    if cfg.verbose {
        eprintln!("scry-cc: translating {} -> {}", ll.display(), out.display());
    }
    let translated = translate_ll(cfg, ll)?;
    let bytes = crate::emit::object_file(&translated, &stem(ll))?;
    std::fs::write(out, bytes).with_context(|| format!("writing {}", out.display()))?;
    Ok(out.to_path_buf())
}
