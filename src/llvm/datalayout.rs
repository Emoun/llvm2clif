//! Data layout: sizes, alignments and offsets of LLVM types.
//!
//! Implements the subset of the LLVM `target datalayout` specification that
//! matters for a 32-bit integer-only target, with LLVM's documented defaults
//! for anything the string does not mention.

use super::ir::{FloatKind, Module, Type};
use std::fmt;

#[derive(Clone, Debug, PartialEq, Eq)]
pub struct DataLayout {
    pub little_endian: bool,
    /// Pointer size in bits (address space 0) and ABI alignment in bits.
    pub ptr_bits: u32,
    pub ptr_align: u32,
    /// (bits, abi_align_bits) for integer types, sorted by bits.
    int_aligns: Vec<(u32, u32)>,
    /// (bits, abi_align_bits) for float types.
    float_aligns: Vec<(u32, u32)>,
    /// (bits, abi_align_bits) for vector types.
    vec_aligns: Vec<(u32, u32)>,
    /// Aggregate ABI alignment in bits.
    aggregate_align: u32,
    /// Natural stack alignment in bits (0 if unspecified).
    pub stack_align: u32,
}

#[derive(Debug)]
pub struct LayoutError(pub String);

impl fmt::Display for LayoutError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(f, "{}", self.0)
    }
}

impl std::error::Error for LayoutError {}

fn set_align(list: &mut Vec<(u32, u32)>, bits: u32, align: u32) {
    match list.binary_search_by_key(&bits, |e| e.0) {
        Ok(i) => list[i].1 = align,
        Err(i) => list.insert(i, (bits, align)),
    }
}

impl Default for DataLayout {
    fn default() -> Self {
        DataLayout {
            little_endian: true,
            ptr_bits: 64,
            ptr_align: 64,
            int_aligns: vec![(1, 8), (8, 8), (16, 16), (32, 32), (64, 32)],
            float_aligns: vec![(16, 16), (32, 32), (64, 64), (128, 128)],
            vec_aligns: vec![(64, 64), (128, 128)],
            aggregate_align: 0,
            stack_align: 0,
        }
    }
}

impl DataLayout {
    /// Parses a `target datalayout` string.
    pub fn parse(s: &str) -> Result<DataLayout, LayoutError> {
        let mut dl = DataLayout::default();
        if s.is_empty() {
            return Ok(dl);
        }
        for spec in s.split('-') {
            if spec.is_empty() {
                continue;
            }
            let (head, rest) = spec.split_at(1);
            let parts: Vec<&str> = rest.split(':').collect();
            let num = |s: &str| -> Result<u32, LayoutError> {
                s.parse::<u32>()
                    .map_err(|_| LayoutError(format!("bad number in datalayout spec `{spec}`")))
            };
            match head {
                "e" => dl.little_endian = true,
                "E" => dl.little_endian = false,
                "m" => {}
                "p" => {
                    // p[n]:<size>:<abi>[:<pref>[:<idx>]]
                    let addrspace = if parts[0].is_empty() {
                        0
                    } else {
                        num(parts[0])?
                    };
                    if addrspace == 0 {
                        if parts.len() < 3 {
                            return Err(LayoutError(format!("bad pointer spec `{spec}`")));
                        }
                        dl.ptr_bits = num(parts[1])?;
                        dl.ptr_align = num(parts[2])?;
                    }
                }
                "i" | "f" | "v" => {
                    if parts.len() < 2 {
                        return Err(LayoutError(format!("bad alignment spec `{spec}`")));
                    }
                    let bits = num(parts[0])?;
                    let abi = num(parts[1])?;
                    let list = match head {
                        "i" => &mut dl.int_aligns,
                        "f" => &mut dl.float_aligns,
                        _ => &mut dl.vec_aligns,
                    };
                    set_align(list, bits, abi);
                }
                "a" => {
                    // a:<abi>[:<pref>]
                    let abi = if parts.len() >= 2 { num(parts[1])? } else { 0 };
                    dl.aggregate_align = abi;
                }
                "S" => dl.stack_align = num(parts[0])?,
                // n (native widths), A (alloca addrspace), G, P, F, ni are
                // irrelevant to layout computation.
                _ => {}
            }
        }
        Ok(dl)
    }

    pub fn ptr_bytes(&self) -> u64 {
        (self.ptr_bits as u64).div_ceil(8)
    }

    /// ABI alignment (bytes) of an integer type of the given width.
    pub fn int_align(&self, bits: u32) -> u64 {
        // LLVM: use the alignment of the smallest specified integer type that
        // is at least as wide; if none, the largest specified one.
        let entry = self
            .int_aligns
            .iter()
            .find(|(b, _)| *b >= bits)
            .or(self.int_aligns.last());
        entry
            .map(|(_, a)| (*a as u64).div_ceil(8).max(1))
            .unwrap_or(1)
    }

    fn float_align(&self, bits: u32) -> u64 {
        self.float_aligns
            .iter()
            .find(|(b, _)| *b == bits)
            .map(|(_, a)| (*a as u64).div_ceil(8).max(1))
            .unwrap_or_else(|| self.int_align(bits))
    }

    /// Size in bytes of a value of the given type when stored in memory
    /// (the "alloc size": store size rounded up to the ABI alignment).
    pub fn alloc_size(&self, m: &Module, ty: &Type) -> Result<u64, LayoutError> {
        let (size, align) = self.size_align(m, ty)?;
        Ok(size.next_multiple_of(align))
    }

    /// Number of bytes a load/store of this type accesses (store size).
    pub fn store_size(&self, m: &Module, ty: &Type) -> Result<u64, LayoutError> {
        Ok(self.size_align(m, ty)?.0)
    }

    pub fn abi_align(&self, m: &Module, ty: &Type) -> Result<u64, LayoutError> {
        Ok(self.size_align(m, ty)?.1)
    }

    /// (store size, ABI alignment) in bytes.
    pub fn size_align(&self, m: &Module, ty: &Type) -> Result<(u64, u64), LayoutError> {
        let ty = m
            .resolve(ty)
            .ok_or_else(|| LayoutError(format!("cannot compute the layout of opaque type {ty}")))?;
        Ok(match ty {
            Type::Void => (0, 1),
            Type::Int(bits) => ((*bits as u64).div_ceil(8), self.int_align(*bits)),
            Type::Ptr(_) => (self.ptr_bytes(), (self.ptr_align as u64).div_ceil(8).max(1)),
            Type::Float(k) => {
                let bits = k.bits();
                let size = match k {
                    FloatKind::X86Fp80 => 10,
                    _ => (bits as u64).div_ceil(8),
                };
                (size, self.float_align(bits))
            }
            Type::Array(n, elem) => {
                let (esize, ealign) = self.size_align(m, elem)?;
                (esize.next_multiple_of(ealign) * n, ealign)
            }
            Type::Vector { len, elem, .. } => {
                let (esize, _) = self.size_align(m, elem)?;
                let bits = (esize * len * 8) as u32;
                let align = self
                    .vec_aligns
                    .iter()
                    .find(|(b, _)| *b == bits)
                    .map(|(_, a)| (*a as u64).div_ceil(8))
                    .unwrap_or_else(|| (bits as u64).div_ceil(8).next_power_of_two().max(1));
                (esize * len, align)
            }
            Type::Struct { fields, packed } => {
                let mut offset = 0u64;
                let mut align = 1u64;
                for f in fields {
                    let (fsize, falign) = self.size_align(m, f)?;
                    let falign = if *packed { 1 } else { falign };
                    offset = offset.next_multiple_of(falign);
                    offset += fsize.next_multiple_of(falign.max(1));
                    align = align.max(falign);
                }
                if fields.is_empty() {
                    align = align.max((self.aggregate_align as u64).div_ceil(8).max(1));
                }
                (offset.next_multiple_of(align), align)
            }
            Type::Named(_) => unreachable!("resolved above"),
            Type::Func(_) | Type::Label | Type::Metadata | Type::Token => {
                return Err(LayoutError(format!("type {ty} has no size")));
            }
            Type::X86Mmx => (8, 8),
            Type::X86Amx => (1024, 64),
        })
    }

    /// Byte offset of field `idx` of a struct type.
    pub fn field_offset(&self, m: &Module, ty: &Type, idx: usize) -> Result<u64, LayoutError> {
        let rty = m
            .resolve(ty)
            .ok_or_else(|| LayoutError(format!("cannot compute the layout of opaque type {ty}")))?;
        match rty {
            Type::Struct { fields, packed } => {
                if idx >= fields.len() {
                    return Err(LayoutError(format!(
                        "field index {idx} out of range for {ty}"
                    )));
                }
                let mut offset = 0u64;
                for (i, f) in fields.iter().enumerate() {
                    let (fsize, falign) = self.size_align(m, f)?;
                    let falign = if *packed { 1 } else { falign };
                    offset = offset.next_multiple_of(falign);
                    if i == idx {
                        return Ok(offset);
                    }
                    offset += fsize.next_multiple_of(falign.max(1));
                }
                unreachable!()
            }
            _ => Err(LayoutError(format!(
                "field offset requested on non-struct type {ty}"
            ))),
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::rc::Rc;

    fn riscv32() -> DataLayout {
        DataLayout::parse("e-m:e-p:32:32-i64:64-n32-S128").unwrap()
    }

    #[test]
    fn scalar_sizes() {
        let dl = riscv32();
        let m = Module::default();
        assert_eq!(dl.size_align(&m, &Type::Int(1)).unwrap(), (1, 1));
        assert_eq!(dl.size_align(&m, &Type::Int(8)).unwrap(), (1, 1));
        assert_eq!(dl.size_align(&m, &Type::Int(16)).unwrap(), (2, 2));
        assert_eq!(dl.size_align(&m, &Type::Int(24)).unwrap(), (3, 4));
        assert_eq!(dl.alloc_size(&m, &Type::Int(24)).unwrap(), 4);
        assert_eq!(dl.size_align(&m, &Type::Int(32)).unwrap(), (4, 4));
        assert_eq!(dl.size_align(&m, &Type::Int(64)).unwrap(), (8, 8));
        assert_eq!(dl.size_align(&m, &Type::Ptr(0)).unwrap(), (4, 4));
        assert_eq!(dl.stack_align, 128);
    }

    #[test]
    fn struct_layout() {
        let dl = riscv32();
        let mut m = Module::default();
        // { i8, i16, i32 } -> offsets 0, 2, 4; size 8, align 4
        let s = Type::Struct {
            fields: vec![Type::Int(8), Type::Int(16), Type::Int(32)],
            packed: false,
        };
        assert_eq!(dl.size_align(&m, &s).unwrap(), (8, 4));
        assert_eq!(dl.field_offset(&m, &s, 1).unwrap(), 2);
        assert_eq!(dl.field_offset(&m, &s, 2).unwrap(), 4);
        // packed
        let p = Type::Struct {
            fields: vec![Type::Int(8), Type::Int(32)],
            packed: true,
        };
        assert_eq!(dl.size_align(&m, &p).unwrap(), (5, 1));
        // named
        m.types.insert(
            "struct.P".into(),
            Some(Type::Struct {
                fields: vec![Type::Int(32), Type::Int(32)],
                packed: false,
            }),
        );
        let n = Type::Named(Rc::from("struct.P"));
        assert_eq!(dl.size_align(&m, &n).unwrap(), (8, 4));
        let arr = Type::Array(3, Rc::new(n.clone()));
        assert_eq!(dl.size_align(&m, &arr).unwrap(), (24, 4));
        // array of i8 inside struct with padding at end: { i8, [3 x i8], i32 }
        let s2 = Type::Struct {
            fields: vec![
                Type::Int(8),
                Type::Array(3, Rc::new(Type::Int(8))),
                Type::Int(32),
            ],
            packed: false,
        };
        assert_eq!(dl.size_align(&m, &s2).unwrap(), (8, 4));
        assert_eq!(dl.field_offset(&m, &s2, 2).unwrap(), 4);
    }
}
