//! Local predicate evaluation for contract expressions.

use std::collections::BTreeMap;

use crate::contracts::{ContractExpr, ContractLiteral, ContractOperator};

use super::value::PropertyValue;

/// Evaluation context for one property case.
#[derive(Debug, Clone, Default, PartialEq)]
pub struct PredicateContext {
    bindings: BTreeMap<String, PropertyValue>,
    result: Option<PropertyValue>,
}

impl PredicateContext {
    /// Adds or replaces a named parameter binding.
    #[must_use]
    pub fn with_binding(mut self, name: impl Into<String>, value: PropertyValue) -> Self {
        self.bindings.insert(name.into(), value);
        self
    }

    /// Adds or replaces the function result binding.
    #[must_use]
    pub fn with_result(mut self, value: PropertyValue) -> Self {
        self.result = Some(value);
        self
    }
}

/// Predicate evaluation error.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct PredicateEvalError {
    /// Stable reason for evidence.
    pub reason: &'static str,
    /// Human-readable details.
    pub detail: String,
}

/// Evaluates a contract expression as a boolean predicate.
pub fn eval_predicate(
    expr: &ContractExpr,
    context: &PredicateContext,
) -> Result<bool, PredicateEvalError> {
    match eval_expr(expr, context)? {
        PropertyValue::Bool(value) => Ok(value),
        other => Err(error(
            "predicate_not_bool",
            format!("predicate evaluated to {}", other.type_label()),
        )),
    }
}

fn eval_expr(
    expr: &ContractExpr,
    context: &PredicateContext,
) -> Result<PropertyValue, PredicateEvalError> {
    match expr {
        ContractExpr::Var(name) if name == "result" => context
            .result
            .clone()
            .ok_or_else(|| error("missing_result", "`result` is not bound")),
        ContractExpr::Var(name) => context
            .bindings
            .get(name)
            .cloned()
            .ok_or_else(|| error("unknown_variable", format!("'{name}' is not bound"))),
        ContractExpr::Const(literal) => Ok(literal_to_value(literal)),
        ContractExpr::Unary { op, expr } => eval_unary(*op, expr, context),
        ContractExpr::Binary { op, left, right } => eval_binary(*op, left, right, context),
        ContractExpr::Nary { op, args } => eval_nary(*op, args, context),
    }
}

fn eval_unary(
    op: ContractOperator,
    expr: &ContractExpr,
    context: &PredicateContext,
) -> Result<PropertyValue, PredicateEvalError> {
    let value = eval_expr(expr, context)?;
    match (op, value) {
        (ContractOperator::Not, PropertyValue::Bool(value)) => Ok(PropertyValue::Bool(!value)),
        (ContractOperator::Length, PropertyValue::String(value)) => {
            Ok(PropertyValue::I64(value.chars().count() as i64))
        }
        (ContractOperator::Length, PropertyValue::Array(values)) => {
            Ok(PropertyValue::I64(values.len() as i64))
        }
        (ContractOperator::Length, PropertyValue::Json(value)) => match value {
            serde_json::Value::Array(values) => Ok(PropertyValue::I64(values.len() as i64)),
            serde_json::Value::Object(values) => Ok(PropertyValue::I64(values.len() as i64)),
            serde_json::Value::String(value) => {
                Ok(PropertyValue::I64(value.chars().count() as i64))
            }
            other => Err(error(
                "unsupported_length_operand",
                format!("cannot take length of json {}", json_kind(&other)),
            )),
        },
        (ContractOperator::Not, other) => Err(error(
            "type_mismatch",
            format!("not operand must be bool, found {}", other.type_label()),
        )),
        (ContractOperator::Length, other) => Err(error(
            "type_mismatch",
            format!(
                "length operand must be string, array, or json, found {}",
                other.type_label()
            ),
        )),
        (other, _) => Err(error(
            "unsupported_operator_shape",
            format!("{other:?} is not a unary operator"),
        )),
    }
}

fn eval_binary(
    op: ContractOperator,
    left: &ContractExpr,
    right: &ContractExpr,
    context: &PredicateContext,
) -> Result<PropertyValue, PredicateEvalError> {
    let left = eval_expr(left, context)?;
    let right = eval_expr(right, context)?;
    match op {
        ContractOperator::Eq => Ok(PropertyValue::Bool(values_equal(&left, &right)?)),
        ContractOperator::Ne => Ok(PropertyValue::Bool(!values_equal(&left, &right)?)),
        ContractOperator::Lt => compare_numeric(
            &left,
            &right,
            |left, right| left < right,
            |left, right| left < right,
        ),
        ContractOperator::Le => compare_numeric(
            &left,
            &right,
            |left, right| left <= right,
            |left, right| left <= right,
        ),
        ContractOperator::Gt => compare_numeric(
            &left,
            &right,
            |left, right| left > right,
            |left, right| left > right,
        ),
        ContractOperator::Ge => compare_numeric(
            &left,
            &right,
            |left, right| left >= right,
            |left, right| left >= right,
        ),
        ContractOperator::Add => arithmetic_numeric(
            &left,
            &right,
            |left, right| left.checked_add(right),
            |left, right| left + right,
            "addition",
        ),
        ContractOperator::Sub => arithmetic_numeric(
            &left,
            &right,
            |left, right| left.checked_sub(right),
            |left, right| left - right,
            "subtraction",
        ),
        ContractOperator::Mul => arithmetic_numeric(
            &left,
            &right,
            |left, right| left.checked_mul(right),
            |left, right| left * right,
            "multiplication",
        ),
        ContractOperator::Div => {
            if numeric_zero(&right)? {
                return Err(error("division_by_zero", "contract division by zero"));
            }
            arithmetic_numeric(
                &left,
                &right,
                |left, right| left.checked_div(right),
                |left, right| left / right,
                "division",
            )
        }
        ContractOperator::Rem => match (&left, &right) {
            (PropertyValue::I64(_), PropertyValue::I64(0)) => {
                Err(error("division_by_zero", "contract remainder by zero"))
            }
            (PropertyValue::I64(left), PropertyValue::I64(right)) => left
                .checked_rem(*right)
                .map(PropertyValue::I64)
                .ok_or_else(|| arithmetic_overflow("remainder")),
            _ => Err(type_error("remainder operands must be i64", &left, &right)),
        },
        other => Err(error(
            "unsupported_operator_shape",
            format!("{other:?} is not a binary operator"),
        )),
    }
}

fn eval_nary(
    op: ContractOperator,
    args: &[ContractExpr],
    context: &PredicateContext,
) -> Result<PropertyValue, PredicateEvalError> {
    match op {
        ContractOperator::And => {
            for arg in args {
                if !eval_predicate(arg, context)? {
                    return Ok(PropertyValue::Bool(false));
                }
            }
            Ok(PropertyValue::Bool(true))
        }
        ContractOperator::Or => {
            for arg in args {
                if eval_predicate(arg, context)? {
                    return Ok(PropertyValue::Bool(true));
                }
            }
            Ok(PropertyValue::Bool(false))
        }
        other => Err(error(
            "unsupported_operator_shape",
            format!("{other:?} is not a variadic operator"),
        )),
    }
}

fn literal_to_value(literal: &ContractLiteral) -> PropertyValue {
    match literal {
        ContractLiteral::Bool(value) => PropertyValue::Bool(*value),
        ContractLiteral::I64(value) => PropertyValue::I64(*value),
        ContractLiteral::F64(value) => PropertyValue::F64(*value),
        ContractLiteral::String(value) => PropertyValue::String(value.clone()),
        ContractLiteral::Json(value) => PropertyValue::Json(value.clone()),
        ContractLiteral::Null => PropertyValue::Json(serde_json::Value::Null),
    }
}

fn values_equal(left: &PropertyValue, right: &PropertyValue) -> Result<bool, PredicateEvalError> {
    match (left, right) {
        (PropertyValue::I64(left), PropertyValue::I64(right)) => Ok(left == right),
        (PropertyValue::F64(left), PropertyValue::F64(right)) => Ok(left == right),
        (PropertyValue::I64(left), PropertyValue::F64(right)) => Ok(*left as f64 == *right),
        (PropertyValue::F64(left), PropertyValue::I64(right)) => Ok(*left == *right as f64),
        (PropertyValue::Bool(left), PropertyValue::Bool(right)) => Ok(left == right),
        (PropertyValue::String(left), PropertyValue::String(right)) => Ok(left == right),
        (PropertyValue::Json(left), PropertyValue::Json(right)) => Ok(left == right),
        (PropertyValue::Array(left), PropertyValue::Array(right)) => Ok(left == right),
        (PropertyValue::Option(left), PropertyValue::Option(right)) => Ok(left == right),
        (PropertyValue::ResultOk(left), PropertyValue::ResultOk(right))
        | (PropertyValue::ResultErr(left), PropertyValue::ResultErr(right)) => Ok(left == right),
        _ => Err(type_error(
            "equality operands have incompatible types",
            left,
            right,
        )),
    }
}

fn compare_numeric(
    left: &PropertyValue,
    right: &PropertyValue,
    compare_i64: impl FnOnce(i64, i64) -> bool,
    compare_f64: impl FnOnce(f64, f64) -> bool,
) -> Result<PropertyValue, PredicateEvalError> {
    if let (PropertyValue::I64(left), PropertyValue::I64(right)) = (left, right) {
        Ok(PropertyValue::Bool(compare_i64(*left, *right)))
    } else {
        Ok(PropertyValue::Bool(compare_f64(
            numeric_value(left)?,
            numeric_value(right)?,
        )))
    }
}

fn arithmetic_numeric(
    left: &PropertyValue,
    right: &PropertyValue,
    checked_i64: impl FnOnce(i64, i64) -> Option<i64>,
    operation: impl FnOnce(f64, f64) -> f64,
    operator: &'static str,
) -> Result<PropertyValue, PredicateEvalError> {
    if let (PropertyValue::I64(left), PropertyValue::I64(right)) = (left, right) {
        checked_i64(*left, *right)
            .map(PropertyValue::I64)
            .ok_or_else(|| arithmetic_overflow(operator))
    } else {
        let value = operation(numeric_value(left)?, numeric_value(right)?);
        Ok(PropertyValue::F64(value))
    }
}

fn arithmetic_overflow(operator: &'static str) -> PredicateEvalError {
    error(
        "arithmetic_overflow",
        format!("i64 contract {operator} overflowed"),
    )
}

fn numeric_value(value: &PropertyValue) -> Result<f64, PredicateEvalError> {
    match value {
        PropertyValue::I64(value) => Ok(*value as f64),
        PropertyValue::F64(value) => Ok(*value),
        other => Err(error(
            "type_mismatch",
            format!("expected numeric value, found {}", other.type_label()),
        )),
    }
}

fn numeric_zero(value: &PropertyValue) -> Result<bool, PredicateEvalError> {
    Ok(match value {
        PropertyValue::I64(value) => *value == 0,
        PropertyValue::F64(value) => *value == 0.0,
        other => {
            return Err(error(
                "type_mismatch",
                format!("expected numeric value, found {}", other.type_label()),
            ));
        }
    })
}

fn type_error(
    detail: &'static str,
    left: &PropertyValue,
    right: &PropertyValue,
) -> PredicateEvalError {
    error(
        "type_mismatch",
        format!(
            "{detail}; found {} and {}",
            left.type_label(),
            right.type_label()
        ),
    )
}

fn json_kind(value: &serde_json::Value) -> &'static str {
    match value {
        serde_json::Value::Null => "null",
        serde_json::Value::Bool(_) => "bool",
        serde_json::Value::Number(_) => "number",
        serde_json::Value::String(_) => "string",
        serde_json::Value::Array(_) => "array",
        serde_json::Value::Object(_) => "object",
    }
}

fn error(reason: &'static str, detail: impl Into<String>) -> PredicateEvalError {
    PredicateEvalError {
        reason,
        detail: detail.into(),
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn var(name: &str) -> ContractExpr {
        ContractExpr::Var(name.to_string())
    }

    fn i64_const(value: i64) -> ContractExpr {
        ContractExpr::Const(ContractLiteral::I64(value))
    }

    #[test]
    fn evaluates_result_postcondition() {
        let expr = ContractExpr::Binary {
            op: ContractOperator::Ge,
            left: Box::new(var("result")),
            right: Box::new(i64_const(0)),
        };
        let context = PredicateContext::default().with_result(PropertyValue::I64(4));
        assert_eq!(eval_predicate(&expr, &context), Ok(true));
    }

    #[test]
    fn evaluates_precondition_false() {
        let expr = ContractExpr::Binary {
            op: ContractOperator::Gt,
            left: Box::new(var("n")),
            right: Box::new(i64_const(0)),
        };
        let context = PredicateContext::default().with_binding("n", PropertyValue::I64(-1));
        assert_eq!(eval_predicate(&expr, &context), Ok(false));
    }

    #[test]
    fn boolean_combinators_short_circuit() {
        let expr = ContractExpr::Nary {
            op: ContractOperator::Or,
            args: vec![
                ContractExpr::Const(ContractLiteral::Bool(true)),
                ContractExpr::Var("missing".to_string()),
            ],
        };
        assert_eq!(
            eval_predicate(&expr, &PredicateContext::default()),
            Ok(true)
        );
    }

    #[test]
    fn reports_unknown_variable() {
        let err = eval_predicate(&var("missing"), &PredicateContext::default()).unwrap_err();
        assert_eq!(err.reason, "unknown_variable");
    }

    #[test]
    fn reports_division_by_zero() {
        let expr = ContractExpr::Binary {
            op: ContractOperator::Div,
            left: Box::new(i64_const(1)),
            right: Box::new(i64_const(0)),
        };
        let err = eval_predicate(&expr, &PredicateContext::default()).unwrap_err();
        assert_eq!(err.reason, "division_by_zero");
    }

    #[test]
    fn i64_arithmetic_stays_exact() {
        let expr = ContractExpr::Binary {
            op: ContractOperator::Add,
            left: Box::new(i64_const(i64::MAX - 1)),
            right: Box::new(i64_const(1)),
        };

        assert_eq!(
            eval_expr(&expr, &PredicateContext::default()),
            Ok(PropertyValue::I64(i64::MAX))
        );
    }

    #[test]
    fn reports_i64_arithmetic_overflow() {
        let expr = ContractExpr::Binary {
            op: ContractOperator::Add,
            left: Box::new(i64_const(i64::MAX)),
            right: Box::new(i64_const(1)),
        };

        let err = eval_expr(&expr, &PredicateContext::default()).unwrap_err();
        assert_eq!(err.reason, "arithmetic_overflow");
    }

    #[test]
    fn reports_min_remainder_negative_one_as_overflow() {
        let expr = ContractExpr::Binary {
            op: ContractOperator::Rem,
            left: Box::new(i64_const(i64::MIN)),
            right: Box::new(i64_const(-1)),
        };

        let err = eval_expr(&expr, &PredicateContext::default()).unwrap_err();
        assert_eq!(err.reason, "arithmetic_overflow");
    }
}
