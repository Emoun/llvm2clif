//! Translation of one LLVM function into a Cranelift function.

use super::data::{alias_target, gep_const_offset};
use super::types::{
    container, is_native, mask, sign_extend_const, truncate_const, truncate_const128, Layout,
    ScalarTy,
};
use super::{ModuleCtx, TResult, TransError};
use crate::llvm::*;
use cranelift_codegen::ir::condcodes::IntCC;
use cranelift_codegen::ir::immediates::{Imm64, Offset32};
use cranelift_codegen::ir::{
    self, types, Block, BlockArg, ExtFuncData, ExternalName, FuncRef, GlobalValue, GlobalValueData,
    InstBuilder, JumpTableData, MemFlagsData, SigRef, Signature, StackSlot, StackSlotData,
    StackSlotKind, TrapCode, UserFuncName, Value,
};
use cranelift_frontend::{FunctionBuilder, FunctionBuilderContext};
use std::collections::{BTreeMap, BTreeSet, HashMap};

/// Constant-size memory operations up to this many bytes are expanded
/// inline; larger ones call the runtime library.
const INLINE_MEMOP_LIMIT: u64 = 64;

/// Translates a function definition.
pub(super) fn translate_function(
    ctx: &ModuleCtx,
    f: &Function,
    runtime_imports: &mut BTreeMap<String, Signature>,
    helpers: &mut BTreeSet<&'static str>,
) -> Result<ir::Function, (u32, TransError)> {
    let sig = ctx
        .declared_signature(f)
        .map_err(|e| (f.line, TransError::unsupported(e)))?;
    let mut func =
        ir::Function::with_name_signature(UserFuncName::testcase(f.name.as_bytes()), sig);
    let mut fbc = FunctionBuilderContext::new();
    let line;
    {
        let b = FunctionBuilder::new(&mut func, &mut fbc);
        let mut t = FuncTranslator {
            ctx,
            llf: f,
            b,
            blocks: Vec::new(),
            vals: HashMap::new(),
            allocas: HashMap::new(),
            funcrefs: HashMap::new(),
            sigrefs: HashMap::new(),
            gvs: HashMap::new(),
            const_vals: HashMap::new(),
            runtime_imports,
            helpers,
            cur_block: BlockId(0),
            cur_line: f.line,
        };
        let r = t.run();
        line = t.cur_line;
        if let Err(e) = r {
            return Err((line, e));
        }
        t.b.finalize(ctx.isa.frontend_config());
    }
    if ctx.options.verify {
        verify(ctx, &func).map_err(|e| (line, e))?;
    }
    Ok(func)
}

/// Translates an alias of a function into a function that forwards its
/// arguments to the target and returns its results.
pub(super) fn translate_alias(
    ctx: &ModuleCtx,
    alias: &Alias,
    target: &str,
) -> TResult<ir::Function> {
    let sig = function_signature_by_name(ctx, target)?;
    let mut func = ir::Function::with_name_signature(
        UserFuncName::testcase(alias.name.as_bytes()),
        sig.clone(),
    );
    let mut fbc = FunctionBuilderContext::new();
    {
        let mut b = FunctionBuilder::new(&mut func, &mut fbc);
        let entry = b.create_block();
        b.append_block_params_for_function_params(entry);
        b.switch_to_block(entry);
        let args = b.block_params(entry).to_vec();
        let sigref = b.import_signature(sig);
        let fref = b.import_function(ExtFuncData {
            name: ExternalName::testcase(target.as_bytes()),
            signature: sigref,
            colocated: false,
            patchable: false,
        });
        let call = b.ins().call(fref, &args);
        let rets = b.inst_results(call).to_vec();
        b.ins().return_(&rets);
        b.seal_all_blocks();
        b.finalize(ctx.isa.frontend_config());
    }
    if ctx.options.verify {
        verify(ctx, &func)?;
    }
    Ok(func)
}

pub(super) fn verify(ctx: &ModuleCtx, func: &ir::Function) -> TResult<()> {
    cranelift_codegen::verify_function(func, &*ctx.isa).map_err(|errs| {
        TransError::invalid(format!(
            "internal error: the generated Cranelift IR failed verification:\n{errs}\n{}",
            func.display()
        ))
    })
}

/// The Cranelift signature of a function or function alias by name.
fn function_signature_by_name(ctx: &ModuleCtx, name: &str) -> TResult<Signature> {
    let mut cur = name;
    for _ in 0..16 {
        if let Some(f) = ctx.functions.get(cur) {
            return ctx.declared_signature(f).map_err(TransError::unsupported);
        }
        match ctx.aliases.get(cur).and_then(|a| alias_target(a)) {
            Some(t) => cur = t,
            None => break,
        }
    }
    Err(TransError::invalid(format!("@{name} is not a function")))
}

pub(super) struct FuncTranslator<'a, 'm> {
    pub(super) ctx: &'a ModuleCtx<'m>,
    pub(super) llf: &'m Function,
    pub(super) b: FunctionBuilder<'a>,
    /// Cranelift block of each LLVM block (indexed by `BlockId`).
    blocks: Vec<Block>,
    /// Cranelift values of each LLVM SSA value (flattened).
    vals: HashMap<ValueId, Vec<Value>>,
    /// Stack slots of allocas; their address is materialized at each use.
    allocas: HashMap<ValueId, StackSlot>,
    funcrefs: HashMap<(String, Signature), FuncRef>,
    sigrefs: HashMap<Signature, SigRef>,
    gvs: HashMap<(String, i64), GlobalValue>,
    /// Known constant values (results of `iconst`), used to specialize the
    /// shifts of wide integers by constant amounts.
    const_vals: HashMap<Value, u64>,
    /// Runtime helper functions referenced by the translation.
    runtime_imports: &'a mut BTreeMap<String, Signature>,
    /// Helper functions generated into the module (see `wide`).
    pub(super) helpers: &'a mut BTreeSet<&'static str>,
    cur_block: BlockId,
    pub(super) cur_line: u32,
}

impl<'a, 'm> FuncTranslator<'a, 'm> {
    pub(super) fn layout(&self) -> &Layout<'m> {
        &self.ctx.layout
    }

    fn run(&mut self) -> TResult<()> {
        let order = reverse_postorder(self.llf);
        let mut reachable = vec![false; self.llf.blocks.len()];
        for &bi in &order {
            reachable[bi] = true;
        }
        // Every block gets a Cranelift block; unreachable ones are created
        // too (they may be branch targets of other unreachable blocks) but
        // are filled with a trap so that the function stays well-formed.
        for _ in 0..self.llf.blocks.len() {
            let blk = self.b.create_block();
            self.blocks.push(blk);
        }
        for (bi, r) in reachable.iter().enumerate() {
            if !*r {
                self.b.switch_to_block(self.blocks[bi]);
                self.b.ins().trap(TrapCode::unwrap_user(1));
            }
        }
        let entry = self.blocks[0];
        self.b.append_block_params_for_function_params(entry);

        // Bind the function parameters.
        let params = self.b.block_params(entry).to_vec();
        let mut idx = 0;
        for p in &self.llf.params {
            let n = self.layout().flat_count(&p.ty)?;
            self.vals.insert(p.value, params[idx..idx + n].to_vec());
            idx += n;
        }

        // Phis become block parameters.
        for (bi, blk) in self.llf.blocks.iter().enumerate() {
            for inst in &blk.insts {
                let InstKind::Phi { .. } = inst.kind else {
                    break;
                };
                self.cur_line = inst.line;
                let leaves = self.layout().flatten(&inst.ty)?;
                let mut vs = Vec::with_capacity(leaves.len());
                for leaf in leaves {
                    vs.push(self.b.append_block_param(self.blocks[bi], leaf.ty.clif()));
                }
                self.vals.insert(inst.result.expect("phi has a result"), vs);
            }
        }

        // Translate the blocks in reverse post-order so that every value is
        // defined before its uses are translated (dominators come first);
        // unreachable blocks are dropped.
        let order = reverse_postorder(self.llf);
        for bi in order {
            self.cur_block = BlockId(bi as u32);
            self.b.switch_to_block(self.blocks[bi]);
            let block = &self.llf.blocks[bi];
            let mut terminated = false;
            for inst in &block.insts {
                self.cur_line = inst.line;
                if terminated {
                    return Err(TransError::invalid(
                        "instruction after the block's terminator",
                    ));
                }
                self.translate_inst(inst)?;
                terminated = inst.is_terminator();
            }
            if !terminated {
                self.cur_line = block.insts.last().map(|i| i.line).unwrap_or(self.llf.line);
                return Err(TransError::invalid(format!(
                    "basic block `{}` does not end in a supported terminator",
                    block.name
                )));
            }
        }
        self.b.seal_all_blocks();
        Ok(())
    }

    // -- small emission helpers ------------------------------------------

    fn scalar_of(&self, ty: &Type) -> TResult<ScalarTy> {
        self.layout().scalar_of(ty).map_err(TransError::from)
    }

    pub(super) fn iconst(&mut self, ty: types::Type, v: u64) -> Value {
        let v = v & mask(ty.bits());
        let val = self.b.ins().iconst(ty, Imm64::new(v as i64));
        self.const_vals.insert(val, v);
        val
    }

    /// The value of `v` if it is a known constant.
    pub(super) fn known_const(&self, v: Value) -> Option<u64> {
        self.const_vals.get(&v).copied()
    }

    /// Masks a container value down to `bits` bits (no-op for native widths).
    pub(super) fn canon(&mut self, v: Value, bits: u32) -> Value {
        if is_native(bits) {
            return v;
        }
        let m = self.iconst(container(bits), mask(bits));
        self.b.ins().band(v, m)
    }

    /// Sign-extends the low `bits` bits of a container value across the whole
    /// container (no-op for native widths).
    pub(super) fn sext_in(&mut self, v: Value, bits: u32) -> Value {
        if is_native(bits) {
            return v;
        }
        if bits == 1 {
            // A boolean container holds 0 or 1; negating it is the sign
            // extension (0 or -1), without the signed shift.
            return self.b.ins().ineg(v);
        }
        let c = container(bits);
        let sh = self.iconst(c, (c.bits() - bits) as u64);
        let t = self.b.ins().ishl(v, sh);
        self.b.ins().sshr(t, sh)
    }

    /// Sign-extends a container value of `from_bits` bits to the container
    /// type `to` (truncating when `to` is narrower).
    pub(super) fn emit_sextend(&mut self, v: Value, from_bits: u32, to: types::Type) -> Value {
        if from_bits == 1 {
            // Widen the boolean first so that no signed operation is needed.
            let z = self.resize_unsigned(v, types::I8, to);
            return self.b.ins().ineg(z);
        }
        let s = self.sext_in(v, from_bits);
        let fc = container(from_bits);
        if fc == to {
            return s;
        }
        if fc.bits() > to.bits() {
            return self.b.ins().ireduce(to, s);
        }
        self.b.ins().sextend(to, s)
    }

    /// Converts a canonical `bits`-bit value to a sign-extended i32.
    pub(super) fn sext_to_i32(&mut self, v: Value, bits: u32) -> Value {
        self.emit_sextend(v, bits, types::I32)
    }

    /// Converts a canonical `bits`-bit value to a zero-extended (or
    /// truncated) i32.
    pub(super) fn zext_to_i32(&mut self, v: Value, bits: u32) -> Value {
        self.resize_unsigned(v, container(bits), types::I32)
    }

    /// Converts a value of Cranelift type `from` to `to` by zero extension or
    /// truncation.
    pub(super) fn resize_unsigned(
        &mut self,
        v: Value,
        from: types::Type,
        to: types::Type,
    ) -> Value {
        if from == to {
            v
        } else if to.bits() > from.bits() {
            self.b.ins().uextend(to, v)
        } else {
            self.b.ins().ireduce(to, v)
        }
    }

    pub(super) fn memflags(&self) -> MemFlagsData {
        MemFlagsData::new().with_notrap()
    }

    fn sig_ref(&mut self, sig: &Signature) -> SigRef {
        if let Some(s) = self.sigrefs.get(sig) {
            return *s;
        }
        let s = self.b.import_signature(sig.clone());
        self.sigrefs.insert(sig.clone(), s);
        s
    }

    pub(super) fn func_ref(&mut self, name: &str, sig: &Signature) -> FuncRef {
        let key = (name.to_string(), sig.clone());
        if let Some(f) = self.funcrefs.get(&key) {
            return *f;
        }
        let sigref = self.sig_ref(sig);
        let f = self.b.import_function(ExtFuncData {
            name: ExternalName::testcase(name.as_bytes()),
            signature: sigref,
            colocated: false,
            patchable: false,
        });
        self.funcrefs.insert(key, f);
        f
    }

    fn global_value(&mut self, name: &str, offset: i64) -> GlobalValue {
        let key = (name.to_string(), offset);
        if let Some(g) = self.gvs.get(&key) {
            return *g;
        }
        let g = self.b.create_global_value(GlobalValueData::Symbol {
            name: ExternalName::testcase(name.as_bytes()),
            offset: Imm64::new(offset),
            colocated: false,
            tls: false,
        });
        self.gvs.insert(key, g);
        g
    }

    /// The address of global symbol `name` plus `offset`.
    pub(super) fn symbol_addr(&mut self, name: &str, offset: i64) -> TResult<Value> {
        if self.ctx.is_function_symbol(name) {
            let sig = function_signature_by_name(self.ctx, name)?;
            let fr = self.func_ref(name, &sig);
            let mut v = self.b.ins().func_addr(types::I32, fr);
            if offset != 0 {
                let o = self.iconst(types::I32, offset as u64);
                v = self.b.ins().iadd(v, o);
            }
            Ok(v)
        } else if self.ctx.globals.contains_key(name) || self.ctx.aliases.contains_key(name) {
            let gv = self.global_value(name, offset);
            Ok(self.b.ins().symbol_value(types::I32, gv))
        } else {
            Err(TransError::invalid(format!(
                "reference to unknown global @{name}"
            )))
        }
    }

    /// Calls a runtime library function (declared as an external symbol).
    pub(super) fn call_runtime(
        &mut self,
        name: &str,
        params: &[types::Type],
        ret: Option<types::Type>,
        args: &[Value],
    ) -> Vec<Value> {
        let mut sig = Signature::new(self.ctx.isa.default_call_conv());
        for p in params {
            sig.params.push(ir::AbiParam::new(*p));
        }
        if let Some(r) = ret {
            sig.returns.push(ir::AbiParam::new(r));
        }
        self.runtime_imports
            .entry(name.to_string())
            .or_insert_with(|| sig.clone());
        let fr = self.func_ref(name, &sig);
        let call = self.b.ins().call(fr, args);
        self.b.inst_results(call).to_vec()
    }

    // -- operands -------------------------------------------------------

    /// The Cranelift values of an operand (materializing constants and
    /// stack addresses in the current block).
    pub(super) fn use_op(&mut self, op: &Operand) -> TResult<Vec<Value>> {
        match op {
            Operand::Local(id) => {
                if let Some(ss) = self.allocas.get(id) {
                    let ss = *ss;
                    return Ok(vec![self.b.ins().stack_addr(types::I32, ss, 0)]);
                }
                match self.vals.get(id) {
                    Some(v) => Ok(v.clone()),
                    None => Err(TransError::invalid(format!(
                        "use of %{} before its definition",
                        self.llf.value_name(*id)
                    ))),
                }
            }
            Operand::Const(c) => self.emit_constant(c),
            Operand::Metadata => Err(TransError::invalid(
                "metadata operand where a value was expected",
            )),
        }
    }

    pub(super) fn use_scalar(&mut self, op: &Operand) -> TResult<Value> {
        let v = self.use_op(op)?;
        if v.len() != 1 {
            return Err(TransError::invalid(format!(
                "expected a scalar value of at most 32 bits, found one of {} parts",
                v.len()
            )));
        }
        Ok(v[0])
    }

    fn define(&mut self, inst: &Instruction, vals: Vec<Value>) {
        if let Some(id) = inst.result {
            self.vals.insert(id, vals);
        }
    }

    /// Materializes a constant.
    pub(super) fn emit_constant(&mut self, c: &Constant) -> TResult<Vec<Value>> {
        match &c.kind {
            ConstKind::Int(v) => {
                let s = self.scalar_of(&c.ty)?;
                if s.is_wide() {
                    let p = self.wide_const(truncate_const128(*v, s.bits()), s.parts());
                    return Ok(self.wide_out(&p, s.bits()));
                }
                Ok(vec![self.iconst(s.clif(), truncate_const(*v, s.bits()))])
            }
            ConstKind::Null => Ok(vec![self.iconst(types::I32, 0)]),
            ConstKind::Undef | ConstKind::Poison | ConstKind::ZeroInit => {
                let leaves = self.layout().flatten(&c.ty)?;
                Ok(leaves.iter().map(|l| self.iconst(l.ty.clif(), 0)).collect())
            }
            ConstKind::Global(name) => Ok(vec![self.symbol_addr(name, 0)?]),
            ConstKind::Array(elems) | ConstKind::Struct(elems) => {
                let mut out = Vec::new();
                for e in elems {
                    out.extend(self.emit_constant(e)?);
                }
                Ok(out)
            }
            ConstKind::String(bytes) => Ok(bytes
                .iter()
                .map(|b| self.iconst(types::I8, *b as u64))
                .collect()),
            ConstKind::Vector(_) => Err(TransError::unsupported(
                "vector constants are not supported",
            )),
            ConstKind::Gep {
                base_ty,
                base,
                indices,
                ..
            } => {
                let offset = gep_const_offset(self.layout(), base_ty, indices)?;
                if let ConstKind::Global(name) = &base.kind {
                    return Ok(vec![self.symbol_addr(name, offset)?]);
                }
                let b = self.emit_constant(base)?;
                if b.len() != 1 {
                    return Err(TransError::invalid(
                        "getelementptr on a non-pointer constant",
                    ));
                }
                if offset == 0 {
                    return Ok(b);
                }
                let o = self.iconst(types::I32, offset as u64);
                Ok(vec![self.b.ins().iadd(b[0], o)])
            }
            ConstKind::Cast(op, inner) => {
                let v = self.emit_constant(inner)?;
                self.emit_cast(*op, &inner.ty, &c.ty, v)
            }
            ConstKind::Binary(op, a, b) => {
                let s = self.scalar_of(&c.ty)?;
                let l = self.emit_constant(a)?;
                let r = self.emit_constant(b)?;
                if s.is_wide() {
                    let lp = self.wide_in(&l, s.bits())?;
                    let rp = self.wide_in(&r, s.bits())?;
                    let p = self.emit_binop_wide(*op, s.bits(), &lp, &rp)?;
                    return Ok(self.wide_out(&p, s.bits()));
                }
                if l.len() != 1 || r.len() != 1 {
                    return Err(TransError::unsupported(
                        "vector constant expressions are not supported",
                    ));
                }
                Ok(vec![self.emit_binop(*op, s.bits(), l[0], r[0])?])
            }
            ConstKind::ICmp(pred, a, b) => {
                let s = self.scalar_of(&a.ty)?;
                let l = self.emit_constant(a)?;
                let r = self.emit_constant(b)?;
                if s.is_wide() {
                    let lp = self.wide_in(&l, s.bits())?;
                    let rp = self.wide_in(&r, s.bits())?;
                    return Ok(vec![self.wide_icmp(*pred, s.bits(), &lp, &rp)]);
                }
                if l.len() != 1 || r.len() != 1 {
                    return Err(TransError::unsupported(
                        "vector constant expressions are not supported",
                    ));
                }
                Ok(vec![self.emit_icmp(*pred, s.bits(), l[0], r[0])])
            }
            ConstKind::Select(cond, a, b) => {
                let cv = self.emit_constant(cond)?;
                let av = self.emit_constant(a)?;
                let bv = self.emit_constant(b)?;
                Ok(av
                    .iter()
                    .zip(bv.iter())
                    .map(|(x, y)| self.b.ins().select(cv[0], *x, *y))
                    .collect())
            }
            ConstKind::BlockAddress(..) => Err(TransError::unsupported(
                "`blockaddress` constants (computed goto) are not supported",
            )),
            ConstKind::DsoLocalEquivalent(f) => Ok(vec![self.symbol_addr(f, 0)?]),
            ConstKind::InlineAsm => {
                Err(TransError::unsupported("inline assembly is not supported"))
            }
            ConstKind::Float(_) => Err(TransError::unsupported(
                "floating point constants are not supported",
            )),
            ConstKind::None => Err(TransError::unsupported("token values are not supported")),
        }
    }

    // -- arithmetic -----------------------------------------------------

    pub(super) fn emit_binop(
        &mut self,
        op: BinOp,
        bits: u32,
        l: Value,
        r: Value,
    ) -> TResult<Value> {
        Ok(match op {
            BinOp::Add => {
                let v = self.b.ins().iadd(l, r);
                self.canon(v, bits)
            }
            BinOp::Sub => {
                let v = self.b.ins().isub(l, r);
                self.canon(v, bits)
            }
            BinOp::Mul => {
                let v = self.b.ins().imul(l, r);
                self.canon(v, bits)
            }
            BinOp::And => self.b.ins().band(l, r),
            BinOp::Or => self.b.ins().bor(l, r),
            BinOp::Xor => self.b.ins().bxor(l, r),
            BinOp::Shl => {
                let v = self.b.ins().ishl(l, r);
                self.canon(v, bits)
            }
            BinOp::LShr => self.b.ins().ushr(l, r),
            BinOp::AShr => {
                let s = self.sext_in(l, bits);
                let v = self.b.ins().sshr(s, r);
                self.canon(v, bits)
            }
            BinOp::UDiv => self.b.ins().udiv(l, r),
            BinOp::URem => self.b.ins().urem(l, r),
            BinOp::SDiv => {
                let (ls, rs) = (self.sext_in(l, bits), self.sext_in(r, bits));
                let v = self.b.ins().sdiv(ls, rs);
                self.canon(v, bits)
            }
            BinOp::SRem => {
                let (ls, rs) = (self.sext_in(l, bits), self.sext_in(r, bits));
                let v = self.b.ins().srem(ls, rs);
                self.canon(v, bits)
            }
            BinOp::FAdd | BinOp::FSub | BinOp::FMul | BinOp::FDiv | BinOp::FRem => {
                return Err(TransError::unsupported(format!(
                    "floating point instruction `{}` is not supported",
                    op.name()
                )))
            }
        })
    }

    pub(super) fn emit_icmp(&mut self, pred: IPred, bits: u32, l: Value, r: Value) -> Value {
        let (l, r) = if pred.is_signed() {
            (self.sext_in(l, bits), self.sext_in(r, bits))
        } else {
            (l, r)
        };
        let cc = match pred {
            IPred::Eq => IntCC::Equal,
            IPred::Ne => IntCC::NotEqual,
            IPred::Ugt => IntCC::UnsignedGreaterThan,
            IPred::Uge => IntCC::UnsignedGreaterThanOrEqual,
            IPred::Ult => IntCC::UnsignedLessThan,
            IPred::Ule => IntCC::UnsignedLessThanOrEqual,
            IPred::Sgt => IntCC::SignedGreaterThan,
            IPred::Sge => IntCC::SignedGreaterThanOrEqual,
            IPred::Slt => IntCC::SignedLessThan,
            IPred::Sle => IntCC::SignedLessThanOrEqual,
        };
        self.b.ins().icmp(cc, l, r)
    }

    pub(super) fn emit_cast(
        &mut self,
        op: CastOp,
        from: &Type,
        to: &Type,
        vals: Vec<Value>,
    ) -> TResult<Vec<Value>> {
        let fs = self.scalar_of(from)?;
        let ts = self.scalar_of(to)?;
        let (fb, tb) = (fs.bits(), ts.bits());
        match op {
            CastOp::FPTrunc
            | CastOp::FPExt
            | CastOp::FPToUI
            | CastOp::FPToSI
            | CastOp::UIToFP
            | CastOp::SIToFP => {
                return Err(TransError::unsupported(format!(
                    "floating point conversion `{}` is not supported",
                    op.name()
                )))
            }
            CastOp::BitCast | CastOp::AddrSpaceCast => {
                if fb != tb {
                    return Err(TransError::invalid(format!(
                        "bitcast between types of different sizes ({from} to {to})"
                    )));
                }
                return Ok(vals);
            }
            _ => {}
        }
        if fs.is_wide() || ts.is_wide() {
            return self.emit_cast_wide(op, fs, ts, vals);
        }
        if vals.len() != 1 {
            return Err(TransError::invalid("cast of a non-scalar value"));
        }
        let v = vals[0];
        let (fc, tc) = (container(fb), container(tb));
        let out = match op {
            CastOp::Trunc | CastOp::PtrToInt if tb < fb => {
                let r = if tc != fc {
                    self.b.ins().ireduce(tc, v)
                } else {
                    v
                };
                self.canon(r, tb)
            }
            CastOp::Trunc | CastOp::PtrToInt | CastOp::IntToPtr | CastOp::ZExt if tb >= fb => {
                if tc != fc {
                    self.b.ins().uextend(tc, v)
                } else {
                    v
                }
            }
            CastOp::IntToPtr | CastOp::ZExt => {
                // Narrowing conversions written as zext/inttoptr do not occur
                // in valid IR, but handle them as truncations.
                let r = if tc != fc {
                    self.b.ins().ireduce(tc, v)
                } else {
                    v
                };
                self.canon(r, tb)
            }
            CastOp::SExt => {
                let r = self.emit_sextend(v, fb, tc);
                self.canon(r, tb)
            }
            _ => unreachable!(),
        };
        Ok(vec![out])
    }

    /// A cast with a wide integer on at least one side.
    fn emit_cast_wide(
        &mut self,
        op: CastOp,
        fs: ScalarTy,
        ts: ScalarTy,
        vals: Vec<Value>,
    ) -> TResult<Vec<Value>> {
        let (fb, tb) = (fs.bits(), ts.bits());
        let sext = matches!(op, CastOp::SExt);
        if fs.is_wide() && ts.is_wide() {
            // Keep the low parts (`wide_out` re-masks the new top part) and
            // extend with zeros or the sign.
            let w = self.wide_in(&vals, fb)?;
            let w = if sext { self.wide_sext_in(&w, fb) } else { w };
            let kt = ts.parts();
            let mut out: Vec<Value> = w.iter().copied().take(kt).collect();
            if kt > w.len() {
                let fill = if sext {
                    let c31 = self.iconst(types::I32, 31);
                    self.b.ins().sshr(w[w.len() - 1], c31)
                } else {
                    self.iconst(types::I32, 0)
                };
                out.resize(kt, fill);
            }
            return Ok(self.wide_out(&out, tb));
        }
        if fs.is_wide() {
            // Wide to narrow (`trunc`, `inttoptr`): the lowest part.
            let w = self.wide_in(&vals, fb)?;
            let r = self.resize_unsigned(w[0], types::I32, container(tb));
            return Ok(vec![self.canon(r, tb)]);
        }
        // Narrow to wide (`zext`, `sext`, `ptrtoint`).
        if vals.len() != 1 {
            return Err(TransError::invalid("cast of a non-scalar value"));
        }
        let v = vals[0];
        let (lo, fill) = if sext {
            let lo = self.sext_to_i32(v, fb);
            let c31 = self.iconst(types::I32, 31);
            (lo, self.b.ins().sshr(lo, c31))
        } else {
            (self.zext_to_i32(v, fb), self.iconst(types::I32, 0))
        };
        let mut out = vec![lo];
        out.resize(ts.parts(), fill);
        Ok(self.wide_out(&out, tb))
    }

    // -- memory ---------------------------------------------------------

    /// Loads a scalar of the given type from `ptr + offset`, producing a
    /// canonical container value.
    pub(super) fn load_scalar(&mut self, sty: ScalarTy, ptr: Value, offset: i64) -> TResult<Value> {
        let flags = self.memflags();
        let off = mem_offset(offset)?;
        let bits = sty.bits();
        let bytes = sty.store_bytes();
        let v = match bytes {
            1 => self.b.ins().load(types::I8, flags, ptr, off),
            2 => self.b.ins().load(types::I16, flags, ptr, off),
            4 => self.b.ins().load(types::I32, flags, ptr, off),
            _ => {
                // Odd sizes (3, 5, 6, 7 bytes): assemble from power-of-two
                // sized loads, least significant part first.
                let c = sty.clif();
                let mut acc: Option<Value> = None;
                for (chunk_off, ty) in chunk_offsets(bytes as u64) {
                    let o = mem_offset(offset + chunk_off as i64)?;
                    let part = self.b.ins().load(ty, flags, ptr, o);
                    let part = self.resize_unsigned(part, ty, c);
                    let part = if chunk_off == 0 {
                        part
                    } else {
                        let sh = self.iconst(c, chunk_off * 8);
                        self.b.ins().ishl(part, sh)
                    };
                    acc = Some(match acc {
                        None => part,
                        Some(a) => self.b.ins().bor(a, part),
                    });
                }
                acc.expect("at least one chunk")
            }
        };
        Ok(self.canon(v, bits))
    }

    /// Stores a scalar (canonical container value) to `ptr + offset`.
    pub(super) fn store_scalar(
        &mut self,
        sty: ScalarTy,
        val: Value,
        ptr: Value,
        offset: i64,
    ) -> TResult<()> {
        let flags = self.memflags();
        let off = mem_offset(offset)?;
        let bytes = sty.store_bytes();
        match bytes {
            1 | 2 | 4 => {
                self.b.ins().store(flags, val, ptr, off);
            }
            _ => {
                let c = sty.clif();
                for (chunk_off, ty) in chunk_offsets(bytes as u64) {
                    let o = mem_offset(offset + chunk_off as i64)?;
                    let part = if chunk_off == 0 {
                        val
                    } else {
                        let sh = self.iconst(c, chunk_off * 8);
                        self.b.ins().ushr(val, sh)
                    };
                    let part = self.b.ins().ireduce(ty, part);
                    self.b.ins().store(flags, part, ptr, o);
                }
            }
        }
        Ok(())
    }

    /// Copies `size` bytes from `src` to `dst`. `overlap` selects `memmove`
    /// semantics.
    pub(super) fn emit_memcpy(
        &mut self,
        dst: Value,
        src: Value,
        size: u64,
        overlap: bool,
    ) -> TResult<()> {
        if size == 0 {
            return Ok(());
        }
        if size > INLINE_MEMOP_LIMIT {
            let n = self.iconst(types::I32, size);
            let name = if overlap { "memmove" } else { "memcpy" };
            self.call_runtime(
                name,
                &[types::I32, types::I32, types::I32],
                Some(types::I32),
                &[dst, src, n],
            );
            return Ok(());
        }
        let flags = self.memflags();
        let chunks = chunk_offsets(size);
        let mut loaded = Vec::with_capacity(chunks.len());
        for &(off, ty) in &chunks {
            let off = Offset32::new(off as i32);
            let v = self.b.ins().load(ty, flags, src, off);
            if overlap {
                loaded.push(v);
            } else {
                self.b.ins().store(flags, v, dst, off);
            }
        }
        if overlap {
            for (&(off, _), v) in chunks.iter().zip(loaded) {
                self.b.ins().store(flags, v, dst, Offset32::new(off as i32));
            }
        }
        Ok(())
    }

    /// Fills `size` bytes at `dst` with the byte `val` (an i8 value).
    pub(super) fn emit_memset(&mut self, dst: Value, val: Value, size: u64) -> TResult<()> {
        if size == 0 {
            return Ok(());
        }
        if size > INLINE_MEMOP_LIMIT {
            let n = self.iconst(types::I32, size);
            let v32 = self.b.ins().uextend(types::I32, val);
            self.call_runtime(
                "memset",
                &[types::I32, types::I32, types::I32],
                Some(types::I32),
                &[dst, v32, n],
            );
            return Ok(());
        }
        let flags = self.memflags();
        let v32 = self.b.ins().uextend(types::I32, val);
        let rep = self.iconst(types::I32, 0x0101_0101);
        let pat32 = self.b.ins().imul(v32, rep);
        let pat16 = self.b.ins().ireduce(types::I16, pat32);
        for (off, ty) in chunk_offsets(size) {
            let v = match ty {
                types::I32 => pat32,
                types::I16 => pat16,
                _ => val,
            };
            self.b.ins().store(flags, v, dst, Offset32::new(off as i32));
        }
        Ok(())
    }

    /// Allocates a stack slot for `ty` and copies the pointee of `src` into
    /// it (for `byval` arguments).
    fn byval_copy(&mut self, ty: &Type, src: Value, align: Option<u64>) -> TResult<Value> {
        let size = self.layout().alloc_size(ty)?;
        let align = align.unwrap_or(0).max(self.layout().abi_align(ty)?).max(1);
        let ss = self.b.create_sized_stack_slot(StackSlotData::new(
            StackSlotKind::ExplicitSlot,
            u32::try_from(size.max(1))
                .map_err(|_| TransError::invalid("byval argument too large"))?,
            align.trailing_zeros() as u8,
        ));
        let dst = self.b.ins().stack_addr(types::I32, ss, 0);
        self.emit_memcpy(dst, src, size, false)?;
        Ok(dst)
    }

    // -- control flow ---------------------------------------------------

    /// The block arguments to pass along the edge from the current block to
    /// `target`: the incoming values of the target's phis.
    fn edge_args(&mut self, target: BlockId) -> TResult<Vec<BlockArg>> {
        let mut args = Vec::new();
        let target_block = &self.llf.blocks[target.0 as usize];
        for inst in &target_block.insts {
            let InstKind::Phi { incoming } = &inst.kind else {
                break;
            };
            let from = self.cur_block;
            let (op, _) = incoming.iter().find(|(_, b)| *b == from).ok_or_else(|| {
                TransError::invalid(format!(
                    "phi in block `{}` has no incoming value from block `{}`",
                    target_block.name, self.llf.blocks[from.0 as usize].name
                ))
            })?;
            for v in self.use_op(op)? {
                args.push(BlockArg::Value(v));
            }
        }
        Ok(args)
    }

    fn translate_switch(
        &mut self,
        val: &TypedOperand,
        default: BlockId,
        cases: &[SwitchCase],
    ) -> TResult<()> {
        let sty = self.scalar_of(&val.ty)?;
        let bits = sty.bits();
        if sty.is_wide() {
            return self.translate_switch_wide(val, default, cases);
        }
        let v = self.use_scalar(&val.op)?;
        let default_args = self.edge_args(default)?;
        if cases.is_empty() {
            self.b
                .ins()
                .jump(self.blocks[default.0 as usize], &default_args);
            return Ok(());
        }
        // Case values as unsigned bit patterns of the switch's width.
        let mut keys: Vec<(u64, BlockId)> = cases
            .iter()
            .map(|c| (truncate_const(c.value, bits), c.target))
            .collect();
        keys.sort_by_key(|k| k.0);
        let min = keys[0].0;
        let max = keys[keys.len() - 1].0;
        let range = max - min + 1;
        let n = keys.len() as u64;

        if range <= 2 * n + 4 && range <= 4096 {
            // Dense: jump table indexed by (v - min).
            let v32 = self.zext_to_i32(v, bits);
            let idx = if min == 0 {
                v32
            } else {
                let m = self.iconst(types::I32, min);
                self.b.ins().isub(v32, m)
            };
            let mut target_calls: HashMap<BlockId, ir::BlockCall> = HashMap::new();
            let default_call = self
                .b
                .func
                .dfg
                .block_call(self.blocks[default.0 as usize], &default_args);
            target_calls.insert(default, default_call);
            let mut table = Vec::with_capacity(range as usize);
            let mut ki = 0;
            for k in 0..range {
                let key = min + k;
                let target = if ki < keys.len() && keys[ki].0 == key {
                    let t = keys[ki].1;
                    ki += 1;
                    t
                } else {
                    default
                };
                let call = match target_calls.get(&target) {
                    Some(c) => *c,
                    None => {
                        let args = self.edge_args(target)?;
                        let c = self
                            .b
                            .func
                            .dfg
                            .block_call(self.blocks[target.0 as usize], &args);
                        target_calls.insert(target, c);
                        c
                    }
                };
                table.push(call);
            }
            let jt = self
                .b
                .create_jump_table(JumpTableData::new(default_call, &table));
            self.b.ins().br_table(idx, jt);
            return Ok(());
        }

        // Sparse: a chain of equality tests.
        for (i, (key, target)) in keys.iter().enumerate() {
            let k = self.iconst(sty.clif(), *key);
            let c = self.b.ins().icmp(IntCC::Equal, v, k);
            let targs = self.edge_args(*target)?;
            let last = i + 1 == keys.len();
            if last {
                self.b.ins().brif(
                    c,
                    self.blocks[target.0 as usize],
                    &targs,
                    self.blocks[default.0 as usize],
                    &default_args,
                );
            } else {
                let next = self.b.create_block();
                self.b
                    .ins()
                    .brif(c, self.blocks[target.0 as usize], &targs, next, &[]);
                self.b.switch_to_block(next);
            }
        }
        Ok(())
    }

    /// A `switch` on a wide integer: a chain of equality tests on the parts.
    fn translate_switch_wide(
        &mut self,
        val: &TypedOperand,
        default: BlockId,
        cases: &[SwitchCase],
    ) -> TResult<()> {
        let sty = self.scalar_of(&val.ty)?;
        let bits = sty.bits();
        let v = self.use_wide(&val.op, bits)?;
        let default_args = self.edge_args(default)?;
        // All the comparisons are made in the current block; the chain of
        // branches below only consumes them.
        let mut conds = Vec::with_capacity(cases.len());
        for c in cases {
            let k = self.wide_const(truncate_const128(c.value, bits), sty.parts());
            let cond = self.wide_icmp(IPred::Eq, bits, &v, &k);
            conds.push((cond, c.target));
        }
        if conds.is_empty() {
            self.b
                .ins()
                .jump(self.blocks[default.0 as usize], &default_args);
            return Ok(());
        }
        for (i, (cond, target)) in conds.iter().enumerate() {
            let targs = self.edge_args(*target)?;
            if i + 1 == conds.len() {
                self.b.ins().brif(
                    *cond,
                    self.blocks[target.0 as usize],
                    &targs,
                    self.blocks[default.0 as usize],
                    &default_args,
                );
            } else {
                let next = self.b.create_block();
                self.b
                    .ins()
                    .brif(*cond, self.blocks[target.0 as usize], &targs, next, &[]);
                self.b.switch_to_block(next);
            }
        }
        Ok(())
    }

    // -- instructions ---------------------------------------------------

    fn translate_inst(&mut self, inst: &Instruction) -> TResult<()> {
        match &inst.kind {
            InstKind::Binary { op, lhs, rhs, .. } => {
                let sty = self.scalar_of(&inst.ty)?;
                if sty.is_wide() {
                    let l = self.use_wide(lhs, sty.bits())?;
                    let r = self.use_wide(rhs, sty.bits())?;
                    let p = self.emit_binop_wide(*op, sty.bits(), &l, &r)?;
                    let vals = self.wide_out(&p, sty.bits());
                    self.define(inst, vals);
                } else {
                    let l = self.use_scalar(lhs)?;
                    let r = self.use_scalar(rhs)?;
                    let v = self.emit_binop(*op, sty.bits(), l, r)?;
                    self.define(inst, vec![v]);
                }
            }
            InstKind::ICmp { pred, ty, lhs, rhs } => {
                let sty = self.scalar_of(ty)?;
                let v = if sty.is_wide() {
                    let l = self.use_wide(lhs, sty.bits())?;
                    let r = self.use_wide(rhs, sty.bits())?;
                    self.wide_icmp(*pred, sty.bits(), &l, &r)
                } else {
                    let l = self.use_scalar(lhs)?;
                    let r = self.use_scalar(rhs)?;
                    self.emit_icmp(*pred, sty.bits(), l, r)
                };
                self.define(inst, vec![v]);
            }
            InstKind::FCmp => {
                return Err(TransError::unsupported(
                    "floating point comparison is not supported",
                ))
            }
            InstKind::Cast { op, val, from } => {
                let v = self.use_op(val)?;
                let r = self.emit_cast(*op, from, &inst.ty, v)?;
                self.define(inst, r);
            }
            InstKind::Select { cond, t, f } => {
                let c = self.use_scalar(&cond.op)?;
                let tv = self.use_op(t)?;
                let fv = self.use_op(f)?;
                let r = tv
                    .iter()
                    .zip(fv.iter())
                    .map(|(x, y)| self.b.ins().select(c, *x, *y))
                    .collect();
                self.define(inst, r);
            }
            InstKind::Phi { .. } => {
                // Already bound to block parameters.
            }
            InstKind::Load { ptr, .. } => {
                let p = self.use_scalar(ptr)?;
                let leaves = self.layout().flatten(&inst.ty)?;
                let mut vals = Vec::with_capacity(leaves.len());
                for leaf in leaves {
                    vals.push(self.load_scalar(leaf.ty, p, leaf.offset as i64)?);
                }
                self.define(inst, vals);
            }
            InstKind::Store { val, ptr, .. } => {
                let vals = self.use_op(&val.op)?;
                let p = self.use_scalar(ptr)?;
                let leaves = self.layout().flatten(&val.ty)?;
                if leaves.len() != vals.len() {
                    return Err(TransError::invalid("stored value does not match its type"));
                }
                for (leaf, v) in leaves.iter().zip(vals) {
                    self.store_scalar(leaf.ty, v, p, leaf.offset as i64)?;
                }
            }
            InstKind::Alloca {
                alloc_ty,
                count,
                align,
            } => {
                let count = match count {
                    None => 1,
                    Some(TypedOperand {
                        op: Operand::Const(c),
                        ..
                    }) => match c.as_int() {
                        Some(n) if n >= 0 => n as u64,
                        _ => {
                            return Err(TransError::invalid("alloca with an invalid element count"))
                        }
                    },
                    Some(_) => {
                        return Err(TransError::unsupported(
                            "dynamically sized `alloca` (variable-length arrays) is not supported",
                        ))
                    }
                };
                let size = self.layout().alloc_size(alloc_ty)?.saturating_mul(count);
                let align = align
                    .unwrap_or(0)
                    .max(self.layout().abi_align(alloc_ty)?)
                    .max(1);
                let size = u32::try_from(size.max(1))
                    .map_err(|_| TransError::invalid("alloca too large"))?;
                let ss = self.b.create_sized_stack_slot(StackSlotData::new(
                    StackSlotKind::ExplicitSlot,
                    size,
                    align.trailing_zeros() as u8,
                ));
                self.allocas
                    .insert(inst.result.expect("alloca has a result"), ss);
            }
            InstKind::Gep {
                base_ty,
                ptr,
                indices,
                ..
            } => {
                let v = self.translate_gep(base_ty, ptr, indices)?;
                self.define(inst, vec![v]);
            }
            InstKind::Call {
                callee,
                fn_ty,
                args,
                ret_attrs,
                ..
            } => {
                let r = self.translate_call(callee, fn_ty, args, ret_attrs, &inst.ty)?;
                self.define(inst, r);
            }
            InstKind::ExtractValue { agg, indices } => {
                let vals = self.use_op(&agg.op)?;
                let (start, len, _) = self.layout().flat_range(&agg.ty, indices)?;
                self.define(inst, vals[start..start + len].to_vec());
            }
            InstKind::InsertValue { agg, val, indices } => {
                let mut vals = self.use_op(&agg.op)?;
                let new = self.use_op(&val.op)?;
                let (start, len, _) = self.layout().flat_range(&agg.ty, indices)?;
                if new.len() != len {
                    return Err(TransError::invalid(
                        "insertvalue operand does not match the selected element type",
                    ));
                }
                vals[start..start + len].copy_from_slice(&new);
                self.define(inst, vals);
            }
            InstKind::Freeze { val } => {
                let v = self.use_op(val)?;
                self.define(inst, v);
            }
            InstKind::Ret(None) => {
                self.b.ins().return_(&[]);
            }
            InstKind::Ret(Some(v)) => {
                let vals = self.use_op(&v.op)?;
                self.b.ins().return_(&vals);
            }
            InstKind::Br(target) => {
                let args = self.edge_args(*target)?;
                self.b.ins().jump(self.blocks[target.0 as usize], &args);
            }
            InstKind::CondBr { cond, t, f } => {
                let c = self.use_scalar(cond)?;
                let targs = self.edge_args(*t)?;
                let fargs = self.edge_args(*f)?;
                self.b.ins().brif(
                    c,
                    self.blocks[t.0 as usize],
                    &targs,
                    self.blocks[f.0 as usize],
                    &fargs,
                );
            }
            InstKind::Switch {
                val,
                default,
                cases,
            } => self.translate_switch(val, *default, cases)?,
            InstKind::Unreachable => {
                self.b.ins().trap(TrapCode::unwrap_user(1));
            }
            InstKind::Unsupported(op) => {
                return Err(TransError::unsupported(format!(
                    "instruction `{op}` is not supported"
                )));
            }
        }
        Ok(())
    }

    fn translate_gep(
        &mut self,
        base_ty: &Type,
        ptr: &Operand,
        indices: &[TypedOperand],
    ) -> TResult<Value> {
        let base = self.use_scalar(ptr)?;
        let mut cur_ty = base_ty.clone();
        let mut const_off: i64 = 0;
        let mut dyn_terms: Vec<Value> = Vec::new();
        for (i, idx) in indices.iter().enumerate() {
            let stride = if i == 0 {
                self.layout().alloc_size(base_ty)?
            } else {
                let resolved = self.layout().resolve(&cur_ty)?.clone();
                match resolved {
                    Type::Struct { fields, .. } => {
                        let k = match &idx.op {
                            Operand::Const(c) => c.as_int().ok_or_else(|| {
                                TransError::invalid("struct field index must be a constant")
                            })?,
                            _ => {
                                return Err(TransError::invalid(
                                    "struct field index must be a constant",
                                ))
                            }
                        };
                        if k < 0 || k as usize >= fields.len() {
                            return Err(TransError::invalid(format!(
                                "struct field index {k} out of range for {cur_ty}"
                            )));
                        }
                        const_off = const_off
                            .wrapping_add(self.layout().field_offset(&cur_ty, k as usize)? as i64);
                        cur_ty = fields[k as usize].clone();
                        continue;
                    }
                    Type::Array(_, elem) | Type::Vector { elem, .. } => {
                        let s = self.layout().alloc_size(&elem)?;
                        cur_ty = (*elem).clone();
                        s
                    }
                    other => {
                        return Err(TransError::invalid(format!(
                            "cannot index into type {other}"
                        )))
                    }
                }
            };
            match &idx.op {
                Operand::Const(c) => {
                    let ibits = match self.layout().resolve(&c.ty)? {
                        Type::Int(b) => *b,
                        _ => {
                            return Err(TransError::invalid(
                                "getelementptr index must be an integer",
                            ))
                        }
                    };
                    let v = c.as_int().ok_or_else(|| {
                        TransError::unsupported("non-integer constant as getelementptr index")
                    })?;
                    let v = sign_extend_const(truncate_const(v, ibits.min(64)), ibits.min(64));
                    const_off = const_off.wrapping_add(v.wrapping_mul(stride as i64));
                }
                op => {
                    let ity = self.scalar_of(&idx.ty)?;
                    let iv = if ity.is_wide() {
                        // Addresses are 32 bits wide: only the lowest part
                        // of a wide index matters.
                        self.use_wide(op, ity.bits())?[0]
                    } else {
                        let iv = self.use_scalar(op)?;
                        self.sext_to_i32(iv, ity.bits())
                    };
                    let term = if stride == 1 {
                        iv
                    } else if stride.is_power_of_two() {
                        let sh = self.iconst(types::I32, stride.trailing_zeros() as u64);
                        self.b.ins().ishl(iv, sh)
                    } else {
                        let s = self.iconst(types::I32, stride);
                        self.b.ins().imul(iv, s)
                    };
                    dyn_terms.push(term);
                }
            }
        }
        let mut addr = base;
        for t in dyn_terms {
            addr = self.b.ins().iadd(addr, t);
        }
        if const_off != 0 {
            let o = self.iconst(types::I32, const_off as u64);
            addr = self.b.ins().iadd(addr, o);
        }
        Ok(addr)
    }

    fn translate_call(
        &mut self,
        callee: &Callee,
        fn_ty: &FuncType,
        args: &[CallArg],
        ret_attrs: &ParamAttrs,
        ret_ty: &Type,
    ) -> TResult<Vec<Value>> {
        // Intrinsics.
        let direct_name: Option<&str> = match callee {
            Callee::Global(name) => Some(name.as_str()),
            Callee::Const(c) => peel_symbol(c),
            _ => None,
        };
        if let Some(name) = direct_name {
            if name.starts_with("llvm.") {
                return self.translate_intrinsic(name, args, ret_ty);
            }
        }
        if fn_ty.varargs && args.len() > fn_ty.params.len() {
            return Err(TransError::unsupported(
                "calls passing variable arguments are not supported",
            ));
        }
        for a in args {
            if a.ty == Type::Metadata {
                return Err(TransError::unsupported(
                    "calls with metadata arguments are not supported",
                ));
            }
        }

        // The signature is built from the call site (argument types and
        // attributes), which is what the caller actually passes.
        let call_fn_ty = FuncType {
            ret: fn_ty.ret.clone(),
            params: args.iter().map(|a| a.ty.clone()).collect(),
            varargs: false,
        };
        let attrs: Vec<ParamAttrs> = args.iter().map(|a| a.attrs.clone()).collect();
        let sig = self.layout().signature(
            &call_fn_ty,
            &attrs,
            ret_attrs,
            self.ctx.isa.default_call_conv(),
        )?;

        let mut argvals = Vec::new();
        for a in args {
            if let Some(bt) = &a.attrs.byval {
                let p = self.use_scalar(&a.op)?;
                argvals.push(self.byval_copy(bt, p, a.attrs.align)?);
            } else {
                argvals.extend(self.use_op(&a.op)?);
            }
        }

        let call = match callee {
            Callee::InlineAsm => {
                return Err(TransError::unsupported("inline assembly is not supported"))
            }
            Callee::Local(id) => {
                let p = self.use_scalar(&Operand::Local(*id))?;
                let sigref = self.sig_ref(&sig);
                self.b.ins().call_indirect(sigref, p, &argvals)
            }
            Callee::Global(_) | Callee::Const(_) => {
                match direct_name {
                    Some(name) if self.ctx.is_function_symbol(name) => {
                        let fr = self.func_ref(name, &sig);
                        self.b.ins().call(fr, &argvals)
                    }
                    Some(name)
                        if !self.ctx.globals.contains_key(name)
                            && !self.ctx.aliases.contains_key(name) =>
                    {
                        // A call to a function the module never declared:
                        // treat it as an external function with the call
                        // site's signature.
                        self.runtime_imports
                            .entry(name.to_string())
                            .or_insert_with(|| sig.clone());
                        let fr = self.func_ref(name, &sig);
                        self.b.ins().call(fr, &argvals)
                    }
                    _ => {
                        let p = match callee {
                            Callee::Global(name) => self.symbol_addr(name, 0)?,
                            Callee::Const(c) => self.use_scalar(&Operand::Const(c.clone()))?,
                            _ => unreachable!(),
                        };
                        let sigref = self.sig_ref(&sig);
                        self.b.ins().call_indirect(sigref, p, &argvals)
                    }
                }
            }
        };
        Ok(self.b.inst_results(call).to_vec())
    }
}

/// If a constant is a (possibly cast) reference to a global symbol, its
/// name.
pub(super) fn peel_symbol(c: &Constant) -> Option<&str> {
    match &c.kind {
        ConstKind::Global(n) => Some(n),
        ConstKind::Cast(CastOp::BitCast | CastOp::AddrSpaceCast, inner) => peel_symbol(inner),
        ConstKind::DsoLocalEquivalent(n) => Some(n),
        _ => None,
    }
}

/// The successors of a block, from its terminator.
fn successors(block: &crate::llvm::Block) -> Vec<BlockId> {
    let Some(term) = block.insts.last() else {
        return Vec::new();
    };
    match &term.kind {
        InstKind::Br(t) => vec![*t],
        InstKind::CondBr { t, f, .. } => vec![*t, *f],
        InstKind::Switch { default, cases, .. } => {
            let mut v = vec![*default];
            v.extend(cases.iter().map(|c| c.target));
            v
        }
        _ => Vec::new(),
    }
}

/// Reverse post-order of the reachable blocks (entry first).
fn reverse_postorder(f: &Function) -> Vec<usize> {
    let n = f.blocks.len();
    let mut visited = vec![false; n];
    let mut post = Vec::with_capacity(n);
    // Iterative DFS with an explicit stack of (block, next successor index).
    let mut stack: Vec<(usize, Vec<BlockId>, usize)> = Vec::new();
    visited[0] = true;
    stack.push((0, successors(&f.blocks[0]), 0));
    while let Some((b, succs, idx)) = stack.last_mut() {
        if *idx < succs.len() {
            let s = succs[*idx].0 as usize;
            *idx += 1;
            if !visited[s] {
                visited[s] = true;
                let ss = successors(&f.blocks[s]);
                stack.push((s, ss, 0));
            }
        } else {
            post.push(*b);
            stack.pop();
        }
    }
    post.reverse();
    post
}

fn mem_offset(offset: i64) -> TResult<Offset32> {
    i32::try_from(offset)
        .map(Offset32::new)
        .map_err(|_| TransError::invalid("memory offset out of range"))
}

/// Splits `size` bytes into (offset, type) chunks of 4, 2 and 1 bytes.
fn chunk_offsets(size: u64) -> Vec<(u64, types::Type)> {
    let mut out = Vec::new();
    let mut off = 0;
    while size - off >= 4 {
        out.push((off, types::I32));
        off += 4;
    }
    if size - off >= 2 {
        out.push((off, types::I16));
        off += 2;
    }
    if size - off >= 1 {
        out.push((off, types::I8));
    }
    out
}
