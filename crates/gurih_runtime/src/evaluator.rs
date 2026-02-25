use crate::datastore::DataStore;
use crate::errors::RuntimeError;
use chrono::{Datelike, NaiveDate, Utc};
use gurih_ir::utils::to_snake_case;
use gurih_ir::{BinaryOperator, Expression, Schema, Symbol, UnaryOperator};
use serde_json::Value;
use std::sync::Arc;

const MAX_RECURSION_DEPTH: usize = 250;

pub async fn evaluate(
    expr: &Expression,
    context: &Value,
    schema: Option<&Schema>,
    datastore: Option<&Arc<dyn DataStore>>,
) -> Result<Value, RuntimeError> {
    evaluate_internal(expr, context, schema, datastore, 0).await
}

async fn evaluate_internal(
    expr: &Expression,
    context: &Value,
    schema: Option<&Schema>,
    datastore: Option<&Arc<dyn DataStore>>,
    depth: usize,
) -> Result<Value, RuntimeError> {
    if depth > MAX_RECURSION_DEPTH {
        return Err(RuntimeError::EvaluationError(
            "Expression recursion limit exceeded".into(),
        ));
    }

    if !needs_async_checked(expr, depth)? {
        return evaluate_sync_checked(expr, context, schema, depth);
    }

    match expr {
        Expression::Field(name) => eval_field(name.as_str(), context),
        Expression::Literal(n) => {
            Ok(Value::Number(serde_json::Number::from_f64(*n).ok_or_else(|| {
                RuntimeError::EvaluationError("Invalid float literal".to_string())
            })?))
        }
        Expression::StringLiteral(s) => Ok(Value::String(s.clone())),
        Expression::BoolLiteral(b) => Ok(Value::Bool(*b)),
        Expression::Grouping(inner) => {
            if needs_async_checked(inner, depth + 1)? {
                Box::pin(evaluate_internal(inner, context, schema, datastore, depth + 1)).await
            } else {
                evaluate_sync_checked(inner, context, schema, depth + 1)
            }
        }
        Expression::UnaryOp { op, expr } => {
            let val = if needs_async_checked(expr, depth + 1)? {
                Box::pin(evaluate_internal(expr, context, schema, datastore, depth + 1)).await?
            } else {
                evaluate_sync_checked(expr, context, schema, depth + 1)?
            };
            eval_unary_op(op, val)
        }
        Expression::BinaryOp { left, op, right } => match op {
            BinaryOperator::And => {
                let l = if needs_async_checked(left, depth + 1)? {
                    Box::pin(evaluate_internal(left, context, schema, datastore, depth + 1)).await?
                } else {
                    evaluate_sync_checked(left, context, schema, depth + 1)?
                };
                if !as_bool(&l)? {
                    return Ok(Value::Bool(false));
                }
                let r = if needs_async_checked(right, depth + 1)? {
                    Box::pin(evaluate_internal(right, context, schema, datastore, depth + 1)).await?
                } else {
                    evaluate_sync_checked(right, context, schema, depth + 1)?
                };
                let r_bool = as_bool(&r)?;
                Ok(Value::Bool(r_bool))
            }
            BinaryOperator::Or => {
                let l = if needs_async_checked(left, depth + 1)? {
                    Box::pin(evaluate_internal(left, context, schema, datastore, depth + 1)).await?
                } else {
                    evaluate_sync_checked(left, context, schema, depth + 1)?
                };
                if as_bool(&l)? {
                    return Ok(Value::Bool(true));
                }
                let r = if needs_async_checked(right, depth + 1)? {
                    Box::pin(evaluate_internal(right, context, schema, datastore, depth + 1)).await?
                } else {
                    evaluate_sync_checked(right, context, schema, depth + 1)?
                };
                let r_bool = as_bool(&r)?;
                Ok(Value::Bool(r_bool))
            }
            _ => {
                let l = if needs_async_checked(left, depth + 1)? {
                    Box::pin(evaluate_internal(left, context, schema, datastore, depth + 1)).await?
                } else {
                    evaluate_sync_checked(left, context, schema, depth + 1)?
                };
                let r = if needs_async_checked(right, depth + 1)? {
                    Box::pin(evaluate_internal(right, context, schema, datastore, depth + 1)).await?
                } else {
                    evaluate_sync_checked(right, context, schema, depth + 1)?
                };
                eval_binary_op(op, l, r)
            }
        },
        Expression::FunctionCall { name, args } => {
            let mut eval_args = Vec::with_capacity(args.len());
            for arg in args {
                if needs_async_checked(arg, depth + 1)? {
                    eval_args.push(Box::pin(evaluate_internal(arg, context, schema, datastore, depth + 1)).await?);
                } else {
                    eval_args.push(evaluate_sync_checked(arg, context, schema, depth + 1)?);
                }
            }

            if let Some(val) = eval_function_sync(name.as_str(), &eval_args)? {
                Ok(val)
            } else {
                eval_function(name.as_str(), &eval_args, schema, datastore).await
            }
        }
    }
}

fn needs_async_checked(expr: &Expression, depth: usize) -> Result<bool, RuntimeError> {
    if depth > MAX_RECURSION_DEPTH {
        return Err(RuntimeError::EvaluationError(
            "Expression recursion limit exceeded".into(),
        ));
    }
    match expr {
        Expression::FunctionCall { name, args } => {
            let n = name.as_str();
            if n == "lookup_field" || n == "exists" {
                return Ok(true);
            }
            for arg in args {
                if needs_async_checked(arg, depth + 1)? {
                    return Ok(true);
                }
            }
            Ok(false)
        }
        Expression::Grouping(inner) => needs_async_checked(inner, depth + 1),
        Expression::UnaryOp { expr, .. } => needs_async_checked(expr, depth + 1),
        Expression::BinaryOp { left, right, .. } => {
            if needs_async_checked(left, depth + 1)? {
                return Ok(true);
            }
            needs_async_checked(right, depth + 1)
        }
        _ => Ok(false),
    }
}

fn evaluate_sync_checked(
    expr: &Expression,
    context: &Value,
    _schema: Option<&Schema>,
    depth: usize,
) -> Result<Value, RuntimeError> {
    if depth > MAX_RECURSION_DEPTH {
        return Err(RuntimeError::EvaluationError(
            "Expression recursion limit exceeded".into(),
        ));
    }
    match expr {
        Expression::Field(name) => eval_field(name.as_str(), context),
        Expression::Literal(n) => {
            Ok(Value::Number(serde_json::Number::from_f64(*n).ok_or_else(|| {
                RuntimeError::EvaluationError("Invalid float literal".to_string())
            })?))
        }
        Expression::StringLiteral(s) => Ok(Value::String(s.clone())),
        Expression::BoolLiteral(b) => Ok(Value::Bool(*b)),
        Expression::Grouping(inner) => evaluate_sync_checked(inner, context, _schema, depth + 1),
        Expression::UnaryOp { op, expr } => {
            let val = evaluate_sync_checked(expr, context, _schema, depth + 1)?;
            eval_unary_op(op, val)
        }
        Expression::BinaryOp { left, op, right } => match op {
            BinaryOperator::And => {
                let l = evaluate_sync_checked(left, context, _schema, depth + 1)?;
                if !as_bool(&l)? {
                    return Ok(Value::Bool(false));
                }
                let r = evaluate_sync_checked(right, context, _schema, depth + 1)?;
                let r_bool = as_bool(&r)?;
                Ok(Value::Bool(r_bool))
            }
            BinaryOperator::Or => {
                let l = evaluate_sync_checked(left, context, _schema, depth + 1)?;
                if as_bool(&l)? {
                    return Ok(Value::Bool(true));
                }
                let r = evaluate_sync_checked(right, context, _schema, depth + 1)?;
                let r_bool = as_bool(&r)?;
                Ok(Value::Bool(r_bool))
            }
            _ => {
                let l = evaluate_sync_checked(left, context, _schema, depth + 1)?;
                let r = evaluate_sync_checked(right, context, _schema, depth + 1)?;
                eval_binary_op(op, l, r)
            }
        },
        Expression::FunctionCall { name, args } => {
            let mut eval_args = Vec::with_capacity(args.len());
            for arg in args {
                eval_args.push(evaluate_sync_checked(arg, context, _schema, depth + 1)?);
            }
            if let Some(val) = eval_function_sync(name.as_str(), &eval_args)? {
                Ok(val)
            } else {
                Err(RuntimeError::EvaluationError(format!(
                    "Async function call in sync context: {}",
                    name
                )))
            }
        }
    }
}

fn eval_field(key: &str, context: &Value) -> Result<Value, RuntimeError> {
    if key.contains('.') {
        // OPTIMIZATION: Iterate directly over split iterator to avoid Vec allocation
        let mut current = context;
        for part in key.split('.') {
            match current {
                Value::Object(map) => {
                    if let Some(val) = map.get(part) {
                        current = val;
                    } else {
                        return Ok(Value::Null);
                    }
                }
                _ => return Ok(Value::Null),
            }
        }
        Ok(current.clone())
    } else {
        Ok(context.get(key).cloned().unwrap_or(Value::Null))
    }
}

fn eval_unary_op(op: &UnaryOperator, val: Value) -> Result<Value, RuntimeError> {
    match op {
        UnaryOperator::Not => match val {
            Value::Bool(b) => Ok(Value::Bool(!b)),
            _ => Err(RuntimeError::EvaluationError(format!(
                "Type mismatch: expected boolean for NOT, found {:?}",
                val
            ))),
        },
        UnaryOperator::Neg => match val {
            Value::Number(n) => {
                if let Some(f) = n.as_f64() {
                    Ok(Value::Number(serde_json::Number::from_f64(-f).unwrap()))
                } else {
                    Err(RuntimeError::EvaluationError("Invalid number for negation".into()))
                }
            }
            _ => Err(RuntimeError::EvaluationError(format!(
                "Type mismatch: expected number for NEG, found {:?}",
                val
            ))),
        },
    }
}

fn eval_binary_op(op: &BinaryOperator, left: Value, right: Value) -> Result<Value, RuntimeError> {
    match op {
        BinaryOperator::Add => {
            // Check for string concatenation if either operand is a string
            if left.is_string() || right.is_string() {
                let to_s = |v: Value| match v {
                    Value::String(s) => s,
                    Value::Number(n) => n.to_string(),
                    Value::Bool(b) => b.to_string(),
                    Value::Null => "".to_string(),
                    _ => format!("{:?}", v),
                };
                return Ok(Value::String(format!("{}{}", to_s(left), to_s(right))));
            }

            let l = as_f64(&left)?;
            let r = as_f64(&right)?;
            Ok(Value::Number(serde_json::Number::from_f64(l + r).ok_or_else(|| {
                RuntimeError::EvaluationError("Result is not a valid number".into())
            })?))
        }
        BinaryOperator::Sub | BinaryOperator::Mul | BinaryOperator::Div => {
            let l = as_f64(&left)?;
            let r = as_f64(&right)?;
            let res = match op {
                BinaryOperator::Sub => l - r,
                BinaryOperator::Mul => l * r,
                BinaryOperator::Div => {
                    if r == 0.0 {
                        return Err(RuntimeError::EvaluationError("Division by zero".into()));
                    }
                    l / r
                }
                _ => unreachable!(),
            };
            Ok(Value::Number(serde_json::Number::from_f64(res).ok_or_else(|| {
                RuntimeError::EvaluationError("Result is not a valid number".into())
            })?))
        }
        BinaryOperator::Eq => Ok(Value::Bool(left == right)),
        BinaryOperator::Neq => Ok(Value::Bool(left != right)),
        BinaryOperator::Gt | BinaryOperator::Lt | BinaryOperator::Gte | BinaryOperator::Lte => {
            if let (Some(l_str), Some(r_str)) = (left.as_str(), right.as_str()) {
                match op {
                    BinaryOperator::Gt => Ok(Value::Bool(l_str > r_str)),
                    BinaryOperator::Lt => Ok(Value::Bool(l_str < r_str)),
                    BinaryOperator::Gte => Ok(Value::Bool(l_str >= r_str)),
                    BinaryOperator::Lte => Ok(Value::Bool(l_str <= r_str)),
                    _ => unreachable!(),
                }
            } else {
                let l = as_f64(&left)?;
                let r = as_f64(&right)?;
                match op {
                    BinaryOperator::Gt => Ok(Value::Bool(l > r)),
                    BinaryOperator::Lt => Ok(Value::Bool(l < r)),
                    BinaryOperator::Gte => Ok(Value::Bool(l >= r)),
                    BinaryOperator::Lte => Ok(Value::Bool(l <= r)),
                    _ => unreachable!(),
                }
            }
        }
        BinaryOperator::And => {
            let l = as_bool(&left)?;
            let r = as_bool(&right)?;
            Ok(Value::Bool(l && r))
        }
        BinaryOperator::Or => {
            let l = as_bool(&left)?;
            let r = as_bool(&right)?;
            Ok(Value::Bool(l || r))
        }
        BinaryOperator::Like => {
            let l_str = as_str(&left)?;
            let r_str = as_str(&right)?;
            Ok(Value::Bool(match_like(l_str, r_str)))
        }
        BinaryOperator::ILike => {
            let l_str = as_str(&left)?;
            let r_str = as_str(&right)?;
            Ok(Value::Bool(match_like(&l_str.to_lowercase(), &r_str.to_lowercase())))
        }
    }
}

fn match_like(s: &str, p: &str) -> bool {
    let s_chars: Vec<char> = s.chars().collect();
    let p_chars: Vec<char> = p.chars().collect();

    let mut s_idx = 0;
    let mut p_idx = 0;
    let mut last_wildcard_idx = None;
    let mut s_backtrack_idx = 0;

    while s_idx < s_chars.len() {
        if p_idx < p_chars.len() && (p_chars[p_idx] == '_' || p_chars[p_idx] == s_chars[s_idx]) {
            // Exact match or single char wildcard
            s_idx += 1;
            p_idx += 1;
        } else if p_idx < p_chars.len() && p_chars[p_idx] == '%' {
            // Wildcard found
            last_wildcard_idx = Some(p_idx);
            p_idx += 1; // Try matching 0 characters first
            s_backtrack_idx = s_idx;
        } else if let Some(wildcard_idx) = last_wildcard_idx {
            // Mismatch, backtrack
            p_idx = wildcard_idx + 1;
            s_backtrack_idx += 1;
            s_idx = s_backtrack_idx;
        } else {
            // Mismatch and no wildcard to backtrack to
            return false;
        }
    }

    // Consume remaining % at the end of pattern
    while p_idx < p_chars.len() && p_chars[p_idx] == '%' {
        p_idx += 1;
    }

    p_idx == p_chars.len()
}

fn eval_function_sync(name: &str, args: &[Value]) -> Result<Option<Value>, RuntimeError> {
    match name {
        "age" => {
            if args.len() != 1 {
                return Err(RuntimeError::EvaluationError("age() takes 1 argument".into()));
            }
            let date_str = as_str(&args[0])?;
            let birth_date = NaiveDate::parse_from_str(date_str, "%Y-%m-%d")
                .map_err(|_| RuntimeError::EvaluationError("Invalid date format YYYY-MM-DD".into()))?;
            let now = Utc::now().date_naive();

            // Age calculation
            let mut age = now.year() - birth_date.year();
            if (now.month(), now.day()) < (birth_date.month(), birth_date.day()) {
                age -= 1;
            }

            Ok(Some(Value::Number(serde_json::Number::from(age))))
        }
        "years_of_service" => {
            if args.len() != 1 {
                return Err(RuntimeError::EvaluationError(
                    "years_of_service() takes 1 argument".into(),
                ));
            }
            let date_str = as_str(&args[0])?;
            let join_date = NaiveDate::parse_from_str(date_str, "%Y-%m-%d")
                .map_err(|_| RuntimeError::EvaluationError("Invalid date format YYYY-MM-DD".into()))?;
            let now = Utc::now().date_naive();

            // Years of service calculation
            let mut years = now.year() - join_date.year();
            if (now.month(), now.day()) < (join_date.month(), join_date.day()) {
                years -= 1;
            }

            Ok(Some(Value::Number(serde_json::Number::from(years))))
        }
        "is_set" => {
            if args.len() != 1 {
                return Err(RuntimeError::EvaluationError("is_set() takes 1 argument".into()));
            }
            let res = match &args[0] {
                Value::Null => Value::Bool(false),
                Value::String(s) => Value::Bool(!s.is_empty()),
                _ => Value::Bool(true),
            };
            Ok(Some(res))
        }
        "valid_date" => {
            if args.len() != 1 {
                return Err(RuntimeError::EvaluationError("valid_date() takes 1 argument".into()));
            }
            let date_str = match &args[0] {
                Value::String(s) => s,
                Value::Null => return Ok(Some(Value::Bool(false))),
                _ => return Ok(Some(Value::Bool(false))),
            };

            let valid = NaiveDate::parse_from_str(date_str, "%Y-%m-%d").is_ok();
            Ok(Some(Value::Bool(valid)))
        }
        "days_between" => {
            if args.len() != 2 {
                return Err(RuntimeError::EvaluationError("days_between() takes 2 arguments".into()));
            }
            let end_str = as_str(&args[0])?;
            let start_str = as_str(&args[1])?;

            let end_date = NaiveDate::parse_from_str(end_str, "%Y-%m-%d")
                .map_err(|_| RuntimeError::EvaluationError("Invalid end date format YYYY-MM-DD".into()))?;
            let start_date = NaiveDate::parse_from_str(start_str, "%Y-%m-%d")
                .map_err(|_| RuntimeError::EvaluationError("Invalid start date format YYYY-MM-DD".into()))?;

            let days = (end_date - start_date).num_days();
            Ok(Some(Value::Number(serde_json::Number::from(days))))
        }
        _ => Ok(None),
    }
}

async fn eval_function(
    name: &str,
    args: &[Value],
    schema: Option<&Schema>,
    datastore: Option<&Arc<dyn DataStore>>,
) -> Result<Value, RuntimeError> {
    match name {
        "lookup_field" => {
            if args.len() != 3 {
                return Err(RuntimeError::EvaluationError("lookup_field() takes 3 arguments".into()));
            }
            if let Some(ds) = datastore {
                let entity_name = as_str(&args[0])?;
                let id = as_str(&args[1])?;
                let field = as_str(&args[2])?;

                let table_name = if let Some(s) = schema {
                    s.entities
                        .get(&Symbol::from(entity_name))
                        .map(|e| e.table_name.to_string())
                        .unwrap_or_else(|| to_snake_case(entity_name))
                } else {
                    to_snake_case(entity_name)
                };

                let record = ds
                    .get(&table_name, id)
                    .await
                    .map_err(|e| RuntimeError::EvaluationError(format!("DB Error in lookup_field: {}", e)))?;

                if let Some(rec) = record {
                    Ok(rec.get(field).cloned().unwrap_or(Value::Null))
                } else {
                    Ok(Value::Null)
                }
            } else {
                Err(RuntimeError::EvaluationError("lookup_field requires datastore".into()))
            }
        }
        "exists" => {
            #[allow(clippy::manual_is_multiple_of)]
            if args.is_empty() || (args.len() - 1) % 2 != 0 {
                return Err(RuntimeError::EvaluationError(
                    "exists() requires entity name and pairs of key/value arguments".into(),
                ));
            }
            if let Some(ds) = datastore {
                let entity_name = as_str(&args[0])?;
                let mut filters = std::collections::HashMap::new();

                for i in (1..args.len()).step_by(2) {
                    let key = as_str(&args[i])?;
                    let val_str = match &args[i + 1] {
                        Value::String(s) => s.clone(),
                        Value::Number(n) => n.to_string(),
                        Value::Bool(b) => b.to_string(),
                        Value::Null => "null".to_string(),
                        _ => args[i + 1].to_string(),
                    };
                    filters.insert(key.to_string(), val_str);
                }

                let table_name = if let Some(s) = schema {
                    s.entities
                        .get(&Symbol::from(entity_name))
                        .map(|e| e.table_name.to_string())
                        .unwrap_or_else(|| to_snake_case(entity_name))
                } else {
                    to_snake_case(entity_name)
                };

                let count = ds
                    .count(&table_name, filters)
                    .await
                    .map_err(RuntimeError::EvaluationError)?;
                Ok(Value::Bool(count > 0))
            } else {
                Err(RuntimeError::EvaluationError("exists() requires datastore".into()))
            }
        }
        _ => Err(RuntimeError::EvaluationError(format!("Unknown function: {}", name))),
    }
}

fn as_f64(v: &Value) -> Result<f64, RuntimeError> {
    gurih_ir::utils::parse_numeric_strict(v).map_err(RuntimeError::EvaluationError)
}

fn as_bool(v: &Value) -> Result<bool, RuntimeError> {
    match v {
        Value::Bool(b) => Ok(*b),
        _ => Err(RuntimeError::EvaluationError(format!(
            "Type mismatch: expected boolean, found {:?}",
            v
        ))),
    }
}

fn as_str(v: &Value) -> Result<&str, RuntimeError> {
    match v {
        Value::String(s) => Ok(s),
        _ => Err(RuntimeError::EvaluationError(format!(
            "Type mismatch: expected string, found {:?}",
            v
        ))),
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use gurih_ir::{BinaryOperator, Expression, Symbol};
    use serde_json::json;

    #[tokio::test]
    async fn test_eval_literal() {
        let expr = Expression::Literal(42.0);
        let ctx = json!({});
        let res = evaluate(&expr, &ctx, None, None).await.unwrap();
        assert_eq!(res, json!(42.0));
    }

    #[tokio::test]
    async fn test_eval_add() {
        let expr = Expression::BinaryOp {
            left: Box::new(Expression::Literal(10.0)),
            op: BinaryOperator::Add,
            right: Box::new(Expression::Literal(32.0)),
        };
        let ctx = json!({});
        let res = evaluate(&expr, &ctx, None, None).await.unwrap();
        assert_eq!(res, json!(42.0));
    }

    #[tokio::test]
    async fn test_eval_field() {
        let expr = Expression::Field(Symbol::from("score"));
        let ctx = json!({"score": 100});
        let res = evaluate(&expr, &ctx, None, None).await.unwrap();
        assert_eq!(res, json!(100));
    }

    #[tokio::test]
    async fn test_eval_age() {
        // Mock current date behavior by comparing relative years
        // This test is tricky because `evaluate` uses `Utc::now()`.
        // Let's assume `age` logic is correct if basic math works.
        // Or we can mock `Utc::now()` but that's hard here.
        // We will just test it returns a number.
        let expr = Expression::FunctionCall {
            name: Symbol::from("age"),
            args: vec![Expression::Literal(0.0)], // Invalid arg type
        };
        let ctx = json!({});
        let res = evaluate(&expr, &ctx, None, None).await;
        assert!(res.is_err()); // expects string

        let expr = Expression::FunctionCall {
            name: Symbol::from("age"),
            args: vec![Expression::StringLiteral("2000-01-01".into())],
        };
        let res = evaluate(&expr, &ctx, None, None).await.unwrap();
        assert!(res.is_number());
    }

    #[tokio::test]
    async fn test_min_age_logic() {
        // MinAge { age: 18 } -> age(field) >= 18
        let expr = Expression::BinaryOp {
            left: Box::new(Expression::FunctionCall {
                name: Symbol::from("age"),
                args: vec![Expression::StringLiteral("2000-01-01".into())],
            }),
            op: BinaryOperator::Gte,
            right: Box::new(Expression::Literal(18.0)),
        };
        let ctx = json!({});
        let res = evaluate(&expr, &ctx, None, None).await.unwrap();
        assert_eq!(res, json!(true)); // Should be > 18 in 2024
    }

    #[tokio::test]
    async fn test_eval_nested_field() {
        let ctx = json!({
            "user": {
                "profile": {
                    "age": 30
                }
            }
        });

        // Test valid nested access
        let expr = Expression::Field(Symbol::from("user.profile.age"));
        let res = evaluate(&expr, &ctx, None, None).await.unwrap();
        assert_eq!(res, json!(30));

        // Test missing nested key
        let expr = Expression::Field(Symbol::from("user.profile.missing"));
        let res = evaluate(&expr, &ctx, None, None).await.unwrap();
        assert_eq!(res, Value::Null);

        // Test non-object intermediate
        let expr = Expression::Field(Symbol::from("user.profile.age.bad"));
        let res = evaluate(&expr, &ctx, None, None).await.unwrap();
        assert_eq!(res, Value::Null);
    }

    #[tokio::test]
    async fn test_short_circuit_and() {
        // false && (1 / 0)
        // If strict, (1/0) causes error. If short-circuit, returns false.
        let expr = Expression::BinaryOp {
            left: Box::new(Expression::BoolLiteral(false)),
            op: BinaryOperator::And,
            right: Box::new(Expression::BinaryOp {
                left: Box::new(Expression::Literal(1.0)),
                op: BinaryOperator::Div,
                right: Box::new(Expression::Literal(0.0)),
            }),
        };
        let ctx = json!({});
        let res = evaluate(&expr, &ctx, None, None).await;

        // Assert successful evaluation (short-circuit)
        assert!(res.is_ok());
        assert_eq!(res.unwrap(), json!(false));
    }

    #[tokio::test]
    async fn test_short_circuit_or() {
        // true || (1 / 0)
        // If strict, (1/0) causes error. If short-circuit, returns true.
        let expr = Expression::BinaryOp {
            left: Box::new(Expression::BoolLiteral(true)),
            op: BinaryOperator::Or,
            right: Box::new(Expression::BinaryOp {
                left: Box::new(Expression::Literal(1.0)),
                op: BinaryOperator::Div,
                right: Box::new(Expression::Literal(0.0)),
            }),
        };
        let ctx = json!({});
        let res = evaluate(&expr, &ctx, None, None).await;

        // Assert successful evaluation (short-circuit)
        assert!(res.is_ok());
        assert_eq!(res.unwrap(), json!(true));
    }

    #[tokio::test]
    async fn test_string_comparison() {
        let ctx = json!({});

        // "apple" < "banana"
        let expr = Expression::BinaryOp {
            left: Box::new(Expression::StringLiteral("apple".into())),
            op: BinaryOperator::Lt,
            right: Box::new(Expression::StringLiteral("banana".into())),
        };
        let res = evaluate(&expr, &ctx, None, None).await.unwrap();
        assert_eq!(res, json!(true));

        // "apple" > "banana" -> false
        let expr = Expression::BinaryOp {
            left: Box::new(Expression::StringLiteral("apple".into())),
            op: BinaryOperator::Gt,
            right: Box::new(Expression::StringLiteral("banana".into())),
        };
        let res = evaluate(&expr, &ctx, None, None).await.unwrap();
        assert_eq!(res, json!(false));

        // "2023-01-01" < "2023-12-31"
        let expr = Expression::BinaryOp {
            left: Box::new(Expression::StringLiteral("2023-01-01".into())),
            op: BinaryOperator::Lt,
            right: Box::new(Expression::StringLiteral("2023-12-31".into())),
        };
        let res = evaluate(&expr, &ctx, None, None).await.unwrap();
        assert_eq!(res, json!(true));
    }

    #[tokio::test]
    async fn test_eval_valid_date_coverage() {
        let ctx = json!({
            "null_field": null
        });

        // 1. Test argument count mismatch (0 args)
        let expr = Expression::FunctionCall {
            name: Symbol::from("valid_date"),
            args: vec![],
        };
        let res = evaluate(&expr, &ctx, None, None).await;
        assert!(res.is_err());

        // 2. Test Null argument
        let expr = Expression::FunctionCall {
            name: Symbol::from("valid_date"),
            args: vec![Expression::Field(Symbol::from("null_field"))],
        };
        let res = evaluate(&expr, &ctx, None, None).await.unwrap();
        assert_eq!(res, json!(false));

        // 3. Test non-string argument (Bool)
        let expr = Expression::FunctionCall {
            name: Symbol::from("valid_date"),
            args: vec![Expression::BoolLiteral(true)],
        };
        let res = evaluate(&expr, &ctx, None, None).await.unwrap();
        assert_eq!(res, json!(false));
    }
}
