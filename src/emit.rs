//! Output of translated modules: textual CLIF, or a Scry ELF object file
//! produced through Cranelift's Scry backend.

use crate::llvm;
use crate::translate::{build_isa, ClifModule, DataInit};
use crate::translate::{ClifData, DataReloc};
use anyhow::{anyhow, bail, Context as _};
use cranelift_codegen::ir::{
    ExternalName, Function, GlobalValueData, UserExternalName, UserFuncName,
};
use cranelift_codegen::Context;
use cranelift_module::{DataDescription, DataId, FuncId, Linkage, Module};
use cranelift_object::{ObjectBuilder, ObjectModule};
use std::collections::{HashMap, HashSet};
use std::fmt::Write as _;

/// Builds a module from textual Cranelift IR (for hand-written CLIF or
/// reproducers). Functions are named by their `%name`; functions that are
/// referenced but not defined become external. Data objects, which have no
/// CLIF syntax, are declared in comments of the form `clif_text` writes:
///
/// ```text
/// ; data %table: 4 bytes, align 4, read-only
/// ;   01020304
/// ; data %buf: 16 zero bytes, align 4, writable, relocations: [+0 -> %table+2]
/// ```
///
/// (`N bytes` without following hex lines is zero-filled; `external`
/// declares a variable defined elsewhere.)
pub fn module_from_clif(text: &str) -> anyhow::Result<ClifModule> {
    let funcs = cranelift_reader::parse_functions(text).map_err(|e| anyhow!("{e}"))?;
    let mut m = ClifModule {
        functions: Vec::new(),
        data: parse_data_comments(text)?,
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

/// Parses the `; data %name: ...` comment lines of a textual module (see
/// `module_from_clif`).
fn parse_data_comments(text: &str) -> anyhow::Result<Vec<ClifData>> {
    let mut data: Vec<ClifData> = Vec::new();
    let mut lines = text.lines().peekable();
    while let Some(line) = lines.next() {
        let Some(rest) = line.strip_prefix("; data %") else {
            continue;
        };
        let (name, spec) = rest
            .split_once(": ")
            .ok_or_else(|| anyhow!("malformed data declaration `{line}`"))?;
        let bad = || anyhow!("malformed data declaration `{line}`");
        let (spec, relocs) = match spec.split_once(", relocations:") {
            Some((s, r)) => (s, r),
            None => (spec, ""),
        };
        let mut parts = spec.split(", ");
        let init = parts.next().ok_or_else(bad)?;
        let align = parts
            .next()
            .and_then(|p| p.strip_prefix("align "))
            .and_then(|p| p.parse::<u64>().ok())
            .ok_or_else(bad)?;
        let writable = match parts.next() {
            Some("writable") => true,
            Some("read-only") => false,
            _ => return Err(bad()),
        };
        let mut hex = String::new();
        while let Some(l) = lines.peek() {
            match l.strip_prefix(";   ") {
                Some(h) if !h.is_empty() && h.bytes().all(|b| b.is_ascii_hexdigit()) => {
                    hex.push_str(h);
                    lines.next();
                }
                _ => break,
            }
        }
        let init = if init == "external" {
            None
        } else if let Some(n) = init.strip_suffix(" zero bytes") {
            Some(DataInit::Zero(n.parse().map_err(|_| bad())?))
        } else if let Some(n) = init.strip_suffix(" bytes") {
            let n: usize = n.parse().map_err(|_| bad())?;
            if hex.is_empty() {
                Some(DataInit::Zero(n as u64))
            } else {
                let bytes: Vec<u8> = (0..hex.len() / 2)
                    .map(|i| u8::from_str_radix(&hex[2 * i..2 * i + 2], 16).unwrap())
                    .collect();
                if bytes.len() != n {
                    bail!("data %{name}: {n} bytes declared, {} given", bytes.len());
                }
                Some(DataInit::Bytes(bytes))
            }
        } else {
            return Err(bad());
        };
        let mut parsed_relocs = Vec::new();
        for r in relocs.split('[').skip(1) {
            // `+OFFSET -> %SYMBOL+ADDEND]`
            let r = r.trim_end().trim_end_matches(']');
            let (off, sym) = r.split_once(" -> %").ok_or_else(bad)?;
            let offset: u32 = off.trim_start_matches('+').parse().map_err(|_| bad())?;
            let split = sym.find(['+', '-']).unwrap_or(sym.len());
            let (symbol, addend) = sym.split_at(split);
            let addend: i64 = if addend.is_empty() {
                0
            } else {
                addend.trim_start_matches('+').parse().map_err(|_| bad())?
            };
            parsed_relocs.push(DataReloc {
                offset,
                symbol: symbol.to_string(),
                addend,
            });
        }
        data.push(ClifData {
            name: name.to_string(),
            linkage: llvm::Linkage::External,
            writable,
            align,
            init,
            relocs: parsed_relocs,
        });
    }
    Ok(data)
}

/// Whether cranelift-reader can parse `name` after a `%`.
fn is_clif_name(name: &str) -> bool {
    !name.is_empty() && name.bytes().all(|b| b.is_ascii_alphanumeric() || b == b'_')
}

/// Maps the symbol names that cranelift-reader could not parse back
/// (LLVM's `.str.1`, `__const.f.x`, ...) to valid ones: other characters
/// become `_`, and a numeric suffix keeps the result distinct.
fn clif_name_map(m: &ClifModule) -> HashMap<String, String> {
    let names: Vec<&str> = m
        .functions
        .iter()
        .map(|f| f.name.as_str())
        .chain(m.externals.iter().map(|e| e.name.as_str()))
        .chain(m.data.iter().map(|d| d.name.as_str()))
        .chain(
            m.data
                .iter()
                .flat_map(|d| d.relocs.iter().map(|r| r.symbol.as_str())),
        )
        .collect();
    let mut taken: HashSet<String> = names
        .iter()
        .filter(|n| is_clif_name(n))
        .map(|n| n.to_string())
        .collect();
    let mut map = HashMap::new();
    for n in names {
        if is_clif_name(n) || map.contains_key(n) {
            continue;
        }
        let base: String = n
            .chars()
            .map(|c| if c.is_ascii_alphanumeric() { c } else { '_' })
            .collect();
        let mut candidate = base.clone();
        for i in 2.. {
            if !taken.contains(&candidate) {
                break;
            }
            candidate = format!("{base}_{i}");
        }
        taken.insert(candidate.clone());
        map.insert(n.to_string(), candidate);
    }
    map
}

/// A copy of `func` with its symbolic names replaced according to `map`.
fn renamed(func: &Function, map: &HashMap<String, String>) -> Function {
    let mut f = func.clone();
    if let UserFuncName::Testcase(tc) = &f.name {
        if let Some(new) = map.get(tc.to_string().trim_start_matches('%')) {
            f.name = UserFuncName::testcase(new.as_str());
        }
    }
    for (_, ext) in f.dfg.ext_funcs.iter_mut() {
        if let Some(new) = testcase_name(&ext.name).and_then(|n| map.get(&n)) {
            ext.name = ExternalName::testcase(new.as_str());
        }
    }
    for (_, gv) in f.global_values.iter_mut() {
        if let GlobalValueData::Symbol { name, .. } = gv {
            if let Some(new) = testcase_name(name).and_then(|n| map.get(&n)) {
                *name = ExternalName::testcase(new.as_str());
            }
        }
    }
    f
}

/// Renders the module as textual Cranelift IR that `module_from_clif` reads
/// back. Data objects, which have no CLIF syntax, are described in
/// comments, and symbol names the CLIF syntax cannot express are mangled
/// (see `clif_name_map`).
pub fn clif_text(m: &ClifModule) -> String {
    let map = clif_name_map(m);
    fn mapped<'a>(map: &'a HashMap<String, String>, n: &'a str) -> &'a str {
        map.get(n).map(String::as_str).unwrap_or(n)
    }
    let name = |n: &str| mapped(&map, n).to_string();
    let mut out = String::new();
    for d in &m.data {
        let _ = write!(out, "; data %{}: ", name(&d.name));
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
                let _ = write!(
                    out,
                    " [+{} -> %{}{:+}]",
                    r.offset,
                    name(&r.symbol),
                    r.addend
                );
            }
        }
        out.push('\n');
        if let Some(DataInit::Bytes(b)) = &d.init {
            for chunk in b.chunks(32) {
                out.push_str(";   ");
                for byte in chunk {
                    let _ = write!(out, "{byte:02x}");
                }
                out.push('\n');
            }
        }
    }
    if !m.data.is_empty() {
        out.push('\n');
    }
    for e in &m.externals {
        let _ = writeln!(out, "; external function %{}{}", name(&e.name), e.signature);
    }
    if !m.externals.is_empty() {
        out.push('\n');
    }
    for (name, reason) in &m.skipped {
        let _ = writeln!(out, "; skipped function %{name}: {reason}");
    }
    for f in &m.functions {
        if map.is_empty() {
            let _ = writeln!(out, "{}", f.func.display());
        } else {
            let _ = writeln!(out, "{}", renamed(&f.func, &map).display());
        }
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

#[cfg(test)]
mod tests {
    use super::*;

    const TEXT: &str = "\
; data %table: 4 bytes, align 4, read-only
;   01020304
; data %buf: 8 zero bytes, align 4, writable, relocations: [+0 -> %table+2] [+4 -> %f+0]
; data %ext: external, align 1, read-only

function %f(i32 uext) -> i32 uext system_v {
    gv0 = symbol %table
    sig0 = (i32 uext) -> i32 uext system_v
    fn0 = %g sig0

block0(v0: i32):
    v1 = symbol_value.i32 gv0
    v2 = load.i32 v1
    v3 = call fn0(v2)
    return v3
}
";

    #[test]
    fn data_comments_are_read_back() {
        let m = module_from_clif(TEXT).unwrap();
        assert_eq!(m.data.len(), 3);
        assert_eq!(m.data[0].name, "table");
        assert!(matches!(&m.data[0].init, Some(DataInit::Bytes(b)) if b == &[1, 2, 3, 4]));
        assert!(!m.data[0].writable);
        assert!(matches!(m.data[1].init, Some(DataInit::Zero(8))));
        assert!(m.data[1].writable);
        assert_eq!(m.data[1].relocs.len(), 2);
        assert_eq!(m.data[1].relocs[0].symbol, "table");
        assert_eq!(m.data[1].relocs[0].addend, 2);
        assert_eq!(m.data[1].relocs[1].offset, 4);
        assert!(m.data[2].init.is_none());
        assert_eq!(m.externals.len(), 1);
        assert_eq!(m.externals[0].name, "g");
        // The text round-trips.
        let again = module_from_clif(&clif_text(&m)).unwrap();
        assert_eq!(clif_text(&again), clif_text(&m));
        // And the object compiles, with the data bound to the function.
        object_file(&m, "t").unwrap();
    }

    #[test]
    fn unparsable_names_are_mangled_in_the_text() {
        let mut m = module_from_clif(TEXT).unwrap();
        // Rename the data object and the callee to names LLVM produces.
        m.data[0].name = "__const.f.table".into();
        m.data[1].relocs[0].symbol = "__const.f.table".into();
        m.externals[0].name = ".str.1".into();
        for (_, gv) in m.functions[0].func.global_values.iter_mut() {
            if let GlobalValueData::Symbol { name, .. } = gv {
                *name = ExternalName::testcase("__const.f.table");
            }
        }
        for (_, ext) in m.functions[0].func.dfg.ext_funcs.iter_mut() {
            ext.name = ExternalName::testcase(".str.1");
        }
        let text = clif_text(&m);
        assert!(text.contains("; data %__const_f_table: 4 bytes"));
        assert!(text.contains("[+0 -> %__const_f_table+2]"));
        assert!(text.contains("gv0 = symbol %__const_f_table"));
        assert!(text.contains("fn0 = %_str_1 sig0"));
        let again = module_from_clif(&text).unwrap();
        assert_eq!(again.data[0].name, "__const_f_table");
        object_file(&again, "t").unwrap();
    }
}
