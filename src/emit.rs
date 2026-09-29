//! Output of translated modules: textual CLIF, or a Scry ELF object file
//! produced through Cranelift's Scry backend.

use crate::llvm;
use crate::translate::{build_isa, ClifModule, DataInit};
use anyhow::{anyhow, bail, Context as _};
use cranelift_codegen::ir::{ExternalName, Function, GlobalValueData, UserExternalName};
use cranelift_codegen::Context;
use cranelift_module::{DataDescription, DataId, FuncId, Linkage, Module};
use cranelift_object::{ObjectBuilder, ObjectModule};
use std::collections::HashMap;
use std::fmt::Write as _;

/// Builds a module from textual Cranelift IR (for hand-written CLIF or
/// reproducers). Functions are named by their `%name`; functions that are
/// referenced but not defined become external.
pub fn module_from_clif(text: &str) -> anyhow::Result<ClifModule> {
    use cranelift_codegen::ir::UserFuncName;
    let funcs = cranelift_reader::parse_functions(text).map_err(|e| anyhow!("{e}"))?;
    let mut m = ClifModule {
        functions: Vec::new(),
        data: Vec::new(),
        externals: Vec::new(),
        skipped: Vec::new(),
        function_names: Vec::new(),
    };
    for f in funcs {
        let name = match &f.name {
            UserFuncName::Testcase(tc) => tc.to_string().trim_start_matches('%').to_string(),
            UserFuncName::User(u) => format!("u{}_{}", u.namespace, u.index),
        };
        m.functions.push(crate::translate::ClifFunction {
            name,
            linkage: llvm::Linkage::External,
            func: f,
        });
    }
    let defined: std::collections::HashSet<String> =
        m.functions.iter().map(|f| f.name.clone()).collect();
    for f in &m.functions {
        for (_, ext) in f.func.dfg.ext_funcs.iter() {
            if let Some(name) = testcase_name(&ext.name) {
                if !defined.contains(&name) && !m.externals.iter().any(|e| e.name == name) {
                    let signature = f.func.dfg.signatures[ext.signature].clone();
                    m.externals
                        .push(crate::translate::ExternalFunction { name, signature });
                }
            }
        }
    }
    m.function_names = m
        .functions
        .iter()
        .map(|f| f.name.clone())
        .chain(m.externals.iter().map(|e| e.name.clone()))
        .collect();
    Ok(m)
}

/// Renders the module as textual Cranelift IR. Data objects, which have no
/// CLIF syntax, are described in comments.
pub fn clif_text(m: &ClifModule) -> String {
    let mut out = String::new();
    for d in &m.data {
        let _ = write!(out, "; data %{}: ", d.name);
        match &d.init {
            None => out.push_str("external"),
            Some(DataInit::Zero(n)) => {
                let _ = write!(out, "{n} zero bytes");
            }
            Some(DataInit::Bytes(b)) => {
                let _ = write!(out, "{} bytes", b.len());
            }
        }
        let _ = write!(
            out,
            ", align {}, {}",
            d.align,
            if d.writable { "writable" } else { "read-only" }
        );
        if !d.relocs.is_empty() {
            out.push_str(", relocations:");
            for r in &d.relocs {
                let _ = write!(out, " [+{} -> %{}{:+}]", r.offset, r.symbol, r.addend);
            }
        }
        out.push('\n');
    }
    if !m.data.is_empty() {
        out.push('\n');
    }
    for e in &m.externals {
        let _ = writeln!(out, "; external function %{}{}", e.name, e.signature);
    }
    if !m.externals.is_empty() {
        out.push('\n');
    }
    for (name, reason) in &m.skipped {
        let _ = writeln!(out, "; skipped function %{name}: {reason}");
    }
    for f in &m.functions {
        let _ = writeln!(out, "{}", f.func.display());
    }
    out
}

fn linkage_of(l: llvm::Linkage, defined: bool) -> Linkage {
    if !defined {
        return Linkage::Import;
    }
    match l {
        llvm::Linkage::Private | llvm::Linkage::Internal => Linkage::Local,
        llvm::Linkage::AvailableExternally | llvm::Linkage::ExternWeak => Linkage::Import,
        llvm::Linkage::Weak
        | llvm::Linkage::LinkOnce
        | llvm::Linkage::WeakOdr
        | llvm::Linkage::LinkOnceOdr
        | llvm::Linkage::Common => Linkage::Preemptible,
        llvm::Linkage::External | llvm::Linkage::Appending => Linkage::Export,
    }
}

fn testcase_name(name: &ExternalName) -> Option<String> {
    match name {
        ExternalName::TestCase(tc) => Some(tc.to_string().trim_start_matches('%').to_string()),
        _ => None,
    }
}

/// Rewrites the symbolic (`%name`) references of a function into the
/// module's function/data ids.
fn bind_symbols(
    func: &mut Function,
    funcs: &HashMap<String, FuncId>,
    data: &HashMap<String, DataId>,
) -> anyhow::Result<()> {
    let ext_funcs: Vec<_> = func.dfg.ext_funcs.keys().collect();
    for fr in ext_funcs {
        let Some(name) = testcase_name(&func.dfg.ext_funcs[fr].name) else {
            continue;
        };
        let id = funcs
            .get(&name)
            .ok_or_else(|| anyhow!("call to undeclared function `{name}`"))?;
        let r = func.declare_imported_user_function(UserExternalName {
            namespace: 0,
            index: id.as_u32(),
        });
        func.dfg.ext_funcs[fr].name = ExternalName::User(r);
    }
    let gvs: Vec<_> = func.global_values.keys().collect();
    for gv in gvs {
        let name = match &func.global_values[gv] {
            GlobalValueData::Symbol { name, .. } => testcase_name(name),
            _ => None,
        };
        let Some(name) = name else { continue };
        let user = if let Some(id) = data.get(&name) {
            UserExternalName {
                namespace: 1,
                index: id.as_u32(),
            }
        } else if let Some(id) = funcs.get(&name) {
            UserExternalName {
                namespace: 0,
                index: id.as_u32(),
            }
        } else {
            bail!("reference to undeclared symbol `{name}`");
        };
        let r = func.declare_imported_user_function(user);
        if let GlobalValueData::Symbol { name, .. } = &mut func.global_values[gv] {
            *name = ExternalName::User(r);
        }
    }
    Ok(())
}

/// Compiles every function for Scry and renders the backend's disassembly
/// (the machine instructions, with symbolic references unresolved).
pub fn assembly_text(m: &ClifModule) -> anyhow::Result<String> {
    let isa = build_isa().map_err(|e| anyhow!(e))?;
    let mut out = String::new();
    for f in &m.functions {
        let mut ctx = Context::for_function(f.func.clone());
        ctx.set_disasm(true);
        let mut plane = cranelift_codegen::control::ControlPlane::default();
        ctx.compile(&*isa, &mut plane)
            .map_err(|e| anyhow!("compiling function `{}` for Scry: {}", f.name, e.inner))?;
        let compiled = ctx.compiled_code().expect("compiled");
        let _ = writeln!(
            out,
            "; function {} ({} bytes of code)",
            f.name,
            compiled.code_buffer().len()
        );
        match &compiled.vcode {
            Some(text) => out.push_str(text),
            None => out.push_str("; (no disassembly available)\n"),
        }
        out.push('\n');
    }
    Ok(out)
}

/// Compiles the module for Scry and returns the bytes of an ELF relocatable
/// object file.
pub fn object_file(m: &ClifModule, module_name: &str) -> anyhow::Result<Vec<u8>> {
    let isa = build_isa().map_err(|e| anyhow!(e))?;
    let mut builder =
        ObjectBuilder::new(isa, module_name, cranelift_module::default_libcall_names())?;
    // One section per function and data object lets the linker discard
    // what the program does not use (`--gc-sections`).
    builder.per_function_section(true);
    builder.per_data_object_section(true);
    let mut obj = ObjectModule::new(builder);

    let mut func_ids: HashMap<String, FuncId> = HashMap::new();
    let mut data_ids: HashMap<String, DataId> = HashMap::new();

    for f in &m.functions {
        let defined = f.linkage != llvm::Linkage::AvailableExternally;
        let id = obj
            .declare_function(&f.name, linkage_of(f.linkage, defined), &f.func.signature)
            .with_context(|| format!("declaring function `{}`", f.name))?;
        func_ids.insert(f.name.clone(), id);
    }
    for e in &m.externals {
        if func_ids.contains_key(&e.name) {
            continue;
        }
        let id = obj
            .declare_function(&e.name, Linkage::Import, &e.signature)
            .with_context(|| format!("declaring function `{}`", e.name))?;
        func_ids.insert(e.name.clone(), id);
    }
    for d in &m.data {
        let id = obj
            .declare_data(
                &d.name,
                linkage_of(d.linkage, d.init.is_some()),
                d.writable,
                false,
            )
            .with_context(|| format!("declaring global `{}`", d.name))?;
        data_ids.insert(d.name.clone(), id);
    }

    for d in &m.data {
        let Some(init) = &d.init else { continue };
        let mut desc = DataDescription::new();
        match init {
            // Zero-initialized data is emitted as explicit zero bytes rather
            // than as `.bss`: the scryer simulator treats the file-less part
            // of a segment as uninitialized memory and faults when it is read.
            DataInit::Zero(n) => desc.define(vec![0u8; *n as usize].into_boxed_slice()),
            DataInit::Bytes(b) => desc.define(b.clone().into_boxed_slice()),
        }
        desc.set_align(d.align);
        for r in &d.relocs {
            if let Some(id) = data_ids.get(&r.symbol) {
                let gv = obj.declare_data_in_data(*id, &mut desc);
                desc.write_data_addr(r.offset, gv, r.addend);
            } else if let Some(id) = func_ids.get(&r.symbol) {
                if r.addend != 0 {
                    bail!("global `{}`: an offset from the address of function `{}` cannot be stored in data", d.name, r.symbol);
                }
                let fref = obj.declare_func_in_data(*id, &mut desc);
                desc.write_function_addr(r.offset, fref);
            } else {
                bail!(
                    "global `{}` refers to undeclared symbol `{}`",
                    d.name,
                    r.symbol
                );
            }
        }
        obj.define_data(data_ids[&d.name], &desc)
            .with_context(|| format!("defining global `{}`", d.name))?;
    }

    for f in &m.functions {
        if f.linkage == llvm::Linkage::AvailableExternally {
            continue;
        }
        let mut func = f.func.clone();
        bind_symbols(&mut func, &func_ids, &data_ids)
            .with_context(|| format!("in function `{}`", f.name))?;
        if std::env::var_os("LLVM2CLIF_TRACE").is_some() {
            eprintln!("llvm2clif: compiling function `{}`", f.name);
        }
        let mut ctx = Context::for_function(func);
        obj.define_function(func_ids[&f.name], &mut ctx)
            .with_context(|| format!("compiling function `{}` for Scry", f.name))?;
    }

    let product = obj.finish();
    Ok(product.emit()?)
}
