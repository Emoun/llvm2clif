//! Recursive-descent parser for LLVM textual IR.
//!
//! The parser accepts the IR that `clang -S -emit-llvm` and `opt -S` produce
//! (LLVM 15+ with opaque pointers). Everything that llvm2clif does not need
//! (metadata, most attributes, comdats, ...) is parsed and discarded.
//! Instructions outside the supported subset are recorded as
//! [`InstKind::Unsupported`] so that the rest of the module can still be
//! processed and a precise error can be reported later.

use super::ir::*;
use super::lexer::{tokenize, Tok, Token};
use std::collections::HashMap;
use std::fmt;
use std::rc::Rc;

#[derive(Debug)]
pub struct ParseError {
    pub line: u32,
    pub message: String,
}

impl fmt::Display for ParseError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(f, "line {}: {}", self.line, self.message)
    }
}

impl std::error::Error for ParseError {}

type PResult<T> = Result<T, ParseError>;

/// Parses a complete module from its textual representation.
pub fn parse_module(src: &str) -> PResult<Module> {
    let toks = tokenize(src).map_err(|e| ParseError {
        line: e.line,
        message: e.message,
    })?;
    let mut p = Parser {
        toks,
        pos: 0,
        module: Module::default(),
    };
    p.parse_top_level()?;
    Ok(p.module)
}

struct Parser {
    toks: Vec<Token>,
    pos: usize,
    module: Module,
}

/// Per-function parsing state: value and block name tables.
struct FnState {
    values: Vec<ValueInfo>,
    value_ids: HashMap<String, ValueId>,
    block_ids: HashMap<String, BlockId>,
    block_names: Vec<String>,
    /// The next implicit number for unnamed values/blocks.
    next_unnamed: u32,
}

impl FnState {
    fn value(&mut self, name: &str) -> ValueId {
        if let Some(id) = self.value_ids.get(name) {
            return *id;
        }
        let id = ValueId(self.values.len() as u32);
        self.values.push(ValueInfo {
            name: name.to_string(),
            def: ValueDef::Undefined,
        });
        self.value_ids.insert(name.to_string(), id);
        id
    }

    fn block(&mut self, name: &str) -> BlockId {
        if let Some(id) = self.block_ids.get(name) {
            return *id;
        }
        let id = BlockId(self.block_names.len() as u32);
        self.block_names.push(name.to_string());
        self.block_ids.insert(name.to_string(), id);
        id
    }
}

const TYPE_KEYWORDS: &[&str] = &[
    "void",
    "half",
    "bfloat",
    "float",
    "double",
    "fp128",
    "x86_fp80",
    "ppc_fp128",
    "x86_mmx",
    "x86_amx",
    "label",
    "metadata",
    "token",
    "ptr",
    "opaque",
];

const VALUE_KEYWORDS: &[&str] = &[
    "true",
    "false",
    "null",
    "undef",
    "poison",
    "zeroinitializer",
    "none",
    "getelementptr",
    "trunc",
    "zext",
    "sext",
    "ptrtoint",
    "inttoptr",
    "bitcast",
    "addrspacecast",
    "fptrunc",
    "fpext",
    "fptoui",
    "fptosi",
    "uitofp",
    "sitofp",
    "add",
    "sub",
    "mul",
    "udiv",
    "sdiv",
    "urem",
    "srem",
    "shl",
    "lshr",
    "ashr",
    "and",
    "or",
    "xor",
    "fadd",
    "fsub",
    "fmul",
    "fdiv",
    "frem",
    "fneg",
    "icmp",
    "fcmp",
    "select",
    "blockaddress",
    "dso_local_equivalent",
    "no_cfi",
    "asm",
    "splat",
    "extractvalue",
    "insertvalue",
    "extractelement",
    "insertelement",
    "shufflevector",
];

fn is_type_keyword(s: &str) -> bool {
    if TYPE_KEYWORDS.contains(&s) {
        return true;
    }
    let mut chars = s.chars();
    chars.next() == Some('i') && s.len() > 1 && chars.all(|c| c.is_ascii_digit())
}

const TOP_LEVEL_KEYWORDS: &[&str] = &[
    "define",
    "declare",
    "attributes",
    "source_filename",
    "target",
    "module",
    "uselistorder",
    "uselistorder_bb",
];

impl Parser {
    // -- token helpers ---------------------------------------------------

    fn peek(&self) -> &Tok {
        &self.toks[self.pos].tok
    }

    fn peek_at(&self, n: usize) -> &Tok {
        let i = (self.pos + n).min(self.toks.len() - 1);
        &self.toks[i].tok
    }

    fn line(&self) -> u32 {
        self.toks[self.pos].line
    }

    fn next(&mut self) -> Tok {
        let t = self.toks[self.pos].tok.clone();
        if self.pos < self.toks.len() - 1 {
            self.pos += 1;
        }
        t
    }

    fn err<T>(&self, msg: impl Into<String>) -> PResult<T> {
        Err(ParseError {
            line: self.line(),
            message: msg.into(),
        })
    }

    fn expect(&mut self, tok: Tok) -> PResult<()> {
        if *self.peek() == tok {
            self.next();
            Ok(())
        } else {
            self.err(format!("expected `{tok}`, found `{}`", self.peek()))
        }
    }

    fn eat(&mut self, tok: &Tok) -> bool {
        if self.peek() == tok {
            self.next();
            true
        } else {
            false
        }
    }

    fn peek_ident(&self) -> Option<&str> {
        match self.peek() {
            Tok::Ident(s) => Some(s.as_str()),
            _ => None,
        }
    }

    fn eat_ident(&mut self, s: &str) -> bool {
        if self.peek_ident() == Some(s) {
            self.next();
            true
        } else {
            false
        }
    }

    fn expect_ident(&mut self, s: &str) -> PResult<()> {
        if self.eat_ident(s) {
            Ok(())
        } else {
            self.err(format!("expected `{s}`, found `{}`", self.peek()))
        }
    }

    fn expect_any_ident(&mut self) -> PResult<String> {
        match self.next() {
            Tok::Ident(s) => Ok(s),
            t => {
                self.pos -= 1;
                self.err(format!("expected an identifier, found `{t}`"))
            }
        }
    }

    fn expect_int(&mut self) -> PResult<i128> {
        match self.next() {
            Tok::Int(v) => Ok(v),
            t => {
                self.pos -= 1;
                self.err(format!("expected an integer, found `{t}`"))
            }
        }
    }

    fn expect_str(&mut self) -> PResult<Vec<u8>> {
        match self.next() {
            Tok::Str(s) => Ok(s),
            t => {
                self.pos -= 1;
                self.err(format!("expected a string, found `{t}`"))
            }
        }
    }

    fn expect_local(&mut self) -> PResult<String> {
        match self.next() {
            Tok::Local(s) => Ok(s),
            t => {
                self.pos -= 1;
                self.err(format!("expected a local name, found `{t}`"))
            }
        }
    }

    fn expect_global(&mut self) -> PResult<String> {
        match self.next() {
            Tok::Global(s) => Ok(s),
            t => {
                self.pos -= 1;
                self.err(format!("expected a global name, found `{t}`"))
            }
        }
    }

    /// Skips tokens until the end of the current source line.
    fn skip_to_next_line(&mut self) {
        let line = self.line();
        while *self.peek() != Tok::Eof && self.line() == line {
            self.next();
        }
    }

    /// Skips a balanced group starting at the current opening bracket.
    fn skip_balanced(&mut self) -> PResult<()> {
        let open = self.next();
        let close = match open {
            Tok::LParen => Tok::RParen,
            Tok::LBracket => Tok::RBracket,
            Tok::LBrace => Tok::RBrace,
            Tok::Less => Tok::Greater,
            t => return self.err(format!("expected an opening bracket, found `{t}`")),
        };
        let mut depth = 1;
        loop {
            let t = self.next();
            if t == Tok::Eof {
                return self.err("unexpected end of file inside a bracketed group");
            }
            if t == open {
                depth += 1;
            } else if t == close {
                depth -= 1;
                if depth == 0 {
                    return Ok(());
                }
            }
        }
    }

    /// Skips a metadata value: `!N`, `!"str"`, `!{...}`, `!DIxxx(...)`, or a
    /// typed value (`i32 %x`) used as metadata.
    fn skip_metadata_value(&mut self, st: Option<&mut FnState>) -> PResult<()> {
        match self.peek().clone() {
            Tok::MetaRef(_) => {
                self.next();
                Ok(())
            }
            Tok::Bang => {
                self.next();
                match self.peek() {
                    Tok::Str(_) => {
                        self.next();
                        Ok(())
                    }
                    Tok::LBrace => self.skip_balanced(),
                    Tok::Ident(_) => {
                        self.next();
                        if *self.peek() == Tok::LParen {
                            self.skip_balanced()?;
                        }
                        Ok(())
                    }
                    _ => self.err("malformed metadata"),
                }
            }
            _ => {
                let _ = self.parse_typed_value_in(st)?;
                Ok(())
            }
        }
    }

    /// Skips trailing `, !kind !node` metadata attachments of an instruction
    /// or global.
    fn skip_metadata_attachments(&mut self) -> PResult<()> {
        while *self.peek() == Tok::Comma && matches!(self.peek_at(1), Tok::MetaRef(_)) {
            self.next();
            self.next();
            self.skip_metadata_value(None)?;
        }
        Ok(())
    }

    // -- top level -------------------------------------------------------

    fn parse_top_level(&mut self) -> PResult<()> {
        loop {
            match self.peek().clone() {
                Tok::Eof => return Ok(()),
                Tok::Ident(s) => match s.as_str() {
                    "source_filename" => {
                        self.next();
                        self.expect(Tok::Equal)?;
                        let s = self.expect_str()?;
                        self.module.source_filename =
                            Some(String::from_utf8_lossy(&s).into_owned());
                    }
                    "target" => {
                        self.next();
                        let what = self.expect_any_ident()?;
                        self.expect(Tok::Equal)?;
                        let s = String::from_utf8_lossy(&self.expect_str()?).into_owned();
                        match what.as_str() {
                            "datalayout" => self.module.datalayout = Some(s),
                            "triple" => self.module.triple = Some(s),
                            _ => return self.err(format!("unknown target property `{what}`")),
                        }
                    }
                    "declare" => {
                        self.next();
                        self.parse_function(false)?;
                    }
                    "define" => {
                        self.next();
                        self.parse_function(true)?;
                    }
                    "attributes" => {
                        self.next();
                        let n = match self.next() {
                            Tok::AttrGroup(n) => n,
                            _ => return self.err("expected `#N` after `attributes`"),
                        };
                        self.expect(Tok::Equal)?;
                        let names = self.parse_attr_group_body()?;
                        self.module.attr_groups.insert(n, names);
                    }
                    "module" => {
                        self.next();
                        self.expect_ident("asm")?;
                        let s = self.expect_str()?;
                        self.module
                            .module_asm
                            .push(String::from_utf8_lossy(&s).into_owned());
                    }
                    "uselistorder" | "uselistorder_bb" => self.skip_to_next_line(),
                    _ => return self.err(format!("unexpected `{s}` at top level")),
                },
                Tok::Local(name) => {
                    // %name = type ...
                    self.next();
                    self.expect(Tok::Equal)?;
                    self.expect_ident("type")?;
                    if self.eat_ident("opaque") {
                        self.module.types.insert(name, None);
                    } else {
                        let ty = self.parse_type()?;
                        self.module.types.insert(name, Some(ty));
                    }
                }
                Tok::Global(name) => {
                    self.next();
                    self.expect(Tok::Equal)?;
                    self.parse_global(name)?;
                }
                Tok::Comdat(_) => self.skip_to_next_line(),
                Tok::MetaRef(_) => self.skip_to_next_line(),
                Tok::Bang => self.skip_to_next_line(),
                t => return self.err(format!("unexpected `{t}` at top level")),
            }
        }
    }

    /// Parses `{ attr attr "str"="val" ... }` collecting attribute names.
    fn parse_attr_group_body(&mut self) -> PResult<Vec<String>> {
        self.expect(Tok::LBrace)?;
        let mut names = Vec::new();
        loop {
            match self.next() {
                Tok::RBrace => return Ok(names),
                Tok::Eof => return self.err("unterminated attribute group"),
                Tok::Ident(s) => {
                    names.push(s);
                    if *self.peek() == Tok::LParen {
                        self.skip_balanced()?;
                    } else if matches!(self.peek(), Tok::Int(_)) {
                        // e.g. `align 8`
                        self.next();
                    }
                }
                Tok::Str(s) => {
                    names.push(String::from_utf8_lossy(&s).into_owned());
                    if self.eat(&Tok::Equal) {
                        self.next();
                    }
                }
                Tok::AttrGroup(_) => {}
                _ => {}
            }
        }
    }

    fn parse_linkage_and_modifiers(&mut self) -> PResult<(Linkage, GlobalMods)> {
        let mut linkage = Linkage::External;
        let mut mods = GlobalMods::default();
        while let Some(id) = self.peek_ident() {
            if let Some(l) = Linkage::from_name(id) {
                linkage = l;
                self.next();
                continue;
            }
            match id {
                "dso_local" | "dso_preemptable" | "default" | "hidden" | "protected"
                | "dllimport" | "dllexport" | "unnamed_addr" | "local_unnamed_addr" => {
                    self.next();
                }
                "externally_initialized" => {
                    mods.externally_initialized = true;
                    self.next();
                }
                "thread_local" => {
                    mods.thread_local = true;
                    self.next();
                    if *self.peek() == Tok::LParen {
                        self.skip_balanced()?;
                    }
                }
                "addrspace" => {
                    self.next();
                    self.skip_balanced()?;
                }
                _ => break,
            }
        }
        Ok((linkage, mods))
    }

    fn parse_global(&mut self, name: String) -> PResult<()> {
        let line = self.line();
        let (linkage, mods) = self.parse_linkage_and_modifiers()?;
        let kind = self.expect_any_ident()?;
        match kind.as_str() {
            "global" | "constant" => {
                let is_const = kind == "constant";
                let ty = self.parse_type()?;
                let init = if self.starts_value() {
                    Some(self.parse_value(&ty)?)
                } else {
                    None
                };
                let mut g = GlobalVar {
                    name,
                    linkage,
                    is_const,
                    ty,
                    init,
                    align: None,
                    section: None,
                    thread_local: mods.thread_local,
                    externally_initialized: mods.externally_initialized,
                    line,
                };
                self.parse_global_trailer(&mut g)?;
                self.module.globals.push(g);
                Ok(())
            }
            "alias" => {
                let ty = self.parse_type()?;
                self.expect(Tok::Comma)?;
                let aliasee = self.parse_typed_constant()?;
                let mut dummy = GlobalVar {
                    name: name.clone(),
                    linkage,
                    is_const: false,
                    ty: ty.clone(),
                    init: None,
                    align: None,
                    section: None,
                    thread_local: false,
                    externally_initialized: false,
                    line,
                };
                self.parse_global_trailer(&mut dummy)?;
                self.module.aliases.push(Alias {
                    name,
                    linkage,
                    ty,
                    aliasee,
                    line,
                });
                Ok(())
            }
            "ifunc" => self.err(format!("`ifunc` (@{name}) is not supported")),
            _ => self.err(format!(
                "expected `global`, `constant` or `alias` after `@{name} =`, found `{kind}`"
            )),
        }
    }

    /// Whether the next token starts a value (as opposed to the next
    /// top-level item or a trailing `, ...` clause).
    fn starts_value(&self) -> bool {
        match self.peek() {
            Tok::Int(_)
            | Tok::Float(_)
            | Tok::CStr(_)
            | Tok::Local(_)
            | Tok::LBracket
            | Tok::LBrace
            | Tok::Less => true,
            Tok::Global(_) => *self.peek_at(1) != Tok::Equal,
            Tok::Ident(s) => VALUE_KEYWORDS.contains(&s.as_str()),
            _ => false,
        }
    }

    /// Parses the `, section "..."`, `, align N`, ... tail of a global.
    fn parse_global_trailer(&mut self, g: &mut GlobalVar) -> PResult<()> {
        while *self.peek() == Tok::Comma {
            self.next();
            match self.next() {
                Tok::Ident(s) => match s.as_str() {
                    "section" => {
                        let sec = self.expect_str()?;
                        g.section = Some(String::from_utf8_lossy(&sec).into_owned());
                    }
                    "align" => g.align = Some(self.expect_int()? as u64),
                    "partition" => {
                        self.expect_str()?;
                    }
                    "comdat" => {
                        if *self.peek() == Tok::LParen {
                            self.skip_balanced()?;
                        }
                    }
                    _ => {
                        if *self.peek() == Tok::LParen {
                            self.skip_balanced()?;
                        }
                    }
                },
                Tok::MetaRef(_) => self.skip_metadata_value(None)?,
                t => return self.err(format!("unexpected `{t}` in global variable definition")),
            }
        }
        Ok(())
    }

    // -- types -----------------------------------------------------------

    fn starts_type(&self) -> bool {
        match self.peek() {
            Tok::Ident(s) => is_type_keyword(s),
            Tok::Local(_) | Tok::LBracket | Tok::LBrace | Tok::Less => true,
            _ => false,
        }
    }

    pub(crate) fn parse_type(&mut self) -> PResult<Type> {
        let base = self.parse_base_type()?;
        // Function types: `ret (params)`.
        if *self.peek() == Tok::LParen {
            self.next();
            let mut params = Vec::new();
            let mut varargs = false;
            if !self.eat(&Tok::RParen) {
                loop {
                    if self.eat(&Tok::Ellipsis) {
                        varargs = true;
                        self.expect(Tok::RParen)?;
                        break;
                    }
                    params.push(self.parse_type()?);
                    if self.eat(&Tok::RParen) {
                        break;
                    }
                    self.expect(Tok::Comma)?;
                }
            }
            return Ok(Type::Func(Rc::new(FuncType {
                ret: Rc::new(base),
                params,
                varargs,
            })));
        }
        Ok(base)
    }

    fn parse_base_type(&mut self) -> PResult<Type> {
        match self.next() {
            Tok::Ident(s) => Ok(match s.as_str() {
                "void" => Type::Void,
                "half" => Type::Float(FloatKind::Half),
                "bfloat" => Type::Float(FloatKind::BFloat),
                "float" => Type::Float(FloatKind::Float),
                "double" => Type::Float(FloatKind::Double),
                "fp128" => Type::Float(FloatKind::Fp128),
                "x86_fp80" => Type::Float(FloatKind::X86Fp80),
                "ppc_fp128" => Type::Float(FloatKind::PpcFp128),
                "x86_mmx" => Type::X86Mmx,
                "x86_amx" => Type::X86Amx,
                "label" => Type::Label,
                "metadata" => Type::Metadata,
                "token" => Type::Token,
                "ptr" => {
                    let mut addrspace = 0;
                    if self.eat_ident("addrspace") {
                        self.expect(Tok::LParen)?;
                        addrspace = self.expect_int()? as u32;
                        self.expect(Tok::RParen)?;
                    }
                    Type::Ptr(addrspace)
                }
                _ => {
                    if let Some(bits) = s.strip_prefix('i').and_then(|b| b.parse::<u32>().ok()) {
                        if bits == 0 {
                            return self.err("integer type width must be positive");
                        }
                        Type::Int(bits)
                    } else {
                        self.pos -= 1;
                        return self.err(format!("expected a type, found `{s}`"));
                    }
                }
            }),
            Tok::Local(name) => Ok(Type::Named(Rc::from(name.as_str()))),
            Tok::LBracket => {
                let n = self.expect_int()?;
                self.expect_ident("x")?;
                let elem = self.parse_type()?;
                self.expect(Tok::RBracket)?;
                Ok(Type::Array(n as u64, Rc::new(elem)))
            }
            Tok::LBrace => {
                let fields = self.parse_type_list(Tok::RBrace)?;
                Ok(Type::Struct {
                    fields,
                    packed: false,
                })
            }
            Tok::Less => {
                if self.eat(&Tok::LBrace) {
                    let fields = self.parse_type_list(Tok::RBrace)?;
                    self.expect(Tok::Greater)?;
                    return Ok(Type::Struct {
                        fields,
                        packed: true,
                    });
                }
                let scalable = self.eat_ident("vscale");
                if scalable {
                    self.expect_ident("x")?;
                }
                let n = self.expect_int()?;
                self.expect_ident("x")?;
                let elem = self.parse_type()?;
                self.expect(Tok::Greater)?;
                Ok(Type::Vector {
                    len: n as u64,
                    elem: Rc::new(elem),
                    scalable,
                })
            }
            t => {
                self.pos -= 1;
                self.err(format!("expected a type, found `{t}`"))
            }
        }
    }

    fn parse_type_list(&mut self, close: Tok) -> PResult<Vec<Type>> {
        let mut tys = Vec::new();
        if self.eat(&close) {
            return Ok(tys);
        }
        loop {
            tys.push(self.parse_type()?);
            if self.eat(&close) {
                return Ok(tys);
            }
            self.expect(Tok::Comma)?;
        }
    }

    // -- attributes ------------------------------------------------------

    /// Parses a run of parameter/return attributes, stopping at the first
    /// token that is not an attribute.
    fn parse_param_attrs(&mut self) -> PResult<ParamAttrs> {
        let mut attrs = ParamAttrs::default();
        loop {
            match self.peek().clone() {
                Tok::Ident(s) => {
                    if is_type_keyword(&s) || VALUE_KEYWORDS.contains(&s.as_str()) {
                        break;
                    }
                    self.next();
                    match s.as_str() {
                        "byval" | "sret" | "elementtype" | "inalloca" | "preallocated"
                        | "byref" => {
                            self.expect(Tok::LParen)?;
                            let ty = self.parse_type()?;
                            self.expect(Tok::RParen)?;
                            match s.as_str() {
                                "byval" => attrs.byval = Some(ty),
                                "sret" => attrs.sret = Some(ty),
                                _ => {}
                            }
                        }
                        "signext" => attrs.signext = true,
                        "zeroext" => attrs.zeroext = true,
                        "inreg" => attrs.inreg = true,
                        "align" => {
                            if *self.peek() == Tok::LParen {
                                self.next();
                                attrs.align = Some(self.expect_int()? as u64);
                                self.expect(Tok::RParen)?;
                            } else {
                                attrs.align = Some(self.expect_int()? as u64);
                            }
                        }
                        _ => {
                            if *self.peek() == Tok::LParen {
                                self.skip_balanced()?;
                            }
                        }
                    }
                }
                Tok::Str(_) => {
                    // "key"="value" string attributes.
                    self.next();
                    if self.eat(&Tok::Equal) {
                        self.next();
                    }
                }
                Tok::AttrGroup(_) => {
                    self.next();
                }
                _ => break,
            }
        }
        Ok(attrs)
    }

    // -- functions -------------------------------------------------------

    fn parse_function(&mut self, is_define: bool) -> PResult<()> {
        let line = self.line();
        let (linkage, _mods) = self.parse_linkage_and_modifiers()?;
        let mut ret_attrs = ParamAttrs::default();
        // Calling convention and return attributes precede the return type.
        loop {
            if self.starts_type() {
                break;
            }
            match self.peek().clone() {
                Tok::Ident(s) => {
                    self.next();
                    match s.as_str() {
                        "signext" => ret_attrs.signext = true,
                        "zeroext" => ret_attrs.zeroext = true,
                        _ => {}
                    }
                    if s == "cc" {
                        self.expect_int()?;
                    } else if *self.peek() == Tok::LParen {
                        self.skip_balanced()?;
                    } else if s == "align" {
                        self.expect_int()?;
                    }
                }
                Tok::Str(_) => {
                    self.next();
                    if self.eat(&Tok::Equal) {
                        self.next();
                    }
                }
                t => return self.err(format!("unexpected `{t}` in function header")),
            }
        }
        // The return type may be followed by return attributes in some
        // printers; handle `ret_attrs ret_ty` (the common form) only.
        let ret_ty = self.parse_type()?;
        let name = self.expect_global()?;

        let mut st = FnState {
            values: Vec::new(),
            value_ids: HashMap::new(),
            block_ids: HashMap::new(),
            block_names: Vec::new(),
            next_unnamed: 0,
        };

        // Parameters.
        self.expect(Tok::LParen)?;
        let mut params = Vec::new();
        let mut varargs = false;
        if !self.eat(&Tok::RParen) {
            loop {
                if self.eat(&Tok::Ellipsis) {
                    varargs = true;
                    self.expect(Tok::RParen)?;
                    break;
                }
                let ty = self.parse_type()?;
                let attrs = self.parse_param_attrs()?;
                let pname = match self.peek() {
                    Tok::Local(n) => {
                        let n = n.clone();
                        self.next();
                        n
                    }
                    _ => {
                        let n = st.next_unnamed.to_string();
                        st.next_unnamed += 1;
                        n
                    }
                };
                if let Ok(n) = pname.parse::<u32>() {
                    st.next_unnamed = n + 1;
                }
                let value = st.value(&pname);
                st.values[value.0 as usize].def = ValueDef::Param(params.len());
                params.push(Param { ty, attrs, value });
                if self.eat(&Tok::RParen) {
                    break;
                }
                self.expect(Tok::Comma)?;
            }
        }

        // Function attributes. A declaration's attributes end with its line;
        // a definition's end at the opening brace of the body.
        let header_line = self.toks[self.pos - 1].line;
        let mut attrs = Vec::new();
        let mut attr_groups = Vec::new();
        loop {
            if !is_define && self.line() != header_line {
                break;
            }
            match self.peek().clone() {
                Tok::AttrGroup(n) => {
                    attr_groups.push(n);
                    self.next();
                }
                Tok::Ident(s) => {
                    if TOP_LEVEL_KEYWORDS.contains(&s.as_str()) {
                        break;
                    }
                    self.next();
                    attrs.push(s.clone());
                    match s.as_str() {
                        "align" => {
                            self.expect_int()?;
                        }
                        "section" | "partition" | "gc" => {
                            self.expect_str()?;
                        }
                        "prefix" | "prologue" | "personality" => {
                            self.parse_typed_constant()?;
                        }
                        "comdat" => {
                            if *self.peek() == Tok::LParen {
                                self.skip_balanced()?;
                            }
                        }
                        _ => {
                            if *self.peek() == Tok::LParen {
                                self.skip_balanced()?;
                            }
                        }
                    }
                }
                Tok::Str(_) => {
                    self.next();
                    if self.eat(&Tok::Equal) {
                        self.next();
                    }
                }
                Tok::MetaRef(_) => {
                    self.next();
                    self.skip_metadata_value(None)?;
                }
                _ => break,
            }
        }

        let mut func = Function {
            name,
            linkage,
            ret_ty,
            ret_attrs,
            params,
            varargs,
            is_declaration: !is_define,
            blocks: Vec::new(),
            values: Vec::new(),
            attrs,
            attr_groups,
            line,
        };

        if is_define {
            self.expect(Tok::LBrace)?;
            self.parse_body(&mut st, &mut func)?;
        }
        func.values = st.values;
        self.module.functions.push(func);
        Ok(())
    }

    /// Pre-scans the function body (from just after `{`) to collect the
    /// block labels in definition order, so that blocks can be numbered
    /// before forward references are resolved.
    fn prescan_labels(&self, st: &mut FnState) -> PResult<()> {
        let mut depth = 1;
        let mut i = self.pos;
        let mut first = true;
        while i < self.toks.len() {
            match &self.toks[i].tok {
                Tok::LBrace => depth += 1,
                Tok::RBrace => {
                    depth -= 1;
                    if depth == 0 {
                        break;
                    }
                }
                Tok::Label(name) if depth == 1 => {
                    if first {
                        first = false;
                    }
                    st.block(name);
                    if let Ok(n) = name.parse::<u32>() {
                        // Keep the implicit numbering in sync for later
                        // unnamed blocks (only the entry may be unnamed).
                        let _ = n;
                    }
                }
                Tok::Eof => break,
                _ => {}
            }
            if first && depth == 1 && i == self.pos && !matches!(self.toks[i].tok, Tok::Label(_)) {
                // Unlabeled entry block: it takes the next unnamed number.
                let name = st.next_unnamed.to_string();
                st.block(&name);
                first = false;
            }
            i += 1;
        }
        Ok(())
    }

    fn parse_body(&mut self, st: &mut FnState, func: &mut Function) -> PResult<()> {
        self.prescan_labels(st)?;
        let mut blocks: Vec<Option<Block>> = (0..st.block_names.len()).map(|_| None).collect();
        let mut first = true;
        loop {
            if self.eat(&Tok::RBrace) {
                break;
            }
            let name = match self.peek().clone() {
                Tok::Label(name) => {
                    self.next();
                    if let Ok(n) = name.parse::<u32>() {
                        st.next_unnamed = n + 1;
                    }
                    name
                }
                _ if first => {
                    let n = st.next_unnamed.to_string();
                    st.next_unnamed += 1;
                    n
                }
                t => return self.err(format!("expected a block label, found `{t}`")),
            };
            first = false;
            let id = st.block(&name);
            let mut insts = Vec::new();
            loop {
                match self.peek() {
                    Tok::Label(_) | Tok::RBrace => break,
                    Tok::Eof => return self.err("unexpected end of file in function body"),
                    _ => {}
                }
                let inst = self.parse_instruction(st, id, insts.len())?;
                insts.push(inst);
            }
            if insts.is_empty() {
                return self.err(format!("basic block `{name}` is empty"));
            }
            if blocks[id.0 as usize].is_some() {
                return self.err(format!("basic block `{name}` defined twice"));
            }
            blocks[id.0 as usize] = Some(Block { name, insts });
        }
        for (i, b) in blocks.into_iter().enumerate() {
            match b {
                Some(b) => func.blocks.push(b),
                None => {
                    return Err(ParseError {
                        line: func.line,
                        message: format!(
                            "basic block `{}` is referenced but never defined",
                            st.block_names[i]
                        ),
                    })
                }
            }
        }
        for v in &st.values {
            if v.def == ValueDef::Undefined {
                return Err(ParseError {
                    line: func.line,
                    message: format!(
                        "value %{} is used but never defined in @{}",
                        v.name, func.name
                    ),
                });
            }
        }
        Ok(())
    }

    // -- instructions ----------------------------------------------------

    fn parse_instruction(
        &mut self,
        st: &mut FnState,
        block: BlockId,
        index: usize,
    ) -> PResult<Instruction> {
        let line = self.line();
        let result_name = if matches!(self.peek(), Tok::Local(_)) && *self.peek_at(1) == Tok::Equal
        {
            let n = self.expect_local()?;
            self.expect(Tok::Equal)?;
            Some(n)
        } else {
            None
        };
        let opcode = self.expect_any_ident()?;
        let (ty, kind) = self.parse_inst_body(st, &opcode, result_name.is_some())?;
        // Trailing metadata (`, !dbg !1`) and, for unsupported instructions,
        // whatever else remained on the line.
        if matches!(kind, InstKind::Unsupported(_)) {
            self.skip_to_next_line();
        } else {
            self.skip_metadata_attachments()?;
        }
        let result = match result_name {
            Some(name) => {
                if let Ok(n) = name.parse::<u32>() {
                    st.next_unnamed = n + 1;
                }
                let id = st.value(&name);
                if st.values[id.0 as usize].def != ValueDef::Undefined {
                    return Err(ParseError {
                        line,
                        message: format!("value %{name} defined twice"),
                    });
                }
                st.values[id.0 as usize].def = ValueDef::Inst(block, index);
                Some(id)
            }
            None => None,
        };
        Ok(Instruction {
            result,
            ty,
            kind,
            line,
        })
    }

    fn parse_label_ref(&mut self, st: &mut FnState) -> PResult<BlockId> {
        self.expect_ident("label")?;
        let n = self.expect_local()?;
        Ok(st.block(&n))
    }

    fn skip_fast_math_flags(&mut self) {
        while let Some(s) = self.peek_ident() {
            match s {
                "fast" | "nnan" | "ninf" | "nsz" | "arcp" | "contract" | "afn" | "reassoc" => {
                    self.next();
                }
                _ => break,
            }
        }
    }

    fn parse_inst_body(
        &mut self,
        st: &mut FnState,
        opcode: &str,
        has_result: bool,
    ) -> PResult<(Type, InstKind)> {
        if let Some(op) = BinOp::from_name(opcode) {
            let mut flags = BinFlags::default();
            if op.is_float() {
                self.skip_fast_math_flags();
            }
            loop {
                if self.eat_ident("nsw") {
                    flags.nsw = true;
                } else if self.eat_ident("nuw") {
                    flags.nuw = true;
                } else if self.eat_ident("exact") {
                    flags.exact = true;
                } else if self.eat_ident("disjoint") {
                    flags.disjoint = true;
                } else {
                    break;
                }
            }
            let ty = self.parse_type()?;
            let lhs = self.parse_value_in(st, &ty)?;
            self.expect(Tok::Comma)?;
            let rhs = self.parse_value_in(st, &ty)?;
            return Ok((
                ty,
                InstKind::Binary {
                    op,
                    flags,
                    lhs,
                    rhs,
                },
            ));
        }
        if let Some(op) = CastOp::from_name(opcode) {
            // `zext nneg`, `trunc nuw nsw` (LLVM 19+)
            while matches!(self.peek_ident(), Some("nneg" | "nuw" | "nsw")) {
                self.next();
            }
            let from = self.parse_type()?;
            let val = self.parse_value_in(st, &from)?;
            self.expect_ident("to")?;
            let to = self.parse_type()?;
            return Ok((to, InstKind::Cast { op, val, from }));
        }
        match opcode {
            "ret" => {
                let ty = self.parse_type()?;
                if ty == Type::Void {
                    return Ok((Type::Void, InstKind::Ret(None)));
                }
                let op = self.parse_value_in(st, &ty)?;
                Ok((Type::Void, InstKind::Ret(Some(TypedOperand { ty, op }))))
            }
            "br" => {
                if self.eat_ident("label") {
                    let n = self.expect_local()?;
                    return Ok((Type::Void, InstKind::Br(st.block(&n))));
                }
                let ty = self.parse_type()?;
                let cond = self.parse_value_in(st, &ty)?;
                self.expect(Tok::Comma)?;
                let t = self.parse_label_ref(st)?;
                self.expect(Tok::Comma)?;
                let f = self.parse_label_ref(st)?;
                Ok((Type::Void, InstKind::CondBr { cond, t, f }))
            }
            "switch" => {
                let ty = self.parse_type()?;
                let op = self.parse_value_in(st, &ty)?;
                self.expect(Tok::Comma)?;
                let default = self.parse_label_ref(st)?;
                self.expect(Tok::LBracket)?;
                let mut cases = Vec::new();
                while !self.eat(&Tok::RBracket) {
                    let cty = self.parse_type()?;
                    let cval = self.parse_value_in(st, &cty)?;
                    let value = match cval {
                        Operand::Const(Constant {
                            kind: ConstKind::Int(v),
                            ..
                        }) => v,
                        _ => return self.err("switch case values must be integer constants"),
                    };
                    self.expect(Tok::Comma)?;
                    let target = self.parse_label_ref(st)?;
                    cases.push(SwitchCase { value, target });
                }
                Ok((
                    Type::Void,
                    InstKind::Switch {
                        val: TypedOperand { ty, op },
                        default,
                        cases,
                    },
                ))
            }
            "unreachable" => Ok((Type::Void, InstKind::Unreachable)),
            "icmp" => {
                let pred_name = self.expect_any_ident()?;
                let pred = IPred::from_name(&pred_name).ok_or_else(|| ParseError {
                    line: self.line(),
                    message: format!("unknown icmp predicate `{pred_name}`"),
                })?;
                let ty = self.parse_type()?;
                let lhs = self.parse_value_in(st, &ty)?;
                self.expect(Tok::Comma)?;
                let rhs = self.parse_value_in(st, &ty)?;
                let result_ty = match &ty {
                    Type::Vector { len, scalable, .. } => Type::Vector {
                        len: *len,
                        elem: Rc::new(Type::Int(1)),
                        scalable: *scalable,
                    },
                    _ => Type::Int(1),
                };
                Ok((result_ty, InstKind::ICmp { pred, ty, lhs, rhs }))
            }
            "fcmp" => {
                self.skip_fast_math_flags();
                self.expect_any_ident()?;
                let ty = self.parse_type()?;
                self.parse_value_in(st, &ty)?;
                self.expect(Tok::Comma)?;
                self.parse_value_in(st, &ty)?;
                Ok((Type::Int(1), InstKind::FCmp))
            }
            "select" => {
                self.skip_fast_math_flags();
                let cty = self.parse_type()?;
                let cond = self.parse_value_in(st, &cty)?;
                self.expect(Tok::Comma)?;
                let ty = self.parse_type()?;
                let t = self.parse_value_in(st, &ty)?;
                self.expect(Tok::Comma)?;
                let fty = self.parse_type()?;
                let f = self.parse_value_in(st, &fty)?;
                Ok((
                    ty,
                    InstKind::Select {
                        cond: TypedOperand { ty: cty, op: cond },
                        t,
                        f,
                    },
                ))
            }
            "phi" => {
                self.skip_fast_math_flags();
                let ty = self.parse_type()?;
                let mut incoming = Vec::new();
                loop {
                    self.expect(Tok::LBracket)?;
                    let v = self.parse_value_in(st, &ty)?;
                    self.expect(Tok::Comma)?;
                    let b = self.expect_local()?;
                    let bid = st.block(&b);
                    self.expect(Tok::RBracket)?;
                    incoming.push((v, bid));
                    if !self.eat(&Tok::Comma) {
                        break;
                    }
                }
                Ok((ty, InstKind::Phi { incoming }))
            }
            "load" => {
                let atomic = self.eat_ident("atomic");
                let volatile = self.eat_ident("volatile");
                let ty = self.parse_type()?;
                self.expect(Tok::Comma)?;
                let pty = self.parse_type()?;
                let ptr = self.parse_value_in(st, &pty)?;
                if atomic {
                    self.skip_atomic_ordering()?;
                }
                let align = self.parse_align_clause()?;
                Ok((
                    ty,
                    InstKind::Load {
                        ptr,
                        align,
                        volatile,
                        atomic,
                    },
                ))
            }
            "store" => {
                let atomic = self.eat_ident("atomic");
                let volatile = self.eat_ident("volatile");
                let vty = self.parse_type()?;
                let val = self.parse_value_in(st, &vty)?;
                self.expect(Tok::Comma)?;
                let pty = self.parse_type()?;
                let ptr = self.parse_value_in(st, &pty)?;
                if atomic {
                    self.skip_atomic_ordering()?;
                }
                let align = self.parse_align_clause()?;
                Ok((
                    Type::Void,
                    InstKind::Store {
                        val: TypedOperand { ty: vty, op: val },
                        ptr,
                        align,
                        volatile,
                        atomic,
                    },
                ))
            }
            "alloca" => {
                let _inalloca = self.eat_ident("inalloca");
                let alloc_ty = self.parse_type()?;
                let mut count = None;
                let mut align = None;
                while *self.peek() == Tok::Comma {
                    // A comma followed by metadata belongs to the attachment
                    // skipper.
                    if matches!(self.peek_at(1), Tok::MetaRef(_)) {
                        break;
                    }
                    self.next();
                    if self.eat_ident("align") {
                        align = Some(self.expect_int()? as u64);
                    } else if self.eat_ident("addrspace") {
                        self.skip_balanced()?;
                    } else {
                        let cty = self.parse_type()?;
                        let cval = self.parse_value_in(st, &cty)?;
                        count = Some(TypedOperand { ty: cty, op: cval });
                    }
                }
                Ok((
                    Type::Ptr(0),
                    InstKind::Alloca {
                        alloc_ty,
                        count,
                        align,
                    },
                ))
            }
            "getelementptr" => {
                let mut inbounds = false;
                loop {
                    if self.eat_ident("inbounds") {
                        inbounds = true;
                    } else if self.eat_ident("nuw") || self.eat_ident("nusw") {
                    } else if self.eat_ident("inrange") {
                        self.skip_balanced()?;
                    } else {
                        break;
                    }
                }
                let base_ty = self.parse_type()?;
                self.expect(Tok::Comma)?;
                let pty = self.parse_type()?;
                let ptr = self.parse_value_in(st, &pty)?;
                let mut indices = Vec::new();
                while *self.peek() == Tok::Comma && !matches!(self.peek_at(1), Tok::MetaRef(_)) {
                    self.next();
                    let ity = self.parse_type()?;
                    let iv = self.parse_value_in(st, &ity)?;
                    indices.push(TypedOperand { ty: ity, op: iv });
                }
                Ok((
                    pty,
                    InstKind::Gep {
                        base_ty,
                        ptr,
                        indices,
                        inbounds,
                    },
                ))
            }
            "tail" | "musttail" | "notail" => {
                self.expect_ident("call")?;
                self.parse_call(st, true)
            }
            "call" => self.parse_call(st, false),
            "extractvalue" => {
                let ty = self.parse_type()?;
                let agg = self.parse_value_in(st, &ty)?;
                let mut indices = Vec::new();
                while *self.peek() == Tok::Comma && matches!(self.peek_at(1), Tok::Int(_)) {
                    self.next();
                    indices.push(self.expect_int()? as u32);
                }
                let rty = self.extract_value_type(&ty, &indices)?;
                Ok((
                    rty,
                    InstKind::ExtractValue {
                        agg: TypedOperand { ty, op: agg },
                        indices,
                    },
                ))
            }
            "insertvalue" => {
                let ty = self.parse_type()?;
                let agg = self.parse_value_in(st, &ty)?;
                self.expect(Tok::Comma)?;
                let vty = self.parse_type()?;
                let val = self.parse_value_in(st, &vty)?;
                let mut indices = Vec::new();
                while *self.peek() == Tok::Comma && matches!(self.peek_at(1), Tok::Int(_)) {
                    self.next();
                    indices.push(self.expect_int()? as u32);
                }
                Ok((
                    ty.clone(),
                    InstKind::InsertValue {
                        agg: TypedOperand { ty, op: agg },
                        val: TypedOperand { ty: vty, op: val },
                        indices,
                    },
                ))
            }
            "freeze" => {
                let ty = self.parse_type()?;
                let val = self.parse_value_in(st, &ty)?;
                Ok((ty, InstKind::Freeze { val }))
            }
            "fneg" | "extractelement" | "insertelement" | "shufflevector" | "va_arg"
            | "landingpad" | "cmpxchg" | "atomicrmw" | "fence" | "catchpad" | "cleanuppad"
            | "invoke" | "resume" | "indirectbr" | "callbr" | "catchswitch" | "catchret"
            | "cleanupret" => {
                // The instruction is skipped to the end of its line by the
                // caller; the result value (if any) still gets a definition
                // so that references to it parse.
                let _ = has_result;
                Ok((Type::Void, InstKind::Unsupported(opcode.to_string())))
            }
            _ => self.err(format!("unknown instruction `{opcode}`")),
        }
    }

    fn skip_atomic_ordering(&mut self) -> PResult<()> {
        if self.eat_ident("syncscope") {
            self.skip_balanced()?;
        }
        match self.peek_ident() {
            Some("unordered" | "monotonic" | "acquire" | "release" | "acq_rel" | "seq_cst") => {
                self.next();
                Ok(())
            }
            _ => self.err("expected an atomic ordering"),
        }
    }

    fn parse_align_clause(&mut self) -> PResult<Option<u64>> {
        let mut align = None;
        while *self.peek() == Tok::Comma && matches!(self.peek_at(1), Tok::Ident(_)) {
            self.next();
            let id = self.expect_any_ident()?;
            match id.as_str() {
                "align" => align = Some(self.expect_int()? as u64),
                _ => {
                    if *self.peek() == Tok::LParen {
                        self.skip_balanced()?;
                    }
                }
            }
        }
        Ok(align)
    }

    fn extract_value_type(&self, ty: &Type, indices: &[u32]) -> PResult<Type> {
        let mut cur = ty.clone();
        for &i in indices {
            let resolved = self
                .module
                .resolve(&cur)
                .cloned()
                .ok_or_else(|| ParseError {
                    line: self.line(),
                    message: format!("cannot index into opaque type {cur}"),
                })?;
            cur = match resolved {
                Type::Struct { fields, .. } => {
                    fields.get(i as usize).cloned().ok_or_else(|| ParseError {
                        line: self.line(),
                        message: format!("extractvalue index {i} out of range for {cur}"),
                    })?
                }
                Type::Array(_, elem) => (*elem).clone(),
                other => {
                    return self.err(format!("extractvalue on non-aggregate type {other}"));
                }
            };
        }
        Ok(cur)
    }

    fn parse_call(&mut self, st: &mut FnState, tail: bool) -> PResult<(Type, InstKind)> {
        self.skip_fast_math_flags();
        let mut ret_attrs = ParamAttrs::default();
        // Calling convention, return attributes, addrspace.
        loop {
            if self.starts_type() {
                break;
            }
            match self.peek().clone() {
                Tok::Ident(s) => {
                    self.next();
                    match s.as_str() {
                        "signext" => ret_attrs.signext = true,
                        "zeroext" => ret_attrs.zeroext = true,
                        _ => {}
                    }
                    if s == "cc" {
                        self.expect_int()?;
                    } else if *self.peek() == Tok::LParen {
                        self.skip_balanced()?;
                    } else if s == "align" && matches!(self.peek(), Tok::Int(_)) {
                        self.next();
                    }
                }
                Tok::Str(_) => {
                    self.next();
                    if self.eat(&Tok::Equal) {
                        self.next();
                    }
                }
                t => return self.err(format!("unexpected `{t}` in call")),
            }
        }
        let ty = self.parse_type()?;
        // Either the full function type or just the return type was given.
        let explicit_fn_ty = match &ty {
            Type::Func(ft) => Some(ft.clone()),
            _ => None,
        };
        let callee = match self.peek().clone() {
            Tok::Global(n) => {
                self.next();
                Callee::Global(n)
            }
            Tok::Local(n) => {
                self.next();
                Callee::Local(st.value(&n))
            }
            Tok::Ident(s) if s == "asm" => {
                self.next();
                while matches!(
                    self.peek_ident(),
                    Some("sideeffect" | "alignstack" | "inteldialect" | "unwind")
                ) {
                    self.next();
                }
                self.expect_str()?;
                self.expect(Tok::Comma)?;
                self.expect_str()?;
                Callee::InlineAsm
            }
            _ => {
                let c = self.parse_constant_value(&Type::Ptr(0))?;
                Callee::Const(c)
            }
        };
        self.expect(Tok::LParen)?;
        let mut args = Vec::new();
        if !self.eat(&Tok::RParen) {
            loop {
                let aty = self.parse_type()?;
                let attrs = self.parse_param_attrs()?;
                let op = if aty == Type::Metadata {
                    self.skip_metadata_value(Some(st))?;
                    Operand::Metadata
                } else {
                    self.parse_value_in(st, &aty)?
                };
                args.push(CallArg { ty: aty, attrs, op });
                if self.eat(&Tok::RParen) {
                    break;
                }
                self.expect(Tok::Comma)?;
            }
        }
        // Function attributes and operand bundles after the argument list.
        // They always sit on the same line as the closing parenthesis; the
        // line check keeps the loop from eating the next instruction.
        let call_line = self.toks[self.pos - 1].line;
        loop {
            if self.line() != call_line {
                break;
            }
            match self.peek().clone() {
                Tok::AttrGroup(_) => {
                    self.next();
                }
                Tok::Ident(s) => {
                    self.next();
                    if *self.peek() == Tok::LParen {
                        self.skip_balanced()?;
                    } else if s == "align" && matches!(self.peek(), Tok::Int(_)) {
                        self.next();
                    }
                }
                Tok::Str(_) => {
                    self.next();
                    if self.eat(&Tok::Equal) {
                        self.next();
                    }
                }
                Tok::LBracket => {
                    self.skip_balanced()?;
                }
                _ => break,
            }
        }
        let fn_ty = match explicit_fn_ty {
            Some(ft) => ft,
            None => Rc::new(FuncType {
                ret: Rc::new(ty.clone()),
                params: args.iter().map(|a| a.ty.clone()).collect(),
                varargs: false,
            }),
        };
        let ret_ty = (*fn_ty.ret).clone();
        Ok((
            ret_ty,
            InstKind::Call {
                callee,
                fn_ty,
                args,
                ret_attrs,
                tail,
            },
        ))
    }

    // -- values ----------------------------------------------------------

    /// Parses a value of the given type inside a function body.
    fn parse_value_in(&mut self, st: &mut FnState, ty: &Type) -> PResult<Operand> {
        match self.peek().clone() {
            Tok::Local(n) => {
                self.next();
                Ok(Operand::Local(st.value(&n)))
            }
            Tok::MetaRef(_) | Tok::Bang => {
                self.skip_metadata_value(Some(st))?;
                Ok(Operand::Metadata)
            }
            _ => Ok(Operand::Const(self.parse_constant_value(ty)?)),
        }
    }

    /// Parses `<type> <value>` inside a function body (or at top level when
    /// `st` is `None`, where locals are not allowed).
    fn parse_typed_value_in(&mut self, st: Option<&mut FnState>) -> PResult<TypedOperand> {
        let ty = self.parse_type()?;
        let op = match st {
            Some(st) => self.parse_value_in(st, &ty)?,
            None => Operand::Const(self.parse_constant_value(&ty)?),
        };
        Ok(TypedOperand { ty, op })
    }

    /// Parses a value of the given type at top level (constants only).
    fn parse_value(&mut self, ty: &Type) -> PResult<Constant> {
        self.parse_constant_value(ty)
    }

    fn parse_typed_constant(&mut self) -> PResult<Constant> {
        let ty = self.parse_type()?;
        self.parse_constant_value(&ty)
    }

    fn parse_constant_value(&mut self, ty: &Type) -> PResult<Constant> {
        let line = self.line();
        let kind = match self.next() {
            Tok::Int(v) => ConstKind::Int(v),
            Tok::Float(s) => ConstKind::Float(s),
            Tok::Global(n) => ConstKind::Global(n),
            Tok::CStr(s) => ConstKind::String(s),
            Tok::LBracket => {
                let mut elems = Vec::new();
                if !self.eat(&Tok::RBracket) {
                    loop {
                        elems.push(self.parse_typed_constant()?);
                        if self.eat(&Tok::RBracket) {
                            break;
                        }
                        self.expect(Tok::Comma)?;
                    }
                }
                ConstKind::Array(elems)
            }
            Tok::LBrace => {
                let mut elems = Vec::new();
                if !self.eat(&Tok::RBrace) {
                    loop {
                        elems.push(self.parse_typed_constant()?);
                        if self.eat(&Tok::RBrace) {
                            break;
                        }
                        self.expect(Tok::Comma)?;
                    }
                }
                ConstKind::Struct(elems)
            }
            Tok::Less => {
                let packed = self.eat(&Tok::LBrace);
                let close = if packed { Tok::RBrace } else { Tok::Greater };
                let mut elems = Vec::new();
                if !self.eat(&close) {
                    loop {
                        elems.push(self.parse_typed_constant()?);
                        if self.eat(&close) {
                            break;
                        }
                        self.expect(Tok::Comma)?;
                    }
                }
                if packed {
                    self.expect(Tok::Greater)?;
                    ConstKind::Struct(elems)
                } else {
                    ConstKind::Vector(elems)
                }
            }
            Tok::Ident(s) => match s.as_str() {
                "true" => ConstKind::Int(1),
                "false" => ConstKind::Int(0),
                "null" => ConstKind::Null,
                "undef" => ConstKind::Undef,
                "poison" => ConstKind::Poison,
                "zeroinitializer" => ConstKind::ZeroInit,
                "none" => ConstKind::None,
                "getelementptr" => {
                    let mut inbounds = false;
                    loop {
                        if self.eat_ident("inbounds") {
                            inbounds = true;
                        } else if self.eat_ident("nuw") || self.eat_ident("nusw") {
                        } else if self.eat_ident("inrange") {
                            self.skip_balanced()?;
                        } else {
                            break;
                        }
                    }
                    self.expect(Tok::LParen)?;
                    let base_ty = self.parse_type()?;
                    self.expect(Tok::Comma)?;
                    let base = self.parse_typed_constant()?;
                    let mut indices = Vec::new();
                    while self.eat(&Tok::Comma) {
                        indices.push(self.parse_typed_constant()?);
                    }
                    self.expect(Tok::RParen)?;
                    ConstKind::Gep {
                        base_ty,
                        base: Box::new(base),
                        indices,
                        inbounds,
                    }
                }
                "blockaddress" => {
                    self.expect(Tok::LParen)?;
                    let f = self.expect_global()?;
                    self.expect(Tok::Comma)?;
                    let b = self.expect_local()?;
                    self.expect(Tok::RParen)?;
                    ConstKind::BlockAddress(f, b)
                }
                "dso_local_equivalent" | "no_cfi" => {
                    let f = self.expect_global()?;
                    ConstKind::DsoLocalEquivalent(f)
                }
                "asm" => {
                    while matches!(
                        self.peek_ident(),
                        Some("sideeffect" | "alignstack" | "inteldialect" | "unwind")
                    ) {
                        self.next();
                    }
                    self.expect_str()?;
                    self.expect(Tok::Comma)?;
                    self.expect_str()?;
                    ConstKind::InlineAsm
                }
                "icmp" => {
                    let pred_name = self.expect_any_ident()?;
                    let pred = IPred::from_name(&pred_name).ok_or_else(|| ParseError {
                        line,
                        message: format!("unknown icmp predicate `{pred_name}`"),
                    })?;
                    self.expect(Tok::LParen)?;
                    let a = self.parse_typed_constant()?;
                    self.expect(Tok::Comma)?;
                    let b = self.parse_typed_constant()?;
                    self.expect(Tok::RParen)?;
                    ConstKind::ICmp(pred, Box::new(a), Box::new(b))
                }
                "select" => {
                    self.expect(Tok::LParen)?;
                    let c = self.parse_typed_constant()?;
                    self.expect(Tok::Comma)?;
                    let a = self.parse_typed_constant()?;
                    self.expect(Tok::Comma)?;
                    let b = self.parse_typed_constant()?;
                    self.expect(Tok::RParen)?;
                    ConstKind::Select(Box::new(c), Box::new(a), Box::new(b))
                }
                "splat" => {
                    self.expect(Tok::LParen)?;
                    let v = self.parse_typed_constant()?;
                    self.expect(Tok::RParen)?;
                    ConstKind::Vector(vec![v])
                }
                _ => {
                    if let Some(op) = CastOp::from_name(&s) {
                        while matches!(self.peek_ident(), Some("nneg" | "nuw" | "nsw")) {
                            self.next();
                        }
                        self.expect(Tok::LParen)?;
                        let v = self.parse_typed_constant()?;
                        self.expect_ident("to")?;
                        let _to = self.parse_type()?;
                        self.expect(Tok::RParen)?;
                        ConstKind::Cast(op, Box::new(v))
                    } else if let Some(op) = BinOp::from_name(&s) {
                        while matches!(
                            self.peek_ident(),
                            Some("nsw" | "nuw" | "exact" | "disjoint")
                        ) {
                            self.next();
                        }
                        self.expect(Tok::LParen)?;
                        let a = self.parse_typed_constant()?;
                        self.expect(Tok::Comma)?;
                        let b = self.parse_typed_constant()?;
                        self.expect(Tok::RParen)?;
                        ConstKind::Binary(op, Box::new(a), Box::new(b))
                    } else {
                        self.pos -= 1;
                        return self.err(format!("expected a constant of type {ty}, found `{s}`"));
                    }
                }
            },
            t => {
                self.pos -= 1;
                return self.err(format!("expected a constant of type {ty}, found `{t}`"));
            }
        };
        Ok(Constant {
            ty: ty.clone(),
            kind,
        })
    }
}

#[derive(Default)]
struct GlobalMods {
    thread_local: bool,
    externally_initialized: bool,
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn parses_small_module() {
        let src = r#"
; ModuleID = 't.c'
source_filename = "t.c"
target datalayout = "e-m:e-p:32:32-i64:64-n32-S128"
target triple = "riscv32-unknown-none-elf"

%struct.P = type { i32, i32 }

@.str = private unnamed_addr constant [6 x i8] c"hello\00", align 1
@msg = dso_local local_unnamed_addr global ptr @.str, align 4
@origin = dso_local global %struct.P { i32 3, i32 4 }, align 4
@ext = external global i32, align 4
@counter = dso_local global i32 0, align 4

; Function Attrs: nounwind
define dso_local i32 @main(i32 noundef %0, ptr nocapture noundef readnone %1) local_unnamed_addr #0 {
  %3 = and i32 %0, 7
  %4 = getelementptr inbounds [8 x i32], ptr @counter, i32 0, i32 %3
  %5 = load i32, ptr %4, align 4, !tbaa !4
  switch i32 %0, label %8 [
    i32 0, label %6
    i32 1, label %7
  ]

6:                                                ; preds = %2
  br label %8

7:
  br label %8

8:                                                ; preds = %2, %6, %7
  %.0 = phi i32 [ 0, %8 ], [ 1, %6 ], [ 10, %2 ], [ 3, %7 ]
  %9 = tail call fastcc i32 @fib(i32 noundef %.0) #2
  %10 = call { i32, i1 } @llvm.sadd.with.overflow.i32(i32 %9, i32 %5)
  %11 = extractvalue { i32, i1 } %10, 1
  %12 = select i1 %11, i32 -1, i32 %9
  store i32 %12, ptr @counter, align 4, !tbaa !4
  call void @llvm.dbg.value(metadata i32 %12, metadata !5, metadata !DIExpression())
  ret i32 %12
}

define internal fastcc i32 @fib(i32 noundef %0) unnamed_addr #1 {
  %2 = icmp slt i32 %0, 2
  br i1 %2, label %exit, label %rec

rec:
  %3 = add nsw i32 %0, -1
  %4 = tail call fastcc i32 @fib(i32 noundef %3)
  br label %exit

exit:
  %r = phi i32 [ %0, %1 ], [ %4, %rec ]
  ret i32 %r
}

declare { i32, i1 } @llvm.sadd.with.overflow.i32(i32, i32) #3
declare void @llvm.dbg.value(metadata, metadata, metadata) #3

attributes #0 = { nounwind "target-cpu"="generic-rv32" }
attributes #1 = { nofree nosync nounwind memory(none) }
attributes #2 = { nounwind }
attributes #3 = { nocallback nofree nosync nounwind speculatable willreturn memory(none) }

!llvm.module.flags = !{!0, !1}
!0 = !{i32 1, !"wchar_size", i32 4}
!1 = !{i32 1, !"target-abi", !"ilp32"}
!4 = !{!5, !5, i64 0}
!5 = !{!"int", !6, i64 0}
!6 = !{!"omnipotent char", !7, i64 0}
!7 = !{!"Simple C/C++ TBAA"}
"#;
        let m = parse_module(src).unwrap();
        assert_eq!(
            m.datalayout.as_deref(),
            Some("e-m:e-p:32:32-i64:64-n32-S128")
        );
        assert_eq!(m.globals.len(), 5);
        assert_eq!(m.globals[0].name, ".str");
        assert!(m.globals[0].is_const);
        assert_eq!(m.globals[0].linkage, Linkage::Private);
        assert_eq!(m.globals[3].init, None);
        assert_eq!(m.functions.len(), 4);
        let main = &m.functions[0];
        assert_eq!(main.name, "main");
        assert_eq!(main.params.len(), 2);
        assert_eq!(main.blocks.len(), 4);
        assert_eq!(main.blocks[0].name, "2");
        assert_eq!(main.blocks[1].name, "6");
        assert_eq!(main.blocks[3].name, "8");
        assert!(matches!(
            main.blocks[0].insts.last().unwrap().kind,
            InstKind::Switch { .. }
        ));
        let phi = &main.blocks[3].insts[0];
        assert!(matches!(phi.kind, InstKind::Phi { .. }));
        assert_eq!(main.value_name(phi.result.unwrap()), ".0");
        assert_eq!(main.attr_groups, vec![0]);
        let fib = &m.functions[1];
        assert_eq!(fib.linkage, Linkage::Internal);
        assert_eq!(fib.blocks[0].name, "1");
        assert_eq!(fib.blocks[2].name, "exit");
        let decl = &m.functions[2];
        assert!(decl.is_declaration);
        assert_eq!(
            decl.ret_ty,
            Type::Struct {
                fields: vec![Type::Int(32), Type::Int(1)],
                packed: false
            }
        );
        assert_eq!(
            m.attr_groups[&1],
            vec!["nofree", "nosync", "nounwind", "memory"]
        );
    }

    #[test]
    fn parses_constant_expressions_and_aggregates() {
        let src = r#"
%struct.S = type { i8, [3 x i8], i32 }
@a = global [2 x ptr] [ptr @b, ptr getelementptr (i8, ptr @b, i32 4)], align 4
@b = global %struct.S { i8 1, [3 x i8] zeroinitializer, i32 ptrtoint (ptr @a to i32) }
@c = constant <{ i8, i32 }> <{ i8 1, i32 2 }>
@d = alias i32, ptr @b
@e = external hidden global i16
"#;
        let m = parse_module(src).unwrap();
        assert_eq!(m.globals.len(), 4);
        match &m.globals[0].init.as_ref().unwrap().kind {
            ConstKind::Array(elems) => {
                assert_eq!(elems.len(), 2);
                assert!(matches!(elems[1].kind, ConstKind::Gep { .. }));
            }
            other => panic!("unexpected {other:?}"),
        }
        assert_eq!(m.aliases.len(), 1);
        assert_eq!(m.globals[3].linkage, Linkage::External);
    }

    #[test]
    fn unsupported_instructions_are_recorded() {
        let src = r#"
define float @f(float %a, float %b) {
  %r = fadd fast float %a, %b
  %x = atomicrmw add ptr null, i32 1 seq_cst, align 4
  ret float %r
}
"#;
        let m = parse_module(src).unwrap();
        let f = &m.functions[0];
        assert!(matches!(
            f.blocks[0].insts[0].kind,
            InstKind::Binary {
                op: BinOp::FAdd,
                ..
            }
        ));
        assert!(
            matches!(&f.blocks[0].insts[1].kind, InstKind::Unsupported(op) if op == "atomicrmw")
        );
    }
}
