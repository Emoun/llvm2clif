//! Translation of calls to LLVM intrinsics.
//!
//! Intrinsics the Scry backend cannot express directly (population count,
//! leading/trailing zero counts, funnel shifts, multiplication overflow,
//! saturating arithmetic, anything on integers wider than a word, ...) are
//! expanded into sequences of supported Cranelift instructions.

use super::func::FuncTranslator;
use super::types::{container, is_native, mask, word_bits, word_ty, ScalarTy};
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
            "invariant.start" => Ok(vec![self.iconst(word_ty(), 0)]),
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
                    let two = 2 * word_bits();
                    if s.bits() != two {
                        return Err(TransError::unsupported(format!(
                            "llvm.bswap on i{} is not supported",
                            s.bits()
                        )));
                    }
                    // Swap the halves and byte-swap each.
                    let w = self.use_wide(&args[0].op, two)?;
                    let nlo = self.b.ins().bswap(w[1]);
                    let nhi = self.b.ins().bswap(w[0]);
                    return Ok(self.wide_out(&[nlo, nhi], two));
                }
                let x = self.use_scalar(&args[0].op)?;
                match s.bits() {
                    8 => Ok(vec![x]),
                    16 | 32 | 64 if is_native(s.bits()) => Ok(vec![self.b.ins().bswap(x)]),
                    b => Err(TransError::unsupported(format!(
                        "llvm.bswap on i{b} is not supported"
                    ))),
                }
            }
            "ctpop" => {
                let s = self.arg_scalar_ty(args, 0)?;
                if s.is_wide() {
                    let w = self.use_wide(&args[0].op, s.bits())?;
                    let mut r = self.popcount_word(w[0]);
                    for p in &w[1..] {
                        let c = self.popcount_word(*p);
                        r = self.b.ins().iadd(r, c);
                    }
                    let zero = self.iconst(word_ty(), 0);
                    let mut out = vec![r];
                    out.resize(s.parts(), zero);
                    return Ok(self.wide_out(&out, s.bits()));
                }
                let x = self.use_scalar(&args[0].op)?;
                let xw = self.zext_to_word(x, s.bits());
                let r = self.popcount_word(xw);
                Ok(vec![self.resize_unsigned(r, word_ty(), s.clif())])
            }
            "ctlz" => {
                let s = self.arg_scalar_ty(args, 0)?;
                let wb = word_bits();
                if s.is_wide() {
                    // ctlz over two words = hi == 0 ? w + ctlz(lo) :
                    // ctlz(hi); the width adjustment below handles widths
                    // between one and two words.
                    let (lo, hi) = self.wide_halves(base, args)?;
                    let clo = self.ctlz_word(lo);
                    let chi = self.ctlz_word(hi);
                    let zero = self.iconst(word_ty(), 0);
                    let hi_zero = self.b.ins().icmp(IntCC::Equal, hi, zero);
                    let cw = self.iconst(word_ty(), wb as u64);
                    let lo_path = self.b.ins().iadd(clo, cw);
                    let r = self.b.ins().select(hi_zero, lo_path, chi);
                    let adj = self.iconst(word_ty(), (2 * wb - s.bits()) as u64);
                    let r = self.b.ins().isub(r, adj);
                    return Ok(self.wide_out(&[r, zero], s.bits()));
                }
                let x = self.use_scalar(&args[0].op)?;
                let xw = self.zext_to_word(x, s.bits());
                let r = self.ctlz_word(xw);
                // ctlz of the narrow value = ctlz of the word - (w - width)
                let adj = self.iconst(word_ty(), (wb - s.bits()) as u64);
                let r = self.b.ins().isub(r, adj);
                Ok(vec![self.resize_unsigned(r, word_ty(), s.clif())])
            }
            "cttz" => {
                let s = self.arg_scalar_ty(args, 0)?;
                if s.is_wide() {
                    let (lo, hi) = self.wide_halves(base, args)?;
                    let tlo = self.cttz_word(lo);
                    let thi = self.cttz_word(hi);
                    let zero = self.iconst(word_ty(), 0);
                    let lo_zero = self.b.ins().icmp(IntCC::Equal, lo, zero);
                    let cw = self.iconst(word_ty(), word_bits() as u64);
                    let hi_path = self.b.ins().iadd(thi, cw);
                    let r = self.b.ins().select(lo_zero, hi_path, tlo);
                    let w = self.iconst(word_ty(), s.bits() as u64);
                    let r = self.b.ins().umin(r, w);
                    return Ok(self.wide_out(&[r, zero], s.bits()));
                }
                let x = self.use_scalar(&args[0].op)?;
                let xw = self.zext_to_word(x, s.bits());
                let r = self.cttz_word(xw);
                let w = self.iconst(word_ty(), s.bits() as u64);
                let r = self.b.ins().umin(r, w);
                Ok(vec![self.resize_unsigned(r, word_ty(), s.clif())])
            }
            "bitreverse" => {
                let s = self.arg_scalar_ty(args, 0)?;
                let wb = word_bits();
                if s.is_wide() {
                    let (lo, hi) = self.wide_halves(base, args)?;
                    let rlo = self.bitreverse_word(hi);
                    let rhi = self.bitreverse_word(lo);
                    let mut r = vec![rlo, rhi];
                    if s.bits() < 2 * wb {
                        let sh = self.iconst(word_ty(), (2 * wb - s.bits()) as u64);
                        r = self.wide_lshr(&r, sh);
                    }
                    return Ok(self.wide_out(&r, s.bits()));
                }
                let x = self.use_scalar(&args[0].op)?;
                let xw = self.zext_to_word(x, s.bits());
                let r = self.bitreverse_word(xw);
                let r = if s.bits() < wb {
                    let sh = self.iconst(word_ty(), (wb - s.bits()) as u64);
                    self.b.ins().ushr(r, sh)
                } else {
                    r
                };
                Ok(vec![self.resize_unsigned(r, word_ty(), s.clif())])
            }
            "fshl" | "fshr" => {
                let s = self.arg_scalar_ty(args, 0)?;
                let left = base == "fshl";
                if s.is_wide() {
                    let two = 2 * word_bits();
                    if s.bits() != two {
                        return Err(TransError::unsupported(format!(
                            "funnel shift on i{} is not supported",
                            s.bits()
                        )));
                    }
                    let a = self.use_wide(&args[0].op, two)?;
                    let b = self.use_wide(&args[1].op, two)?;
                    let c = self.use_wide(&args[2].op, two)?;
                    let c63 = self.iconst(word_ty(), (two - 1) as u64);
                    let sh = self.b.ins().band(c[0], c63);
                    let c64 = self.iconst(word_ty(), two as u64);
                    let inv = self.b.ins().isub(c64, sh);
                    let zero = self.iconst(word_ty(), 0);
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
                    return Ok(self.wide_out(&r, two));
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
                if same_operand(&args[0].op, &args[1].op) && s.bits() <= word_bits() {
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
                            (r, self.wide_icmp(IPred::Ult, 2 * word_bits(), &a, &b))
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
                    let mut out = self.wide_out(&r, 2 * word_bits());
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
                    let mut out = self.wide_out(&r, 2 * word_bits());
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
                    let wb = word_bits();
                    let (a, b) = self.wide_args64(base, args)?;
                    let r = match base {
                        "uadd.sat" => {
                            let (r, c) = self.wide_add_carry(&a, &b);
                            let all = self.wide_const(u128::MAX >> (128 - 2 * wb), 2);
                            self.wide_select(c, &all, &r)
                        }
                        "usub.sat" => {
                            let r = self.wide_sub(&a, &b);
                            let o = self.wide_icmp(IPred::Ult, 2 * wb, &a, &b);
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
                            let c31 = self.iconst(word_ty(), (wb - 1) as u64);
                            let sign = self.b.ins().sshr(a[1], c31);
                            let max_lo = self.iconst(word_ty(), mask(wb));
                            let max_hi = self.iconst(word_ty(), mask(wb) >> 1);
                            let sat = [
                                self.b.ins().bxor(max_lo, sign),
                                self.b.ins().bxor(max_hi, sign),
                            ];
                            self.wide_select(o, &sat, &r)
                        }
                    };
                    return Ok(self.wide_out(&r, 2 * wb));
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
                    let lo = self.emit_sextend(r, 8, word_ty());
                    let c31 = self.iconst(word_ty(), (word_bits() - 1) as u64);
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
                    self.resize_unsigned(m, ms.clif(), word_ty())
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
                        let w = word_ty();
                        self.call_runtime(f, &[w, w, w], Some(w), &[dst, src, n]);
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
                        let w = word_ty();
                        let vw = self.b.ins().uextend(w, val);
                        self.call_runtime("memset", &[w, w, w], Some(w), &[dst, vw, n]);
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

    /// A length argument as a word (memory sizes are one word; a wide
    /// length only contributes its low part).
    fn length_arg(&mut self, args: &[CallArg], i: usize) -> TResult<Value> {
        let s = self.arg_scalar_ty(args, i)?;
        if s.is_wide() {
            return Ok(self.use_wide(&args[i].op, s.bits())?[0]);
        }
        let v = self.use_scalar(&args[i].op)?;
        Ok(self.resize_unsigned(v, s.clif(), word_ty()))
    }

    /// The first two arguments of a two-word intrinsic as parts (other wide
    /// widths are rejected).
    fn wide_args64(&mut self, base: &str, args: &[CallArg]) -> TResult<(Wide, Wide)> {
        let s = self.arg_scalar_ty(args, 0)?;
        let two = 2 * word_bits();
        if s.bits() != two {
            return Err(TransError::unsupported(format!(
                "llvm.{base} on i{} is not supported",
                s.bits()
            )));
        }
        let a = self.use_wide(&args[0].op, two)?;
        let b = self.use_wide(&args[1].op, two)?;
        Ok((a, b))
    }

    /// The first argument of an intrinsic on a two-part wide integer as its
    /// `(lo, hi)` halves; wider integers are rejected.
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

    /// Count of trailing zeros of a word (the word size for zero).
    fn cttz_word(&mut self, x: Value) -> Value {
        // cttz(x) = popcount((x & -x) - 1)
        let neg = self.b.ins().ineg(x);
        let low = self.b.ins().band(x, neg);
        let one = self.iconst(word_ty(), 1);
        let m = self.b.ins().isub(low, one);
        self.popcount_word(m)
    }

    /// The byte `b` repeated over a word.
    fn byte_pattern(b: u64) -> u64 {
        (b * 0x0101_0101_0101_0101) & mask(word_bits())
    }

    /// Population count of a word.
    fn popcount_word(&mut self, x: Value) -> Value {
        let w = word_ty();
        let c1 = self.iconst(w, 1);
        let m1 = self.iconst(w, Self::byte_pattern(0x55));
        let t = self.b.ins().ushr(x, c1);
        let t = self.b.ins().band(t, m1);
        let x = self.b.ins().isub(x, t);
        let c2 = self.iconst(w, 2);
        let m2 = self.iconst(w, Self::byte_pattern(0x33));
        let a = self.b.ins().band(x, m2);
        let b = self.b.ins().ushr(x, c2);
        let b = self.b.ins().band(b, m2);
        let x = self.b.ins().iadd(a, b);
        let c4 = self.iconst(w, 4);
        let m4 = self.iconst(w, Self::byte_pattern(0x0f));
        let t = self.b.ins().ushr(x, c4);
        let x = self.b.ins().iadd(x, t);
        let x = self.b.ins().band(x, m4);
        let mul = self.iconst(w, Self::byte_pattern(0x01));
        let x = self.b.ins().imul(x, mul);
        let top = self.iconst(w, (word_bits() - 8) as u64);
        self.b.ins().ushr(x, top)
    }

    /// Count of leading zeros of a word (the word size for zero).
    fn ctlz_word(&mut self, x: Value) -> Value {
        let w = word_ty();
        let wb = word_bits() as u64;
        let mut n = self.iconst(w, 0);
        let mut x = x;
        let zero = self.iconst(w, 0);
        let mut step = wb / 2;
        while step >= 1 {
            let sh = self.iconst(w, wb - step);
            let top = self.b.ins().ushr(x, sh);
            let is_zero = self.b.ins().icmp(IntCC::Equal, top, zero);
            let stepv = self.iconst(w, step);
            let add = self.b.ins().select(is_zero, stepv, zero);
            n = self.b.ins().iadd(n, add);
            let shifted = self.b.ins().ishl(x, stepv);
            x = self.b.ins().select(is_zero, shifted, x);
            step /= 2;
        }
        // After normalization the top bit is set unless x was zero.
        let c31 = self.iconst(w, wb - 1);
        let top = self.b.ins().ushr(x, c31);
        let one = self.iconst(w, 1);
        let extra = self.b.ins().isub(one, top);
        self.b.ins().iadd(n, extra)
    }

    /// Bit reversal of a word.
    fn bitreverse_word(&mut self, x: Value) -> Value {
        let w = word_ty();
        let wb = word_bits();
        let mut x = x;
        let mut shift = 1u64;
        while shift < wb as u64 {
            // The mask selecting the low half of each 2*shift-bit group.
            let group = 2 * shift;
            let unit = (1u128 << shift) - 1;
            let mut m: u128 = 0;
            let mut pos = 0;
            while pos < wb as u128 {
                m |= unit << pos;
                pos += group as u128;
            }
            let s = self.iconst(w, shift);
            let mv = self.iconst(w, m as u64);
            let hi = self.b.ins().ushr(x, s);
            let hi = self.b.ins().band(hi, mv);
            let lo = self.b.ins().band(x, mv);
            let lo = self.b.ins().ishl(lo, s);
            x = self.b.ins().bor(hi, lo);
            shift *= 2;
        }
        x
    }

    /// Multiplication with overflow detection for native widths.
    fn mul_overflow(&mut self, a: Value, b: Value, s: ScalarTy, signed: bool) -> (Value, Value) {
        let w = s.bits();
        if w >= word_bits() {
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
        // Narrower than a word: multiply in a word and check that the
        // product fits.
        let wt = word_ty();
        let (a32, b32) = if signed {
            (self.emit_sextend(a, w, wt), self.emit_sextend(b, w, wt))
        } else {
            (self.b.ins().uextend(wt, a), self.b.ins().uextend(wt, b))
        };
        let prod = self.b.ins().imul(a32, b32);
        let lo = self.b.ins().ireduce(container(w), prod);
        let back = if signed {
            self.emit_sextend(lo, w, wt)
        } else {
            self.b.ins().uextend(wt, lo)
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
