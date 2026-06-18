//! Network codec for `AST` elements
//!
//! Provides encoding and decoding functions to map between the native
//! `AST` types and the serialized network representation variants

use crate::net::ast::{NetActionStmt, NetExpr, NetField, NetValue};
use crate::runtime::ast::{ActionStmt, Expr, Field, Value};
use crate::runtime::interner::Interner;

/// Encode a runtime `Value` into a network representation
///
/// Args:
///     val (`&Value`): The runtime `Value` to encode
///     interner (`&Interner`): The `Interner` for symbol lookup
///
/// Returns:
///     `NetValue`: The encoded `NetValue` network representation
pub fn encode_value(val: &Value, interner: &Interner) -> NetValue {
    match val {
        Value::Number { val } => NetValue::Number { val: *val },
        Value::Bool { val } => NetValue::Bool { val: *val },
        Value::String { val } => NetValue::String { val: val.clone() },
        Value::Closure {
            params,
            body,
            env,
            service_name,
        } => {
            let encoded_params = params
                .iter()
                .map(|p| interner.get(*p).to_string())
                .collect();
            let encoded_body = Box::new(encode_expr(body, interner));
            let encoded_env = env
                .iter()
                .map(|(k, v)| (interner.get(*k).to_string(), encode_value(v, interner)))
                .collect();
            let encoded_service = interner.get(*service_name).to_string();
            NetValue::Closure {
                params: encoded_params,
                body: encoded_body,
                env: encoded_env,
                service_name: encoded_service,
            }
        }
        Value::ActionClosure {
            stmts,
            env,
            service_net_id,
        } => {
            let encoded_stmts = stmts
                .iter()
                .map(|s| encode_action_stmt(s, interner))
                .collect();
            let encoded_env = env
                .iter()
                .map(|(k, v)| (interner.get(*k).to_string(), encode_value(v, interner)))
                .collect();
            NetValue::ActionClosure {
                stmts: encoded_stmts,
                env: encoded_env,
                service_net_id: service_net_id.clone(),
            }
        }
    }
}

/// Decode a network `NetValue` representation into a runtime `Value`
///
/// Args:
///     val (`NetValue`): The network `NetValue` to decode
///     interner (`&mut Interner`): The `Interner` for symbol creation
///
/// Returns:
///     `Value`: The decoded runtime `Value`
pub fn decode_value(val: NetValue, interner: &mut Interner) -> Value {
    match val {
        NetValue::Number { val } => Value::Number { val },
        NetValue::Bool { val } => Value::Bool { val },
        NetValue::String { val } => Value::String { val },
        NetValue::Closure {
            params,
            body,
            env,
            service_name,
        } => {
            let decoded_params = params.into_iter().map(|p| interner.insert(&p)).collect();
            let decoded_body = Box::new(decode_expr(*body, interner));
            let decoded_env = env
                .into_iter()
                .map(|(k, v)| (interner.insert(&k), decode_value(v, interner)))
                .collect();
            let decoded_service = interner.insert(&service_name);
            Value::Closure {
                params: decoded_params,
                body: decoded_body,
                env: decoded_env,
                service_name: decoded_service,
            }
        }
        NetValue::ActionClosure {
            stmts,
            env,
            service_net_id,
        } => {
            let decoded_stmts = stmts
                .into_iter()
                .map(|s| decode_action_stmt(s, interner))
                .collect();
            let decoded_env = env
                .into_iter()
                .map(|(k, v)| (interner.insert(&k), decode_value(v, interner)))
                .collect();
            Value::ActionClosure {
                stmts: decoded_stmts,
                env: decoded_env,
                service_net_id,
            }
        }
    }
}

/// Encode a runtime `Expr` into a network representation
///
/// Args:
///     expr (`&Expr`): The runtime `Expr` to encode
///     interner (`&Interner`): The `Interner` for symbol lookup
///
/// Returns:
///     `NetExpr`: The encoded `NetExpr` network representation
pub fn encode_expr(expr: &Expr, interner: &Interner) -> NetExpr {
    match expr {
        Expr::Literal { val } => NetExpr::Literal {
            val: encode_value(val, interner),
        },
        Expr::Variable { name } => NetExpr::Variable {
            name: interner.get(*name).to_string(),
        },
        Expr::Tuple { val } => NetExpr::Tuple {
            val: val.iter().map(|e| encode_expr(e, interner)).collect(),
        },
        Expr::KeyVal { name, value } => NetExpr::KeyVal {
            name: interner.get(*name).to_string(),
            value: Box::new(encode_expr(value, interner)),
        },
        Expr::Unop { op, expr } => NetExpr::Unop {
            op: *op,
            expr: Box::new(encode_expr(expr, interner)),
        },
        Expr::Binop { op, expr1, expr2 } => NetExpr::Binop {
            op: *op,
            expr1: Box::new(encode_expr(expr1, interner)),
            expr2: Box::new(encode_expr(expr2, interner)),
        },
        Expr::If { cond, expr1, expr2 } => NetExpr::If {
            cond: Box::new(encode_expr(cond, interner)),
            expr1: Box::new(encode_expr(expr1, interner)),
            expr2: Box::new(encode_expr(expr2, interner)),
        },
        Expr::Func { params, body } => NetExpr::Func {
            params: params
                .iter()
                .map(|p| interner.get(*p).to_string())
                .collect(),
            body: Box::new(encode_expr(body, interner)),
        },
        Expr::Call { func, args } => NetExpr::Call {
            func: Box::new(encode_expr(func, interner)),
            args: args.iter().map(|e| encode_expr(e, interner)).collect(),
        },
        Expr::Action(stmts) => NetExpr::Action(
            stmts
                .iter()
                .map(|s| encode_action_stmt(s, interner))
                .collect(),
        ),
        Expr::MemberAccess {
            service_name,
            member_name,
        } => NetExpr::MemberAccess {
            service_name: interner.get(*service_name).to_string(),
            member_name: interner.get(*member_name).to_string(),
        },
        Expr::Select {
            table_name,
            column_names,
            where_clause,
        } => NetExpr::Select {
            table_name: interner.get(*table_name).to_string(),
            column_names: column_names
                .iter()
                .map(|c| interner.get(*c).to_string())
                .collect(),
            where_clause: Box::new(encode_expr(where_clause, interner)),
        },
        Expr::Table { schema, records } => NetExpr::Table {
            schema: schema.iter().map(|f| encode_field(f, interner)).collect(),
            records: records.iter().map(|r| encode_expr(r, interner)).collect(),
        },
        Expr::Fold {
            table_name,
            column_name,
            operation,
            identity,
        } => NetExpr::Fold {
            table_name: interner.get(*table_name).to_string(),
            column_name: interner.get(*column_name).to_string(),
            operation: Box::new(encode_expr(operation, interner)),
            identity: Box::new(encode_expr(identity, interner)),
        },
    }
}

/// Decode a network `NetExpr` representation into a runtime `Expr`
///
/// Args:
///     expr (`NetExpr`): The network `NetExpr` to decode
///     interner (`&mut Interner`): The `Interner` for symbol creation
///
/// Returns:
///     `Expr`: The decoded runtime `Expr`
pub fn decode_expr(expr: NetExpr, interner: &mut Interner) -> Expr {
    match expr {
        NetExpr::Literal { val } => Expr::Literal {
            val: decode_value(val, interner),
        },
        NetExpr::Variable { name } => Expr::Variable {
            name: interner.insert(&name),
        },
        NetExpr::Tuple { val } => Expr::Tuple {
            val: val.into_iter().map(|e| decode_expr(e, interner)).collect(),
        },
        NetExpr::KeyVal { name, value } => Expr::KeyVal {
            name: interner.insert(&name),
            value: Box::new(decode_expr(*value, interner)),
        },
        NetExpr::Unop { op, expr } => Expr::Unop {
            op,
            expr: Box::new(decode_expr(*expr, interner)),
        },
        NetExpr::Binop { op, expr1, expr2 } => Expr::Binop {
            op,
            expr1: Box::new(decode_expr(*expr1, interner)),
            expr2: Box::new(decode_expr(*expr2, interner)),
        },
        NetExpr::If { cond, expr1, expr2 } => Expr::If {
            cond: Box::new(decode_expr(*cond, interner)),
            expr1: Box::new(decode_expr(*expr1, interner)),
            expr2: Box::new(decode_expr(*expr2, interner)),
        },
        NetExpr::Func { params, body } => Expr::Func {
            params: params.into_iter().map(|p| interner.insert(&p)).collect(),
            body: Box::new(decode_expr(*body, interner)),
        },
        NetExpr::Call { func, args } => Expr::Call {
            func: Box::new(decode_expr(*func, interner)),
            args: args.into_iter().map(|e| decode_expr(e, interner)).collect(),
        },
        NetExpr::Action(stmts) => Expr::Action(
            stmts
                .into_iter()
                .map(|s| decode_action_stmt(s, interner))
                .collect(),
        ),
        NetExpr::MemberAccess {
            service_name,
            member_name,
        } => Expr::MemberAccess {
            service_name: interner.insert(&service_name),
            member_name: interner.insert(&member_name),
        },
        NetExpr::Select {
            table_name,
            column_names,
            where_clause,
        } => Expr::Select {
            table_name: interner.insert(&table_name),
            column_names: column_names
                .into_iter()
                .map(|c| interner.insert(&c))
                .collect(),
            where_clause: Box::new(decode_expr(*where_clause, interner)),
        },
        NetExpr::Table { schema, records } => Expr::Table {
            schema: schema
                .into_iter()
                .map(|f| decode_field(f, interner))
                .collect(),
            records: records
                .into_iter()
                .map(|r| decode_expr(r, interner))
                .collect(),
        },
        NetExpr::Fold {
            table_name,
            column_name,
            operation,
            identity,
        } => Expr::Fold {
            table_name: interner.insert(&table_name),
            column_name: interner.insert(&column_name),
            operation: Box::new(decode_expr(*operation, interner)),
            identity: Box::new(decode_expr(*identity, interner)),
        },
    }
}

/// Encode a runtime `ActionStmt` into a network representation
///
/// Args:
///     stmt (`&ActionStmt`): The runtime `ActionStmt` to encode
///     interner (`&Interner`): The `Interner` for symbol lookup
///
/// Returns:
///     `NetActionStmt`: The encoded `NetActionStmt` network representation
pub fn encode_action_stmt(stmt: &ActionStmt, interner: &Interner) -> NetActionStmt {
    match stmt {
        ActionStmt::Let { name, expr } => NetActionStmt::Let {
            name: interner.get(*name).to_string(),
            expr: encode_expr(expr, interner),
        },
        ActionStmt::Expr(expr) => NetActionStmt::Expr(encode_expr(expr, interner)),
        ActionStmt::Do(expr) => NetActionStmt::Do(encode_expr(expr, interner)),
        ActionStmt::Assert(expr) => NetActionStmt::Assert(encode_expr(expr, interner)),
        ActionStmt::Assign { name, expr } => NetActionStmt::Assign {
            name: interner.get(*name).to_string(),
            expr: encode_expr(expr, interner),
        },
        ActionStmt::Insert { row, table_name } => NetActionStmt::Insert {
            row: encode_expr(row, interner),
            table_name: interner.get(*table_name).to_string(),
        },
    }
}

/// Decode a network `NetActionStmt` into a runtime `ActionStmt`
///
/// Args:
///     stmt (`NetActionStmt`): The network `NetActionStmt` to decode
///     interner (`&mut Interner`): The `Interner` for symbol creation
///
/// Returns:
///     `ActionStmt`: The decoded runtime `ActionStmt`
pub fn decode_action_stmt(stmt: NetActionStmt, interner: &mut Interner) -> ActionStmt {
    match stmt {
        NetActionStmt::Let { name, expr } => ActionStmt::Let {
            name: interner.insert(&name),
            expr: decode_expr(expr, interner),
        },
        NetActionStmt::Expr(expr) => ActionStmt::Expr(decode_expr(expr, interner)),
        NetActionStmt::Do(expr) => ActionStmt::Do(decode_expr(expr, interner)),
        NetActionStmt::Assert(expr) => ActionStmt::Assert(decode_expr(expr, interner)),
        NetActionStmt::Assign { name, expr } => ActionStmt::Assign {
            name: interner.insert(&name),
            expr: decode_expr(expr, interner),
        },
        NetActionStmt::Insert { row, table_name } => ActionStmt::Insert {
            row: decode_expr(row, interner),
            table_name: interner.insert(&table_name),
        },
    }
}

/// Encode a runtime `Field` into a network representation
///
/// Args:
///     field (`&Field`): The runtime `Field` to encode
///     interner (`&Interner`): The `Interner` for symbol lookup
///
/// Returns:
///     `NetField`: The encoded `NetField` network representation
pub fn encode_field(field: &Field, interner: &Interner) -> NetField {
    NetField {
        name: interner.get(field.name).to_string(),
        type_: field.type_.clone(),
    }
}

/// Decode a network `NetField` representation into a runtime `Field`
///
/// Args:
///     field (`NetField`): The network `NetField` to decode
///     interner (`&mut Interner`): The `Interner` for symbol creation
///
/// Returns:
///     `Field`: The decoded runtime `Field`
pub fn decode_field(field: NetField, interner: &mut Interner) -> Field {
    Field {
        name: interner.insert(&field.name),
        type_: field.type_,
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::net::ServiceNetId;

    /// Verify round-trip encoding, serialization, deserialization, and decoding of `AST` types
    #[test]
    fn test_value_codec_roundtrip() {
        let mut interner_orig = Interner::new();
        let service_net_id = ServiceNetId::new("test_service");

        let var_x = interner_orig.insert("x");
        let tbl_t = interner_orig.insert("t");

        let stmt1 = ActionStmt::Let {
            name: var_x,
            expr: Expr::Literal {
                val: Value::Number { val: 42 },
            },
        };
        let stmt2 = ActionStmt::Insert {
            row: Expr::Variable { name: var_x },
            table_name: tbl_t,
        };

        let env_var = interner_orig.insert("y");
        let env = vec![(env_var, Value::Bool { val: true })];

        let original_value = Value::ActionClosure {
            stmts: vec![stmt1, stmt2],
            env,
            service_net_id,
        };

        let orig_str = format!("{}", original_value);

        let encoded = encode_value(&original_value, &interner_orig);

        let json_str = serde_json::to_string(&encoded).unwrap();
        let decoded_net_val: NetValue = serde_json::from_str(&json_str).unwrap();

        let mut interner_new = Interner::new();
        let decoded_value = decode_value(decoded_net_val, &mut interner_new);

        let new_str = format!("{}", decoded_value);

        assert_eq!(orig_str, new_str);
    }
}
