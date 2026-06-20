//! dependency analysis for var/def node in meerkat
//!

use std::{
    collections::{HashMap, HashSet},
    fmt::Display,
};

use crate::ast;
use crate::runtime::interner::Symbol;

pub mod alpha_rename;
pub mod dep_analysis;
pub mod read_write;

pub struct DependAnalysis {
    pub vars: HashSet<Symbol>,
    pub defs: HashSet<Symbol>,
    pub tables: HashSet<Symbol>,
    pub dep_graph: HashMap<Symbol, HashSet<Symbol>>,
    /// Topological order of variables and definitions represented by `Symbol`
    pub topo_order: Vec<Symbol>,
    /// Transitively dependent variables and definitions of a name
    pub dep_transitive: HashMap<Symbol, HashSet<Symbol>>,
    /// Transitively dependent variables of a name
    ///
    /// The `dep_vars` map values are a subset of `dep_transitive`
    pub dep_vars: HashMap<Symbol, HashSet<Symbol>>,
}

/// Implement the `Display` trait for the `DependAnalysis` struct
///
/// This formats and displays the dependency graph, transitive
/// dependencies, and topological order
impl Display for DependAnalysis {
    /// Format the dependency analysis result for display
    ///
    /// Args:
    ///     `f` (`&mut std::fmt::Formatter<'_>`): The formatter target
    ///
    /// Returns:
    ///     `std::fmt::Result`: The result of the formatting operation
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        writeln!(f, "Dependency Graph ")?;
        for (name, deps) in self.dep_graph.iter() {
            write!(f, "{} -> ", name)?;
            for dep in deps.iter() {
                write!(f, "{},", dep)?;
            }
            writeln!(f)?;
        }
        writeln!(f, "Transitive Dependency (Var only) ")?;
        for (name, deps) in self.dep_vars.iter() {
            write!(f, "{} -> ", name)?;
            for dep in deps.iter() {
                write!(f, "{},", dep)?;
            }
            writeln!(f)?;
        }

        writeln!(f, "Topological Order ")?;
        for name in self.topo_order.iter() {
            write!(f, "{} ", name)?;
        }
        writeln!(f)?;
        Ok(())
    }
}

/// Calculate dependencies for a service from its declarations
///
/// Args:
///     `decls` (`&[ast::Decl]`): The service declarations
///
/// Returns:
///     `DependAnalysis`: The calculated dependency analysis state
pub fn calc_dep_srv(decls: &[ast::Decl]) -> DependAnalysis {
    let mut da = DependAnalysis::new(decls);
    da.calc_dep_vars();
    //println!("{}", da);
    da
}

/// Calculate dependencies for all services in a program
///
/// Args:
///     `stmts` (`&[ast::Stmt]`): The statements of the program
pub fn calc_dep_prog(stmts: &[ast::Stmt]) {
    for stmt in stmts.iter() {
        match stmt {
            &ast::Stmt::Service { ref decls, .. } | &ast::Stmt::Update { ref decls, .. } => {
                let _ = calc_dep_srv(decls);
            }
            &ast::Stmt::ActionStmt(_) => {}
            &ast::Stmt::Connect { .. }
            | &ast::Stmt::Import { .. }
            | &ast::Stmt::Test { .. }
            | &ast::Stmt::Watch { .. } => {}
        }
    }
}

/// Unit tests for variable dependency analysis
#[cfg(test)]
mod tests {
    use super::*;
    use crate::ast::{Decl, Expr, Stmt, Value};
    use crate::runtime::interner::Interner;

    /// Verify dependency analysis on standard service definitions
    #[test]
    fn test_calc_dep_prog_service() {
        let mut interner = Interner::new();
        let s1 = interner.insert("s1");
        let v2 = interner.insert("v2");
        let v3 = interner.insert("v3");
        let stmts = vec![Stmt::Service {
            name: s1,
            decls: vec![
                Decl::VarDecl {
                    name: v2,
                    val: Expr::Literal {
                        val: Value::Number { val: 1 },
                    },
                },
                Decl::DefDecl {
                    name: v3,
                    val: Expr::Variable { name: v2 },
                    is_pub: false,
                },
            ],
        }];

        calc_dep_prog(&stmts);
    }

    /// Verify dependency analysis on service updates
    #[test]
    fn test_calc_dep_prog_update_service() {
        let mut interner = Interner::new();
        let s1 = interner.insert("s1");
        let v2 = interner.insert("v2");
        let v3 = interner.insert("v3");
        let stmts = vec![Stmt::Update {
            service_name: s1,
            decls: vec![
                Decl::VarDecl {
                    name: v2,
                    val: Expr::Literal {
                        val: Value::Number { val: 2 },
                    },
                },
                Decl::DefDecl {
                    name: v3,
                    val: Expr::Variable { name: v2 },
                    is_pub: true,
                },
            ],
        }];

        calc_dep_prog(&stmts);
    }
}
