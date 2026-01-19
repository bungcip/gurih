pub mod ast;
pub mod compiler;
pub mod diagnostics;
pub mod errors;
pub mod parser;

pub use compiler::compile;
