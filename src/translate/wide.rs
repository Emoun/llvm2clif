//! Lowering of integers wider than 32 bits to 32-bit parts.
//!
//! The Scry backend has no 64-bit support yet, so an `iN` with `32 < N <=
//! 128` ("wide" integer) is represented by `ceil(N / 32)` Cranelift values
//! (see [`super::types`]): 32-bit parts, least significant first, the last
//! one holding the remaining bits zero-extended in its own container. This
//! module implements the arithmetic on such values with 32-bit operations.
//! It works on vectors of `i32` parts ([`Wide`]); [`FuncTranslator::wide_in`]
//! and [`FuncTranslator::wide_out`] convert between the stored parts and
//! those vectors. 64-bit division and remainder call a small helper function
//! ([`UDIVMOD64`]) that the translation adds to the module.
//!
//! Most C programs only need the two-part (64-bit) case; wider integers
//! appear when `opt` widens arithmetic to prove it overflow-free (`i65`
//! trip counts and closed-form loop results), which needs multiplication,
//! shifts, comparisons and conversions but no division.

use super::func::{verify, FuncTranslator};
use super::types::{container, mask};
use super::{ModuleCtx, TResult, TransError};
use crate::llvm::{BinOp, IPred, Operand};
use cranelift_codegen::ir::condcodes::IntCC;
use cranelift_codegen::ir::immediates::Imm64;
use cranelift_codegen::ir::{
    self, types, AbiParam, BlockArg, InstBuilder, Signature, UserFuncName, Value,
};
use cranelift_codegen::isa::CallConv;
use cranelift_frontend::{FunctionBuilder, FunctionBuilderContext};

/// A wide value as its 32-bit parts (`i32` values), least significant
/// first. The top part is zero-extended (canonical) unless it was
/// sign-extended explicitly with [`FuncTranslator::wide_sext_in`].
pub(super) type Wide = Vec<Value>;

/// The module-local helper `(n_lo, n_hi, d_lo, d_hi) -> (q_lo, q_hi, r_lo,
/// r_hi)` computing the unsigned 64-bit quotient and remainder.
pub(super) const UDIVMOD64: &str = "__llvm2clif_udivmod64";

/// The number of 32-bit parts of a `bits`-bit integer.
pub(super) fn parts_of(bits: u32) -> usize {
    bits.div_ceil(32) as usize
}

/// The width of the most significant part of a `bits`-bit integer.
fn top_bits(bits: u32) -> u32 {
    bits - 32 * (parts_of(bits) as u32 - 1)
}

impl FuncTranslator<'_, '_> {
    /// The parts of an operand of wide type (`bits` bits).
    pub(super) fn use_wide(&mut self, op: &Operand, bits: u32) -> TResult<Wide> {
        let vals = self.use_op(op)?;
        self.wide_in(&vals, bits)
    }

    /// Converts the stored parts of a `bits`-bit value to `i32` parts.
    pub(super) fn wide_in(&mut self, vals: &[Value], bits: u32) -> TResult<Wide> {
        let k = parts_of(bits);
        if vals.len() != k {
            return Err(TransError::invalid(format!(
                "expected an i{bits} value, found {} scalar(s)",
                vals.len()
            )));
        }
        let mut w = vals.to_vec();
        w[k - 1] = self.resize_unsigned(vals[k - 1], container(top_bits(bits)), types::I32);
        Ok(w)
    }

    /// Converts `i32` parts to the stored parts of a `bits`-bit value,
    /// reducing the top part to its canonical (zero-extended) form.
    pub(super) fn wide_out(&mut self, w: &[Value], bits: u32) -> Vec<Value> {
        let k = parts_of(bits);
        debug_assert_eq!(w.len(), k);
        let tb = top_bits(bits);
        let mut out = w.to_vec();
        let top = self.canon32(w[k - 1], tb);
        out[k - 1] = self.resize_unsigned(top, types::I32, container(tb));
        out
    }

    /// Masks an `i32` down to its low `k` bits (no-op for `k == 32`).
    pub(super) fn canon32(&mut self, v: Value, k: u32) -> Value {
        if k >= 32 {
            return v;
        }
        let m = self.iconst(types::I32, mask(k));
        self.b.ins().band(v, m)
    }

    /// Sign-extends the low `k` bits of an `i32` across the whole `i32`.
    pub(super) fn sext_in32(&mut self, v: Value, k: u32) -> Value {
        if k >= 32 {
            return v;
        }
        let sh = self.iconst(types::I32, (32 - k) as u64);
        let t = self.b.ins().ishl(v, sh);
        self.emit_sshr(t, sh, 32)
    }

    /// Sign-extends the top part of canonical `bits`-bit parts across its
    /// whole `i32`, making the value a two's complement number of
    /// `32 * parts` bits.
    pub(super) fn wide_sext_in(&mut self, w: &[Value], bits: u32) -> Wide {
        let mut out = w.to_vec();
        let k = out.len();
        out[k - 1] = self.sext_in32(w[k - 1], top_bits(bits));
        out
    }

    /// The constant `v` as `k` parts.
    pub(super) fn wide_const(&mut self, v: u128, k: usize) -> Wide {
        (0..k)
            .map(|i| self.iconst(types::I32, ((v >> (32 * i)) & 0xffff_ffff) as u64))
            .collect()
    }

    /// An `i8` condition as an `i32` 0/1.
    fn bool32(&mut self, c: Value) -> Value {
        self.b.ins().uextend(types::I32, c)
    }

    /// The sign bit of sign-extended parts as an `i32` 0/1.
    pub(super) fn wide_sign(&mut self, w: &[Value]) -> Value {
        let c31 = self.iconst(types::I32, 31);
        self.b.ins().ushr(w[w.len() - 1], c31)
    }

    pub(super) fn wide_select(&mut self, c: Value, a: &[Value], b: &[Value]) -> Wide {
        a.iter()
            .zip(b)
            .map(|(x, y)| self.b.ins().select(c, *x, *y))
            .collect()
    }

    pub(super) fn wide_or(&mut self, a: &[Value], b: &[Value]) -> Wide {
        a.iter()
            .zip(b)
            .map(|(x, y)| self.b.ins().bor(*x, *y))
            .collect()
    }

    pub(super) fn wide_add(&mut self, a: &[Value], b: &[Value]) -> Wide {
        self.wide_add_carry(a, b).0
    }

    /// Addition with the carry out of the top part (an `i8` 0/1).
    pub(super) fn wide_add_carry(&mut self, a: &[Value], b: &[Value]) -> (Wide, Value) {
        let mut out = Vec::with_capacity(a.len());
        let mut carry: Option<Value> = None;
        let mut carry_out = None;
        for (x, y) in a.iter().zip(b) {
            let s1 = self.b.ins().iadd(*x, *y);
            let c1 = self.b.ins().icmp(IntCC::UnsignedLessThan, s1, *x);
            let (s, c) = match carry {
                None => (s1, c1),
                Some(cin) => {
                    let s2 = self.b.ins().iadd(s1, cin);
                    let c2 = self.b.ins().icmp(IntCC::UnsignedLessThan, s2, s1);
                    (s2, self.b.ins().bor(c1, c2))
                }
            };
            out.push(s);
            carry = Some(self.bool32(c));
            carry_out = Some(c);
        }
        (out, carry_out.expect("at least one part"))
    }

    pub(super) fn wide_sub(&mut self, a: &[Value], b: &[Value]) -> Wide {
        let mut out = Vec::with_capacity(a.len());
        let mut borrow: Option<Value> = None;
        for (x, y) in a.iter().zip(b) {
            let d1 = self.b.ins().isub(*x, *y);
            let b1 = self.b.ins().icmp(IntCC::UnsignedLessThan, *x, *y);
            let (d, bo) = match borrow {
                None => (d1, b1),
                Some(bin) => {
                    let d2 = self.b.ins().isub(d1, bin);
                    let b2 = self.b.ins().icmp(IntCC::UnsignedLessThan, d1, bin);
                    (d2, self.b.ins().bor(b1, b2))
                }
            };
            out.push(d);
            borrow = Some(self.bool32(bo));
        }
        out
    }

    /// Negates (two's complement over all parts) when `neg` (an `i32` 0/1)
    /// is set.
    pub(super) fn wide_negate_if(&mut self, x: &[Value], neg: Value) -> Wide {
        // m is all ones when negating: -x == (x ^ m) - m.
        let m = self.b.ins().ineg(neg);
        let xs: Vec<Value> = x.iter().map(|v| self.b.ins().bxor(*v, m)).collect();
        let ms = vec![m; x.len()];
        self.wide_sub(&xs, &ms)
    }

    /// Signed overflow flag (`i8`) of `r = a + b` on full-width parts.
    pub(super) fn wide_sadd_overflow(&mut self, a: &[Value], b: &[Value], r: &[Value]) -> Value {
        let k = a.len() - 1;
        let x = self.b.ins().bxor(a[k], r[k]);
        let y = self.b.ins().bxor(b[k], r[k]);
        let t = self.b.ins().band(x, y);
        let c31 = self.iconst(types::I32, 31);
        let o = self.b.ins().ushr(t, c31);
        self.b.ins().ireduce(types::I8, o)
    }

    /// Signed overflow flag (`i8`) of `r = a - b` on full-width parts.
    pub(super) fn wide_ssub_overflow(&mut self, a: &[Value], b: &[Value], r: &[Value]) -> Value {
        let k = a.len() - 1;
        let x = self.b.ins().bxor(a[k], b[k]);
        let y = self.b.ins().bxor(a[k], r[k]);
        let t = self.b.ins().band(x, y);
        let c31 = self.iconst(types::I32, 31);
        let o = self.b.ins().ushr(t, c31);
        self.b.ins().ireduce(types::I8, o)
    }

    /// Multiplication modulo `2^(32 * parts)` (schoolbook on 32-bit parts).
    pub(super) fn wide_mul(&mut self, a: &[Value], b: &[Value]) -> Wide {
        let k = a.len();
        let zero = self.iconst(types::I32, 0);
        let mut r = vec![zero; k];
        for i in 0..k {
            let mut carry: Option<Value> = None;
            for j in 0..k - i {
                let lo = self.b.ins().imul(a[i], b[j]);
                if i + j + 1 == k {
                    // The last column: only its low word is kept.
                    let t = self.b.ins().iadd(lo, r[i + j]);
                    r[i + j] = match carry {
                        Some(c) => self.b.ins().iadd(t, c),
                        None => t,
                    };
                    break;
                }
                // t = a[i]*b[j] + r[i+j] + carry, which fits in 64 bits.
                let hi = self.b.ins().umulhi(a[i], b[j]);
                let lo1 = self.b.ins().iadd(lo, r[i + j]);
                let c1 = self.b.ins().icmp(IntCC::UnsignedLessThan, lo1, lo);
                let c1 = self.bool32(c1);
                let hi1 = self.b.ins().iadd(hi, c1);
                let (lo2, hi2) = match carry {
                    Some(c) => {
                        let lo2 = self.b.ins().iadd(lo1, c);
                        let c2 = self.b.ins().icmp(IntCC::UnsignedLessThan, lo2, lo1);
                        let c2 = self.bool32(c2);
                        (lo2, self.b.ins().iadd(hi1, c2))
                    }
                    None => (lo1, hi1),
                };
                r[i + j] = lo2;
                carry = Some(hi2);
            }
        }
        r
    }

    /// Unsigned 64-bit multiplication with overflow detection (`i8`).
    pub(super) fn wide_umul_overflow(&mut self, a: &[Value], b: &[Value]) -> (Wide, Value) {
        debug_assert_eq!(a.len(), 2);
        let zero = self.iconst(types::I32, 0);
        // Partial products a.lo*b.lo, a.lo*b.hi and a.hi*b.lo (a.hi*b.hi is
        // an overflow whenever both are non-zero).
        let ll_lo = self.b.ins().imul(a[0], b[0]);
        let ll_hi = self.b.ins().umulhi(a[0], b[0]);
        let lh_lo = self.b.ins().imul(a[0], b[1]);
        let lh_hi = self.b.ins().umulhi(a[0], b[1]);
        let hl_lo = self.b.ins().imul(a[1], b[0]);
        let hl_hi = self.b.ins().umulhi(a[1], b[0]);
        let a_hi_nz = self.b.ins().icmp(IntCC::NotEqual, a[1], zero);
        let b_hi_nz = self.b.ins().icmp(IntCC::NotEqual, b[1], zero);
        let hh = self.b.ins().band(a_hi_nz, b_hi_nz);
        let s1 = self.b.ins().iadd(ll_hi, lh_lo);
        let c1 = self.b.ins().icmp(IntCC::UnsignedLessThan, s1, ll_hi);
        let s2 = self.b.ins().iadd(s1, hl_lo);
        let c2 = self.b.ins().icmp(IntCC::UnsignedLessThan, s2, s1);
        let lh_nz = self.b.ins().icmp(IntCC::NotEqual, lh_hi, zero);
        let hl_nz = self.b.ins().icmp(IntCC::NotEqual, hl_hi, zero);
        let o = self.b.ins().bor(hh, lh_nz);
        let o = self.b.ins().bor(o, hl_nz);
        let o = self.b.ins().bor(o, c1);
        let o = self.b.ins().bor(o, c2);
        (vec![ll_lo, s2], o)
    }

    /// Signed 64-bit multiplication with overflow detection (`i8`).
    pub(super) fn wide_smul_overflow(&mut self, a: &[Value], b: &[Value]) -> (Wide, Value) {
        // Multiply the magnitudes; the product fits when it is at most
        // 2^63 - 1, or exactly 2^63 for a negative result.
        let a_neg = self.wide_sign(a);
        let b_neg = self.wide_sign(b);
        let aa = self.wide_negate_if(a, a_neg);
        let ab = self.wide_negate_if(b, b_neg);
        let (p, uo) = self.wide_umul_overflow(&aa, &ab);
        let r_neg = self.b.ins().bxor(a_neg, b_neg);
        let top = self.iconst(types::I32, 0x8000_0000);
        let zero = self.iconst(types::I32, 0);
        let big = self
            .b
            .ins()
            .icmp(IntCC::UnsignedGreaterThanOrEqual, p[1], top);
        let hi_is_top = self.b.ins().icmp(IntCC::Equal, p[1], top);
        let lo_is_zero = self.b.ins().icmp(IntCC::Equal, p[0], zero);
        let exact_min = self.b.ins().band(hi_is_top, lo_is_zero);
        let r_neg8 = self.b.ins().ireduce(types::I8, r_neg);
        let exact_min = self.b.ins().band(exact_min, r_neg8);
        let one8 = self.iconst(types::I8, 1);
        let not_exact_min = self.b.ins().bxor(exact_min, one8);
        let so = self.b.ins().band(big, not_exact_min);
        let o = self.b.ins().bor(uo, so);
        let r = self.wide_negate_if(&p, r_neg);
        (r, o)
    }

    /// The word and bit components of a shift amount `n` (an `i32`): the
    /// constants when `n` is known, otherwise the values `(n >> 5, n & 31)`
    /// and, for each possible word shift, the `i8` flag selecting it.
    #[allow(clippy::type_complexity)]
    fn shift_amount(
        &mut self,
        n: Value,
        k: usize,
    ) -> Result<(usize, u32), (Value, Value, Vec<Value>)> {
        if let Some(c) = self.known_const(n) {
            return Ok(((c >> 5) as usize, (c & 31) as u32));
        }
        let c5 = self.iconst(types::I32, 5);
        let c31 = self.iconst(types::I32, 31);
        let wv = self.b.ins().ushr(n, c5);
        let nl = self.b.ins().band(n, c31);
        let sel = (0..k)
            .map(|w| {
                let wc = self.iconst(types::I32, w as u64);
                self.b.ins().icmp(IntCC::Equal, wv, wc)
            })
            .collect();
        Err((wv, nl, sel))
    }

    /// Left shift by `n` (an `i32`; amounts of `32 * parts` or more give
    /// zero).
    pub(super) fn wide_shl(&mut self, a: &[Value], n: Value) -> Wide {
        let k = a.len();
        let zero = self.iconst(types::I32, 0);
        match self.shift_amount(n, k) {
            Ok((w, nl)) => (0..k)
                .map(|i| {
                    if i < w {
                        return zero;
                    }
                    let src = i - w;
                    if nl == 0 {
                        return a[src];
                    }
                    let s = self.iconst(types::I32, nl as u64);
                    let inv = self.iconst(types::I32, (32 - nl) as u64);
                    let v = self.b.ins().ishl(a[src], s);
                    if src == 0 {
                        return v;
                    }
                    let spill = self.b.ins().ushr(a[src - 1], inv);
                    self.b.ins().bor(v, spill)
                })
                .collect(),
            Err((_, nl, sel)) => {
                // The spill of the next lower part is (x >> (31 - nl)) >> 1
                // so that nl == 0 gives zero rather than a shift by 32
                // (which Cranelift reduces to a shift by 0).
                let c31 = self.iconst(types::I32, 31);
                let one = self.iconst(types::I32, 1);
                let inv = self.b.ins().isub(c31, nl);
                (0..k)
                    .map(|i| {
                        let mut acc = zero;
                        for (w, s) in sel.iter().enumerate().take(i + 1) {
                            let src = i - w;
                            let mut cand = self.b.ins().ishl(a[src], nl);
                            if src >= 1 {
                                let spill = self.b.ins().ushr(a[src - 1], inv);
                                let spill = self.b.ins().ushr(spill, one);
                                cand = self.b.ins().bor(cand, spill);
                            }
                            acc = self.b.ins().select(*s, cand, acc);
                        }
                        acc
                    })
                    .collect()
            }
        }
    }

    /// Logical right shift by `n`.
    pub(super) fn wide_lshr(&mut self, a: &[Value], n: Value) -> Wide {
        let zero = self.iconst(types::I32, 0);
        self.wide_shr(a, n, zero, false)
    }

    /// Arithmetic right shift of sign-extended parts by `n`.
    pub(super) fn wide_ashr(&mut self, a: &[Value], n: Value) -> Wide {
        let c31 = self.iconst(types::I32, 31);
        let sign = self.emit_sshr(a[a.len() - 1], c31, 32);
        self.wide_shr(a, n, sign, true)
    }

    /// Right shift by `n`, filling with `fill` (zero or the sign) and
    /// shifting the top part arithmetically when `signed`.
    fn wide_shr(&mut self, a: &[Value], n: Value, fill: Value, signed: bool) -> Wide {
        let k = a.len();
        match self.shift_amount(n, k) {
            Ok((w, nl)) => (0..k)
                .map(|i| {
                    let src = i + w;
                    if src >= k {
                        return fill;
                    }
                    if nl == 0 {
                        return a[src];
                    }
                    let s = self.iconst(types::I32, nl as u64);
                    if src + 1 == k {
                        return if signed {
                            self.emit_sshr(a[src], s, 32)
                        } else {
                            self.b.ins().ushr(a[src], s)
                        };
                    }
                    let inv = self.iconst(types::I32, (32 - nl) as u64);
                    let v = self.b.ins().ushr(a[src], s);
                    let spill = self.b.ins().ishl(a[src + 1], inv);
                    self.b.ins().bor(v, spill)
                })
                .collect(),
            Err((_, nl, sel)) => {
                let c31 = self.iconst(types::I32, 31);
                let one = self.iconst(types::I32, 1);
                let inv = self.b.ins().isub(c31, nl);
                (0..k)
                    .map(|i| {
                        let mut acc = fill;
                        for (w, s) in sel.iter().enumerate().take(k - i) {
                            let src = i + w;
                            let cand = if src + 1 == k {
                                if signed {
                                    self.emit_sshr(a[src], nl, 32)
                                } else {
                                    self.b.ins().ushr(a[src], nl)
                                }
                            } else {
                                let v = self.b.ins().ushr(a[src], nl);
                                let spill = self.b.ins().ishl(a[src + 1], inv);
                                let spill = self.b.ins().ishl(spill, one);
                                self.b.ins().bor(v, spill)
                            };
                            acc = self.b.ins().select(*s, cand, acc);
                        }
                        acc
                    })
                    .collect()
            }
        }
    }

    /// Comparison of canonical `bits`-bit parts; the result is an `i8`.
    pub(super) fn wide_icmp(&mut self, pred: IPred, bits: u32, a: &[Value], b: &[Value]) -> Value {
        let k = a.len();
        let (mut a, mut b, mut pred) = (a.to_vec(), b.to_vec(), pred);
        if pred.is_signed() {
            a = self.wide_sext_in(&a, bits);
            b = self.wide_sext_in(&b, bits);
            if self.ctx.options.signed_via_unsigned {
                a[k - 1] = self.flip_sign(a[k - 1], 32);
                b[k - 1] = self.flip_sign(b[k - 1], 32);
                pred = match pred {
                    IPred::Sgt => IPred::Ugt,
                    IPred::Sge => IPred::Uge,
                    IPred::Slt => IPred::Ult,
                    IPred::Sle => IPred::Ule,
                    p => p,
                };
            }
        }
        match pred {
            IPred::Eq => {
                let mut r = self.b.ins().icmp(IntCC::Equal, a[0], b[0]);
                for i in 1..k {
                    let e = self.b.ins().icmp(IntCC::Equal, a[i], b[i]);
                    r = self.b.ins().band(r, e);
                }
                r
            }
            IPred::Ne => {
                let mut r = self.b.ins().icmp(IntCC::NotEqual, a[0], b[0]);
                for i in 1..k {
                    let e = self.b.ins().icmp(IntCC::NotEqual, a[i], b[i]);
                    r = self.b.ins().bor(r, e);
                }
                r
            }
            _ => {
                // The most significant differing part decides. Only the top
                // part is compared as signed; the lowest part uses the full
                // (possibly non-strict) predicate.
                let (top_cc, lo_cc, mid_cc) = match pred {
                    IPred::Ult => (
                        IntCC::UnsignedLessThan,
                        IntCC::UnsignedLessThan,
                        IntCC::UnsignedLessThan,
                    ),
                    IPred::Ule => (
                        IntCC::UnsignedLessThan,
                        IntCC::UnsignedLessThanOrEqual,
                        IntCC::UnsignedLessThan,
                    ),
                    IPred::Ugt => (
                        IntCC::UnsignedGreaterThan,
                        IntCC::UnsignedGreaterThan,
                        IntCC::UnsignedGreaterThan,
                    ),
                    IPred::Uge => (
                        IntCC::UnsignedGreaterThan,
                        IntCC::UnsignedGreaterThanOrEqual,
                        IntCC::UnsignedGreaterThan,
                    ),
                    IPred::Slt => (
                        IntCC::SignedLessThan,
                        IntCC::UnsignedLessThan,
                        IntCC::UnsignedLessThan,
                    ),
                    IPred::Sle => (
                        IntCC::SignedLessThan,
                        IntCC::UnsignedLessThanOrEqual,
                        IntCC::UnsignedLessThan,
                    ),
                    IPred::Sgt => (
                        IntCC::SignedGreaterThan,
                        IntCC::UnsignedGreaterThan,
                        IntCC::UnsignedGreaterThan,
                    ),
                    IPred::Sge => (
                        IntCC::SignedGreaterThan,
                        IntCC::UnsignedGreaterThanOrEqual,
                        IntCC::UnsignedGreaterThan,
                    ),
                    IPred::Eq | IPred::Ne => unreachable!(),
                };
                let mut r = self.b.ins().icmp(lo_cc, a[0], b[0]);
                for i in 1..k {
                    let cc = if i + 1 == k { top_cc } else { mid_cc };
                    let s = self.b.ins().icmp(cc, a[i], b[i]);
                    let e = self.b.ins().icmp(IntCC::Equal, a[i], b[i]);
                    let t = self.b.ins().band(e, r);
                    r = self.b.ins().bor(s, t);
                }
                r
            }
        }
    }

    /// A binary operation on canonical `bits`-bit parts. The result may
    /// carry garbage above `bits`; callers pass it through `wide_out`.
    pub(super) fn emit_binop_wide(
        &mut self,
        op: BinOp,
        bits: u32,
        l: &[Value],
        r: &[Value],
    ) -> TResult<Wide> {
        Ok(match op {
            BinOp::Add => self.wide_add(l, r),
            BinOp::Sub => self.wide_sub(l, r),
            BinOp::Mul => self.wide_mul(l, r),
            BinOp::And => l
                .iter()
                .zip(r)
                .map(|(x, y)| self.b.ins().band(*x, *y))
                .collect(),
            BinOp::Or => self.wide_or(l, r),
            BinOp::Xor => l
                .iter()
                .zip(r)
                .map(|(x, y)| self.b.ins().bxor(*x, *y))
                .collect(),
            BinOp::Shl => self.wide_shl(l, r[0]),
            BinOp::LShr => self.wide_lshr(l, r[0]),
            BinOp::AShr => {
                let ls = self.wide_sext_in(l, bits);
                self.wide_ashr(&ls, r[0])
            }
            BinOp::UDiv => self.wide_udivmod(l, r)?.0,
            BinOp::URem => self.wide_udivmod(l, r)?.1,
            BinOp::SDiv | BinOp::SRem => {
                let ls = self.wide_sext_in(l, bits);
                let rs = self.wide_sext_in(r, bits);
                let (q, rem) = self.wide_sdivmod(&ls, &rs)?;
                if matches!(op, BinOp::SDiv) {
                    q
                } else {
                    rem
                }
            }
            BinOp::FAdd | BinOp::FSub | BinOp::FMul | BinOp::FDiv | BinOp::FRem => {
                return Err(TransError::unsupported(format!(
                    "floating point instruction `{}` is not supported",
                    op.name()
                )))
            }
        })
    }

    /// Unsigned 64-bit division: `(quotient, remainder)`, through the
    /// [`UDIVMOD64`] helper.
    pub(super) fn wide_udivmod(&mut self, n: &[Value], d: &[Value]) -> TResult<(Wide, Wide)> {
        if n.len() != 2 {
            return Err(TransError::unsupported(
                "division of integers wider than 64 bits is not supported",
            ));
        }
        self.helpers.insert(UDIVMOD64);
        let sig = udivmod64_signature(self.ctx.isa.default_call_conv());
        let fr = self.func_ref(UDIVMOD64, &sig);
        let call = self.b.ins().call(fr, &[n[0], n[1], d[0], d[1]]);
        let r = self.b.inst_results(call).to_vec();
        Ok((vec![r[0], r[1]], vec![r[2], r[3]]))
    }

    /// Signed 64-bit division of sign-extended parts: the quotient rounds
    /// towards zero and the remainder has the sign of the dividend.
    pub(super) fn wide_sdivmod(&mut self, n: &[Value], d: &[Value]) -> TResult<(Wide, Wide)> {
        let n_neg = self.wide_sign(n);
        let d_neg = self.wide_sign(d);
        let an = self.wide_negate_if(n, n_neg);
        let ad = self.wide_negate_if(d, d_neg);
        let (q, r) = self.wide_udivmod(&an, &ad)?;
        let q_neg = self.b.ins().bxor(n_neg, d_neg);
        let q = self.wide_negate_if(&q, q_neg);
        let r = self.wide_negate_if(&r, n_neg);
        Ok((q, r))
    }
}

pub(super) fn udivmod64_signature(cc: CallConv) -> Signature {
    let mut sig = Signature::new(cc);
    for _ in 0..4 {
        sig.params.push(AbiParam::new(types::I32));
    }
    for _ in 0..4 {
        sig.returns.push(AbiParam::new(types::I32));
    }
    sig
}

/// Builds the [`UDIVMOD64`] helper: restoring shift-subtract division over
/// 64 bits, with a fast path through the native 32-bit instructions when
/// both operands fit in 32 bits.
pub(super) fn build_udivmod64(ctx: &ModuleCtx) -> TResult<ir::Function> {
    let sig = udivmod64_signature(ctx.isa.default_call_conv());
    let mut func =
        ir::Function::with_name_signature(UserFuncName::testcase(UDIVMOD64.as_bytes()), sig);
    let mut fbc = FunctionBuilderContext::new();
    {
        let mut b = FunctionBuilder::new(&mut func, &mut fbc);
        let entry = b.create_block();
        let fast = b.create_block();
        // Loop state: remaining steps, dividend, quotient, remainder.
        let head = b.create_block();
        let body = b.create_block();
        let exit = b.create_block();
        b.append_block_params_for_function_params(entry);
        for _ in 0..7 {
            b.append_block_param(head, types::I32);
        }
        for _ in 0..4 {
            b.append_block_param(exit, types::I32);
        }

        b.switch_to_block(entry);
        let p = b.block_params(entry).to_vec();
        let (n_lo, n_hi, d_lo, d_hi) = (p[0], p[1], p[2], p[3]);
        let zero = b.ins().iconst(types::I32, Imm64::new(0));
        let hi_or = b.ins().bor(n_hi, d_hi);
        let small = b.ins().icmp(IntCC::Equal, hi_or, zero);
        let steps = b.ins().iconst(types::I32, Imm64::new(64));
        let init = [steps, n_lo, n_hi, zero, zero, zero, zero].map(BlockArg::Value);
        b.ins().brif(small, fast, &[], head, &init);

        b.switch_to_block(fast);
        let q = b.ins().udiv(n_lo, d_lo);
        let r = b.ins().urem(n_lo, d_lo);
        let out = [q, zero, r, zero].map(BlockArg::Value);
        b.ins().jump(exit, &out);

        b.switch_to_block(head);
        let hp = b.block_params(head).to_vec();
        let (i, nl, nh, ql, qh, rl, rh) = (hp[0], hp[1], hp[2], hp[3], hp[4], hp[5], hp[6]);
        let done = b.ins().icmp(IntCC::Equal, i, zero);
        let out = [ql, qh, rl, rh].map(BlockArg::Value);
        b.ins().brif(done, exit, &out, body, &[]);

        b.switch_to_block(body);
        let one = b.ins().iconst(types::I32, Imm64::new(1));
        let c31 = b.ins().iconst(types::I32, Imm64::new(31));
        // r = (r << 1) | (n >> 63); n <<= 1; q <<= 1.
        let n_top = b.ins().ushr(nh, c31);
        let rl_top = b.ins().ushr(rl, c31);
        let rh1 = b.ins().ishl(rh, one);
        let rh1 = b.ins().bor(rh1, rl_top);
        let rl1 = b.ins().ishl(rl, one);
        let rl1 = b.ins().bor(rl1, n_top);
        let nl_top = b.ins().ushr(nl, c31);
        let nh1 = b.ins().ishl(nh, one);
        let nh1 = b.ins().bor(nh1, nl_top);
        let nl1 = b.ins().ishl(nl, one);
        let ql_top = b.ins().ushr(ql, c31);
        let qh1 = b.ins().ishl(qh, one);
        let qh1 = b.ins().bor(qh1, ql_top);
        let ql1 = b.ins().ishl(ql, one);
        // if r >= d { r -= d; q |= 1 }
        let hi_gt = b.ins().icmp(IntCC::UnsignedGreaterThan, rh1, d_hi);
        let hi_eq = b.ins().icmp(IntCC::Equal, rh1, d_hi);
        let lo_ge = b.ins().icmp(IntCC::UnsignedGreaterThanOrEqual, rl1, d_lo);
        let t = b.ins().band(hi_eq, lo_ge);
        let ge = b.ins().bor(hi_gt, t);
        let sl = b.ins().isub(rl1, d_lo);
        let borrow = b.ins().icmp(IntCC::UnsignedLessThan, rl1, d_lo);
        let borrow = b.ins().uextend(types::I32, borrow);
        let sh = b.ins().isub(rh1, d_hi);
        let sh = b.ins().isub(sh, borrow);
        let rl2 = b.ins().select(ge, sl, rl1);
        let rh2 = b.ins().select(ge, sh, rh1);
        let ge32 = b.ins().uextend(types::I32, ge);
        let ql2 = b.ins().bor(ql1, ge32);
        let i1 = b.ins().isub(i, one);
        let next = [i1, nl1, nh1, ql2, qh1, rl2, rh2].map(BlockArg::Value);
        b.ins().jump(head, &next);

        b.switch_to_block(exit);
        let rets = b.block_params(exit).to_vec();
        b.ins().return_(&rets);
        b.seal_all_blocks();
        b.finalize(ctx.isa.frontend_config());
    }
    if ctx.options.verify {
        verify(ctx, &func)?;
    }
    Ok(func)
}
