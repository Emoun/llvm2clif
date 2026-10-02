//! Translation of calls to LLVM intrinsics.
//!
//! Intrinsics the Scry backend cannot express directly (population count,
//! leading/trailing zero counts, funnel shifts, multiplication overflow,
//! saturating arithmetic, anything on integers wider than 32 bits, ...) are
//! expanded into sequences of supported Cranelift instructions.

use super::func::FuncTranslator;
use super::types::{container, is_native, mask, ScalarTy};
use super::wide::Wide;
use super::{TResult, TransError};
use crate::llvm::*;
use cranelift_codegen::ir::condcodes::IntCC;
use cranelift_codegen::ir::{types, InstBuilder, TrapCode, Value};

impl FuncTranslator<'_, '_> {
    /// Translates a call to the intrinsic `name`; returns the (flattened)
    /// result values.
    pub(super) fn translate_intrinsic(
        &mut self,
        name: &str,
        args: &[CallArg],
        ret_ty: &Type,
    ) -> TResult<Vec<Value>> {
        let base = name.strip_prefix("llvm.").unwrap_or(name);
        // Drop the type suffixes: `sadd.with.overflow.i32` -> `sadd.with.overflow`.
        let base = strip_type_suffixes(base);
        match base {
            // Pure markers with no runtime effect.
            "lifetime.start"
            | "lifetime.end"
            | "dbg.value"
            | "dbg.declare"
            | "dbg.assign"
            | "dbg.label"
            | "assume"
            | "experimental.noalias.scope.decl"
            | "donothing"
            | "sideeffect"
            | "prefetch"
            | "var.annotation"
            | "invariant.end"
            | "instrprof.increment"
            | "instrprof.value.profile"
            | "clear_cache" => Ok(vec![]),
            // Identity-like.
            "expect"
            | "expect.with.probability"
            | "launder.invariant.group"
            | "strip.invariant.group"
            | "ptr.annotation"
            | "annotation"
            | "ssa.copy" => self.use_op(&args[0].op),
            "invariant.start" => Ok(vec![self.iconst(types::I32, 0)]),
            "is.constant" => Ok(vec![self.iconst(types::I8, 0)]),
            "objectsize" => {
                let s = self.ret_scalar(ret_ty)?;
                let min = args.get(1).and_then(|a| const_int(&a.op)).unwrap_or(0) != 0;
                let v = if min { 0 } else { mask(s.bits()) };
                if s.is_wide() {
                    let p = self.wide_const(v as u128, s.parts());
                    return Ok(self.wide_out(&p, s.bits()));
                }
                Ok(vec![self.iconst(s.clif(), v)])
            }
            "trap" | "debugtrap" | "ubsantrap" => {
                // A conditional trap that always fires keeps the block open
                // for the `unreachable` that follows in the IR.
                let one = self.iconst(types::I8, 1);
                self.b.ins().trapnz(one, TrapCode::unwrap_user(2));
                Ok(vec![])
            }
            "abs" => {
                let s = self.arg_scalar_ty(args, 0)?;
                if s.is_wide() {
                    let x = self.use_wide(&args[0].op, s.bits())?;
                    let x = self.wide_sext_in(&x, s.bits());
                    let neg = self.wide_sign(&x);
                    let r = self.wide_negate_if(&x, neg);
                    return Ok(self.wide_out(&r, s.bits()));
                }
                let x = self.use_scalar(&args[0].op)?;
                let xs = self.sext_in(x, s.bits());
                let r = self.b.ins().iabs(xs);
                Ok(vec![self.canon(r, s.bits())])
            }
            "smax" | "smin" | "umax" | "umin" => {
                let s = self.arg_scalar_ty(args, 0)?;
                if s.is_wide() {
                    let a = self.use_wide(&args[0].op, s.bits())?;
                    let b = self.use_wide(&args[1].op, s.bits())?;
                    let pred = match base {
                        "smax" => IPred::Sgt,
                        "smin" => IPred::Slt,
                        "umax" => IPred::Ugt,
                        _ => IPred::Ult,
                    };
                    let c = self.wide_icmp(pred, s.bits(), &a, &b);
                    let r = self.wide_select(c, &a, &b);
                    return Ok(self.wide_out(&r, s.bits()));
                }
                let a = self.use_scalar(&args[0].op)?;
                let b = self.use_scalar(&args[1].op)?;
                let signed = base.starts_with('s');
                let (a, b) = if signed {
                    (self.sext_in(a, s.bits()), self.sext_in(b, s.bits()))
                } else {
                    (a, b)
                };
                let r = match base {
                    "smax" => self.b.ins().smax(a, b),
                    "smin" => self.b.ins().smin(a, b),
                    "umax" => self.b.ins().umax(a, b),
                    _ => self.b.ins().umin(a, b),
                };
                Ok(vec![if signed { self.canon(r, s.bits()) } else { r }])
            }
            "bswap" => {
                let s = self.arg_scalar_ty(args, 0)?;
                if s.is_wide() {
                    if s.bits() != 64 {
                        return Err(TransError::unsupported(format!(
                            "llvm.bswap on i{} is not supported",
                            s.bits()
                        )));
                    }
                    // Swap the halves and byte-swap each.
                    let w = self.use_wide(&args[0].op, 64)?;
                    let nlo = self.b.ins().bswap(w[1]);
                    let nhi = self.b.ins().bswap(w[0]);
                    return Ok(self.wide_out(&[nlo, nhi], 64));
                }
                let x = self.use_scalar(&args[0].op)?;
                match s.bits() {
                    8 => Ok(vec![x]),
                    16 | 32 => Ok(vec![self.b.ins().bswap(x)]),
                    b => Err(TransError::unsupported(format!(
                        "llvm.bswap on i{b} is not supported"
                    ))),
                }
            }
            "ctpop" => {
                let s = self.arg_scalar_ty(args, 0)?;
                if s.is_wide() {
                    let w = self.use_wide(&args[0].op, s.bits())?;
                    let mut r = self.popcount32(w[0]);
                    for p in &w[1..] {
                        let c = self.popcount32(*p);
                        r = self.b.ins().iadd(r, c);
                    }
                    let zero = self.iconst(types::I32, 0);
                    let mut out = vec![r];
                    out.resize(s.parts(), zero);
                    return Ok(self.wide_out(&out, s.bits()));
                }
                let x = self.use_scalar(&args[0].op)?;
                let x32 = self.zext_to_i32(x, s.bits());
                let r = self.popcount32(x32);
                Ok(vec![self.resize_unsigned(r, types::I32, s.clif())])
            }
            "ctlz" => {
                let s = self.arg_scalar_ty(args, 0)?;
                if s.is_wide() {
                    // ctlz64 = hi == 0 ? 32 + ctlz(lo) : ctlz(hi); the width
                    // adjustment below handles i33..i63.
                    let (lo, hi) = self.wide_halves(base, args)?;
                    let clo = self.ctlz32(lo);
                    let chi = self.ctlz32(hi);
                    let zero = self.iconst(types::I32, 0);
                    let hi_zero = self.b.ins().icmp(IntCC::Equal, hi, zero);
                    let c32 = self.iconst(types::I32, 32);
                    let lo_path = self.b.ins().iadd(clo, c32);
                    let r = self.b.ins().select(hi_zero, lo_path, chi);
                    let adj = self.iconst(types::I32, (64 - s.bits()) as u64);
                    let r = self.b.ins().isub(r, adj);
                    return Ok(self.wide_out(&[r, zero], s.bits()));
                }
                let x = self.use_scalar(&args[0].op)?;
                let x32 = self.zext_to_i32(x, s.bits());
                let r = self.ctlz32(x32);
                // ctlz of the narrow value = ctlz32 - (32 - width)
                let adj = self.iconst(types::I32, (32 - s.bits()) as u64);
                let r = self.b.ins().isub(r, adj);
                Ok(vec![self.resize_unsigned(r, types::I32, s.clif())])
            }
            "cttz" => {
                let s = self.arg_scalar_ty(args, 0)?;
                if s.is_wide() {
                    let (lo, hi) = self.wide_halves(base, args)?;
                    let tlo = self.cttz32(lo);
                    let thi = self.cttz32(hi);
                    let zero = self.iconst(types::I32, 0);
                    let lo_zero = self.b.ins().icmp(IntCC::Equal, lo, zero);
                    let c32 = self.iconst(types::I32, 32);
                    let hi_path = self.b.ins().iadd(thi, c32);
                    let r = self.b.ins().select(lo_zero, hi_path, tlo);
                    let w = self.iconst(types::I32, s.bits() as u64);
                    let r = self.b.ins().umin(r, w);
                    return Ok(self.wide_out(&[r, zero], s.bits()));
                }
                let x = self.use_scalar(&args[0].op)?;
                let x32 = self.zext_to_i32(x, s.bits());
                let r = self.cttz32(x32);
                let w = self.iconst(types::I32, s.bits() as u64);
                let r = self.b.ins().umin(r, w);
                Ok(vec![self.resize_unsigned(r, types::I32, s.clif())])
            }
            "bitreverse" => {
                let s = self.arg_scalar_ty(args, 0)?;
                if s.is_wide() {
                    let (lo, hi) = self.wide_halves(base, args)?;
                    let rlo = self.bitreverse32(hi);
                    let rhi = self.bitreverse32(lo);
                    let mut r = vec![rlo, rhi];
                    if s.bits() < 64 {
                        let sh = self.iconst(types::I32, (64 - s.bits()) as u64);
                        r = self.wide_lshr(&r, sh);
                    }
                    return Ok(self.wide_out(&r, s.bits()));
                }
                let x = self.use_scalar(&args[0].op)?;
                let x32 = self.zext_to_i32(x, s.bits());
                let r = self.bitreverse32(x32);
                let r = if s.bits() < 32 {
                    let sh = self.iconst(types::I32, (32 - s.bits()) as u64);
                    self.b.ins().ushr(r, sh)
                } else {
                    r
                };
                Ok(vec![self.resize_unsigned(r, types::I32, s.clif())])
            }
            "fshl" | "fshr" => {
                let s = self.arg_scalar_ty(args, 0)?;
                let left = base == "fshl";
                if s.is_wide() {
                    if s.bits() != 64 {
                        return Err(TransError::unsupported(format!(
                            "funnel shift on i{} is not supported",
                            s.bits()
                        )));
                    }
                    let a = self.use_wide(&args[0].op, 64)?;
                    let b = self.use_wide(&args[1].op, 64)?;
                    let c = self.use_wide(&args[2].op, 64)?;
                    let c63 = self.iconst(types::I32, 63);
                    let sh = self.b.ins().band(c[0], c63);
                    let c64 = self.iconst(types::I32, 64);
                    let inv = self.b.ins().isub(c64, sh);
                    let zero = self.iconst(types::I32, 0);
                    let is_zero = self.b.ins().icmp(IntCC::Equal, sh, zero);
                    let r = if left {
                        let hi = self.wide_shl(&a, sh);
                        let lo = self.wide_lshr(&b, inv);
                        let combined = self.wide_or(&hi, &lo);
                        self.wide_select(is_zero, &a, &combined)
                    } else {
                        let lo = self.wide_lshr(&b, sh);
                        let hi = self.wide_shl(&a, inv);
                        let combined = self.wide_or(&hi, &lo);
                        self.wide_select(is_zero, &b, &combined)
                    };
                    return Ok(self.wide_out(&r, 64));
                }
                if !is_native(s.bits()) {
                    return Err(TransError::unsupported(format!(
                        "funnel shift on i{} is not supported",
                        s.bits()
                    )));
                }
                let a = self.use_scalar(&args[0].op)?;
                let b = self.use_scalar(&args[1].op)?;
                let c = self.use_scalar(&args[2].op)?;
                if same_operand(&args[0].op, &args[1].op) && s.bits() <= 32 {
                    let r = if left {
                        self.b.ins().rotl(a, c)
                    } else {
                        self.b.ins().rotr(a, c)
                    };
                    return Ok(vec![r]);
                }
                let w = s.bits();
                let wm1 = self.iconst(s.clif(), (w - 1) as u64);
                let sh = self.b.ins().band(c, wm1);
                let wv = self.iconst(s.clif(), w as u64);
                let inv = self.b.ins().isub(wv, sh);
                let zero = self.iconst(s.clif(), 0);
                let is_zero = self.b.ins().icmp(IntCC::Equal, sh, zero);
                let r = if left {
                    let hi = self.b.ins().ishl(a, sh);
                    let lo = self.b.ins().ushr(b, inv);
                    let combined = self.b.ins().bor(hi, lo);
                    self.b.ins().select(is_zero, a, combined)
                } else {
                    let lo = self.b.ins().ushr(b, sh);
                    let hi = self.b.ins().ishl(a, inv);
                    let combined = self.b.ins().bor(hi, lo);
                    self.b.ins().select(is_zero, b, combined)
                };
                Ok(vec![r])
            }
            "sadd.with.overflow" | "uadd.with.overflow" | "ssub.with.overflow"
            | "usub.with.overflow" => {
                let s = self.arg_scalar_ty(args, 0)?;
                if s.is_wide() {
                    let (a, b) = self.wide_args64(base, args)?;
                    let (r, o) = match base {
                        "uadd.with.overflow" => self.wide_add_carry(&a, &b),
                        "usub.with.overflow" => {
                            let r = self.wide_sub(&a, &b);
                            (r, self.wide_icmp(IPred::Ult, 64, &a, &b))
                        }
                        "sadd.with.overflow" => {
                            let r = self.wide_add(&a, &b);
                            let o = self.wide_sadd_overflow(&a, &b, &r);
                            (r, o)
                        }
                        _ => {
                            let r = self.wide_sub(&a, &b);
                            let o = self.wide_ssub_overflow(&a, &b, &r);
                            (r, o)
                        }
                    };
                    let mut out = self.wide_out(&r, 64);
                    out.push(o);
                    return Ok(out);
                }
                if !is_native(s.bits()) {
                    return Err(TransError::unsupported(format!(
                        "llvm.{base} on i{} is not supported",
                        s.bits()
                    )));
                }
                let a = self.use_scalar(&args[0].op)?;
                let b = self.use_scalar(&args[1].op)?;
                let (r, o) = match base {
                    "sadd.with.overflow" => self.b.ins().sadd_overflow(a, b),
                    "uadd.with.overflow" => self.b.ins().uadd_overflow(a, b),
                    "ssub.with.overflow" => self.b.ins().ssub_overflow(a, b),
                    _ => self.b.ins().usub_overflow(a, b),
                };
                Ok(vec![r, o])
            }
            "smul.with.overflow" | "umul.with.overflow" => {
                let s = self.arg_scalar_ty(args, 0)?;
                if s.is_wide() {
                    let (a, b) = self.wide_args64(base, args)?;
                    let (r, o) = if base == "smul.with.overflow" {
                        self.wide_smul_overflow(&a, &b)
                    } else {
                        self.wide_umul_overflow(&a, &b)
                    };
                    let mut out = self.wide_out(&r, 64);
                    out.push(o);
                    return Ok(out);
                }
                if !is_native(s.bits()) {
                    return Err(TransError::unsupported(format!(
                        "llvm.{base} on i{} is not supported",
                        s.bits()
                    )));
                }
                let a = self.use_scalar(&args[0].op)?;
                let b = self.use_scalar(&args[1].op)?;
                let signed = base == "smul.with.overflow";
                let (r, o) = self.mul_overflow(a, b, s, signed);
                Ok(vec![r, o])
            }
            "sadd.sat" | "ssub.sat" | "uadd.sat" | "usub.sat" => {
                let s = self.arg_scalar_ty(args, 0)?;
                if s.is_wide() {
                    let (a, b) = self.wide_args64(base, args)?;
                    let r = match base {
                        "uadd.sat" => {
                            let (r, c) = self.wide_add_carry(&a, &b);
                            let all = self.wide_const(u64::MAX as u128, 2);
                            self.wide_select(c, &all, &r)
                        }
                        "usub.sat" => {
                            let r = self.wide_sub(&a, &b);
                            let o = self.wide_icmp(IPred::Ult, 64, &a, &b);
                            let zero = self.wide_const(0, 2);
                            self.wide_select(o, &zero, &r)
                        }
                        _ => {
                            let (r, o) = if base == "sadd.sat" {
                                let r = self.wide_add(&a, &b);
                                let o = self.wide_sadd_overflow(&a, &b, &r);
                                (r, o)
                            } else {
                                let r = self.wide_sub(&a, &b);
                                let o = self.wide_ssub_overflow(&a, &b, &r);
                                (r, o)
                            };
                            // Saturate towards the sign of the first operand.
                            let c31 = self.iconst(types::I32, 31);
                            let sign = self.b.ins().sshr(a[1], c31);
                            let max_lo = self.iconst(types::I32, 0xffff_ffff);
                            let max_hi = self.iconst(types::I32, 0x7fff_ffff);
                            let sat = [
                                self.b.ins().bxor(max_lo, sign),
                                self.b.ins().bxor(max_hi, sign),
                            ];
                            self.wide_select(o, &sat, &r)
                        }
                    };
                    return Ok(self.wide_out(&r, 64));
                }
                if !is_native(s.bits()) {
                    return Err(TransError::unsupported(format!(
                        "llvm.{base} on i{} is not supported",
                        s.bits()
                    )));
                }
                let a = self.use_scalar(&args[0].op)?;
                let b = self.use_scalar(&args[1].op)?;
                let r = match base {
                    "uadd.sat" => {
                        let (r, o) = self.b.ins().uadd_overflow(a, b);
                        let all = self.iconst(s.clif(), mask(s.bits()));
                        self.b.ins().select(o, all, r)
                    }
                    "usub.sat" => {
                        let (r, o) = self.b.ins().usub_overflow(a, b);
                        let zero = self.iconst(s.clif(), 0);
                        self.b.ins().select(o, zero, r)
                    }
                    _ => {
                        let (r, o) = if base == "sadd.sat" {
                            self.b.ins().sadd_overflow(a, b)
                        } else {
                            self.b.ins().ssub_overflow(a, b)
                        };
                        // Saturate towards the sign of the first operand.
                        let shift = self.iconst(s.clif(), (s.bits() - 1) as u64);
                        let sign = self.b.ins().sshr(a, shift);
                        let max = self.iconst(s.clif(), mask(s.bits()) >> 1);
                        let sat = self.b.ins().bxor(max, sign);
                        self.b.ins().select(o, sat, r)
                    }
                };
                Ok(vec![r])
            }
            "scmp" | "ucmp" => {
                let s = self.arg_scalar_ty(args, 0)?;
                let rs = self.ret_scalar(ret_ty)?;
                let (pg, pl) = if base == "scmp" {
                    (IPred::Sgt, IPred::Slt)
                } else {
                    (IPred::Ugt, IPred::Ult)
                };
                let (gt, lt) = if s.is_wide() {
                    let a = self.use_wide(&args[0].op, s.bits())?;
                    let b = self.use_wide(&args[1].op, s.bits())?;
                    (
                        self.wide_icmp(pg, s.bits(), &a, &b),
                        self.wide_icmp(pl, s.bits(), &a, &b),
                    )
                } else {
                    let a = self.use_scalar(&args[0].op)?;
                    let b = self.use_scalar(&args[1].op)?;
                    (
                        self.emit_icmp(pg, s.bits(), a, b),
                        self.emit_icmp(pl, s.bits(), a, b),
                    )
                };
                if rs.is_wide() {
                    // -1, 0 or 1, sign-extended over all parts.
                    let r = self.b.ins().isub(gt, lt);
                    let lo = self.emit_sextend(r, 8, types::I32);
                    let c31 = self.iconst(types::I32, 31);
                    let hi = self.b.ins().sshr(lo, c31);
                    let mut out = vec![lo];
                    out.resize(rs.parts(), hi);
                    return Ok(self.wide_out(&out, rs.bits()));
                }
                let gt = self.resize_unsigned(gt, types::I8, rs.clif());
                let lt = self.resize_unsigned(lt, types::I8, rs.clif());
                let r = self.b.ins().isub(gt, lt);
                Ok(vec![self.canon(r, rs.bits())])
            }
            "ptrmask" => {
                let p = self.use_scalar(&args[0].op)?;
                let ms = self.arg_scalar_ty(args, 1)?;
                let m = if ms.is_wide() {
                    self.use_wide(&args[1].op, ms.bits())?[0]
                } else {
                    let m = self.use_scalar(&args[1].op)?;
                    self.resize_unsigned(m, ms.clif(), types::I32)
                };
                Ok(vec![self.b.ins().band(p, m)])
            }
            "memcpy" | "memcpy.inline" | "memmove" => {
                let dst = self.use_scalar(&args[0].op)?;
                let src = self.use_scalar(&args[1].op)?;
                let overlap = base == "memmove";
                match const_int(&args[2].op) {
                    Some(n) => self.emit_memcpy(dst, src, n as u64, overlap)?,
                    None => {
                        let n = self.length_arg(args, 2)?;
                        let f = if overlap { "memmove" } else { "memcpy" };
                        self.call_runtime(
                            f,
                            &[types::I32, types::I32, types::I32],
                            Some(types::I32),
                            &[dst, src, n],
                        );
                    }
                }
                Ok(vec![])
            }
            "memset" | "memset.inline" => {
                let dst = self.use_scalar(&args[0].op)?;
                let val = self.use_scalar(&args[1].op)?;
                match const_int(&args[2].op) {
                    Some(n) => self.emit_memset(dst, val, n as u64)?,
                    None => {
                        let n = self.length_arg(args, 2)?;
                        let v32 = self.b.ins().uextend(types::I32, val);
                        self.call_runtime(
                            "memset",
                            &[types::I32, types::I32, types::I32],
                            Some(types::I32),
                            &[dst, v32, n],
                        );
                    }
                }
                Ok(vec![])
            }
            "stacksave" | "stackrestore" => Err(TransError::unsupported(
                "dynamic stack allocation (llvm.stacksave/stackrestore) is not supported",
            )),
            "va_start" | "va_end" | "va_copy" => Err(TransError::unsupported(
                "variadic functions (va_start/va_arg) are not supported",
            )),
            _ => Err(TransError::unsupported(format!(
                "intrinsic `{name}` is not supported"
            ))),
        }
    }

    fn arg_scalar_ty(&self, args: &[CallArg], i: usize) -> TResult<ScalarTy> {
        let a = args
            .get(i)
            .ok_or_else(|| TransError::invalid("missing intrinsic argument"))?;
        self.layout().scalar_of(&a.ty).map_err(TransError::from)
    }

    fn ret_scalar(&self, ret_ty: &Type) -> TResult<ScalarTy> {
        self.layout().scalar_of(ret_ty).map_err(TransError::from)
    }

    /// A length argument as an i32 value (memory sizes are 32 bits; a wide
    /// length only contributes its low part).
    fn length_arg(&mut self, args: &[CallArg], i: usize) -> TResult<Value> {
        let s = self.arg_scalar_ty(args, i)?;
        if s.is_wide() {
            return Ok(self.use_wide(&args[i].op, s.bits())?[0]);
        }
        let v = self.use_scalar(&args[i].op)?;
        Ok(self.resize_unsigned(v, s.clif(), types::I32))
    }

    /// The first two arguments of a 64-bit intrinsic as parts (other wide
    /// widths are rejected).
    fn wide_args64(&mut self, base: &str, args: &[CallArg]) -> TResult<(Wide, Wide)> {
        let s = self.arg_scalar_ty(args, 0)?;
        if s.bits() != 64 {
            return Err(TransError::unsupported(format!(
                "llvm.{base} on i{} is not supported",
                s.bits()
            )));
        }
        let a = self.use_wide(&args[0].op, 64)?;
        let b = self.use_wide(&args[1].op, 64)?;
        Ok((a, b))
    }

    /// The first argument of an intrinsic on a two-part (33..64-bit) wide
    /// integer as its `(lo, hi)` halves; wider integers are rejected.
    fn wide_halves(&mut self, base: &str, args: &[CallArg]) -> TResult<(Value, Value)> {
        let s = self.arg_scalar_ty(args, 0)?;
        if s.parts() != 2 {
            return Err(TransError::unsupported(format!(
                "llvm.{base} on i{} is not supported",
                s.bits()
            )));
        }
        let w = self.use_wide(&args[0].op, s.bits())?;
        Ok((w[0], w[1]))
    }

    /// Count of trailing zeros of an i32 value (32 for zero).
    fn cttz32(&mut self, x: Value) -> Value {
        // cttz(x) = popcount((x & -x) - 1)
        let neg = self.b.ins().ineg(x);
        let low = self.b.ins().band(x, neg);
        let one = self.iconst(types::I32, 1);
        let m = self.b.ins().isub(low, one);
        self.popcount32(m)
    }

    /// Population count of an i32 value.
    fn popcount32(&mut self, x: Value) -> Value {
        let c1 = self.iconst(types::I32, 1);
        let m1 = self.iconst(types::I32, 0x5555_5555);
        let t = self.b.ins().ushr(x, c1);
        let t = self.b.ins().band(t, m1);
        let x = self.b.ins().isub(x, t);
        let c2 = self.iconst(types::I32, 2);
        let m2 = self.iconst(types::I32, 0x3333_3333);
        let a = self.b.ins().band(x, m2);
        let b = self.b.ins().ushr(x, c2);
        let b = self.b.ins().band(b, m2);
        let x = self.b.ins().iadd(a, b);
        let c4 = self.iconst(types::I32, 4);
        let m4 = self.iconst(types::I32, 0x0f0f_0f0f);
        let t = self.b.ins().ushr(x, c4);
        let x = self.b.ins().iadd(x, t);
        let x = self.b.ins().band(x, m4);
        let mul = self.iconst(types::I32, 0x0101_0101);
        let x = self.b.ins().imul(x, mul);
        let c24 = self.iconst(types::I32, 24);
        self.b.ins().ushr(x, c24)
    }

    /// Count of leading zeros of an i32 value (32 for zero).
    fn ctlz32(&mut self, x: Value) -> Value {
        let mut n = self.iconst(types::I32, 0);
        let mut x = x;
        let zero = self.iconst(types::I32, 0);
        for step in [16u64, 8, 4, 2, 1] {
            let sh = self.iconst(types::I32, 32 - step);
            let top = self.b.ins().ushr(x, sh);
            let is_zero = self.b.ins().icmp(IntCC::Equal, top, zero);
            let stepv = self.iconst(types::I32, step);
            let add = self.b.ins().select(is_zero, stepv, zero);
            n = self.b.ins().iadd(n, add);
            let shifted = self.b.ins().ishl(x, stepv);
            x = self.b.ins().select(is_zero, shifted, x);
        }
        // After normalization the top bit is set unless x was zero.
        let c31 = self.iconst(types::I32, 31);
        let top = self.b.ins().ushr(x, c31);
        let one = self.iconst(types::I32, 1);
        let extra = self.b.ins().isub(one, top);
        self.b.ins().iadd(n, extra)
    }

    /// Bit reversal of an i32 value.
    fn bitreverse32(&mut self, x: Value) -> Value {
        let mut x = x;
        for (shift, m) in [
            (1u64, 0x5555_5555u64),
            (2, 0x3333_3333),
            (4, 0x0f0f_0f0f),
            (8, 0x00ff_00ff),
            (16, 0x0000_ffff),
        ] {
            let s = self.iconst(types::I32, shift);
            let mv = self.iconst(types::I32, m);
            let hi = self.b.ins().ushr(x, s);
            let hi = self.b.ins().band(hi, mv);
            let lo = self.b.ins().band(x, mv);
            let lo = self.b.ins().ishl(lo, s);
            x = self.b.ins().bor(hi, lo);
        }
        x
    }

    /// Multiplication with overflow detection for native widths.
    fn mul_overflow(&mut self, a: Value, b: Value, s: ScalarTy, signed: bool) -> (Value, Value) {
        let w = s.bits();
        if w >= 32 {
            let c = s.clif();
            let lo = self.b.ins().imul(a, b);
            let hi = if signed {
                self.b.ins().smulhi(a, b)
            } else {
                self.b.ins().umulhi(a, b)
            };
            let expect = if signed {
                let top = self.iconst(c, (w - 1) as u64);
                self.b.ins().sshr(lo, top)
            } else {
                self.iconst(c, 0)
            };
            let o = self.b.ins().icmp(IntCC::NotEqual, hi, expect);
            return (lo, o);
        }
        // 8/16 bits: multiply in 32 bits and check that the product fits.
        let (a32, b32) = if signed {
            (
                self.emit_sextend(a, w, types::I32),
                self.emit_sextend(b, w, types::I32),
            )
        } else {
            (
                self.b.ins().uextend(types::I32, a),
                self.b.ins().uextend(types::I32, b),
            )
        };
        let prod = self.b.ins().imul(a32, b32);
        let lo = self.b.ins().ireduce(container(w), prod);
        let back = if signed {
            self.emit_sextend(lo, w, types::I32)
        } else {
            self.b.ins().uextend(types::I32, lo)
        };
        let o = self.b.ins().icmp(IntCC::NotEqual, prod, back);
        (lo, o)
    }
}

/// Removes the trailing type-suffix components (`.i32`, `.p0`, `.v4i32`)
/// from an intrinsic name.
fn strip_type_suffixes(name: &str) -> &str {
    let mut end = name.len();
    loop {
        let head = &name[..end];
        let Some(dot) = head.rfind('.') else { break };
        let last = &head[dot + 1..];
        if is_type_suffix(last) {
            end = dot;
        } else {
            break;
        }
    }
    &name[..end]
}

fn is_type_suffix(s: &str) -> bool {
    if let Some(rest) = s.strip_prefix('i') {
        return !rest.is_empty() && rest.chars().all(|c| c.is_ascii_digit());
    }
    if let Some(rest) = s.strip_prefix('p') {
        return !rest.is_empty() && rest.chars().all(|c| c.is_ascii_digit());
    }
    if let Some(rest) = s.strip_prefix('v') {
        // v4i32, nxv2i8, ...
        return rest.chars().next().is_some_and(|c| c.is_ascii_digit()) && rest.contains('i');
    }
    matches!(s, "f16" | "f32" | "f64" | "f128" | "bf16")
}

fn const_int(op: &Operand) -> Option<i128> {
    match op {
        Operand::Const(c) => c.as_int(),
        _ => None,
    }
}

fn same_operand(a: &Operand, b: &Operand) -> bool {
    match (a, b) {
        (Operand::Local(x), Operand::Local(y)) => x == y,
        (Operand::Const(x), Operand::Const(y)) => x == y,
        _ => false,
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn strips_type_suffixes() {
        assert_eq!(strip_type_suffixes("memcpy.p0.p0.i32"), "memcpy");
        assert_eq!(
            strip_type_suffixes("sadd.with.overflow.i32"),
            "sadd.with.overflow"
        );
        assert_eq!(strip_type_suffixes("ctlz.i8"), "ctlz");
        assert_eq!(strip_type_suffixes("lifetime.start.p0"), "lifetime.start");
        assert_eq!(strip_type_suffixes("dbg.value"), "dbg.value");
        assert_eq!(strip_type_suffixes("objectsize.i32.p0"), "objectsize");
        assert_eq!(strip_type_suffixes("scmp.i8.i32"), "scmp");
        assert_eq!(
            strip_type_suffixes("experimental.noalias.scope.decl"),
            "experimental.noalias.scope.decl"
        );
        assert_eq!(
            strip_type_suffixes("memcpy.inline.p0.p0.i32"),
            "memcpy.inline"
        );
    }
}
