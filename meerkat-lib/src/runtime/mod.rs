//! Runtime components for the Meerkat language.
//!
//! This module coordinates AST, interner, interpreter, txn management,
//! type definitions, and semantic analysis modules

pub mod ast;
pub mod html;
pub mod interner;
pub mod interpreter;
pub mod limits;
pub mod manager;
pub mod parser;
pub mod semantic_analysis;
pub mod tt;
pub mod txn;

pub use html::Html;
pub use interner::{Interner, Symbol};
pub use manager::Manager;
