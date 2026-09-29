//! Mapping of LLVM types onto Cranelift types for the Scry target.
//!
//! Scry (as supported by the Cranelift backend) has 8-, 16- and 32-bit
//! integers and 32-bit pointers. Every LLVM first-class value is represented
//! as a flat list of Cranelift values, one per scalar leaf of the type:
//!
//! * `iN` with `N <= 64` maps to the smallest of `i8`/`i16`/`i32`/`i64` that
//!   holds it. Widths other than 8, 16, 32 and 64 are kept *zero-extended*
//!   inside the container ("canonical form"), so unsigned operations work
//!   unchanged and signed operations sign-extend on demand.
//! * `ptr` maps to `i32`.
//! * Structs and arrays are flattened leaf by leaf (in memory order).
//! * Integers wider than 64 bits, floating point and vector types are
//!   rejected.

use crate::llvm::{DataLayout, FuncType, Module, ParamAttrs, Type};
use cranelift_codegen::ir::{types, AbiParam, ArgumentExtension, Signature};
use cranelift_codegen::isa::CallConv;

/// A scalar leaf type.
#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash)]
pub enum ScalarTy {
    /// An integer of the given width in bits (1..=32).
    Int(u32),
    /// A pointer (32 bits).
    Ptr,
}

impl ScalarTy {
    pub fn bits(self) -> u32 {
        match self {
            ScalarTy::Int(b) => b,
            ScalarTy::Ptr => 32,
        }
    }

    /// The Cranelift type holding this scalar.
    pub fn clif(self) -> types::Type {
        container(self.bits())
    }

    /// Whether the width matches its container exactly (8, 16 or 32 bits).
    pub fn is_native(self) -> bool {
        is_native(self.bits())
    }

    /// Number of bytes a load/store of this scalar accesses.
    pub fn store_bytes(self) -> u32 {
        self.bits().div_ceil(8)
    }
}

/// The Cranelift container type for an integer of `bits` bits.
pub fn container(bits: u32) -> types::Type {
    if bits <= 8 {
        types::I8
    } else if bits <= 16 {
        types::I16
    } else if bits <= 32 {
        types::I32
    } else {
        types::I64
    }
}

pub fn is_native(bits: u32) -> bool {
    matches!(bits, 8 | 16 | 32 | 64)
}

/// A scalar leaf of a flattened type together with its byte offset within
/// the enclosing value's memory layout.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct Scalar {
    pub ty: ScalarTy,
    pub offset: u64,
}

/// Type layout and flattening service for one module.
pub struct Layout<'m> {
    pub module: &'m Module,
    pub dl: DataLayout,
}

impl<'m> Layout<'m> {
    pub fn new(module: &'m Module) -> Result<Self, String> {
        let dl = match &module.datalayout {
            Some(s) => DataLayout::parse(s).map_err(|e| e.to_string())?,
            None => DataLayout::parse("e-m:e-p:32:32-i64:64-n32-S128").unwrap(),
        };
        if dl.ptr_bits != 32 {
            return Err(format!(
                "the module's data layout has {}-bit pointers; Scry needs a 32-bit target (compile with e.g. `--target=riscv32-unknown-none-elf`)",
                dl.ptr_bits
            ));
        }
        if !dl.little_endian {
            return Err("the module's data layout is big-endian; Scry is little-endian".into());
        }
        Ok(Layout { module, dl })
    }

    pub fn resolve<'a>(&'a self, ty: &'a Type) -> Result<&'a Type, String> {
        self.module
            .resolve(ty)
            .ok_or_else(|| format!("type {ty} is opaque or undefined"))
    }

    pub fn alloc_size(&self, ty: &Type) -> Result<u64, String> {
        self.dl
            .alloc_size(self.module, ty)
            .map_err(|e| e.to_string())
    }

    pub fn store_size(&self, ty: &Type) -> Result<u64, String> {
        self.dl
            .store_size(self.module, ty)
            .map_err(|e| e.to_string())
    }

    pub fn abi_align(&self, ty: &Type) -> Result<u64, String> {
        self.dl
            .abi_align(self.module, ty)
            .map_err(|e| e.to_string())
    }

    pub fn field_offset(&self, ty: &Type, idx: usize) -> Result<u64, String> {
        self.dl
            .field_offset(self.module, ty, idx)
            .map_err(|e| e.to_string())
    }

    /// The scalar leaf type of a non-aggregate type.
    pub fn scalar_of(&self, ty: &Type) -> Result<ScalarTy, String> {
        match self.resolve(ty)? {
            Type::Int(bits) => {
                if *bits > 64 {
                    Err(format!("i{bits} is not supported by the Scry backend (integers must be at most 64 bits wide)"))
                } else {
                    Ok(ScalarTy::Int(*bits))
                }
            }
            Type::Ptr(_) => Ok(ScalarTy::Ptr),
            Type::Float(_) => Err(format!(
                "floating point type {ty} is not supported by the Scry backend"
            )),
            Type::Vector { .. } => Err(format!(
                "vector type {ty} is not supported by the Scry backend"
            )),
            Type::Array(..) | Type::Struct { .. } => {
                Err(format!("expected a scalar type, found aggregate {ty}"))
            }
            other => Err(format!("type {other} cannot be used as a value")),
        }
    }

    /// Flattens a first-class type into its scalar leaves (memory order).
    pub fn flatten(&self, ty: &Type) -> Result<Vec<Scalar>, String> {
        let mut out = Vec::new();
        self.flatten_into(ty, 0, &mut out)?;
        Ok(out)
    }

    fn flatten_into(&self, ty: &Type, base: u64, out: &mut Vec<Scalar>) -> Result<(), String> {
        match self.resolve(ty)? {
            Type::Void => Ok(()),
            Type::Array(n, elem) => {
                let esize = self.alloc_size(elem)?;
                for i in 0..*n {
                    self.flatten_into(elem, base + i * esize, out)?;
                }
                Ok(())
            }
            Type::Struct { fields, .. } => {
                for (i, f) in fields.iter().enumerate() {
                    let off = self.field_offset(ty, i)?;
                    self.flatten_into(f, base + off, out)?;
                }
                Ok(())
            }
            other => {
                let s = self.scalar_of(other)?;
                out.push(Scalar {
                    ty: s,
                    offset: base,
                });
                Ok(())
            }
        }
    }

    pub fn flat_count(&self, ty: &Type) -> Result<usize, String> {
        Ok(self.flatten(ty)?.len())
    }

    /// Locates the sub-value selected by an `extractvalue`/`insertvalue`
    /// index path inside the flattened representation of `ty`: returns the
    /// start index, the number of scalars, and the selected type.
    pub fn flat_range(&self, ty: &Type, indices: &[u32]) -> Result<(usize, usize, Type), String> {
        let mut start = 0usize;
        let mut cur = ty.clone();
        for &idx in indices {
            let resolved = self.resolve(&cur)?.clone();
            match resolved {
                Type::Struct { fields, .. } => {
                    if idx as usize >= fields.len() {
                        return Err(format!("index {idx} out of range for {cur}"));
                    }
                    for f in &fields[..idx as usize] {
                        start += self.flat_count(f)?;
                    }
                    cur = fields[idx as usize].clone();
                }
                Type::Array(n, elem) => {
                    if idx as u64 >= n {
                        return Err(format!("index {idx} out of range for {cur}"));
                    }
                    start += idx as usize * self.flat_count(&elem)?;
                    cur = (*elem).clone();
                }
                other => return Err(format!("cannot index into non-aggregate type {other}")),
            }
        }
        let len = self.flat_count(&cur)?;
        Ok((start, len, cur))
    }

    /// Builds the Cranelift signature of a function type. `param_attrs`
    /// (parallel to the parameters, may be shorter) supplies `signext` /
    /// `zeroext` hints.
    pub fn signature(
        &self,
        fn_ty: &FuncType,
        param_attrs: &[ParamAttrs],
        ret_attrs: &ParamAttrs,
        call_conv: CallConv,
        honor_signext: bool,
    ) -> Result<Signature, String> {
        // Scry values carry a signedness tag, and the ABI extension attribute
        // is what tells the backend which tag a parameter or result has on
        // both sides of a call. Everything is passed with an unsigned tag so
        // that callers and callees always agree; `signext` (which clang puts
        // on sub-word signed integers) is only honored when signed
        // operations are emitted natively.
        let ext = |a: Option<&ParamAttrs>| {
            if honor_signext && a.is_some_and(|a| a.signext) {
                ArgumentExtension::Sext
            } else {
                ArgumentExtension::Uext
            }
        };
        let mut sig = Signature::new(call_conv);
        for (i, p) in fn_ty.params.iter().enumerate() {
            let leaves = self.flatten(p).map_err(|e| format!("parameter {i}: {e}"))?;
            let attrs = param_attrs.get(i);
            for leaf in leaves {
                let mut ap = AbiParam::new(leaf.ty.clif());
                ap.extension = ext(attrs);
                sig.params.push(ap);
            }
        }
        for leaf in self
            .flatten(&fn_ty.ret)
            .map_err(|e| format!("return type: {e}"))?
        {
            let mut ap = AbiParam::new(leaf.ty.clif());
            ap.extension = ext(Some(ret_attrs));
            sig.returns.push(ap);
        }
        Ok(sig)
    }
}

/// The bit mask selecting the low `bits` bits of a value.
pub fn mask(bits: u32) -> u64 {
    if bits >= 64 {
        u64::MAX
    } else {
        (1u64 << bits) - 1
    }
}

/// Truncates an integer constant to `bits` bits, as an unsigned (zero
/// extended) value.
pub fn truncate_const(v: i128, bits: u32) -> u64 {
    (v as u64) & mask(bits)
}

/// Sign extends the low `bits` bits of `v`.
pub fn sign_extend_const(v: u64, bits: u32) -> i64 {
    if bits >= 64 {
        return v as i64;
    }
    let shift = 64 - bits;
    ((v << shift) as i64) >> shift
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::llvm::parse_module;

    #[test]
    fn flattening() {
        let m = parse_module(
            "target datalayout = \"e-m:e-p:32:32-i64:64-n32-S128\"\n%struct.R = type { i8, i16, i32 }\n%struct.N = type { %struct.R, [2 x ptr] }\n",
        )
        .unwrap();
        let l = Layout::new(&m).unwrap();
        let r = Type::Named("struct.R".into());
        let f = l.flatten(&r).unwrap();
        assert_eq!(
            f,
            vec![
                Scalar {
                    ty: ScalarTy::Int(8),
                    offset: 0
                },
                Scalar {
                    ty: ScalarTy::Int(16),
                    offset: 2
                },
                Scalar {
                    ty: ScalarTy::Int(32),
                    offset: 4
                }
            ]
        );
        let n = Type::Named("struct.N".into());
        let f = l.flatten(&n).unwrap();
        assert_eq!(f.len(), 5);
        assert_eq!(
            f[3],
            Scalar {
                ty: ScalarTy::Ptr,
                offset: 8
            }
        );
        assert_eq!(
            f[4],
            Scalar {
                ty: ScalarTy::Ptr,
                offset: 12
            }
        );
        assert_eq!(l.flat_range(&n, &[1, 1]).unwrap(), (4, 1, Type::Ptr(0)));
        assert_eq!(l.flat_range(&n, &[0]).unwrap().0, 0);
        assert_eq!(l.flat_range(&n, &[0]).unwrap().1, 3);
        assert_eq!(
            l.flatten(&Type::Int(64)).unwrap(),
            vec![Scalar {
                ty: ScalarTy::Int(64),
                offset: 0
            }]
        );
        assert!(l.flatten(&Type::Int(128)).is_err());
        assert!(l
            .flatten(&Type::Float(crate::llvm::FloatKind::Float))
            .is_err());
    }

    #[test]
    fn constants_helpers() {
        assert_eq!(truncate_const(-1, 8), 0xff);
        assert_eq!(truncate_const(-1, 32), 0xffff_ffff);
        assert_eq!(truncate_const(300, 8), 44);
        assert_eq!(sign_extend_const(0xff, 8), -1);
        assert_eq!(sign_extend_const(0x7f, 8), 127);
        assert_eq!(sign_extend_const(1, 1), -1);
        assert_eq!(container(1), types::I8);
        assert_eq!(container(9), types::I16);
        assert_eq!(container(24), types::I32);
        assert_eq!(container(64), types::I64);
        assert_eq!(container(40), types::I64);
    }
}
