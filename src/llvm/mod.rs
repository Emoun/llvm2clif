//! LLVM textual IR front end: lexer, parser, IR data structures and data
//! layout computations.

pub mod datalayout;
pub mod ir;
pub mod lexer;
pub mod parser;

pub use datalayout::DataLayout;
pub use ir::*;
pub use parser::{parse_module, ParseError};
