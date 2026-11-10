//! Metrics for RISC-V (RV32IMC or RV64IMC) programs: static from the ELF file,
//! dynamic from a qemu execution log.
//!
//! ```sh
//! qemu-riscv32 -d exec,in_asm,nochain prog.elf 2>&1 >/dev/null | scrymetrics rv32
//! qemu-riscv64 -d exec,in_asm,nochain prog.elf 2>&1 >/dev/null | scrymetrics rv64
//! ```
//!
//! The `in_asm` blocks list every translated block (address and encoding of
//! each instruction), the `Trace` lines give the order in which the blocks
//! executed, and `nochain` makes qemu report every block execution. Only the
//! encodings are used: the instructions are decoded here, so the metrics do
//! not depend on the disassembler's spelling. The register data flow is
//! derived from the decoded operands, which gives the same lifetime and
//! occupancy figures as the Scry pass without needing register values.

use crate::{json_map, Hist};
use object::{Object, ObjectSection};
use std::collections::{BTreeMap, HashMap};

#[derive(Clone, Copy, PartialEq, Eq)]
enum Class {
    Alu,
    Constant,
    Move,
    Control,
    Load,
    Store,
    Nop,
    System,
    Other,
}

impl Class {
    fn name(self) -> &'static str {
        match self {
            Class::Alu => "alu",
            Class::Constant => "constant",
            Class::Move => "move",
            Class::Control => "control",
            Class::Load | Class::Store => "memory",
            Class::Nop => "nop",
            Class::System => "system",
            Class::Other => "other",
        }
    }
}

#[derive(Clone, Copy, PartialEq, Eq)]
enum Ctl {
    None,
    Branch,
    Jump,
    Call,
    Return,
}

/// A decoded instruction: what the metrics need and nothing else.
#[derive(Clone, Copy)]
struct Insn {
    pc: u32,
    size: u8,
    class: Class,
    ctl: Ctl,
    mnemonic: &'static str,
    /// Destination and source registers; 0 means none (or `x0`).
    rd: u8,
    rs1: u8,
    rs2: u8,
    /// Width of a memory access in bytes.
    bytes: u8,
    /// `Some(d)` when the instruction adds the constant `d` to `sp`.
    sp_delta: Option<i32>,
}

const SP: u8 = 2;
const RA: u8 = 1;

fn sext(v: u32, bits: u32) -> i32 {
    ((v << (32 - bits)) as i32) >> (32 - bits)
}

fn insn(pc: u32, size: u8, class: Class, mnemonic: &'static str) -> Insn {
    Insn {
        pc,
        size,
        class,
        ctl: Ctl::None,
        mnemonic,
        rd: 0,
        rs1: 0,
        rs2: 0,
        bytes: 0,
        sp_delta: None,
    }
}

fn control(mut i: Insn, ctl: Ctl) -> Insn {
    i.class = Class::Control;
    i.ctl = ctl;
    i
}

fn jump_kind(rd: u8, rs1: u8) -> Ctl {
    if rd == RA {
        Ctl::Call
    } else if rd == 0 && rs1 == RA {
        Ctl::Return
    } else {
        Ctl::Jump
    }
}

fn decode32(pc: u32, w: u32, rv64: bool) -> Insn {
    let opcode = w & 0x7f;
    let rd = ((w >> 7) & 0x1f) as u8;
    let funct3 = (w >> 12) & 7;
    let rs1 = ((w >> 15) & 0x1f) as u8;
    let rs2 = ((w >> 20) & 0x1f) as u8;
    let funct7 = w >> 25;
    let mut i = insn(pc, 4, Class::Other, "unknown");
    match opcode {
        0x37 => {
            i = insn(pc, 4, Class::Constant, "lui");
            i.rd = rd;
        }
        0x17 => {
            i = insn(pc, 4, Class::Constant, "auipc");
            i.rd = rd;
        }
        0x6f => {
            i = control(insn(pc, 4, Class::Control, "jal"), jump_kind(rd, 0));
            i.rd = rd;
        }
        0x67 => {
            i = control(insn(pc, 4, Class::Control, "jalr"), jump_kind(rd, rs1));
            i.rd = rd;
            i.rs1 = rs1;
        }
        0x63 => {
            let m = match funct3 {
                0 => "beq",
                1 => "bne",
                4 => "blt",
                5 => "bge",
                6 => "bltu",
                7 => "bgeu",
                _ => "unknown",
            };
            i = control(insn(pc, 4, Class::Control, m), Ctl::Branch);
            i.rs1 = rs1;
            i.rs2 = rs2;
        }
        0x03 => {
            let (m, bytes) = match funct3 {
                0 => ("lb", 1),
                1 => ("lh", 2),
                2 => ("lw", 4),
                3 if rv64 => ("ld", 8),
                4 => ("lbu", 1),
                5 => ("lhu", 2),
                6 if rv64 => ("lwu", 4),
                _ => ("unknown", 0),
            };
            i = insn(pc, 4, Class::Load, m);
            i.rd = rd;
            i.rs1 = rs1;
            i.bytes = bytes;
        }
        0x23 => {
            let (m, bytes) = match funct3 {
                0 => ("sb", 1),
                1 => ("sh", 2),
                2 => ("sw", 4),
                3 if rv64 => ("sd", 8),
                _ => ("unknown", 0),
            };
            i = insn(pc, 4, Class::Store, m);
            i.rs1 = rs1;
            i.rs2 = rs2;
            i.bytes = bytes;
        }
        0x13 => {
            let imm = sext(w >> 20, 12);
            let (class, m) = match funct3 {
                0 if rd == 0 && rs1 == 0 => (Class::Nop, "nop"),
                0 if rs1 == 0 => (Class::Constant, "li"),
                0 if imm == 0 => (Class::Move, "mv"),
                0 => (Class::Alu, "addi"),
                1 => (Class::Alu, "slli"),
                2 => (Class::Alu, "slti"),
                3 => (Class::Alu, "sltiu"),
                4 => (Class::Alu, "xori"),
                5 if funct7 & 0x20 != 0 => (Class::Alu, "srai"),
                5 => (Class::Alu, "srli"),
                6 => (Class::Alu, "ori"),
                _ => (Class::Alu, "andi"),
            };
            i = insn(pc, 4, class, m);
            i.rd = rd;
            i.rs1 = rs1;
            if m == "addi" && rd == SP && rs1 == SP {
                i.sp_delta = Some(imm);
            }
        }
        0x33 => {
            let m = if funct7 == 1 {
                match funct3 {
                    0 => "mul",
                    1 => "mulh",
                    2 => "mulhsu",
                    3 => "mulhu",
                    4 => "div",
                    5 => "divu",
                    6 => "rem",
                    _ => "remu",
                }
            } else {
                match funct3 {
                    0 if funct7 & 0x20 != 0 => "sub",
                    0 => "add",
                    1 => "sll",
                    2 => "slt",
                    3 => "sltu",
                    4 => "xor",
                    5 if funct7 & 0x20 != 0 => "sra",
                    5 => "srl",
                    6 => "or",
                    _ => "and",
                }
            };
            i = insn(pc, 4, Class::Alu, m);
            i.rd = rd;
            i.rs1 = rs1;
            i.rs2 = rs2;
        }
        0x1b if rv64 => {
            // OP-IMM-32: addiw, slliw, srliw, sraiw.
            let imm = sext(w >> 20, 12);
            let (class, m) = match funct3 {
                0 if rs1 == 0 => (Class::Constant, "li"),
                0 if imm == 0 => (Class::Alu, "sext.w"),
                0 => (Class::Alu, "addiw"),
                1 => (Class::Alu, "slliw"),
                5 if funct7 & 0x20 != 0 => (Class::Alu, "sraiw"),
                5 => (Class::Alu, "srliw"),
                _ => (Class::Other, "unknown"),
            };
            i = insn(pc, 4, class, m);
            i.rd = rd;
            i.rs1 = rs1;
        }
        0x3b if rv64 => {
            // OP-32: addw, subw, sllw, srlw, sraw and the M extension's *w.
            let m = if funct7 == 1 {
                match funct3 {
                    0 => "mulw",
                    4 => "divw",
                    5 => "divuw",
                    6 => "remw",
                    7 => "remuw",
                    _ => "unknown",
                }
            } else {
                match funct3 {
                    0 if funct7 & 0x20 != 0 => "subw",
                    0 => "addw",
                    1 => "sllw",
                    5 if funct7 & 0x20 != 0 => "sraw",
                    5 => "srlw",
                    _ => "unknown",
                }
            };
            i = insn(
                pc,
                4,
                if m == "unknown" {
                    Class::Other
                } else {
                    Class::Alu
                },
                m,
            );
            i.rd = rd;
            i.rs1 = rs1;
            i.rs2 = rs2;
        }
        0x0f => i = insn(pc, 4, Class::System, "fence"),
        0x73 => {
            let m = match w {
                0x73 => "ecall",
                0x0010_0073 => "ebreak",
                _ => "csr",
            };
            i = insn(pc, 4, Class::System, m);
            if m == "csr" {
                i.rd = rd;
                i.rs1 = rs1;
            }
        }
        // Floating point (loads, stores, arithmetic) and atomics: counted as
        // `other` under their own mnemonics, so that a program that was not
        // supposed to use them shows it.
        0x07 => i = insn(pc, 4, Class::Other, "fload"),
        0x27 => i = insn(pc, 4, Class::Other, "fstore"),
        0x43 | 0x47 | 0x4b | 0x4f | 0x53 => i = insn(pc, 4, Class::Other, "fp"),
        0x2f => i = insn(pc, 4, Class::Other, "atomic"),
        _ => {}
    }
    i
}

fn decode16(pc: u32, w: u32, rv64: bool) -> Insn {
    let op = w & 3;
    let f3 = (w >> 13) & 7;
    let rs1p = 8 + ((w >> 7) & 7) as u8;
    let rs2p = 8 + ((w >> 2) & 7) as u8;
    let rd = ((w >> 7) & 0x1f) as u8;
    let rs2 = ((w >> 2) & 0x1f) as u8;
    let bit12 = (w >> 12) & 1;
    let mut i = insn(pc, 2, Class::Other, "unknown");
    match (op, f3) {
        (0, 0) if w != 0 => {
            // c.addi4spn
            i = insn(pc, 2, Class::Alu, "addi");
            i.rd = rs2p;
            i.rs1 = SP;
        }
        (0, 2) => {
            i = insn(pc, 2, Class::Load, "lw");
            i.rd = rs2p;
            i.rs1 = rs1p;
            i.bytes = 4;
        }
        (0, 6) => {
            i = insn(pc, 2, Class::Store, "sw");
            i.rs2 = rs2p;
            i.rs1 = rs1p;
            i.bytes = 4;
        }
        (0, 3) if rv64 => {
            i = insn(pc, 2, Class::Load, "ld");
            i.rd = rs2p;
            i.rs1 = rs1p;
            i.bytes = 8;
        }
        (0, 7) if rv64 => {
            i = insn(pc, 2, Class::Store, "sd");
            i.rs2 = rs2p;
            i.rs1 = rs1p;
            i.bytes = 8;
        }
        (0, 1) | (0, 5) => {
            i = insn(
                pc,
                2,
                Class::Other,
                if f3 == 1 { "fload" } else { "fstore" },
            )
        }
        (0, 3) | (0, 7) => {
            i = insn(
                pc,
                2,
                Class::Other,
                if f3 == 3 { "fload" } else { "fstore" },
            )
        }
        (1, 0) => {
            if rd == 0 {
                i = insn(pc, 2, Class::Nop, "nop");
            } else {
                i = insn(pc, 2, Class::Alu, "addi");
                i.rd = rd;
                i.rs1 = rd;
                if rd == SP {
                    i.sp_delta = Some(sext((bit12 << 5) | ((w >> 2) & 0x1f), 6));
                }
            }
        }
        (1, 1) if rv64 => {
            // c.addiw
            i = insn(pc, 2, Class::Alu, if rd == 0 { "unknown" } else { "addiw" });
            i.rd = rd;
            i.rs1 = rd;
        }
        (1, 1) => {
            i = control(insn(pc, 2, Class::Control, "jal"), Ctl::Call);
            i.rd = RA;
        }
        (1, 2) => {
            i = insn(pc, 2, Class::Constant, "li");
            i.rd = rd;
        }
        (1, 3) => {
            if rd == SP {
                // c.addi16sp
                i = insn(pc, 2, Class::Alu, "addi");
                i.rd = SP;
                i.rs1 = SP;
                let imm = (bit12 << 9)
                    | (((w >> 6) & 1) << 4)
                    | (((w >> 5) & 1) << 6)
                    | (((w >> 3) & 3) << 7)
                    | (((w >> 2) & 1) << 5);
                i.sp_delta = Some(sext(imm, 10));
            } else {
                i = insn(pc, 2, Class::Constant, "lui");
                i.rd = rd;
            }
        }
        (1, 4) => {
            let m = match (w >> 10) & 3 {
                0 => "srli",
                1 => "srai",
                2 => "andi",
                _ if bit12 == 1 && rv64 => match (w >> 5) & 3 {
                    0 => "subw",
                    1 => "addw",
                    _ => "unknown",
                },
                _ if bit12 == 1 => "unknown",
                _ => match (w >> 5) & 3 {
                    0 => "sub",
                    1 => "xor",
                    2 => "or",
                    _ => "and",
                },
            };
            i = insn(
                pc,
                2,
                if m == "unknown" {
                    Class::Other
                } else {
                    Class::Alu
                },
                m,
            );
            i.rd = rs1p;
            i.rs1 = rs1p;
            if matches!(m, "sub" | "xor" | "or" | "and" | "subw" | "addw") {
                i.rs2 = rs2p;
            }
        }
        (1, 5) => i = control(insn(pc, 2, Class::Control, "jal"), Ctl::Jump),
        (1, 6) | (1, 7) => {
            i = control(
                insn(pc, 2, Class::Control, if f3 == 6 { "beq" } else { "bne" }),
                Ctl::Branch,
            );
            i.rs1 = rs1p;
        }
        (2, 0) => {
            i = insn(pc, 2, Class::Alu, "slli");
            i.rd = rd;
            i.rs1 = rd;
        }
        (2, 2) => {
            i = insn(pc, 2, Class::Load, "lw");
            i.rd = rd;
            i.rs1 = SP;
            i.bytes = 4;
        }
        (2, 4) => {
            if bit12 == 0 {
                if rs2 == 0 {
                    // c.jr
                    i = control(insn(pc, 2, Class::Control, "jalr"), jump_kind(0, rd));
                    i.rs1 = rd;
                } else {
                    i = insn(pc, 2, Class::Move, "mv");
                    i.rd = rd;
                    i.rs2 = rs2;
                }
            } else if rs2 == 0 {
                if rd == 0 {
                    i = insn(pc, 2, Class::System, "ebreak");
                } else {
                    // c.jalr
                    i = control(insn(pc, 2, Class::Control, "jalr"), Ctl::Call);
                    i.rd = RA;
                    i.rs1 = rd;
                }
            } else {
                i = insn(pc, 2, Class::Alu, "add");
                i.rd = rd;
                i.rs1 = rd;
                i.rs2 = rs2;
            }
        }
        (2, 6) => {
            i = insn(pc, 2, Class::Store, "sw");
            i.rs2 = rs2;
            i.rs1 = SP;
            i.bytes = 4;
        }
        (2, 3) if rv64 => {
            // c.ldsp
            i = insn(pc, 2, Class::Load, "ld");
            i.rd = rd;
            i.rs1 = SP;
            i.bytes = 8;
        }
        (2, 7) if rv64 => {
            // c.sdsp
            i = insn(pc, 2, Class::Store, "sd");
            i.rs2 = rs2;
            i.rs1 = SP;
            i.bytes = 8;
        }
        (2, 1) | (2, 5) => {
            i = insn(
                pc,
                2,
                Class::Other,
                if f3 == 1 { "fload" } else { "fstore" },
            )
        }
        (2, 3) | (2, 7) => {
            i = insn(
                pc,
                2,
                Class::Other,
                if f3 == 3 { "fload" } else { "fstore" },
            )
        }
        _ => {}
    }
    i
}

const CLASSES: [Class; 9] = [
    Class::Alu,
    Class::Constant,
    Class::Move,
    Class::Control,
    Class::Load,
    Class::Store,
    Class::Nop,
    Class::System,
    Class::Other,
];

fn categories(by_class: &[u64; 9]) -> BTreeMap<String, u64> {
    let mut m: BTreeMap<String, u64> = BTreeMap::new();
    for (class, n) in CLASSES.iter().zip(by_class) {
        if *n > 0 {
            *m.entry(class.name().to_string()).or_default() += n;
        }
    }
    m
}

/// `scrymetrics rv32-static <elf>` / `rv64-static`: decodes the `.text*`
/// sections of a RISC-V ELF file and counts the instructions by mnemonic,
/// category and size.
pub fn run_static(path: &str, rv64: bool) -> Result<String, String> {
    let data = std::fs::read(path).map_err(|e| format!("{path}: {e}"))?;
    let file = object::File::parse(&*data).map_err(|e| format!("{path}: {e}"))?;
    let mut by_mnemonic: HashMap<&'static str, u64> = HashMap::new();
    let mut by_class = [0u64; 9];
    let (mut count, mut compressed, mut bytes) = (0u64, 0u64, 0u64);
    for section in file.sections() {
        if !section.name().unwrap_or("").starts_with(".text") {
            continue;
        }
        let code = section.data().map_err(|e| format!("{path}: {e}"))?;
        bytes += code.len() as u64;
        let mut off = 0;
        while off + 2 <= code.len() {
            let lo = u16::from_le_bytes([code[off], code[off + 1]]) as u32;
            let i = if lo & 3 != 3 {
                off += 2;
                decode16(0, lo, rv64)
            } else {
                if off + 4 > code.len() {
                    break;
                }
                let w =
                    u32::from_le_bytes([code[off], code[off + 1], code[off + 2], code[off + 3]]);
                off += 4;
                decode32(0, w, rv64)
            };
            count += 1;
            if i.size == 2 {
                compressed += 1;
            }
            by_class[i.class as usize] += 1;
            *by_mnemonic.entry(i.mnemonic).or_default() += 1;
        }
    }
    let by_mnemonic: BTreeMap<String, u64> = by_mnemonic
        .iter()
        .map(|(k, v)| (k.to_string(), *v))
        .collect();
    Ok(format!(
        "{{\"file\": \"{}\", \"text_bytes\": {}, \"instructions\": {}, \"compressed\": {}, \"by_category\": {}, \"by_mnemonic\": {}}}",
        path,
        bytes,
        count,
        compressed,
        json_map(&categories(&by_class)),
        json_map(&by_mnemonic)
    ))
}

#[derive(Default)]
pub struct Rv32 {
    /// Decode RV64 (`ld`, `sd`, the `*w` operations and their compressed
    /// forms) instead of RV32.
    rv64: bool,
    arena: Vec<Insn>,
    /// Translated blocks by start address: (index into `arena`, length).
    blocks: HashMap<u32, (u32, u32)>,
    in_block: bool,
    block_start: Option<(u32, usize)>,
    traces: u64,
    missing_blocks: u64,

    steps: u64,
    by_mnemonic: HashMap<&'static str, u64>,
    by_class: [u64; 9],
    compressed: u64,
    branches: u64,
    branches_taken: u64,
    jumps: u64,
    calls: u64,
    returns: u64,
    /// After a conditional branch: the address of the next instruction if it
    /// is not taken.
    fallthrough: Option<u32>,
    /// The register the previous instruction wrote with `lui`, `auipc` or
    /// `li`: an `addi` on it completes a constant or address, a load from it
    /// reads a constant pool (Cranelift's riscv64 backend keeps 64-bit
    /// constants and symbol addresses in the code), so both count as
    /// constant materialization rather than arithmetic or memory traffic.
    const_base: Option<u8>,
    pool_loads: u64,

    // Register values: position of the current definition and of its last use.
    def_pos: [Option<u64>; 32],
    last_use: [Option<u64>; 32],
    defined: u64,
    discarded: u64,
    live_at_end: u64,
    lifetime: Hist,
    discarded_lifetime: Hist,
    /// Occupancy difference array, one entry per executed instruction.
    occupancy_diff: Vec<i8>,
    occupancy: Hist,

    block_len: u64,
    block_lengths: Hist,

    loads: u64,
    stores: u64,
    load_bytes: u64,
    store_bytes: u64,
    stack_loads: u64,
    stack_stores: u64,
    /// `sp` relative to its initial value; `None` once an instruction wrote it
    /// in a way that cannot be followed without register values.
    sp: Option<i64>,
    sp_known: bool,
    sp_min: i64,
}

impl Rv32 {
    pub fn new(rv64: bool) -> Self {
        Rv32 {
            rv64,
            sp: Some(0),
            sp_known: true,
            ..Default::default()
        }
    }

    fn close_block(&mut self) {
        if let Some((pc, start)) = self.block_start.take() {
            let len = self.arena.len() - start;
            self.blocks.insert(pc, (start as u32, len as u32));
        }
        self.in_block = false;
    }

    pub fn line(&mut self, line: &str) {
        let line = line.trim_end();
        if line.starts_with("IN:") {
            self.close_block();
            self.in_block = true;
            return;
        }
        if self.in_block {
            if let Some(rest) = line.strip_prefix("0x") {
                let mut parts = rest.split_whitespace();
                let pc = parts
                    .next()
                    .and_then(|p| u32::from_str_radix(p.trim_end_matches(':'), 16).ok());
                let enc = parts.next();
                if let (Some(pc), Some(enc)) = (pc, enc) {
                    if let Ok(w) = u32::from_str_radix(enc, 16) {
                        let decoded = if enc.len() <= 4 {
                            decode16(pc, w, self.rv64)
                        } else {
                            decode32(pc, w, self.rv64)
                        };
                        if self.block_start.is_none() {
                            self.block_start = Some((pc, self.arena.len()));
                        }
                        self.arena.push(decoded);
                        return;
                    }
                }
            }
            // Anything else ends the block (normally the empty line after it).
            self.close_block();
        }
        if let Some(rest) = line.strip_prefix("Trace ") {
            self.traces += 1;
            let pc = rest
                .find('[')
                .and_then(|o| rest[o + 1..].split('/').nth(1))
                .and_then(|p| u64::from_str_radix(p, 16).ok());
            match pc.and_then(|pc| self.blocks.get(&(pc as u32)).copied()) {
                Some((start, len)) => {
                    for k in start..start + len {
                        let i = self.arena[k as usize];
                        self.step(i);
                    }
                }
                None => self.missing_blocks += 1,
            }
        }
    }

    fn step(&mut self, mut i: Insn) {
        let t = self.steps;
        self.steps += 1;
        self.occupancy_diff.push(0);
        // The second half of a two-instruction constant or address.
        let base = self.const_base.take();
        let continues = base
            .is_some_and(|b| b == i.rs1 && (i.mnemonic == "addi" || i.mnemonic == "addiw"))
            || (base.is_some_and(|b| b == i.rs1) && i.class == Class::Load && i.rd != SP);
        if continues {
            if i.class == Class::Load {
                self.pool_loads += 1;
                i.mnemonic = "ld.pool";
            }
            i.class = Class::Constant;
        }
        if matches!(i.mnemonic, "lui" | "auipc" | "li") && i.rd != 0 {
            self.const_base = Some(i.rd);
        }
        self.by_class[i.class as usize] += 1;
        *self.by_mnemonic.entry(i.mnemonic).or_default() += 1;
        if i.size == 2 {
            self.compressed += 1;
        }
        if let Some(ft) = self.fallthrough.take() {
            if i.pc != ft {
                self.branches_taken += 1;
            }
        }
        for r in [i.rs1, i.rs2] {
            if r != 0 {
                self.last_use[r as usize] = Some(t);
            }
        }
        if i.rd != 0 {
            let r = i.rd as usize;
            if let Some(d) = self.def_pos[r] {
                match self.last_use[r] {
                    Some(u) => {
                        self.lifetime.add((u - d) as usize);
                        self.occupancy_diff[d as usize] += 1;
                        self.occupancy_diff[u as usize] -= 1;
                    }
                    None => {
                        self.discarded += 1;
                        self.discarded_lifetime.add((t - d) as usize);
                        self.occupancy_diff[d as usize] += 1;
                        self.occupancy_diff[t as usize] -= 1;
                    }
                }
            }
            self.defined += 1;
            self.def_pos[r] = Some(t);
            self.last_use[r] = None;
            if i.rd == SP {
                match (i.sp_delta, self.sp) {
                    (Some(d), Some(sp)) => {
                        let sp = sp + d as i64;
                        self.sp = Some(sp);
                        self.sp_min = self.sp_min.min(sp);
                    }
                    _ => {
                        self.sp = None;
                        self.sp_known = false;
                    }
                }
            }
        }
        match i.class {
            Class::Load => {
                self.loads += 1;
                self.load_bytes += i.bytes as u64;
                if i.rs1 == SP {
                    self.stack_loads += 1;
                }
            }
            Class::Store => {
                self.stores += 1;
                self.store_bytes += i.bytes as u64;
                if i.rs1 == SP {
                    self.stack_stores += 1;
                }
            }
            _ => {}
        }
        self.block_len += 1;
        if i.ctl != Ctl::None {
            self.block_lengths.add(self.block_len as usize);
            self.block_len = 0;
            match i.ctl {
                Ctl::Branch => {
                    self.branches += 1;
                    self.fallthrough = Some(i.pc + i.size as u32);
                }
                Ctl::Jump => self.jumps += 1,
                Ctl::Call => self.calls += 1,
                Ctl::Return => self.returns += 1,
                Ctl::None => {}
            }
        }
    }

    pub fn finish(&mut self) {
        self.close_block();
        for r in 1..32 {
            if let Some(d) = self.def_pos[r] {
                match self.last_use[r] {
                    Some(u) => {
                        self.lifetime.add((u - d) as usize);
                        self.occupancy_diff[d as usize] += 1;
                        self.occupancy_diff[u as usize] -= 1;
                    }
                    None => {
                        self.live_at_end += 1;
                        self.occupancy_diff[d as usize] += 1;
                    }
                }
            }
        }
        let mut live: i64 = 0;
        for &d in &self.occupancy_diff {
            live += d as i64;
            self.occupancy.add(live.max(0) as usize);
        }
    }

    pub fn json(&self) -> String {
        let by_category = categories(&self.by_class);
        let by_mnemonic: BTreeMap<String, u64> = self
            .by_mnemonic
            .iter()
            .map(|(k, v)| (k.to_string(), *v))
            .collect();
        let mut forms: BTreeMap<String, u64> = BTreeMap::new();
        forms.insert("compressed".into(), self.compressed);
        forms.insert("branches".into(), self.branches);
        forms.insert("branches_taken".into(), self.branches_taken);
        forms.insert("jumps".into(), self.jumps);
        forms.insert("calls".into(), self.calls);
        forms.insert("returns".into(), self.returns);
        format!(
            "{{\"instructions\": {}, \"blocks_executed\": {}, \"missing_blocks\": {}, \"by_category\": {}, \"by_mnemonic\": {}, \"forms\": {}, \
             \"operands\": {{\"defined\": {}, \"discarded\": {}, \"live_at_end\": {}, \"lifetime\": {}, \"discarded_lifetime\": {}}}, \
             \"occupancy\": {}, \"blocks\": {}, \
             \"memory\": {{\"loads\": {}, \"stores\": {}, \"load_bytes\": {}, \"store_bytes\": {}, \"stack_loads\": {}, \"stack_stores\": {}, \"pool_loads\": {}}}, \
             \"stack\": {{\"max_depth_bytes\": {}, \"known\": {}}}}}",
            self.steps,
            self.traces,
            self.missing_blocks,
            json_map(&by_category),
            json_map(&by_mnemonic),
            json_map(&forms),
            self.defined,
            self.discarded,
            self.live_at_end,
            self.lifetime.json(),
            self.discarded_lifetime.json(),
            self.occupancy.json(),
            self.block_lengths.json(),
            self.loads,
            self.stores,
            self.load_bytes,
            self.store_bytes,
            self.stack_loads,
            self.stack_stores,
            self.pool_loads,
            -self.sp_min,
            self.sp_known,
        )
    }
}
