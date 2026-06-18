use super::DependAnalysis;
use crate::ast;
use crate::runtime::interner::Symbol;
use std::collections::{HashMap, HashSet};

impl DependAnalysis {
    /// Initialize a new `DependAnalysis` from the declarations
    ///
    /// Args:
    ///     `decls` (`&[ast::Decl]`): The sequence of declarations to analyze
    ///
    /// Returns:
    ///     `DependAnalysis`: The new analysis state instance
    pub fn new(decls: &[ast::Decl]) -> DependAnalysis {
        let mut vars: HashSet<Symbol> = HashSet::new();
        let mut defs: HashSet<Symbol> = HashSet::new();
        let mut reactive_names = HashSet::new();
        let mut tables: HashSet<Symbol> = HashSet::new();

        let mut dep_graph: HashMap<Symbol, HashSet<Symbol>> = HashMap::new();

        for decl in decls.iter() {
            match decl {
                ast::Decl::VarDecl { name, .. } => {
                    vars.insert(*name);
                    reactive_names.insert(*name);
                    dep_graph.insert(*name, HashSet::new());
                }
                ast::Decl::DefDecl { name, val, .. } => {
                    defs.insert(*name);
                    reactive_names.insert(*name);
                    // Calculated all reactive names so far
                    let deps = val.free_var(&reactive_names, &HashSet::new());
                    dep_graph.insert(*name, deps);
                }
                ast::Decl::TableDecl { name, .. } => {
                    tables.insert(*name);
                    dep_graph.insert(*name, HashSet::new());
                }
            }
        }

        DependAnalysis {
            vars,
            defs,
            tables,
            dep_graph,
            topo_order: Vec::new(),
            dep_transitive: HashMap::new(),
            dep_vars: HashMap::new(),
        }
    }

    /// Performs a depth-first search on the dependency graph to calculate
    /// the transitive dependencies for a given variable or definition
    ///
    /// Args:
    ///     `graph` (`&HashMap<Symbol, HashSet<Symbol>>`): The dependency graph
    ///     `vars` (`&HashSet<Symbol>`): Set of variable names (no dependencies)
    ///     `tables` (`&HashSet<Symbol>`): Set of table names (no dependencies)
    ///     `visited` (`&mut HashSet<Symbol>`): Set of visited nodes in DFS
    ///     `finished` (`&mut Vec<Symbol>`): List of finished nodes
    ///     `calced` (`&mut HashMap<Symbol, HashSet<Symbol>>`): Map of definitions to their computed dependencies
    ///     `name` (`Symbol`): The current symbol to process
    ///
    /// Raises:
    ///     `Panic`: If a cycle is detected in the graph
    fn dfs_helper(
        graph: &HashMap<Symbol, HashSet<Symbol>>,
        vars: &HashSet<Symbol>,
        tables: &HashSet<Symbol>,
        visited: &mut HashSet<Symbol>,
        finished: &mut Vec<Symbol>,
        calced: &mut HashMap<Symbol, HashSet<Symbol>>,
        name: Symbol,
    ) {
        if calced.contains_key(&name) {
            return;
        }

        if visited.contains(&name) {
            panic!("Cycle detected in dependency graph of var and defs");
        }

        visited.insert(name);
        // If visiting a `var`, note that a `var` transitively depends on itself
        if vars.contains(&name) || tables.contains(&name) {
            calced.insert(name, HashSet::from([name]));
            finished.push(name);
            return;
        }

        // Otherwise visit the `def`
        let mut dep = HashSet::new();

        for dep_name in graph
            .get(&name)
            .unwrap_or_else(|| panic!("No such name in dep graph: {:?}", name))
        {
            Self::dfs_helper(graph, vars, tables, visited, finished, calced, *dep_name);
            dep.extend(
                calced
                    .get(dep_name)
                    .unwrap_or_else(|| {
                        panic!(
                            "Not finished transitive dependency calculation of: {:?}",
                            dep_name
                        )
                    })
                    .clone(),
            );
            dep.insert(*dep_name);
        }

        calced.insert(name, dep);
        finished.push(name);
    }

    /// Calculate dependent variables for all variables and definitions
    pub fn calc_dep_vars(&mut self) {
        let mut visited = HashSet::new();

        for name in self
            .vars
            .iter()
            .chain(self.defs.iter().chain(self.tables.iter()))
        {
            Self::dfs_helper(
                &self.dep_graph,
                &self.vars,
                &self.tables,
                &mut visited,
                &mut self.topo_order,
                &mut self.dep_transitive,
                *name,
            );
        }

        let vars_and_tables: HashSet<_> = self.vars.union(&self.tables).cloned().collect();
        for name in self
            .vars
            .iter()
            .chain(self.defs.iter().chain(self.tables.iter()))
        {
            self.dep_vars.insert(
                *name,
                self.dep_transitive
                    .get(name)
                    .unwrap_or_else(|| panic!("cannot find def {:?} in trans dep", name))
                    .intersection(&vars_and_tables)
                    .cloned()
                    .collect(),
            );
        }
    }
}
