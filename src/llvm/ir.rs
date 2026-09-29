//! In-memory representation of the LLVM IR subset understood by llvm2clif.
//!
//! The structures mirror the textual IR closely. Values inside a function are
//! identified by [`ValueId`] (an index into [`Function::values`]) and basic
//! blocks by [`BlockId`] (an index into [`Function::blocks`]).

use std::collections::HashMap;
use std::fmt;
use std::rc::Rc;

// ---------------------------------------------------------------------------
// Types
// ---------------------------------------------------------------------------

#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash)]
pub enum FloatKind {
    Half,
    BFloat,
    Float,
    Double,
    Fp128,
    X86Fp80,
    PpcFp128,
}

impl FloatKind {
    pub fn bits(self) -> u32 {
        match self {
            FloatKind::Half | FloatKind::BFloat => 16,
            FloatKind::Float => 32,
            FloatKind::Double => 64,
            FloatKind::Fp128 | FloatKind::PpcFp128 => 128,
            FloatKind::X86Fp80 => 80,
        }
    }
}

#[derive(Clone, Debug, PartialEq, Eq, Hash)]
pub struct FuncType {
    pub ret: Rc<Type>,
    pub params: Vec<Type>,
    pub varargs: bool,
}

#[derive(Clone, Debug, PartialEq, Eq, Hash)]
pub enum Type {
    Void,
    /// `iN`
    Int(u32),
    /// `ptr` (opaque pointer) in the given address space.
    Ptr(u32),
    Float(FloatKind),
    /// `[N x T]`
    Array(u64, Rc<Type>),
    /// `<N x T>` (`scalable` for `<vscale x N x T>`).
    Vector {
        len: u64,
        elem: Rc<Type>,
        scalable: bool,
    },
    /// `{ ... }` or `<{ ... }>` (packed).
    Struct {
        fields: Vec<Type>,
        packed: bool,
    },
    /// `%name`, resolved through [`Module::types`].
    Named(Rc<str>),
    /// `ret (params...)`
    Func(Rc<FuncType>),
    Label,
    Metadata,
    Token,
    X86Mmx,
    X86Amx,
}

impl Type {
    pub fn int(bits: u32) -> Type {
        Type::Int(bits)
    }

    pub fn ptr() -> Type {
        Type::Ptr(0)
    }

    pub fn is_void(&self) -> bool {
        matches!(self, Type::Void)
    }

    pub fn is_int(&self) -> bool {
        matches!(self, Type::Int(_))
    }

    pub fn is_ptr(&self) -> bool {
        matches!(self, Type::Ptr(_))
    }

    /// Whether this is a first-class aggregate (struct or array) before
    /// resolving named types.
    pub fn is_aggregate_shallow(&self) -> bool {
        matches!(self, Type::Array(..) | Type::Struct { .. })
    }

    pub fn int_bits(&self) -> Option<u32> {
        match self {
            Type::Int(b) => Some(*b),
            _ => None,
        }
    }
}

impl fmt::Display for Type {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Type::Void => write!(f, "void"),
            Type::Int(b) => write!(f, "i{b}"),
            Type::Ptr(0) => write!(f, "ptr"),
            Type::Ptr(n) => write!(f, "ptr addrspace({n})"),
            Type::Float(k) => write!(
                f,
                "{}",
                match k {
                    FloatKind::Half => "half",
                    FloatKind::BFloat => "bfloat",
                    FloatKind::Float => "float",
                    FloatKind::Double => "double",
                    FloatKind::Fp128 => "fp128",
                    FloatKind::X86Fp80 => "x86_fp80",
                    FloatKind::PpcFp128 => "ppc_fp128",
                }
            ),
            Type::Array(n, t) => write!(f, "[{n} x {t}]"),
            Type::Vector {
                len,
                elem,
                scalable,
            } => {
                if *scalable {
                    write!(f, "<vscale x {len} x {elem}>")
                } else {
                    write!(f, "<{len} x {elem}>")
                }
            }
            Type::Struct { fields, packed } => {
                if *packed {
                    write!(f, "<")?;
                }
                write!(f, "{{ ")?;
                for (i, t) in fields.iter().enumerate() {
                    if i > 0 {
                        write!(f, ", ")?;
                    }
                    write!(f, "{t}")?;
                }
                if fields.is_empty() {
                    write!(f, "}}")?;
                } else {
                    write!(f, " }}")?;
                }
                if *packed {
                    write!(f, ">")?;
                }
                Ok(())
            }
            Type::Named(n) => write!(f, "%{n}"),
            Type::Func(ft) => {
                write!(f, "{} (", ft.ret)?;
                for (i, t) in ft.params.iter().enumerate() {
                    if i > 0 {
                        write!(f, ", ")?;
                    }
                    write!(f, "{t}")?;
                }
                if ft.varargs {
                    if !ft.params.is_empty() {
                        write!(f, ", ")?;
                    }
                    write!(f, "...")?;
                }
                write!(f, ")")
            }
            Type::Label => write!(f, "label"),
            Type::Metadata => write!(f, "metadata"),
            Type::Token => write!(f, "token"),
            Type::X86Mmx => write!(f, "x86_mmx"),
            Type::X86Amx => write!(f, "x86_amx"),
        }
    }
}

// ---------------------------------------------------------------------------
// Constants
// ---------------------------------------------------------------------------

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum BinOp {
    Add,
    Sub,
    Mul,
    UDiv,
    SDiv,
    URem,
    SRem,
    Shl,
    LShr,
    AShr,
    And,
    Or,
    Xor,
    // Floating point operations are parsed so that modules containing them
    // can still be read; translating them is an error.
    FAdd,
    FSub,
    FMul,
    FDiv,
    FRem,
}

impl BinOp {
    pub fn from_name(s: &str) -> Option<BinOp> {
        Some(match s {
            "add" => BinOp::Add,
            "sub" => BinOp::Sub,
            "mul" => BinOp::Mul,
            "udiv" => BinOp::UDiv,
            "sdiv" => BinOp::SDiv,
            "urem" => BinOp::URem,
            "srem" => BinOp::SRem,
            "shl" => BinOp::Shl,
            "lshr" => BinOp::LShr,
            "ashr" => BinOp::AShr,
            "and" => BinOp::And,
            "or" => BinOp::Or,
            "xor" => BinOp::Xor,
            "fadd" => BinOp::FAdd,
            "fsub" => BinOp::FSub,
            "fmul" => BinOp::FMul,
            "fdiv" => BinOp::FDiv,
            "frem" => BinOp::FRem,
            _ => return None,
        })
    }

    pub fn name(self) -> &'static str {
        match self {
            BinOp::Add => "add",
            BinOp::Sub => "sub",
            BinOp::Mul => "mul",
            BinOp::UDiv => "udiv",
            BinOp::SDiv => "sdiv",
            BinOp::URem => "urem",
            BinOp::SRem => "srem",
            BinOp::Shl => "shl",
            BinOp::LShr => "lshr",
            BinOp::AShr => "ashr",
            BinOp::And => "and",
            BinOp::Or => "or",
            BinOp::Xor => "xor",
            BinOp::FAdd => "fadd",
            BinOp::FSub => "fsub",
            BinOp::FMul => "fmul",
            BinOp::FDiv => "fdiv",
            BinOp::FRem => "frem",
        }
    }

    pub fn is_float(self) -> bool {
        matches!(
            self,
            BinOp::FAdd | BinOp::FSub | BinOp::FMul | BinOp::FDiv | BinOp::FRem
        )
    }
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum CastOp {
    Trunc,
    ZExt,
    SExt,
    PtrToInt,
    IntToPtr,
    BitCast,
    AddrSpaceCast,
    FPTrunc,
    FPExt,
    FPToUI,
    FPToSI,
    UIToFP,
    SIToFP,
}

impl CastOp {
    pub fn from_name(s: &str) -> Option<CastOp> {
        Some(match s {
            "trunc" => CastOp::Trunc,
            "zext" => CastOp::ZExt,
            "sext" => CastOp::SExt,
            "ptrtoint" => CastOp::PtrToInt,
            "inttoptr" => CastOp::IntToPtr,
            "bitcast" => CastOp::BitCast,
            "addrspacecast" => CastOp::AddrSpaceCast,
            "fptrunc" => CastOp::FPTrunc,
            "fpext" => CastOp::FPExt,
            "fptoui" => CastOp::FPToUI,
            "fptosi" => CastOp::FPToSI,
            "uitofp" => CastOp::UIToFP,
            "sitofp" => CastOp::SIToFP,
            _ => return None,
        })
    }

    pub fn name(self) -> &'static str {
        match self {
            CastOp::Trunc => "trunc",
            CastOp::ZExt => "zext",
            CastOp::SExt => "sext",
            CastOp::PtrToInt => "ptrtoint",
            CastOp::IntToPtr => "inttoptr",
            CastOp::BitCast => "bitcast",
            CastOp::AddrSpaceCast => "addrspacecast",
            CastOp::FPTrunc => "fptrunc",
            CastOp::FPExt => "fpext",
            CastOp::FPToUI => "fptoui",
            CastOp::FPToSI => "fptosi",
            CastOp::UIToFP => "uitofp",
            CastOp::SIToFP => "sitofp",
        }
    }
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum IPred {
    Eq,
    Ne,
    Ugt,
    Uge,
    Ult,
    Ule,
    Sgt,
    Sge,
    Slt,
    Sle,
}

impl IPred {
    pub fn from_name(s: &str) -> Option<IPred> {
        Some(match s {
            "eq" => IPred::Eq,
            "ne" => IPred::Ne,
            "ugt" => IPred::Ugt,
            "uge" => IPred::Uge,
            "ult" => IPred::Ult,
            "ule" => IPred::Ule,
            "sgt" => IPred::Sgt,
            "sge" => IPred::Sge,
            "slt" => IPred::Slt,
            "sle" => IPred::Sle,
            _ => return None,
        })
    }

    pub fn is_signed(self) -> bool {
        matches!(self, IPred::Sgt | IPred::Sge | IPred::Slt | IPred::Sle)
    }
}

/// A constant together with its type.
#[derive(Clone, Debug, PartialEq)]
pub struct Constant {
    pub ty: Type,
    pub kind: ConstKind,
}

#[derive(Clone, Debug, PartialEq)]
pub enum ConstKind {
    /// Integer constant; the value is sign-agnostic bits stored in an i128
    /// (the parser keeps the literal's numeric value).
    Int(i128),
    /// Floating point literal text (unsupported for translation).
    Float(String),
    Null,
    Undef,
    Poison,
    ZeroInit,
    /// Reference to a global variable or function by name.
    Global(String),
    /// `[ ... ]`
    Array(Vec<Constant>),
    /// `c"..."`
    String(Vec<u8>),
    /// `{ ... }` / `<{ ... }>`
    Struct(Vec<Constant>),
    /// `< ... >`
    Vector(Vec<Constant>),
    /// `getelementptr [inbounds] (T, ptr base, idx...)`
    Gep {
        base_ty: Type,
        base: Box<Constant>,
        indices: Vec<Constant>,
        inbounds: bool,
    },
    /// `<castop> (C to T)`
    Cast(CastOp, Box<Constant>),
    /// `<binop> (C, C)`
    Binary(BinOp, Box<Constant>, Box<Constant>),
    /// `icmp pred (C, C)`
    ICmp(IPred, Box<Constant>, Box<Constant>),
    /// `select (C, C, C)`
    Select(Box<Constant>, Box<Constant>, Box<Constant>),
    /// `blockaddress(@f, %bb)`
    BlockAddress(String, String),
    /// `dso_local_equivalent @f` / `no_cfi @f`
    DsoLocalEquivalent(String),
    /// Inline assembly (`asm "..."`) used as a call target.
    InlineAsm,
    /// `none` (token constant)
    None,
}

impl Constant {
    pub fn int(ty: Type, v: i128) -> Constant {
        Constant {
            ty,
            kind: ConstKind::Int(v),
        }
    }

    pub fn as_int(&self) -> Option<i128> {
        match &self.kind {
            ConstKind::Int(v) => Some(*v),
            _ => None,
        }
    }
}

// ---------------------------------------------------------------------------
// Functions
// ---------------------------------------------------------------------------

#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash, PartialOrd, Ord)]
pub struct ValueId(pub u32);

#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash, PartialOrd, Ord)]
pub struct BlockId(pub u32);

/// An instruction operand: either an SSA value or a constant.
#[derive(Clone, Debug, PartialEq)]
pub enum Operand {
    Local(ValueId),
    Const(Constant),
    /// A metadata operand (only in calls to metadata intrinsics); ignored.
    Metadata,
}

#[derive(Clone, Debug, PartialEq)]
pub struct TypedOperand {
    pub ty: Type,
    pub op: Operand,
}

/// Parameter/argument attributes that matter for the ABI or translation.
#[derive(Clone, Debug, Default, PartialEq)]
pub struct ParamAttrs {
    pub byval: Option<Type>,
    pub sret: Option<Type>,
    pub signext: bool,
    pub zeroext: bool,
    pub align: Option<u64>,
    pub inreg: bool,
}

#[derive(Clone, Debug, PartialEq)]
pub struct Param {
    pub ty: Type,
    pub attrs: ParamAttrs,
    pub value: ValueId,
}

#[derive(Clone, Copy, Debug, Default, PartialEq, Eq)]
pub struct BinFlags {
    pub nsw: bool,
    pub nuw: bool,
    pub exact: bool,
    pub disjoint: bool,
}

#[derive(Clone, Debug, PartialEq)]
pub enum Callee {
    Global(String),
    Local(ValueId),
    Const(Constant),
    InlineAsm,
}

#[derive(Clone, Debug, PartialEq)]
pub struct CallArg {
    pub ty: Type,
    pub attrs: ParamAttrs,
    pub op: Operand,
}

#[derive(Clone, Debug, PartialEq)]
pub struct SwitchCase {
    pub value: i128,
    pub target: BlockId,
}

#[derive(Clone, Debug, PartialEq)]
pub enum InstKind {
    Binary {
        op: BinOp,
        flags: BinFlags,
        lhs: Operand,
        rhs: Operand,
    },
    ICmp {
        pred: IPred,
        ty: Type,
        lhs: Operand,
        rhs: Operand,
    },
    /// Floating point compare: parsed, not translatable.
    FCmp,
    Cast {
        op: CastOp,
        val: Operand,
        from: Type,
    },
    Select {
        cond: TypedOperand,
        t: Operand,
        f: Operand,
    },
    Phi {
        incoming: Vec<(Operand, BlockId)>,
    },
    Load {
        ptr: Operand,
        align: Option<u64>,
        volatile: bool,
        atomic: bool,
    },
    Store {
        val: TypedOperand,
        ptr: Operand,
        align: Option<u64>,
        volatile: bool,
        atomic: bool,
    },
    Alloca {
        alloc_ty: Type,
        count: Option<TypedOperand>,
        align: Option<u64>,
    },
    Gep {
        base_ty: Type,
        ptr: Operand,
        indices: Vec<TypedOperand>,
        inbounds: bool,
    },
    Call {
        callee: Callee,
        fn_ty: Rc<FuncType>,
        args: Vec<CallArg>,
        ret_attrs: ParamAttrs,
        tail: bool,
    },
    ExtractValue {
        agg: TypedOperand,
        indices: Vec<u32>,
    },
    InsertValue {
        agg: TypedOperand,
        val: TypedOperand,
        indices: Vec<u32>,
    },
    Freeze {
        val: Operand,
    },
    // Terminators
    Ret(Option<TypedOperand>),
    Br(BlockId),
    CondBr {
        cond: Operand,
        t: BlockId,
        f: BlockId,
    },
    Switch {
        val: TypedOperand,
        default: BlockId,
        cases: Vec<SwitchCase>,
    },
    Unreachable,
    /// Any other instruction (float ops, atomics, exceptions, vectors...).
    /// The opcode is recorded so the translator can report it.
    Unsupported(String),
}

#[derive(Clone, Debug, PartialEq)]
pub struct Instruction {
    /// The SSA value defined by this instruction, if it produces one.
    pub result: Option<ValueId>,
    /// The result type (`void` when there is no result).
    pub ty: Type,
    pub kind: InstKind,
    pub line: u32,
}

impl Instruction {
    pub fn is_terminator(&self) -> bool {
        matches!(
            self.kind,
            InstKind::Ret(_)
                | InstKind::Br(_)
                | InstKind::CondBr { .. }
                | InstKind::Switch { .. }
                | InstKind::Unreachable
        )
    }
}

#[derive(Clone, Debug, PartialEq)]
pub struct Block {
    pub name: String,
    pub insts: Vec<Instruction>,
}

/// Where an SSA value is defined.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum ValueDef {
    Param(usize),
    /// (block index, instruction index)
    Inst(BlockId, usize),
    /// Referenced but never defined (invalid IR).
    Undefined,
}

#[derive(Clone, Debug, PartialEq)]
pub struct ValueInfo {
    pub name: String,
    pub def: ValueDef,
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum Linkage {
    Private,
    Internal,
    AvailableExternally,
    LinkOnce,
    Weak,
    Common,
    Appending,
    ExternWeak,
    LinkOnceOdr,
    WeakOdr,
    External,
}

impl Linkage {
    pub fn from_name(s: &str) -> Option<Linkage> {
        Some(match s {
            "private" => Linkage::Private,
            "internal" => Linkage::Internal,
            "available_externally" => Linkage::AvailableExternally,
            "linkonce" => Linkage::LinkOnce,
            "weak" => Linkage::Weak,
            "common" => Linkage::Common,
            "appending" => Linkage::Appending,
            "extern_weak" => Linkage::ExternWeak,
            "linkonce_odr" => Linkage::LinkOnceOdr,
            "weak_odr" => Linkage::WeakOdr,
            "external" => Linkage::External,
            _ => return None,
        })
    }

    pub fn is_local(self) -> bool {
        matches!(self, Linkage::Private | Linkage::Internal)
    }
}

#[derive(Clone, Debug, PartialEq)]
pub struct Function {
    pub name: String,
    pub linkage: Linkage,
    pub ret_ty: Type,
    pub ret_attrs: ParamAttrs,
    pub params: Vec<Param>,
    pub varargs: bool,
    pub is_declaration: bool,
    pub blocks: Vec<Block>,
    pub values: Vec<ValueInfo>,
    /// Attribute group references (`#N`) and inline attribute names.
    pub attrs: Vec<String>,
    pub attr_groups: Vec<u32>,
    pub line: u32,
}

impl Function {
    pub fn fn_type(&self) -> FuncType {
        FuncType {
            ret: Rc::new(self.ret_ty.clone()),
            params: self.params.iter().map(|p| p.ty.clone()).collect(),
            varargs: self.varargs,
        }
    }

    pub fn value_name(&self, id: ValueId) -> &str {
        &self.values[id.0 as usize].name
    }

    pub fn block_by_name(&self, name: &str) -> Option<BlockId> {
        self.blocks
            .iter()
            .position(|b| b.name == name)
            .map(|i| BlockId(i as u32))
    }
}

// ---------------------------------------------------------------------------
// Module
// ---------------------------------------------------------------------------

#[derive(Clone, Debug, PartialEq)]
pub struct GlobalVar {
    pub name: String,
    pub linkage: Linkage,
    pub is_const: bool,
    pub ty: Type,
    pub init: Option<Constant>,
    pub align: Option<u64>,
    pub section: Option<String>,
    pub thread_local: bool,
    pub externally_initialized: bool,
    pub line: u32,
}

#[derive(Clone, Debug, PartialEq)]
pub struct Alias {
    pub name: String,
    pub linkage: Linkage,
    pub ty: Type,
    pub aliasee: Constant,
    pub line: u32,
}

#[derive(Clone, Debug, Default, PartialEq)]
pub struct Module {
    pub source_filename: Option<String>,
    pub datalayout: Option<String>,
    pub triple: Option<String>,
    /// Named types; `None` for opaque types.
    pub types: HashMap<String, Option<Type>>,
    pub globals: Vec<GlobalVar>,
    pub aliases: Vec<Alias>,
    pub functions: Vec<Function>,
    /// Attribute groups: `#N = { ... }` as the list of attribute names.
    pub attr_groups: HashMap<u32, Vec<String>>,
    /// `module asm` blocks (unsupported, reported by the translator).
    pub module_asm: Vec<String>,
}

impl Module {
    /// Resolves a named type to its definition (recursively). Returns `None`
    /// for opaque or unknown named types.
    pub fn resolve<'a>(&'a self, ty: &'a Type) -> Option<&'a Type> {
        let mut t = ty;
        let mut depth = 0;
        while let Type::Named(name) = t {
            t = self.types.get(&**name)?.as_ref()?;
            depth += 1;
            if depth > 64 {
                return None;
            }
        }
        Some(t)
    }

    pub fn function(&self, name: &str) -> Option<&Function> {
        self.functions.iter().find(|f| f.name == name)
    }

    pub fn global(&self, name: &str) -> Option<&GlobalVar> {
        self.globals.iter().find(|g| g.name == name)
    }
}
