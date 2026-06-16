//! Shared v1 contract metadata for function-level property checks.
//!
//! This module only defines and parses the contract vocabulary. Validation,
//! generation, execution, shrinking, and evidence writing live in later
//! property-checking modules.

use serde_json::Value;

/// Function effect classification used to decide property-test eligibility.
#[derive(Debug, Clone, Default, PartialEq, Eq)]
pub enum EffectClass {
    /// No explicit effect was declared.
    #[default]
    Unspecified,
    /// Function is expected to be pure and deterministic.
    Pure,
    /// Function may read deterministic local state but has no external effect.
    ReadOnlyDeterministic,
    /// Function uses IO, state, resources, or another effectful surface.
    Effectful,
}

/// A set of contracts attached to one function.
#[derive(Debug, Clone, Default, PartialEq)]
pub struct ContractSet {
    /// Declared effect class.
    pub effect: EffectClass,
    /// Preconditions used to filter generated inputs before execution.
    pub preconditions: Vec<ContractClause>,
    /// Postconditions checked after successful execution.
    pub postconditions: Vec<ContractClause>,
    /// Invariants preserved for future verifier work.
    pub invariants: Vec<ContractClause>,
}

impl ContractSet {
    /// Returns true when no contract metadata was declared.
    #[allow(dead_code)] // Used by later property-runner and describe/query cycles.
    #[must_use]
    pub fn is_empty(&self) -> bool {
        self.effect == EffectClass::Unspecified
            && self.preconditions.is_empty()
            && self.postconditions.is_empty()
            && self.invariants.is_empty()
    }
}

/// One named contract predicate.
#[derive(Debug, Clone, PartialEq)]
pub struct ContractClause {
    /// Stable clause id used by evidence and diagnostics.
    pub id: Option<String>,
    /// Optional human-facing label.
    pub label: Option<String>,
    /// Predicate expression.
    pub expr: ContractExpr,
}

/// Typed predicate expression node.
#[derive(Debug, Clone, PartialEq)]
pub enum ContractExpr {
    /// Reference to a function parameter or to `result` in postconditions.
    Var(String),
    /// Literal value.
    Const(ContractLiteral),
    /// Unary predicate or value expression.
    Unary {
        /// Operator.
        op: ContractOperator,
        /// Operand.
        expr: Box<ContractExpr>,
    },
    /// Binary predicate or value expression.
    Binary {
        /// Operator.
        op: ContractOperator,
        /// Left operand.
        left: Box<ContractExpr>,
        /// Right operand.
        right: Box<ContractExpr>,
    },
    /// Variadic predicate expression such as `and` or `or`.
    Nary {
        /// Operator.
        op: ContractOperator,
        /// Operands.
        args: Vec<ContractExpr>,
    },
}

/// Literal values supported by v1 predicate parsing.
#[derive(Debug, Clone, PartialEq)]
pub enum ContractLiteral {
    /// Boolean literal.
    Bool(bool),
    /// Signed 64-bit integer literal.
    I64(i64),
    /// Finite 64-bit floating-point literal.
    F64(f64),
    /// String literal.
    String(String),
    /// JSON literal preserved for later evaluator support.
    Json(Value),
    /// Null literal.
    Null,
}

/// Contract predicate operator.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum ContractOperator {
    /// Equality comparison.
    Eq,
    /// Inequality comparison.
    Ne,
    /// Less-than comparison.
    Lt,
    /// Less-than-or-equal comparison.
    Le,
    /// Greater-than comparison.
    Gt,
    /// Greater-than-or-equal comparison.
    Ge,
    /// Boolean conjunction.
    And,
    /// Boolean disjunction.
    Or,
    /// Boolean negation.
    Not,
    /// Addition.
    Add,
    /// Subtraction.
    Sub,
    /// Multiplication.
    Mul,
    /// Division.
    Div,
    /// Remainder.
    Rem,
    /// Bounded length operation.
    Length,
}

/// Parses a JSON-LD `duumbi:contracts` value into typed metadata.
///
/// The parser rejects malformed structure and unknown operators early, but it
/// intentionally leaves type compatibility and parameter-reference validation
/// for the graph validator.
pub fn parse_contract_set(value: &Value) -> Result<ContractSet, String> {
    let obj = value
        .as_object()
        .ok_or_else(|| "duumbi:contracts must be an object".to_string())?;

    let effect = match obj.get("duumbi:effect") {
        Some(value) => parse_effect(value)?,
        None => EffectClass::Unspecified,
    };

    Ok(ContractSet {
        effect,
        preconditions: parse_clause_array(obj.get("duumbi:preconditions"), "duumbi:preconditions")?,
        postconditions: parse_clause_array(
            obj.get("duumbi:postconditions"),
            "duumbi:postconditions",
        )?,
        invariants: parse_clause_array(obj.get("duumbi:invariants"), "duumbi:invariants")?,
    })
}

fn parse_effect(value: &Value) -> Result<EffectClass, String> {
    let effect = value
        .as_str()
        .ok_or_else(|| "duumbi:effect must be a string".to_string())?;
    match effect {
        "pure" => Ok(EffectClass::Pure),
        "read_only_deterministic" | "read-only-deterministic" => {
            Ok(EffectClass::ReadOnlyDeterministic)
        }
        "effectful" | "unsupported" => Ok(EffectClass::Effectful),
        other => Err(format!("unknown duumbi:effect '{other}'")),
    }
}

fn parse_clause_array(value: Option<&Value>, field: &str) -> Result<Vec<ContractClause>, String> {
    let Some(value) = value else {
        return Ok(Vec::new());
    };
    let clauses = value
        .as_array()
        .ok_or_else(|| format!("{field} must be an array"))?;
    clauses
        .iter()
        .enumerate()
        .map(|(index, clause)| parse_clause(clause, field, index))
        .collect()
}

fn parse_clause(value: &Value, field: &str, index: usize) -> Result<ContractClause, String> {
    let obj = value
        .as_object()
        .ok_or_else(|| format!("{field}[{index}] must be an object"))?;
    let id = obj
        .get("duumbi:id")
        .map(parse_optional_string)
        .transpose()?;
    let label = obj
        .get("duumbi:label")
        .map(parse_optional_string)
        .transpose()?;
    let expr = obj
        .get("duumbi:expr")
        .ok_or_else(|| format!("{field}[{index}] missing duumbi:expr"))
        .and_then(parse_expr)?;

    Ok(ContractClause { id, label, expr })
}

fn parse_optional_string(value: &Value) -> Result<String, String> {
    value
        .as_str()
        .map(ToOwned::to_owned)
        .ok_or_else(|| "contract id/label fields must be strings".to_string())
}

fn parse_expr(value: &Value) -> Result<ContractExpr, String> {
    let obj = value
        .as_object()
        .ok_or_else(|| "contract expression must be an object".to_string())?;

    if let Some(var) = obj.get("duumbi:var") {
        return var
            .as_str()
            .map(|name| ContractExpr::Var(name.to_string()))
            .ok_or_else(|| "duumbi:var must be a string".to_string());
    }

    if let Some(literal) = obj.get("duumbi:const") {
        return Ok(ContractExpr::Const(parse_literal(literal)));
    }

    let op_value = obj
        .get("duumbi:op")
        .ok_or_else(|| "contract expression missing duumbi:op".to_string())?;
    let op = parse_operator(
        op_value
            .as_str()
            .ok_or_else(|| "duumbi:op must be a string".to_string())?,
    )?;

    if let Some(args_value) = obj.get("duumbi:args") {
        let args = args_value
            .as_array()
            .ok_or_else(|| "duumbi:args must be an array".to_string())?
            .iter()
            .map(parse_expr)
            .collect::<Result<Vec<_>, _>>()?;
        return Ok(ContractExpr::Nary { op, args });
    }

    if let Some(expr_value) = obj.get("duumbi:expr") {
        return Ok(ContractExpr::Unary {
            op,
            expr: Box::new(parse_expr(expr_value)?),
        });
    }

    let left = obj
        .get("duumbi:left")
        .ok_or_else(|| "binary contract expression missing duumbi:left".to_string())
        .and_then(parse_expr)?;
    let right = obj
        .get("duumbi:right")
        .ok_or_else(|| "binary contract expression missing duumbi:right".to_string())
        .and_then(parse_expr)?;
    Ok(ContractExpr::Binary {
        op,
        left: Box::new(left),
        right: Box::new(right),
    })
}

fn parse_literal(value: &Value) -> ContractLiteral {
    match value {
        Value::Bool(value) => ContractLiteral::Bool(*value),
        Value::Number(value) => {
            if let Some(value) = value.as_i64() {
                ContractLiteral::I64(value)
            } else {
                ContractLiteral::F64(value.as_f64().unwrap_or_default())
            }
        }
        Value::String(value) => ContractLiteral::String(value.clone()),
        Value::Null => ContractLiteral::Null,
        other => ContractLiteral::Json(other.clone()),
    }
}

fn parse_operator(value: &str) -> Result<ContractOperator, String> {
    match value {
        "==" | "eq" => Ok(ContractOperator::Eq),
        "!=" | "ne" => Ok(ContractOperator::Ne),
        "<" | "lt" => Ok(ContractOperator::Lt),
        "<=" | "le" => Ok(ContractOperator::Le),
        ">" | "gt" => Ok(ContractOperator::Gt),
        ">=" | "ge" => Ok(ContractOperator::Ge),
        "and" => Ok(ContractOperator::And),
        "or" => Ok(ContractOperator::Or),
        "not" => Ok(ContractOperator::Not),
        "+" | "add" => Ok(ContractOperator::Add),
        "-" | "sub" => Ok(ContractOperator::Sub),
        "*" | "mul" => Ok(ContractOperator::Mul),
        "/" | "div" => Ok(ContractOperator::Div),
        "%" | "rem" => Ok(ContractOperator::Rem),
        "len" | "length" => Ok(ContractOperator::Length),
        other => Err(format!("unknown contract operator '{other}'")),
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn parses_contract_set_with_postcondition() {
        let value = serde_json::json!({
            "duumbi:effect": "pure",
            "duumbi:postconditions": [{
                "duumbi:id": "result-nonnegative",
                "duumbi:expr": {
                    "duumbi:op": ">=",
                    "duumbi:left": { "duumbi:var": "result" },
                    "duumbi:right": { "duumbi:const": 0 }
                }
            }]
        });

        let contracts = parse_contract_set(&value).expect("contract parse should succeed");
        assert_eq!(contracts.effect, EffectClass::Pure);
        assert_eq!(contracts.postconditions.len(), 1);
        assert_eq!(
            contracts.postconditions[0].expr,
            ContractExpr::Binary {
                op: ContractOperator::Ge,
                left: Box::new(ContractExpr::Var("result".to_string())),
                right: Box::new(ContractExpr::Const(ContractLiteral::I64(0))),
            }
        );
    }

    #[test]
    fn rejects_unknown_operator() {
        let value = serde_json::json!({
            "duumbi:postconditions": [{
                "duumbi:expr": {
                    "duumbi:op": "exec",
                    "duumbi:left": { "duumbi:var": "result" },
                    "duumbi:right": { "duumbi:const": 0 }
                }
            }]
        });

        let err = parse_contract_set(&value).unwrap_err();
        assert!(err.contains("unknown contract operator"));
    }
}
