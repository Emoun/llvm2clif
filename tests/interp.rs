//! Translator tests using Cranelift's interpreter as the oracle.
//!
//! Each test program's LLVM IR (checked in under `tests/programs/ll/`, see
//! `update_ll.py`) is translated to Cranelift IR and its `test` function is
//! run by `cranelift-interpreter` on the host for every `// CASES:` line.
//! This checks the translation independently of the Scry backend and needs
//! no external tools. Programs that use globals, function pointers or runtime
//! library calls cannot run in the interpreter and are skipped.

use cranelift_codegen::data_value::DataValue;
use cranelift_codegen::ir::{GlobalValueData, Opcode};
use cranelift_interpreter::environment::FunctionStore;
use cranelift_interpreter::interpreter::{Interpreter, InterpreterState};
use cranelift_interpreter::step::ControlFlow;
use llvm2clif::translate::{translate_module, ClifModule, Options};
use std::path::{Path, PathBuf};

fn programs_dir() -> PathBuf {
    Path::new(env!("CARGO_MANIFEST_DIR"))
        .join("tests")
        .join("programs")
}

/// `(args, expected)` pairs from the `// CASES:` lines.
fn cases(src: &str) -> Vec<(Vec<i32>, i32)> {
    src.lines()
        .filter_map(|l| l.strip_prefix("// CASES:"))
        .map(|rest| {
            let (args, expected) = rest.split_once("=>").expect("case expectation");
            (
                args.split_whitespace()
                    .map(|a| a.parse().unwrap())
                    .collect(),
                expected.trim().parse().unwrap(),
            )
        })
        .collect()
}

/// Why a module cannot run in the interpreter, if it cannot.
fn interpreter_limitation(m: &ClifModule) -> Option<String> {
    if !m.externals.is_empty() {
        return Some(format!(
            "calls external functions ({})",
            m.externals
                .iter()
                .map(|e| e.name.as_str())
                .collect::<Vec<_>>()
                .join(", ")
        ));
    }
    for f in &m.functions {
        for (_, gv) in f.func.global_values.iter() {
            if matches!(gv, GlobalValueData::Symbol { .. }) {
                return Some("uses global variables".into());
            }
        }
        for block in f.func.layout.blocks() {
            for inst in f.func.layout.block_insts(block) {
                if matches!(
                    f.func.dfg.insts[inst].opcode(),
                    Opcode::FuncAddr | Opcode::CallIndirect
                ) {
                    return Some("uses function pointers".into());
                }
            }
        }
    }
    None
}

fn run_program(name: &str) -> Result<(), String> {
    let c_path = programs_dir().join(format!("{name}.c"));
    let ll_path = programs_dir().join("ll").join(format!("{name}.ll"));
    let cases = cases(&std::fs::read_to_string(&c_path).unwrap());
    if cases.is_empty() {
        return Ok(());
    }
    let src = std::fs::read_to_string(&ll_path)
        .map_err(|e| format!("{}: {e} (run update_ll.py)", ll_path.display()))?;
    let module = llvm2clif::llvm::parse_module(&src).map_err(|e| e.to_string())?;
    let translated = translate_module(&module, &Options::default()).map_err(|e| e.to_string())?;
    if let Some(why) = interpreter_limitation(&translated) {
        eprintln!("skipped {name}: {why}");
        return Ok(());
    }
    // The interpreter resolves calls by the displayed `%name`.
    let mut store = FunctionStore::default();
    for f in &translated.functions {
        store.add(format!("%{}", f.name), &f.func);
    }
    let mut failures = Vec::new();
    for (args, expected) in cases {
        let mut args = args;
        args.resize(4, 0);
        let state = InterpreterState::default().with_function_store(store.clone());
        let mut interp = Interpreter::new(state);
        let dv: Vec<DataValue> = args.iter().map(|a| DataValue::I32(*a)).collect();
        match interp.call_by_name("%test", &dv) {
            Ok(ControlFlow::Return(vals)) => match vals.first() {
                Some(DataValue::I32(v)) if *v == expected => {}
                other => failures.push(format!(
                    "{name}({:?}): expected {expected}, got {other:?}",
                    args
                )),
            },
            Ok(other) => failures.push(format!(
                "{name}({:?}): unexpected control flow {other:?}",
                args
            )),
            Err(e) => failures.push(format!("{name}({:?}): interpreter error: {e:?}", args)),
        }
    }
    if failures.is_empty() {
        eprintln!("ok: {name}");
        Ok(())
    } else {
        Err(failures.join("\n"))
    }
}

#[test]
fn programs_run_in_interpreter() {
    let mut names: Vec<String> = std::fs::read_dir(programs_dir())
        .unwrap()
        .map(|e| e.unwrap().path())
        .filter(|p| {
            p.extension().is_some_and(|e| e == "c")
                && p.file_name().is_some_and(|n| n != "native_main.c")
        })
        .map(|p| p.file_stem().unwrap().to_string_lossy().into_owned())
        .collect();
    names.sort();
    let mut failures = Vec::new();
    for name in &names {
        match std::panic::catch_unwind(|| run_program(name)) {
            Ok(Ok(())) => {}
            Ok(Err(e)) => failures.push(e),
            Err(e) => {
                let msg = e
                    .downcast_ref::<String>()
                    .cloned()
                    .or_else(|| e.downcast_ref::<&str>().map(|s| s.to_string()))
                    .unwrap_or_default();
                failures.push(format!("{name}: panic: {msg}"));
            }
        }
    }
    assert!(
        failures.is_empty(),
        "{} programs failed:\n{}",
        failures.len(),
        failures.join("\n")
    );
}
