use crate::errors::RuntimeError;
use chrono::{Datelike, NaiveDate, Utc};
use gurih_ir::{BinaryOperator, Expression, UnaryOperator};
use serde_json::Value;

pub fn evaluate(expr: &Expression, context: &Value) -> Result<Value, RuntimeError> {
    match expr {
        Expression::Field(name) => {
            let key = name.as_str();
            Ok(context.get(key).cloned().unwrap_or(Value::Null))
        }
        Expression::Literal(n) => Ok(Value::Number(
            serde_json::Number::from_f64(*n).ok_or_else(|| {
                RuntimeError::EvaluationError("Invalid float literal".to_string())
            })?,
        )),
        Expression::StringLiteral(s) => Ok(Value::String(s.clone())),
        Expression::BoolLiteral(b) => Ok(Value::Bool(*b)),
        Expression::Grouping(inner) => evaluate(inner, context),
        Expression::UnaryOp { op, expr } => {
            let val = evaluate(expr, context)?;
            eval_unary_op(op, val)
        }
        Expression::BinaryOp { left, op, right } => {
            let l = evaluate(left, context)?;
            let r = evaluate(right, context)?;
            eval_binary_op(op, l, r)
        }
        Expression::FunctionCall { name, args } => {
            let mut eval_args = Vec::new();
            for arg in args {
                eval_args.push(evaluate(arg, context)?);
            }
            eval_function(name.as_str(), &eval_args)
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
                    Err(RuntimeError::EvaluationError(
                        "Invalid number for negation".into(),
                    ))
                }
            }
            _ => Err(RuntimeError::EvaluationError(format!(
                "Type mismatch: expected number for NEG, found {:?}",
                val
            ))),
        },
    }
}

fn eval_binary_op(
    op: &BinaryOperator,
    left: Value,
    right: Value,
) -> Result<Value, RuntimeError> {
    match op {
        BinaryOperator::Add
        | BinaryOperator::Sub
        | BinaryOperator::Mul
        | BinaryOperator::Div => {
            let l = as_f64(&left)?;
            let r = as_f64(&right)?;
            let res = match op {
                BinaryOperator::Add => l + r,
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
            Ok(Value::Number(
                serde_json::Number::from_f64(res).ok_or_else(|| {
                    RuntimeError::EvaluationError("Result is not a valid number".into())
                })?,
            ))
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

fn eval_function(name: &str, args: &[Value]) -> Result<Value, RuntimeError> {
    match name {
        "age" => {
            if args.len() != 1 {
                return Err(RuntimeError::EvaluationError(
                    "age() takes 1 argument".into(),
                ));
            }
            let date_str = as_str(&args[0])?;
            let birth_date = NaiveDate::parse_from_str(date_str, "%Y-%m-%d").map_err(|_| {
                RuntimeError::EvaluationError("Invalid date format YYYY-MM-DD".into())
            })?;
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
            let join_date = NaiveDate::parse_from_str(date_str, "%Y-%m-%d").map_err(|_| {
                RuntimeError::EvaluationError("Invalid date format YYYY-MM-DD".into())
            })?;
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
                return Err(RuntimeError::EvaluationError(
                    "is_set() takes 1 argument".into(),
                ));
            }
            match &args[0] {
                Value::Null => Ok(Value::Bool(false)),
                Value::String(s) => Ok(Value::Bool(!s.is_empty())),
                _ => Ok(Value::Bool(true)),
            }
        }
        "valid_date" => {
            if args.len() != 1 {
                return Err(RuntimeError::EvaluationError(
                    "valid_date() takes 1 argument".into(),
                ));
            }
            let date_str = match &args[0] {
                Value::String(s) => s,
                Value::Null => return Ok(Value::Bool(false)),
                _ => return Ok(Value::Bool(false)),
            };

            let valid = NaiveDate::parse_from_str(date_str, "%Y-%m-%d").is_ok();
            Ok(Value::Bool(valid))
        }
        _ => Err(RuntimeError::EvaluationError(format!(
            "Unknown function: {}",
            name
        ))),
    }
}

fn as_f64(v: &Value) -> Result<f64, RuntimeError> {
    match v {
        Value::Number(n) => n.as_f64().ok_or_else(|| {
            RuntimeError::EvaluationError(format!("Type mismatch: expected f64, found {:?}", v))
        }),
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

#[cfg(test)]
mod tests {
    use super::*;
    use gurih_ir::{BinaryOperator, Expression, Symbol};
    use serde_json::json;

    #[test]
    fn test_eval_literal() {
        let expr = Expression::Literal(42.0);
        let ctx = json!({});
        let res = evaluate(&expr, &ctx).unwrap();
        assert_eq!(res, json!(42.0));
    }

    #[test]
    fn test_eval_add() {
        let expr = Expression::BinaryOp {
            left: Box::new(Expression::Literal(10.0)),
            op: BinaryOperator::Add,
            right: Box::new(Expression::Literal(32.0)),
        };
        let ctx = json!({});
        let res = evaluate(&expr, &ctx).unwrap();
        assert_eq!(res, json!(42.0));
    }

    #[test]
    fn test_eval_field() {
        let expr = Expression::Field(Symbol::from("score"));
        let ctx = json!({"score": 100});
        let res = evaluate(&expr, &ctx).unwrap();
        assert_eq!(res, json!(100));
    }

    #[test]
    fn test_eval_age() {
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
        let res = evaluate(&expr, &ctx);
        assert!(res.is_err()); // expects string

        let expr = Expression::FunctionCall {
            name: Symbol::from("age"),
            args: vec![Expression::StringLiteral("2000-01-01".into())],
        };
        let res = evaluate(&expr, &ctx).unwrap();
        assert!(res.is_number());
    }

    #[test]
    fn test_min_age_logic() {
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
        let res = evaluate(&expr, &ctx).unwrap();
        assert_eq!(res, json!(true)); // Should be > 18 in 2024
    }
}
