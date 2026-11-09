//! Metrics for Scry programs.
//!
//! `scrymetrics static <elf>` decodes the `.text` section and counts the
//! instructions by mnemonic and category.
//!
//! `scrymetrics dynamic` reads a scryer event trace (`scryer prog.elf
//! --print-events`) from standard input and reports the dynamic instruction
//! mix, operand and control-flow lifetimes, queue occupancy, memory traffic
//! and stack use. Both print one JSON object.

use object::{Object, ObjectSection};
use std::collections::{BTreeMap, HashMap};
use std::fmt::Write as _;
use std::io::{self, BufRead, Write};

/// The coarse categories; the per-mnemonic counts are reported too.
fn category(mnemonic: &str) -> &'static str {
    match mnemonic {
        "echo" | "dup" | "pick" => "dataflow",
        "jmp" | "call" | "ret" | "trap" => "control",
        "const" | "grow" => "constant",
        "ld" | "st" | "saddr" | "rsrv" | "free" => "memory",
        "cast" => "cast",
        "nop" => "nop",
        "add" | "sub" | "mul" | "div" | "and" | "or" | "xor" | "sh" | "eq" | "lt" | "gt"
        | "isnar" | "narto" => "alu",
        _ => "other",
    }
}

/// A histogram of small non-negative integers (lifetimes, occupancies).
#[derive(Default)]
struct Hist {
    counts: Vec<u64>,
    total: u64,
    sum: u128,
}

impl Hist {
    fn add(&mut self, v: usize) {
        if self.counts.len() <= v {
            self.counts.resize(v + 1, 0);
        }
        self.counts[v] += 1;
        self.total += 1;
        self.sum += v as u128;
    }
    fn quantile(&self, q: f64) -> usize {
        if self.total == 0 {
            return 0;
        }
        let target = ((self.total as f64) * q).ceil().max(1.0) as u64;
        let mut seen = 0;
        for (v, c) in self.counts.iter().enumerate() {
            seen += c;
            if seen >= target {
                return v;
            }
        }
        self.counts.len() - 1
    }
    fn max(&self) -> usize {
        self.counts.iter().rposition(|&c| c > 0).unwrap_or(0)
    }
    fn mean(&self) -> f64 {
        if self.total == 0 {
            0.0
        } else {
            self.sum as f64 / self.total as f64
        }
    }
    fn json(&self) -> String {
        format!(
            "{{\"count\": {}, \"mean\": {:.2}, \"median\": {}, \"p90\": {}, \"p99\": {}, \"max\": {}}}",
            self.total,
            self.mean(),
            self.quantile(0.5),
            self.quantile(0.9),
            self.quantile(0.99),
            self.max()
        )
    }
}

fn json_map<V: std::fmt::Display>(m: &BTreeMap<String, V>) -> String {
    let mut s = String::from("{");
    for (i, (k, v)) in m.iter().enumerate() {
        if i > 0 {
            s.push_str(", ");
        }
        let _ = write!(s, "\"{k}\": {v}");
    }
    s.push('}');
    s
}

fn run_static(path: &str) -> Result<String, String> {
    let data = std::fs::read(path).map_err(|e| format!("{path}: {e}"))?;
    let file = object::File::parse(&*data).map_err(|e| format!("{path}: {e}"))?;
    let text = file
        .section_by_name(".text")
        .ok_or_else(|| format!("{path}: no .text section"))?;
    let bytes = text.data().map_err(|e| format!("{path}: {e}"))?;
    let mut by_mnemonic: BTreeMap<String, u64> = BTreeMap::new();
    let mut by_category: BTreeMap<String, u64> = BTreeMap::new();
    let mut forms: BTreeMap<String, u64> = BTreeMap::new();
    let mut invalid = 0u64;
    for w in bytes.as_chunks::<2>().0 {
        let inst = scry_isa::Instruction::decode(u16::from_le_bytes(*w));
        use scry_isa::Instruction as I;
        let form = match &inst {
            I::Invalid(_) => {
                invalid += 1;
                continue;
            }
            I::EchoLong(_) => Some("echo_long"),
            I::Echo(..) => Some("echo_short"),
            // A far jump takes its target from an operand and leaves the
            // 7-bit immediate zero.
            I::Jump(target, _) if target.value == 0 => Some("jmp_far"),
            I::Jump(..) => Some("jmp_short"),
            _ => None,
        };
        if let Some(f) = form {
            *forms.entry(f.to_string()).or_default() += 1;
        }
        let m = inst.mnemonic();
        *by_mnemonic.entry(m.to_string()).or_default() += 1;
        *by_category.entry(category(m).to_string()).or_default() += 1;
    }
    let total: u64 = by_mnemonic.values().sum();
    Ok(format!(
        "{{\"file\": \"{}\", \"text_bytes\": {}, \"instructions\": {}, \"invalid_words\": {}, \"by_category\": {}, \"by_mnemonic\": {}, \"forms\": {}}}",
        path,
        bytes.len(),
        total,
        invalid,
        json_map(&by_category),
        json_map(&by_mnemonic),
        json_map(&forms)
    ))
}

#[derive(Default)]
struct Dynamic {
    steps: u64,
    by_mnemonic: BTreeMap<String, u64>,
    by_category: BTreeMap<String, u64>,
    /// Instruction forms worth telling apart: far jumps (target from an
    /// operand, `jmp 0, N`) against short ones (7-bit immediate), and long
    /// echoes against short ones.
    forms: BTreeMap<String, u64>,
    // Operands: id -> (birth step, forwards so far)
    live: HashMap<u64, (u64, u32)>,
    produced: u64,
    consumed: u64,
    discarded: u64,
    forwards: u64,
    by_type: BTreeMap<String, u64>,
    lifetime: Hist,
    discarded_lifetime: Hist,
    forwards_per_operand: Hist,
    occupancy: Hist,
    // Control: id -> (kind, issue step)
    control_live: HashMap<u64, (String, u64)>,
    control_lifetime: BTreeMap<String, Hist>,
    loads: u64,
    load_bytes: u64,
    stores: u64,
    store_bytes: u64,
    stack_loads: u64,
    stack_stores: u64,
    stack_min_base: Option<u64>,
    stack_max_top: Option<u64>,
    stack_max_depth: u64,
    stack_base: Option<u64>,
    block_len: u64,
    block_lengths: Hist,
    returned: String,
    /// The simulator stopped at its instruction/time limit (`----------  Timeout  ----------`).
    timed_out: bool,
}

fn parse_hex(s: &str) -> Option<u64> {
    let s = s.trim_start_matches("0x");
    u64::from_str_radix(s, 16).ok()
}

/// `Name(a,b)` -> ("Name", ["a", "b"])
fn call_like(s: &str) -> Option<(&str, Vec<&str>)> {
    let open = s.find('(')?;
    let close = s.rfind(')')?;
    let args = s[open + 1..close].split(',').map(str::trim).collect();
    Some((&s[..open], args))
}

impl Dynamic {
    fn line(&mut self, line: &str) {
        let line = line.trim_end();
        if line == "STEP_END" {
            self.steps += 1;
            self.occupancy.add(self.live.len());
            return;
        }
        // Memory events carry their address in the name: `Load(0x1100): 4`.
        let (kind, rest) = match line.split_once(": ") {
            Some((k, r)) if k.starts_with("Load") || k.starts_with("Store") => {
                let name = &k[..k.find('(').unwrap_or(k.len())];
                let (name, _) = (name, ());
                (name, r)
            }
            Some(kr) => kr,
            None => ("", line),
        };
        if kind.is_empty() {
            if line.starts_with("---") && line.contains("Timeout") {
                self.timed_out = true;
                return;
            }
            if !line.is_empty()
                && !line.starts_with("---")
                && self.returned.is_empty()
                && self.steps > 0
            {
                self.returned = line.trim().trim_end_matches(',').to_string();
            }
            return;
        };
        match kind {
            "Instruction" => {
                // "(0x1100) const u32, 42"
                let text = rest.split_once(") ").map(|x| x.1).unwrap_or(rest);
                let m = text.split_whitespace().next().unwrap_or("?");
                *self.by_mnemonic.entry(m.to_string()).or_default() += 1;
                let form = match m {
                    "jmp" if text.starts_with("jmp 0,") => Some("jmp_far"),
                    "jmp" => Some("jmp_short"),
                    // A short echo's output references are 5-bit (0..31); the
                    // long form prints a single reference beyond that reach.
                    "echo" => {
                        let first_ref: u32 = text
                            .split("=>")
                            .nth(1)
                            .and_then(|r| r.trim().trim_end_matches(',').parse().ok())
                            .unwrap_or(0);
                        Some(if first_ref > 31 {
                            "echo_long"
                        } else {
                            "echo_short"
                        })
                    }
                    _ => None,
                };
                if let Some(f) = form {
                    *self.forms.entry(f.to_string()).or_default() += 1;
                }
                let cat = category(m);
                *self.by_category.entry(cat.to_string()).or_default() += 1;
                self.block_len += 1;
                if cat == "control" {
                    self.block_lengths.add(self.block_len as usize);
                    self.block_len = 0;
                }
            }
            "Operand" => {
                let Some((ev, args)) = call_like(rest) else {
                    return;
                };
                let id: u64 = args.first().and_then(|a| a.parse().ok()).unwrap_or(0);
                match ev {
                    "Produce" => {
                        self.produced += 1;
                        *self
                            .by_type
                            .entry(args.get(1).unwrap_or(&"?").to_string())
                            .or_default() += 1;
                        self.live.insert(id, (self.steps, 0));
                    }
                    "Forward" => {
                        self.forwards += 1;
                        if let Some(e) = self.live.get_mut(&id) {
                            e.1 += 1;
                        }
                    }
                    "Consume" | "Discard" => {
                        if let Some((birth, fw)) = self.live.remove(&id) {
                            let life = (self.steps - birth) as usize;
                            if ev == "Consume" {
                                self.consumed += 1;
                                self.lifetime.add(life);
                            } else {
                                self.discarded += 1;
                                self.discarded_lifetime.add(life);
                            }
                            self.forwards_per_operand.add(fw as usize);
                        }
                    }
                    _ => {}
                }
            }
            "Issue" => {
                if let Some((ev, args)) = call_like(rest) {
                    let id: u64 = args.first().and_then(|a| a.parse().ok()).unwrap_or(0);
                    self.control_live.insert(id, (ev.to_string(), self.steps));
                }
            }
            "Trigger" => {
                if let Some((ev, args)) = call_like(rest) {
                    let id: u64 = args.first().and_then(|a| a.parse().ok()).unwrap_or(0);
                    let issue = self.control_live.remove(&id).map(|(_, s)| s);
                    let h = self.control_lifetime.entry(ev.to_string()).or_default();
                    h.add(issue.map(|s| (self.steps - s) as usize).unwrap_or(0));
                }
            }
            "Load" | "LoadStack" | "Store" | "StoreStack" => {
                let bytes: u64 = rest.trim().parse().unwrap_or(0);
                match kind {
                    "Load" => {
                        self.loads += 1;
                        self.load_bytes += bytes;
                    }
                    "LoadStack" => {
                        self.stack_loads += 1;
                        self.load_bytes += bytes;
                    }
                    "Store" => {
                        self.stores += 1;
                        self.store_bytes += bytes;
                    }
                    _ => {
                        self.stack_stores += 1;
                        self.store_bytes += bytes;
                    }
                }
            }
            "Stack" => {
                if let Some((ev, args)) = call_like(rest) {
                    let addr = args.first().and_then(|a| parse_hex(a));
                    if let Some(addr) = addr {
                        match ev {
                            "Base" => {
                                self.stack_base = Some(addr);
                                self.stack_min_base =
                                    Some(self.stack_min_base.map_or(addr, |b| b.min(addr)));
                            }
                            "Top" => {
                                self.stack_max_top =
                                    Some(self.stack_max_top.map_or(addr, |t| t.max(addr)));
                                if let Some(b) = self.stack_min_base {
                                    self.stack_max_depth =
                                        self.stack_max_depth.max(addr.saturating_sub(b));
                                }
                            }
                            _ => {}
                        }
                    }
                }
            }
            _ => {}
        }
    }

    fn json(&self) -> String {
        let mut control = String::from("{");
        for (i, (k, h)) in self.control_lifetime.iter().enumerate() {
            if i > 0 {
                control.push_str(", ");
            }
            let _ = write!(control, "\"{k}\": {}", h.json());
        }
        control.push('}');
        format!(
            "{{\"steps\": {}, \"timed_out\": {}, \"returned\": \"{}\", \"by_category\": {}, \"by_mnemonic\": {}, \"forms\": {}, \
             \"operands\": {{\"produced\": {}, \"consumed\": {}, \"discarded\": {}, \"forwards\": {}, \"live_at_end\": {}, \
             \"by_type\": {}, \"lifetime\": {}, \"discarded_lifetime\": {}, \"forwards_per_operand\": {}}}, \
             \"occupancy\": {}, \"control\": {}, \
             \"memory\": {{\"loads\": {}, \"load_bytes\": {}, \"stores\": {}, \"store_bytes\": {}, \"stack_loads\": {}, \"stack_stores\": {}}}, \
             \"stack\": {{\"max_depth_bytes\": {}}}, \"blocks\": {}}}",
            self.steps,
            self.timed_out,
            self.returned,
            json_map(&self.by_category),
            json_map(&self.by_mnemonic),
            json_map(&self.forms),
            self.produced,
            self.consumed,
            self.discarded,
            self.forwards,
            self.live.len(),
            json_map(&self.by_type),
            self.lifetime.json(),
            self.discarded_lifetime.json(),
            self.forwards_per_operand.json(),
            self.occupancy.json(),
            control,
            self.loads,
            self.load_bytes,
            self.stores,
            self.store_bytes,
            self.stack_loads,
            self.stack_stores,
            self.stack_max_depth,
            self.block_lengths.json(),
        )
    }
}

fn main() {
    let args: Vec<String> = std::env::args().skip(1).collect();
    let out = io::stdout();
    let mut out = out.lock();
    match args.first().map(String::as_str) {
        Some("static") => {
            for path in &args[1..] {
                match run_static(path) {
                    Ok(json) => {
                        let _ = writeln!(out, "{json}");
                    }
                    Err(e) => {
                        eprintln!("scrymetrics: {e}");
                        std::process::exit(1);
                    }
                }
            }
        }
        Some("dynamic") => {
            let stdin = io::stdin();
            let mut reader = io::BufReader::with_capacity(1 << 20, stdin.lock());
            let mut d = Dynamic::default();
            let mut line = String::new();
            loop {
                line.clear();
                match reader.read_line(&mut line) {
                    Ok(0) => break,
                    Ok(_) => d.line(&line),
                    Err(e) => {
                        eprintln!("scrymetrics: {e}");
                        std::process::exit(1);
                    }
                }
            }
            let _ = writeln!(out, "{}", d.json());
        }
        _ => {
            eprintln!("usage: scrymetrics static <elf>...  |  scryer prog.elf --print-events | scrymetrics dynamic");
            std::process::exit(2);
        }
    }
}
