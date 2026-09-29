//! Translation of global variables into byte images with relocations.

use super::types::{truncate_const, Layout};
use super::{ClifData, DataInit, DataReloc, ModuleCtx, TResult, TransError};
use crate::llvm::*;

/// The symbol an alias ultimately refers to, if it is a plain (possibly
/// cast) reference to one.
pub(super) fn alias_target(a: &Alias) -> Option<&str> {
    super::func::peel_symbol(&a.aliasee)
}

/// The constant byte offset of a `getelementptr` constant expression.
pub(super) fn gep_const_offset(
    layout: &Layout,
    base_ty: &Type,
    indices: &[Constant],
) -> TResult<i64> {
    let mut off: i64 = 0;
    let mut cur = base_ty.clone();
    for (i, idx) in indices.iter().enumerate() {
        let k = idx
            .as_int()
            .ok_or_else(|| TransError::unsupported("non-integer getelementptr constant index"))?;
        let ibits = match layout.resolve(&idx.ty)? {
            Type::Int(b) => (*b).min(64),
            _ => {
                return Err(TransError::invalid(
                    "getelementptr index must be an integer",
                ))
            }
        };
        let k = super::types::sign_extend_const(truncate_const(k, ibits), ibits);
        if i == 0 {
            off = off.wrapping_add(k.wrapping_mul(layout.alloc_size(base_ty)? as i64));
            continue;
        }
        let resolved = layout.resolve(&cur)?.clone();
        match resolved {
            Type::Struct { fields, .. } => {
                if k < 0 || k as usize >= fields.len() {
                    return Err(TransError::invalid(format!(
                        "struct field index {k} out of range for {cur}"
                    )));
                }
                off = off.wrapping_add(layout.field_offset(&cur, k as usize)? as i64);
                cur = fields[k as usize].clone();
            }
            Type::Array(_, elem) | Type::Vector { elem, .. } => {
                off = off.wrapping_add(k.wrapping_mul(layout.alloc_size(&elem)? as i64));
                cur = (*elem).clone();
            }
            other => {
                return Err(TransError::invalid(format!(
                    "cannot index into type {other}"
                )))
            }
        }
    }
    Ok(off)
}

/// Evaluates an integer constant expression, if it is one.
fn fold_int(layout: &Layout, c: &Constant) -> TResult<Option<i128>> {
    Ok(match &c.kind {
        ConstKind::Int(v) => Some(*v),
        ConstKind::Null | ConstKind::ZeroInit | ConstKind::Undef | ConstKind::Poison => Some(0),
        ConstKind::Cast(op, inner) => {
            let Some(v) = fold_int(layout, inner)? else {
                return Ok(None);
            };
            let from_bits = int_bits(layout, &inner.ty)?;
            let to_bits = int_bits(layout, &c.ty)?;
            let v = truncate_const(v, from_bits.min(64)) as i128;
            Some(match op {
                CastOp::SExt => {
                    super::types::sign_extend_const(v as u64, from_bits.min(64)) as i128
                }
                CastOp::Trunc => truncate_const(v, to_bits.min(64)) as i128,
                _ => v,
            })
        }
        ConstKind::Binary(op, a, b) => {
            let (Some(x), Some(y)) = (fold_int(layout, a)?, fold_int(layout, b)?) else {
                return Ok(None);
            };
            let bits = int_bits(layout, &c.ty)?.min(64);
            let (xu, yu) = (truncate_const(x, bits), truncate_const(y, bits));
            let (xs, ys) = (
                super::types::sign_extend_const(xu, bits),
                super::types::sign_extend_const(yu, bits),
            );
            let r: u64 = match op {
                BinOp::Add => xu.wrapping_add(yu),
                BinOp::Sub => xu.wrapping_sub(yu),
                BinOp::Mul => xu.wrapping_mul(yu),
                BinOp::And => xu & yu,
                BinOp::Or => xu | yu,
                BinOp::Xor => xu ^ yu,
                BinOp::Shl => xu.wrapping_shl(yu as u32),
                BinOp::LShr => xu.wrapping_shr(yu as u32),
                BinOp::AShr => (xs.wrapping_shr(yu as u32)) as u64,
                BinOp::UDiv => {
                    if yu == 0 {
                        return Err(TransError::invalid(
                            "division by zero in constant expression",
                        ));
                    }
                    xu / yu
                }
                BinOp::URem => {
                    if yu == 0 {
                        return Err(TransError::invalid(
                            "division by zero in constant expression",
                        ));
                    }
                    xu % yu
                }
                BinOp::SDiv => {
                    if ys == 0 {
                        return Err(TransError::invalid(
                            "division by zero in constant expression",
                        ));
                    }
                    xs.wrapping_div(ys) as u64
                }
                BinOp::SRem => {
                    if ys == 0 {
                        return Err(TransError::invalid(
                            "division by zero in constant expression",
                        ));
                    }
                    xs.wrapping_rem(ys) as u64
                }
                _ => {
                    return Err(TransError::unsupported(
                        "floating point constant expression",
                    ))
                }
            };
            Some(truncate_const(r as i128, bits) as i128)
        }
        ConstKind::ICmp(pred, a, b) => {
            let (Some(x), Some(y)) = (fold_int(layout, a)?, fold_int(layout, b)?) else {
                return Ok(None);
            };
            let bits = int_bits(layout, &a.ty)?.min(64);
            let (xu, yu) = (truncate_const(x, bits), truncate_const(y, bits));
            let (xs, ys) = (
                super::types::sign_extend_const(xu, bits),
                super::types::sign_extend_const(yu, bits),
            );
            let r = match pred {
                IPred::Eq => xu == yu,
                IPred::Ne => xu != yu,
                IPred::Ugt => xu > yu,
                IPred::Uge => xu >= yu,
                IPred::Ult => xu < yu,
                IPred::Ule => xu <= yu,
                IPred::Sgt => xs > ys,
                IPred::Sge => xs >= ys,
                IPred::Slt => xs < ys,
                IPred::Sle => xs <= ys,
            };
            Some(r as i128)
        }
        ConstKind::Select(cond, a, b) => match fold_int(layout, cond)? {
            Some(0) => fold_int(layout, b)?,
            Some(_) => fold_int(layout, a)?,
            None => None,
        },
        _ => None,
    })
}

fn int_bits(layout: &Layout, ty: &Type) -> TResult<u32> {
    match layout.resolve(ty)? {
        Type::Int(b) => Ok(*b),
        Type::Ptr(_) => Ok(32),
        other => Err(TransError::invalid(format!(
            "expected an integer type, found {other}"
        ))),
    }
}

/// If the constant evaluates to the address of a symbol plus an offset,
/// returns (symbol, offset).
fn symbol_offset(layout: &Layout, c: &Constant) -> TResult<Option<(String, i64)>> {
    Ok(match &c.kind {
        ConstKind::Global(n) | ConstKind::DsoLocalEquivalent(n) => Some((n.clone(), 0)),
        ConstKind::Gep {
            base_ty,
            base,
            indices,
            ..
        } => match symbol_offset(layout, base)? {
            Some((n, off)) => Some((
                n,
                off.wrapping_add(gep_const_offset(layout, base_ty, indices)?),
            )),
            None => None,
        },
        ConstKind::Cast(
            CastOp::PtrToInt | CastOp::IntToPtr | CastOp::BitCast | CastOp::AddrSpaceCast,
            inner,
        ) => symbol_offset(layout, inner)?,
        ConstKind::Binary(BinOp::Add, a, b) => {
            match (symbol_offset(layout, a)?, fold_int(layout, b)?) {
                (Some((n, off)), Some(k)) => Some((n, off.wrapping_add(k as i64))),
                _ => match (fold_int(layout, a)?, symbol_offset(layout, b)?) {
                    (Some(k), Some((n, off))) => Some((n, off.wrapping_add(k as i64))),
                    _ => None,
                },
            }
        }
        ConstKind::Binary(BinOp::Sub, a, b) => {
            match (symbol_offset(layout, a)?, fold_int(layout, b)?) {
                (Some((n, off)), Some(k)) => Some((n, off.wrapping_sub(k as i64))),
                _ => None,
            }
        }
        _ => None,
    })
}

fn is_zero_constant(layout: &Layout, c: &Constant) -> TResult<bool> {
    Ok(match &c.kind {
        ConstKind::Null | ConstKind::ZeroInit | ConstKind::Undef | ConstKind::Poison => true,
        ConstKind::Int(v) => *v == 0,
        ConstKind::String(bytes) => bytes.iter().all(|b| *b == 0),
        ConstKind::Array(elems) | ConstKind::Struct(elems) | ConstKind::Vector(elems) => {
            for e in elems {
                if !is_zero_constant(layout, e)? {
                    return Ok(false);
                }
            }
            true
        }
        _ => match fold_int(layout, c)? {
            Some(v) => v == 0 && symbol_offset(layout, c)?.is_none(),
            None => false,
        },
    })
}

/// Writes constant `c` (of type `ty`) into `buf` at `offset`.
fn write_const(
    ctx: &ModuleCtx,
    c: &Constant,
    ty: &Type,
    buf: &mut [u8],
    offset: u64,
    relocs: &mut Vec<DataReloc>,
) -> TResult<()> {
    let layout = &ctx.layout;
    match &c.kind {
        ConstKind::Null
        | ConstKind::ZeroInit
        | ConstKind::Undef
        | ConstKind::Poison
        | ConstKind::None => Ok(()),
        ConstKind::Int(v) => {
            let bits = int_bits(layout, ty)?;
            if bits > 64 {
                return Err(TransError::unsupported(format!(
                    "i{bits} constants in data are not supported"
                )));
            }
            let bytes = layout.store_size(ty)? as usize;
            let val = truncate_const(*v, bits).to_le_bytes();
            buf[offset as usize..offset as usize + bytes].copy_from_slice(&val[..bytes]);
            Ok(())
        }
        ConstKind::String(bytes) => {
            let n = bytes.len().min(buf.len().saturating_sub(offset as usize));
            buf[offset as usize..offset as usize + n].copy_from_slice(&bytes[..n]);
            Ok(())
        }
        ConstKind::Array(elems) => {
            let resolved = layout.resolve(ty)?.clone();
            let Type::Array(_, elem) = resolved else {
                return Err(TransError::invalid(format!(
                    "array constant with non-array type {ty}"
                )));
            };
            let esize = layout.alloc_size(&elem)?;
            for (i, e) in elems.iter().enumerate() {
                write_const(ctx, e, &e.ty, buf, offset + i as u64 * esize, relocs)?;
            }
            Ok(())
        }
        ConstKind::Struct(elems) => {
            let resolved = layout.resolve(ty)?.clone();
            let Type::Struct { .. } = resolved else {
                return Err(TransError::invalid(format!(
                    "struct constant with non-struct type {ty}"
                )));
            };
            for (i, e) in elems.iter().enumerate() {
                let off = layout.field_offset(ty, i)?;
                write_const(ctx, e, &e.ty, buf, offset + off, relocs)?;
            }
            Ok(())
        }
        ConstKind::Vector(_) => Err(TransError::unsupported(
            "vector constants in data are not supported",
        )),
        ConstKind::Float(_) => Err(TransError::unsupported(
            "floating point constants in data are not supported",
        )),
        ConstKind::BlockAddress(..) => Err(TransError::unsupported(
            "`blockaddress` constants are not supported",
        )),
        ConstKind::InlineAsm => Err(TransError::unsupported("inline assembly is not supported")),
        _ => {
            // Global references, constant expressions.
            if let Some((symbol, addend)) = symbol_offset(layout, c)? {
                let size = layout.store_size(ty)?;
                if size != 4 {
                    return Err(TransError::unsupported(format!(
                        "a symbol address stored in a {size}-byte field is not supported"
                    )));
                }
                if !ctx.is_function_symbol(&symbol)
                    && !ctx.globals.contains_key(symbol.as_str())
                    && !ctx.aliases.contains_key(symbol.as_str())
                {
                    return Err(TransError::invalid(format!(
                        "reference to unknown global @{symbol}"
                    )));
                }
                relocs.push(DataReloc {
                    offset: offset as u32,
                    symbol,
                    addend,
                });
                return Ok(());
            }
            if let Some(v) = fold_int(layout, c)? {
                let bits = int_bits(layout, ty)?;
                let bytes = layout.store_size(ty)? as usize;
                let val = truncate_const(v, bits.min(64)).to_le_bytes();
                buf[offset as usize..offset as usize + bytes].copy_from_slice(&val[..bytes]);
                return Ok(());
            }
            Err(TransError::unsupported(
                "constant expression in a global initializer is not supported",
            ))
        }
    }
}

pub(super) fn translate_global(ctx: &ModuleCtx, g: &GlobalVar) -> TResult<ClifData> {
    if g.thread_local {
        return Err(TransError::unsupported(
            "thread-local variables are not supported",
        ));
    }
    let layout = &ctx.layout;
    let size = layout.alloc_size(&g.ty)?;
    let align = g.align.unwrap_or(0).max(layout.abi_align(&g.ty)?).max(1);
    let mut relocs = Vec::new();
    let init = match &g.init {
        None => None,
        Some(c) if is_zero_constant(layout, c)? => Some(DataInit::Zero(size)),
        Some(c) => {
            let mut buf = vec![0u8; size as usize];
            write_const(ctx, c, &g.ty, &mut buf, 0, &mut relocs)?;
            Some(DataInit::Bytes(buf))
        }
    };
    Ok(ClifData {
        name: g.name.clone(),
        linkage: g.linkage,
        writable: !g.is_const,
        align,
        init,
        relocs,
    })
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::llvm::parse_module;
    use crate::translate::{ModuleCtx, Options};

    fn data_of(src: &str, name: &str) -> ClifData {
        let m = parse_module(src).unwrap();
        let ctx = ModuleCtx::new(&m, Options::default()).unwrap();
        translate_global(&ctx, m.global(name).unwrap()).unwrap()
    }

    #[test]
    fn scalar_and_aggregate_initializers() {
        let src = r#"
target datalayout = "e-m:e-p:32:32-i64:64-n32-S128"
%struct.S = type { i8, i16, i32 }
@a = global i32 -2, align 4
@b = constant [3 x i8] c"hi\00"
@c = global %struct.S { i8 1, i16 2, i32 3 }
@z = global [4 x i32] zeroinitializer
@e = external global i32
@p = global ptr @a
@q = global ptr getelementptr (i8, ptr @b, i32 1)
@r = global i32 ptrtoint (ptr @c to i32)
@s = global [2 x ptr] [ptr null, ptr @p]
@t = global i1 true
"#;
        assert_eq!(
            data_of(src, "a").init,
            Some(DataInit::Bytes(vec![0xfe, 0xff, 0xff, 0xff]))
        );
        assert_eq!(
            data_of(src, "b").init,
            Some(DataInit::Bytes(b"hi\0".to_vec()))
        );
        let b = data_of(src, "b");
        assert!(!b.writable);
        assert_eq!(
            data_of(src, "c").init,
            Some(DataInit::Bytes(vec![1, 0, 2, 0, 3, 0, 0, 0]))
        );
        assert_eq!(data_of(src, "z").init, Some(DataInit::Zero(16)));
        assert_eq!(data_of(src, "e").init, None);
        let p = data_of(src, "p");
        assert_eq!(p.init, Some(DataInit::Bytes(vec![0; 4])));
        assert_eq!(
            p.relocs,
            vec![DataReloc {
                offset: 0,
                symbol: "a".into(),
                addend: 0
            }]
        );
        assert_eq!(
            data_of(src, "q").relocs,
            vec![DataReloc {
                offset: 0,
                symbol: "b".into(),
                addend: 1
            }]
        );
        assert_eq!(
            data_of(src, "r").relocs,
            vec![DataReloc {
                offset: 0,
                symbol: "c".into(),
                addend: 0
            }]
        );
        assert_eq!(
            data_of(src, "s").relocs,
            vec![DataReloc {
                offset: 4,
                symbol: "p".into(),
                addend: 0
            }]
        );
        assert_eq!(data_of(src, "t").init, Some(DataInit::Bytes(vec![1])));
    }
}
