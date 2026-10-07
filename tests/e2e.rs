//! End-to-end tests: compile the C programs in `tests/programs/` with
//! `scry-cc` (clang -> opt -> llvm2clif -> wild) and run them on the scryer
//! simulator, checking the results against the `// CASES:` expectations
//! recorded in each file (computed natively by `update_expected.py`).
//!
//! The external tools are located through `SCRY_CLANG`, `SCRY_OPT`,
//! `SCRY_WILD` and `SCRY_SCRYER`, or on `PATH`. When one of them is missing
//! the tests are skipped with a notice, unless `LLVM2CLIF_REQUIRE_TOOLS` is
//! set, in which case they fail.
//!
//! Programs listed in [`KNOWN_BACKEND_FAILURES`] fail because of issues in
//! the Scry backend or simulator (see `docs/backend-issues/README.md`); their
//! failures are reported but do not fail the test, and a program that starts
//! passing is reported so that the list can be pruned. The backend is not
//! deterministic (issue 6 there), so a listed program may also pass in some
//! runs.

use std::path::{Path, PathBuf};
use std::process::Command;

/// Programs that currently fail end to end because of known issues in the
/// Scry backend/simulator (all of them pass in the Cranelift interpreter, see
/// `tests/interp.rs`). Each entry names the issue in
/// `docs/backend-issues/README.md`.
const KNOWN_BACKEND_FAILURES: &[(&str, &str)] = &[];

struct Tools {
    scryer: PathBuf,
}

fn tool(env: &str, default: &str) -> PathBuf {
    match std::env::var_os(env) {
        Some(p) if !p.is_empty() => PathBuf::from(p),
        _ => PathBuf::from(default),
    }
}

fn runs(path: &Path, arg: &str) -> bool {
    Command::new(path)
        .arg(arg)
        .output()
        .map(|o| o.status.success())
        .unwrap_or(false)
}

/// Locates the toolchain, or returns `None` (after printing why) when it is
/// unavailable and not required.
fn tools() -> Option<Tools> {
    let clang = tool("SCRY_CLANG", "clang");
    let opt = tool("SCRY_OPT", "opt");
    let wild = tool("SCRY_WILD", "wild");
    let scryer = tool("SCRY_SCRYER", "scryer");
    let missing: Vec<String> = [
        (&clang, "--version"),
        (&opt, "--version"),
        (&wild, "--version"),
        (&scryer, "--help"),
    ]
    .iter()
    .filter(|(p, a)| !runs(p, a))
    .map(|(p, _)| p.display().to_string())
    .collect();
    if missing.is_empty() {
        return Some(Tools { scryer });
    }
    let msg = format!(
        "end-to-end tests need clang, opt, wild and scryer; not found: {}",
        missing.join(", ")
    );
    if std::env::var_os("LLVM2CLIF_REQUIRE_TOOLS").is_some() {
        panic!("{msg}");
    }
    eprintln!("SKIPPED: {msg}");
    None
}

struct Case {
    args: Vec<i64>,
    expected: i64,
}

/// Parses the `// CASES: a b c d => r` lines and the optional `// EXIT: n`
/// line of a test program.
fn parse_spec(src: &str) -> (Vec<Case>, Option<i32>) {
    let mut cases = Vec::new();
    let mut exit = None;
    for line in src.lines() {
        if let Some(rest) = line.strip_prefix("// CASES:") {
            let (args, expected) = rest
                .split_once("=>")
                .unwrap_or_else(|| panic!("case without expectation: {line}"));
            let args = args
                .split_whitespace()
                .map(|a| a.parse::<i64>().expect("case argument"))
                .collect();
            let expected = expected.trim().parse::<i64>().expect("case expectation");
            cases.push(Case { args, expected });
        } else if let Some(rest) = line.strip_prefix("// EXIT:") {
            exit = Some(rest.trim().parse::<i32>().expect("exit code"));
        }
    }
    (cases, exit)
}

/// Parses the first returned operand of scryer's output (e.g. `42u32,` or
/// `-5i32,`) as a 32-bit value.
fn parse_result(stdout: &str) -> Option<u32> {
    let line = stdout
        .lines()
        .skip_while(|l| !l.contains("Returned Operands"))
        .nth(1)?;
    let first = line.split(',').next()?.trim();
    let split = first.find(['u', 'i'])?;
    let (num, ty) = first.split_at(split);
    let v: i128 = num.parse().ok()?;
    match ty {
        "u32" | "i32" | "u16" | "i16" | "u8" | "i8" => Some(v as u32),
        _ => None,
    }
}

fn temp_dir(name: &str) -> PathBuf {
    let dir = PathBuf::from(env!("CARGO_TARGET_TMPDIR"))
        .join("e2e")
        .join(name);
    std::fs::create_dir_all(&dir).unwrap();
    dir
}

fn compile(src: &Path, entry: &str, out: &Path) {
    // The backend has been seen to hang on some inputs; bound the compile.
    let mut child = Command::new(env!("CARGO_BIN_EXE_scry-cc"))
        .arg("--entry")
        .arg(entry)
        .arg("-o")
        .arg(out)
        .arg(src)
        .stdout(std::process::Stdio::piped())
        .stderr(std::process::Stdio::piped())
        .spawn()
        .expect("running scry-cc");
    let start = std::time::Instant::now();
    let limit = std::time::Duration::from_secs(120);
    let status = loop {
        match child.try_wait().expect("waiting for scry-cc") {
            Some(status) => break status,
            None if start.elapsed() > limit => {
                let _ = child.kill();
                let _ = child.wait();
                panic!(
                    "scry-cc did not finish within {}s for {}",
                    limit.as_secs(),
                    src.display()
                );
            }
            None => std::thread::sleep(std::time::Duration::from_millis(50)),
        }
    };
    let output = child.wait_with_output().expect("collecting scry-cc output");
    if !status.success() {
        let stderr = String::from_utf8_lossy(&output.stderr);
        let brief: Vec<&str> = stderr
            .lines()
            .filter(|l| !l.trim_start().starts_with("at ") && !l.trim().is_empty())
            .take(6)
            .collect();
        panic!(
            "scry-cc failed for {}: {}",
            src.display(),
            brief.join(" | ")
        );
    }
}

fn run_program(
    tools: &Tools,
    elf: &Path,
    args: &[i64],
    machine_mode: bool,
) -> std::process::Output {
    let mut cmd = Command::new(&tools.scryer);
    cmd.arg(elf)
        .arg("--target=scry32-unknown-none-elf")
        .arg("--timeout")
        .arg("60");
    if machine_mode {
        cmd.arg("--machine-mode");
    }
    for a in args {
        // Arguments are passed with an unsigned tag (the ABI llvm2clif uses).
        cmd.arg(format!("-i={}u32", *a as i32 as u32));
    }
    cmd.output().expect("running scryer")
}

fn check_program(tools: &Tools, src: &Path) {
    let text = std::fs::read_to_string(src).unwrap();
    let (cases, exit) = parse_spec(&text);
    let name = src.file_stem().unwrap().to_str().unwrap();
    let dir = temp_dir(name);
    let elf = dir.join(format!("{name}.elf"));
    let entry = if cases.is_empty() { "main" } else { "test" };
    compile(src, entry, &elf);
    let mut failures = Vec::new();
    for case in &cases {
        let mut args = case.args.clone();
        args.resize(4, 0);
        let out = run_program(tools, &elf, &args, false);
        let stdout = String::from_utf8_lossy(&out.stdout);
        match parse_result(&stdout) {
            Some(v) if v == case.expected as i32 as u32 => {}
            Some(v) => failures.push(format!(
                "{name}({:?}): expected {}, got {}",
                case.args, case.expected, v as i32
            )),
            None => {
                // Keep the simulator's error line, drop its metrics dump.
                let brief: Vec<&str> = stdout
                    .lines()
                    .take_while(|l| !l.contains("Simulation Metrics"))
                    .filter(|l| !l.trim().is_empty())
                    .collect();
                let stderr = String::from_utf8_lossy(&out.stderr);
                let stderr_brief: Vec<&str> = stderr
                    .lines()
                    .filter(|l| !l.trim_start().starts_with("at ") && !l.trim().is_empty())
                    .take(4)
                    .collect();
                failures.push(format!(
                    "{name}({:?}): no result (exit {:?}): {} {}",
                    case.args,
                    out.status.code(),
                    brief.join(" | "),
                    stderr_brief.join(" | ")
                ))
            }
        }
    }
    if let Some(code) = exit {
        let out = run_program(tools, &elf, &[], true);
        if out.status.code() != Some(code) {
            failures.push(format!(
                "{name}: expected exit code {code}, got {:?}",
                out.status.code()
            ));
        }
    }
    assert!(failures.is_empty(), "{}", failures.join("\n"));
}

fn programs_dir() -> PathBuf {
    Path::new(env!("CARGO_MANIFEST_DIR"))
        .join("tests")
        .join("programs")
}

#[test]
fn programs_run_on_scryer() {
    let Some(tools) = tools() else { return };
    let mut entries: Vec<PathBuf> = std::fs::read_dir(programs_dir())
        .unwrap()
        .map(|e| e.unwrap().path())
        .filter(|p| {
            p.extension().is_some_and(|e| e == "c")
                && p.file_name().is_some_and(|n| n != "native_main.c")
        })
        .collect();
    entries.sort();
    assert!(!entries.is_empty());
    let mut failures = Vec::new();
    let mut expected_failures = Vec::new();
    let mut unexpected_passes = Vec::new();
    for p in &entries {
        let name = p.file_name().unwrap().to_string_lossy().into_owned();
        let stem = p.file_stem().unwrap().to_string_lossy().into_owned();
        let known = KNOWN_BACKEND_FAILURES
            .iter()
            .find(|(n, _)| *n == stem)
            .map(|(_, why)| *why);
        let result = std::panic::catch_unwind(|| check_program(&tools, p));
        match (result, known) {
            (Ok(()), None) => eprintln!("ok: {name}"),
            (Ok(()), Some(why)) => {
                eprintln!("ok (unexpectedly, listed as known backend failure {why}): {name}");
                unexpected_passes.push(name);
            }
            (Err(e), known) => {
                let msg = e
                    .downcast_ref::<String>()
                    .cloned()
                    .or_else(|| e.downcast_ref::<&str>().map(|s| s.to_string()))
                    .unwrap_or_default();
                match known {
                    Some(why) => {
                        eprintln!("known backend failure {why}: {name}: {msg}");
                        expected_failures.push(name);
                    }
                    None => failures.push(format!("{name}: {msg}")),
                }
            }
        }
    }
    if !expected_failures.is_empty() {
        eprintln!(
            "{} known backend failures (see docs/backend-issues/README.md): {}",
            expected_failures.len(),
            expected_failures.join(", ")
        );
    }
    if !unexpected_passes.is_empty() {
        eprintln!(
            "NOTE: these programs pass now and can be removed from KNOWN_BACKEND_FAILURES: {}",
            unexpected_passes.join(", ")
        );
    }
    assert!(
        failures.is_empty(),
        "{} of {} programs failed:\n{}",
        failures.len(),
        entries.len(),
        failures.join("\n")
    );
}

#[test]
fn spec_parsing() {
    let (cases, exit) = parse_spec("// CASES: 1 -2 => 3\n// EXIT: 7\nint x;\n");
    assert_eq!(cases.len(), 1);
    assert_eq!(cases[0].args, vec![1, -2]);
    assert_eq!(cases[0].expected, 3);
    assert_eq!(exit, Some(7));
    assert_eq!(
        parse_result("----------  Returned Operands  ----------\n42u32, \n"),
        Some(42)
    );
    assert_eq!(
        parse_result("x\n----------  Returned Operands  ----------\n-5i32, \n"),
        Some(-5i32 as u32)
    );
    assert_eq!(
        parse_result("----------  Returned Operands  ----------\n4294967291u32, \n"),
        Some(-5i32 as u32)
    );
}
