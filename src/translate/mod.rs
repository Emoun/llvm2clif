//! Translation of LLVM IR modules into Cranelift IR.
//!
//! [`translate_module`] turns a parsed [`Module`] into a [`ClifModule`]: one
//! Cranelift [`ir::Function`] per defined LLVM function (with symbolic
//! references to other functions and globals), plus the module's global
//! variables as byte images with relocations. The object emitter binds the
//! symbolic references when it writes an object file.

mod data;
mod func;
mod intrinsics;
pub mod types;
mod wide;

use crate::llvm::{Linkage, Module, ParamAttrs};
use cranelift_codegen::ir::{self, Signature};
use cranelift_codegen::isa::OwnedTargetIsa;
use cranelift_codegen::settings::{self, Configurable};
use std::collections::{BTreeMap, BTreeSet, HashMap};
use std::fmt;
use types::Layout;

/// The machine the translated code is compiled for.
#[derive(Clone, Copy, Debug, Default, PartialEq, Eq)]
pub enum Target {
    /// Scry, through the Scry backend of the Cranelift fork (32-bit words).
    #[default]
    Scry32,
    /// RISC-V RV64IMC, through Cranelift's riscv64 backend (64-bit words);
    /// for comparing the ISAs through the same translation.
    Riscv64,
}

impl Target {
    /// The width of the target's registers and pointers.
    pub fn word_bits(self) -> u32 {
        match self {
            Target::Scry32 => 32,
            Target::Riscv64 => 64,
        }
    }

    /// The target triple Cranelift knows the ISA by.
    pub fn triple(self) -> &'static str {
        match self {
            Target::Scry32 => "scry32-unknown-none-elf",
            Target::Riscv64 => "riscv64-unknown-none-elf",
        }
    }
}

impl fmt::Display for Target {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.write_str(match self {
            Target::Scry32 => "scry32",
            Target::Riscv64 => "riscv64",
        })
    }
}

/// How much Cranelift optimizes the translated functions.
#[derive(Clone, Copy, Debug, Default, PartialEq, Eq)]
pub enum OptLevel {
    /// No mid-end optimization: LLVM's `opt` has already optimized the
    /// program, and the block structure reaches the backend unchanged.
    #[default]
    None,
    /// Cranelift's own optimizations (`opt_level=speed`).
    Speed,
}

/// Translation options.
#[derive(Clone, Debug)]
pub struct Options {
    /// Skip (with a warning) functions that use unsupported constructs
    /// instead of failing the whole translation.
    pub skip_unsupported: bool,
    /// Run the Cranelift verifier on every translated function.
    pub verify: bool,
    /// The machine to compile for.
    pub target: Target,
    /// Cranelift's optimization level.
    pub opt_level: OptLevel,
}

impl Default for Options {
    fn default() -> Self {
        Options {
            skip_unsupported: false,
            verify: true,
            target: Target::Scry32,
            opt_level: OptLevel::None,
        }
    }
}

/// A translated function.
pub struct ClifFunction {
    pub name: String,
    pub linkage: Linkage,
    pub func: ir::Function,
}

/// The initial contents of a data object.
#[derive(Clone, Debug, PartialEq, Eq)]
pub enum DataInit {
    /// All zero bytes (`.bss`).
    Zero(u64),
    Bytes(Vec<u8>),
}

/// A pointer-sized absolute relocation inside a data object.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct DataReloc {
    pub offset: u32,
    pub symbol: String,
    pub addend: i64,
}

/// A translated global variable.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct ClifData {
    pub name: String,
    pub linkage: Linkage,
    pub writable: bool,
    pub align: u64,
    /// `None` for a declaration of a variable defined in another object.
    pub init: Option<DataInit>,
    pub relocs: Vec<DataReloc>,
}

/// A function that is referenced but defined outside the module.
#[derive(Clone, Debug)]
pub struct ExternalFunction {
    pub name: String,
    pub signature: Signature,
}

/// The result of translating a module.
pub struct ClifModule {
    pub functions: Vec<ClifFunction>,
    pub data: Vec<ClifData>,
    /// Functions declared but not defined in the module, and runtime helpers
    /// (`memcpy`, ...) the translation introduced.
    pub externals: Vec<ExternalFunction>,
    /// Functions skipped because of unsupported constructs: (name, reason).
    pub skipped: Vec<(String, String)>,
    /// Set of symbol names that are functions (defined or external).
    pub function_names: Vec<String>,
    /// The machine the module is compiled for.
    pub target: Target,
    /// Cranelift's optimization level when compiling it.
    pub opt_level: OptLevel,
}

/// A translation failure.
#[derive(Clone, Debug)]
pub struct Diagnostic {
    pub function: Option<String>,
    pub line: u32,
    pub message: String,
    /// Whether the failure is a deliberately unsupported construct (as
    /// opposed to malformed input or an internal error).
    pub unsupported: bool,
}

impl fmt::Display for Diagnostic {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match &self.function {
            Some(name) if self.line > 0 => write!(
                f,
                "in function @{name} (line {}): {}",
                self.line, self.message
            ),
            Some(name) => write!(f, "in function @{name}: {}", self.message),
            None if self.line > 0 => write!(f, "line {}: {}", self.line, self.message),
            None => write!(f, "{}", self.message),
        }
    }
}

impl std::error::Error for Diagnostic {}

/// An error raised while translating one function (before the function name
/// and line are attached).
#[derive(Clone, Debug)]
pub struct TransError {
    pub message: String,
    pub unsupported: bool,
}

impl TransError {
    pub fn unsupported(msg: impl Into<String>) -> Self {
        TransError {
            message: msg.into(),
            unsupported: true,
        }
    }

    pub fn invalid(msg: impl Into<String>) -> Self {
        TransError {
            message: msg.into(),
            unsupported: false,
        }
    }
}

impl From<String> for TransError {
    fn from(s: String) -> Self {
        // Type mapping errors ("iN is not supported", ...) are unsupported
        // constructs; anything else is treated the same way, since all the
        // string errors originate from type and layout checks.
        TransError {
            message: s,
            unsupported: true,
        }
    }
}

pub type TResult<T> = Result<T, TransError>;

/// Module-wide context shared by all function translations.
pub struct ModuleCtx<'m> {
    pub module: &'m Module,
    pub layout: Layout<'m>,
    pub isa: OwnedTargetIsa,
    pub options: Options,
    pub functions: HashMap<&'m str, &'m crate::llvm::Function>,
    pub globals: HashMap<&'m str, &'m crate::llvm::GlobalVar>,
    pub aliases: HashMap<&'m str, &'m crate::llvm::Alias>,
}

impl<'m> ModuleCtx<'m> {
    pub fn new(module: &'m Module, options: Options) -> Result<Self, Diagnostic> {
        types::set_word_bits(options.target.word_bits());
        let layout = Layout::new(module).map_err(|message| Diagnostic {
            function: None,
            line: 0,
            message,
            unsupported: true,
        })?;
        let isa = build_isa(options.target, options.opt_level).map_err(|message| Diagnostic {
            function: None,
            line: 0,
            message,
            unsupported: false,
        })?;
        let functions = module
            .functions
            .iter()
            .map(|f| (f.name.as_str(), f))
            .collect();
        let globals = module
            .globals
            .iter()
            .map(|g| (g.name.as_str(), g))
            .collect();
        let aliases = module
            .aliases
            .iter()
            .map(|a| (a.name.as_str(), a))
            .collect();
        Ok(ModuleCtx {
            module,
            layout,
            isa,
            options,
            functions,
            globals,
            aliases,
        })
    }

    /// The Cranelift signature of a function known to the module, from its
    /// declaration (which carries the parameter attributes).
    pub fn declared_signature(&self, f: &crate::llvm::Function) -> Result<Signature, String> {
        let attrs: Vec<ParamAttrs> = f.params.iter().map(|p| p.attrs.clone()).collect();
        self.layout.signature(
            &f.fn_type(),
            &attrs,
            &f.ret_attrs,
            self.isa.default_call_conv(),
        )
    }

    /// Whether `name` refers to a function (defined or declared) or an
    /// alias of one.
    pub fn is_function_symbol(&self, name: &str) -> bool {
        if self.functions.contains_key(name) {
            return true;
        }
        if let Some(a) = self.aliases.get(name) {
            if let Some(target) = data::alias_target(a) {
                return self.is_function_symbol(target);
            }
        }
        false
    }
}

/// Builds the Cranelift ISA used for verification and object emission.
pub fn build_isa(target: Target, opt_level: OptLevel) -> Result<OwnedTargetIsa, String> {
    let mut flag_builder = settings::builder();
    // By default Cranelift's optimizer stays off: LLVM's `opt` has already
    // optimized the program, so the block structure reaches the backend
    // unchanged (and the Scry backend does its own scheduling).
    let level = match opt_level {
        OptLevel::None => "none",
        OptLevel::Speed => "speed",
    };
    flag_builder
        .set("opt_level", level)
        .map_err(|e| e.to_string())?;
    let mut builder =
        cranelift_codegen::isa::lookup_by_name(target.triple()).map_err(|e| e.to_string())?;
    if target == Target::Riscv64 {
        // The backend insists on the G extensions (IMAFD) being enabled,
        // but the translated programs use no floating point or atomics, so
        // the code it emits is RV64IM; `has_zca` adds the compressed
        // encodings (RV64IMC). No bit-manipulation extensions.
        flag_builder
            .set("unwind_info", "false")
            .map_err(|e| e.to_string())?;
        builder.set("has_zca", "true").map_err(|e| e.to_string())?;
    }
    let flags = settings::Flags::new(flag_builder);
    builder.finish(flags).map_err(|e| e.to_string())
}

/// Translates a whole module.
pub fn translate_module(module: &Module, options: &Options) -> Result<ClifModule, Diagnostic> {
    let ctx = ModuleCtx::new(module, options.clone())?;

    if !module.module_asm.is_empty() {
        return Err(Diagnostic {
            function: None,
            line: 0,
            message: "module-level inline assembly is not supported".into(),
            unsupported: true,
        });
    }

    let mut out = ClifModule {
        functions: Vec::new(),
        data: Vec::new(),
        externals: Vec::new(),
        skipped: Vec::new(),
        function_names: Vec::new(),
        target: options.target,
        opt_level: options.opt_level,
    };
    let mut runtime_imports: BTreeMap<String, Signature> = BTreeMap::new();
    let mut helpers: BTreeSet<&'static str> = BTreeSet::new();

    for f in &module.functions {
        if f.is_declaration {
            continue;
        }
        match func::translate_function(&ctx, f, &mut runtime_imports, &mut helpers) {
            Ok(func) => out.functions.push(ClifFunction {
                name: f.name.clone(),
                linkage: f.linkage,
                func,
            }),
            Err((line, e)) => {
                if e.unsupported && options.skip_unsupported {
                    let reason = if line > 0 {
                        format!("line {line}: {}", e.message)
                    } else {
                        e.message.clone()
                    };
                    out.skipped.push((f.name.clone(), reason));
                } else {
                    return Err(Diagnostic {
                        function: Some(f.name.clone()),
                        line,
                        message: e.message,
                        unsupported: e.unsupported,
                    });
                }
            }
        }
    }

    // Aliases of functions become forwarding functions.
    for a in &module.aliases {
        match data::alias_target(a) {
            Some(target) if ctx.is_function_symbol(target) => {
                match func::translate_alias(&ctx, a, target) {
                    Ok(func) => out.functions.push(ClifFunction {
                        name: a.name.clone(),
                        linkage: a.linkage,
                        func,
                    }),
                    Err(e) => {
                        return Err(Diagnostic {
                            function: Some(a.name.clone()),
                            line: a.line,
                            message: e.message,
                            unsupported: e.unsupported,
                        })
                    }
                }
            }
            _ => {
                return Err(Diagnostic {
                    function: None,
                    line: a.line,
                    message: format!(
                    "alias @{} is not supported: only plain aliases of functions can be translated",
                    a.name
                ),
                    unsupported: true,
                })
            }
        }
    }

    // Helper functions the translation relies on, generated into the module
    // (local symbols, so that every object file can carry its own copy).
    for name in &helpers {
        let func = match *name {
            n if n == wide::udivmod_name() => wide::build_udivmod(&ctx),
            other => Err(TransError::invalid(format!("unknown helper `{other}`"))),
        }
        .map_err(|e| Diagnostic {
            function: Some(name.to_string()),
            line: 0,
            message: e.message,
            unsupported: e.unsupported,
        })?;
        out.functions.push(ClifFunction {
            name: name.to_string(),
            linkage: Linkage::Internal,
            func,
        });
    }

    for g in &module.globals {
        match data::translate_global(&ctx, g) {
            Ok(d) => out.data.push(d),
            Err(e) => {
                return Err(Diagnostic {
                    function: None,
                    line: g.line,
                    message: format!("in global @{}: {}", g.name, e.message),
                    unsupported: e.unsupported,
                })
            }
        }
    }

    // External functions: declared-but-undefined LLVM functions that are
    // not intrinsics, plus the runtime helpers.
    let defined: std::collections::HashSet<&str> =
        out.functions.iter().map(|f| f.name.as_str()).collect();
    for f in &module.functions {
        if !f.is_declaration || f.name.starts_with("llvm.") || defined.contains(f.name.as_str()) {
            continue;
        }
        match ctx.declared_signature(f) {
            Ok(signature) => out.externals.push(ExternalFunction {
                name: f.name.clone(),
                signature,
            }),
            Err(message) => {
                // An unsupported signature only matters if the function is
                // actually called; calls fail with their own diagnostic.
                if !options.skip_unsupported {
                    return Err(Diagnostic {
                        function: Some(f.name.clone()),
                        line: f.line,
                        message,
                        unsupported: true,
                    });
                }
            }
        }
    }
    for (name, signature) in runtime_imports {
        if !defined.contains(name.as_str()) && !out.externals.iter().any(|e| e.name == name) {
            out.externals.push(ExternalFunction { name, signature });
        }
    }
    out.function_names = out
        .functions
        .iter()
        .map(|f| f.name.clone())
        .chain(out.externals.iter().map(|e| e.name.clone()))
        .collect();
    Ok(out)
}
