use crate::datastore::DataStore;
use crate::errors::RuntimeError;
use chrono::{Datelike, NaiveDate, Utc};
use gurih_ir::{BinaryOperator, Expression, Schema, Symbol, UnaryOperator};
use serde_json::Value;
use std::sync::Arc;

pub async fn evaluate(
    expr: &Expression,
    context: &Value,
    schema: Option<&Schema>,
    datastore: Option<&Arc<dyn DataStore>>,
) -> Result<Value, RuntimeError> {
    match expr {
        Expression::Field(name) => {
            let key = name.as_str();
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
        Expression::Literal(n) => {
            Ok(Value::Number(serde_json::Number::from_f64(*n).ok_or_else(|| {
                RuntimeError::EvaluationError("Invalid float literal".to_string())
            })?))
        }
        Expression::StringLiteral(s) => Ok(Value::String(s.clone())),
        Expression::BoolLiteral(b) => Ok(Value::Bool(*b)),
        Expression::Grouping(inner) => Box::pin(evaluate(inner, context, schema, datastore)).await,
        Expression::UnaryOp { op, expr } => {
            let val = Box::pin(evaluate(expr, context, schema, datastore)).await?;
            eval_unary_op(op, val)
        }
        Expression::BinaryOp { left, op, right } => {
            match op {
                BinaryOperator::And => {
                    let l = Box::pin(evaluate(left, context, schema, datastore)).await?;
                    if !as_bool(&l)? {
                        return Ok(Value::Bool(false));
                    }
                    let r = Box::pin(evaluate(right, context, schema, datastore)).await?;
                    let r_bool = as_bool(&r)?;
                    Ok(Value::Bool(r_bool))
                }
                BinaryOperator::Or => {
                    let l = Box::pin(evaluate(left, context, schema, datastore)).await?;
                    if as_bool(&l)? {
                        return Ok(Value::Bool(true));
                    }
                    let r = Box::pin(evaluate(right, context, schema, datastore)).await?;
                    let r_bool = as_bool(&r)?;
                    Ok(Value::Bool(r_bool))
                }
                _ => {
                    let l = Box::pin(evaluate(left, context, schema, datastore)).await?;
                    let r = Box::pin(evaluate(right, context, schema, datastore)).await?;
                    eval_binary_op(op, l, r)
                }
            }
        }
        Expression::FunctionCall { name, args } => {
            let mut eval_args = Vec::new();
            for arg in args {
                eval_args.push(Box::pin(evaluate(arg, context, schema, datastore)).await?);
            }
            eval_function(name.as_str(), &eval_args, schema, datastore).await
        }
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
        BinaryOperator::Gt => {
            let l = as_f64(&left)?;
            let r = as_f64(&right)?;
            Ok(Value::Bool(l > r))
        }
        BinaryOperator::Lt => {
            let l = as_f64(&left)?;
            let r = as_f64(&right)?;
            Ok(Value::Bool(l < r))
        }
        BinaryOperator::Gte => {
            let l = as_f64(&left)?;
            let r = as_f64(&right)?;
            Ok(Value::Bool(l >= r))
        }
        BinaryOperator::Lte => {
            let l = as_f64(&left)?;
            let r = as_f64(&right)?;
            Ok(Value::Bool(l <= r))
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
    }
}

async fn eval_function(
    name: &str,
    args: &[Value],
    schema: Option<&Schema>,
    datastore: Option<&Arc<dyn DataStore>>,
) -> Result<Value, RuntimeError> {
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

            Ok(Value::Number(serde_json::Number::from(age)))
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

            Ok(Value::Number(serde_json::Number::from(years)))
        }
        "is_set" => {
            if args.len() != 1 {
                return Err(RuntimeError::EvaluationError("is_set() takes 1 argument".into()));
            }
            match &args[0] {
                Value::Null => Ok(Value::Bool(false)),
                Value::String(s) => Ok(Value::Bool(!s.is_empty())),
                _ => Ok(Value::Bool(true)),
            }
        }
        "valid_date" => {
            if args.len() != 1 {
                return Err(RuntimeError::EvaluationError("valid_date() takes 1 argument".into()));
            }
            let date_str = match &args[0] {
                Value::String(s) => s,
                Value::Null => return Ok(Value::Bool(false)),
                _ => return Ok(Value::Bool(false)),
            };

            let valid = NaiveDate::parse_from_str(date_str, "%Y-%m-%d").is_ok();
            Ok(Value::Bool(valid))
        }
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
    match v {
        Value::Number(n) => n
            .as_f64()
            .ok_or_else(|| RuntimeError::EvaluationError(format!("Type mismatch: expected f64, found {:?}", v))),
        Value::String(s) => s
            .parse::<f64>()
            .map_err(|_| RuntimeError::EvaluationError(format!("Invalid number format in string: {}", s))),
        _ => Err(RuntimeError::EvaluationError(format!(
            "Type mismatch: expected number, found {:?}",
            v
        ))),
    }
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

fn to_snake_case(s: &str) -> String {
    let mut result = String::new();
    for (i, c) in s.char_indices() {
        if c.is_uppercase() {
            if i > 0 {
                result.push('_');
            }
            result.push(c.to_ascii_lowercase());
        } else {
            result.push(c);
        }
    }
    result
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
}
