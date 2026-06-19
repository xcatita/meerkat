//! Network representation for `AST` elements
//!
//! This module defines the serialized equivalents of the runtime `AST`
//! types, substituting `Symbol` identifiers with raw `String` names

use crate::net::ServiceNetId;
use serde::{Deserialize, Serialize};

/// Network representation of a field definition
#[derive(Debug, Clone, PartialEq, Eq, Hash, Serialize, Deserialize)]
pub struct NetField {
    /// The name of the field
    pub name: String,
    /// The `NetDataType` of the field
    pub ty: NetDataType,
}

/// Network representation of an action statement
#[derive(Debug, Clone, PartialEq, Eq, Hash, Serialize, Deserialize)]
pub enum NetActionStmt {
    /// Bind a value to a local name using `let`
    Let {
        /// The name to bind
        name: String,
        /// The `NetExpr` to bind
        expr: NetExpr,
    },
    /// A standalone expression statement
    Expr(NetExpr),
    /// A `do` statement to evaluate an expression for side effects
    Do(NetExpr),
    /// An `assert` statement to check invariants
    Assert(NetExpr),
    /// Re-assign a value to an existing variable
    Assign {
        /// The variable name
        name: String,
        /// The `NetExpr` to evaluate
        expr: NetExpr,
    },
    /// Insert a record into a table
    Insert {
        /// The `NetExpr` representing the row
        row: NetExpr,
        /// The destination table name
        table_name: String,
    },
}

/// Network representation of a value
#[derive(Debug, Clone, PartialEq, Eq, Hash, Serialize, Deserialize)]
pub enum NetValue {
    /// A numeric integer value
    Number {
        /// The integer value
        val: i32,
    },
    /// A boolean value
    Bool {
        /// The boolean value
        val: bool,
    },
    /// A `String` literal value
    String {
        /// The string content
        val: String,
    },
    /// A standard closure value with environment
    Closure {
        /// Parameter names
        params: Vec<String>,
        /// The closure body expression
        body: Box<NetExpr>,
        /// The captured environment bindings
        env: Vec<(String, NetValue)>,
        /// The service scope name
        service_name: String,
    },
    /// An action closure value with environment and network ID
    ActionClosure {
        /// Action statements
        stmts: Vec<NetActionStmt>,
        /// The captured environment bindings
        env: Vec<(String, NetValue)>,
        /// The net-addressable service identifier `ServiceNetId`
        service_net_id: ServiceNetId,
    },
}

/// Network representation of an expression
#[derive(Debug, Clone, PartialEq, Eq, Hash, Serialize, Deserialize)]
pub enum NetExpr {
    /// Literal constant value
    Literal {
        /// The constant `NetValue`
        val: NetValue,
    },
    /// Variable reference
    Variable {
        /// The variable name
        name: String,
    },
    /// Tuple construct containing elements
    Tuple {
        /// Inner elements
        val: Vec<NetExpr>,
    },
    /// Key-value attribute binding
    KeyVal {
        /// The key name
        name: String,
        /// The value expression
        value: Box<NetExpr>,
    },
    /// Unary operator application
    Unop {
        /// The `NetUnOp` operator
        op: NetUnOp,
        /// The operand expression
        expr: Box<NetExpr>,
    },
    /// Binary operator application
    Binop {
        /// The `NetBinOp` operator
        op: NetBinOp,
        /// The first operand
        expr1: Box<NetExpr>,
        /// The second operand
        expr2: Box<NetExpr>,
    },
    /// Conditional branching expression
    If {
        /// The condition expression
        cond: Box<NetExpr>,
        /// The true branch
        expr1: Box<NetExpr>,
        /// The false branch
        expr2: Box<NetExpr>,
    },
    /// Anonymous function construct
    Func {
        /// Parameter names
        params: Vec<String>,
        /// The function body
        body: Box<NetExpr>,
    },
    /// Function call expression
    Call {
        /// The function being called
        func: Box<NetExpr>,
        /// Arguments passed to the call
        args: Vec<NetExpr>,
    },
    /// Embedded action statement block
    Action(Vec<NetActionStmt>),
    /// Accessing a remote service member
    MemberAccess {
        /// The remote service name
        service_name: String,
        /// The member name
        member_name: String,
    },
    /// Data selection query
    Select {
        /// Source table name
        table_name: String,
        /// Target column names
        column_names: Vec<String>,
        /// Query filter condition
        where_clause: Box<NetExpr>,
    },
    /// Inline table structure
    Table {
        /// Table schema fields
        schema: Vec<NetField>,
        /// Record entries
        records: Vec<NetExpr>,
    },
    /// Fold aggregation construct
    Fold {
        /// Source table name
        table_name: String,
        /// Target column name
        column_name: String,
        /// Aggregator operation
        operation: Box<NetExpr>,
        /// Base accumulator identity
        identity: Box<NetExpr>,
    },
}

/// Network representation of a unary operator
///
/// This enum defines the serialized unary operators mapped from the
/// runtime counterparts for transmission over the network
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Serialize, Deserialize)]
pub enum NetUnOp {
    /// Negation operator
    Neg,
    /// Logical negation operator
    Not,
}

/// Network representation of a binary operator
///
/// This enum defines the serialized binary operators mapped from the
/// runtime counterparts for transmission over the network
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Serialize, Deserialize)]
pub enum NetBinOp {
    /// Addition operator
    Add,
    /// Subtraction operator
    Sub,
    /// Multiplication operator
    Mul,
    /// Division operator
    Div,
    /// Equality comparison operator
    Eq,
    /// Less-than comparison operator
    Lt,
    /// Greater-than comparison operator
    Gt,
    /// Logical conjunction operator
    And,
    /// Logical disjunction operator
    Or,
}

/// Network representation of a data type
///
/// This enum defines the serialized data types mapped from the
/// runtime counterparts for transmission over the network
#[derive(Debug, Clone, PartialEq, Eq, Hash, Serialize, Deserialize)]
pub enum NetDataType {
    /// String data type representation
    String,
    /// Number data type representation
    Number,
    /// Boolean data type representation
    Bool,
}
