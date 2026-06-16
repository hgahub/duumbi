//! Shared v1 contract metadata for function-level property checks.
//!
//! This module only defines and parses the contract vocabulary. Validation,
//! generation, execution, shrinking, and evidence writing live in later
//! property-checking modules.

use std::collections::{HashMap, HashSet};

use crate::errors::codes;
use crate::types::DuumbiType;
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

/// A validation issue found in parsed contract metadata.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct ContractValidationIssue {
    /// DUUMBI diagnostic code.
    pub code: &'static str,
    /// Human-readable validation message.
    pub message: String,
    /// Optional clause id associated with the issue.
    pub clause_id: Option<String>,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
enum ClauseKind {
    Precondition,
    Postcondition,
    Invariant,
}

impl ClauseKind {
    fn as_str(self) -> &'static str {
        match self {
            ClauseKind::Precondition => "precondition",
            ClauseKind::Postcondition => "postcondition",
            ClauseKind::Invariant => "invariant",
        }
    }

    fn allows_result(self) -> bool {
        matches!(self, ClauseKind::Postcondition)
    }
}

/// Validates parameter references, result usage, duplicate ids, and expression
/// type compatibility for one function's contract set.
#[must_use]
pub fn validate_contract_set(
    contracts: &ContractSet,
    params: &[(String, DuumbiType)],
    return_type: &DuumbiType,
) -> Vec<ContractValidationIssue> {
    let mut issues = Vec::new();
    let param_types: HashMap<&str, &DuumbiType> = params
        .iter()
        .map(|(name, ty)| (name.as_str(), ty))
        .collect();

    check_duplicate_clause_ids(contracts, &mut issues);

    for clause in &contracts.preconditions {
        validate_clause(
            clause,
            ClauseKind::Precondition,
            &param_types,
            return_type,
            &mut issues,
        );
    }
    for clause in &contracts.postconditions {
        validate_clause(
            clause,
            ClauseKind::Postcondition,
            &param_types,
            return_type,
            &mut issues,
        );
    }
    for clause in &contracts.invariants {
        validate_clause(
            clause,
            ClauseKind::Invariant,
            &param_types,
            return_type,
            &mut issues,
        );
    }

    issues
}

fn check_duplicate_clause_ids(contracts: &ContractSet, issues: &mut Vec<ContractValidationIssue>) {
    let mut seen = HashSet::new();
    for clause in contracts
        .preconditions
        .iter()
        .chain(contracts.postconditions.iter())
        .chain(contracts.invariants.iter())
    {
        let Some(id) = &clause.id else {
            continue;
        };
        if !seen.insert(id.as_str()) {
            issues.push(ContractValidationIssue {
                code: codes::E009_SCHEMA_INVALID,
                message: format!("Duplicate contract id '{id}'"),
                clause_id: Some(id.clone()),
            });
        }
    }
}

fn validate_clause(
    clause: &ContractClause,
    kind: ClauseKind,
    param_types: &HashMap<&str, &DuumbiType>,
    return_type: &DuumbiType,
    issues: &mut Vec<ContractValidationIssue>,
) {
    let Some(expr_type) = infer_expr_type(&clause.expr, kind, param_types, return_type, issues)
    else {
        return;
    };
    if expr_type != DuumbiType::Bool {
        issues.push(ContractValidationIssue {
            code: codes::E001_TYPE_MISMATCH,
            message: format!(
                "Contract {} must evaluate to bool, found {}",
                kind.as_str(),
                expr_type
            ),
            clause_id: clause.id.clone(),
        });
    }
}

fn infer_expr_type(
    expr: &ContractExpr,
    kind: ClauseKind,
    param_types: &HashMap<&str, &DuumbiType>,
    return_type: &DuumbiType,
    issues: &mut Vec<ContractValidationIssue>,
) -> Option<DuumbiType> {
    match expr {
        ContractExpr::Var(name) if name == "result" => {
            if kind.allows_result() {
                Some(return_type.clone())
            } else {
                issues.push(ContractValidationIssue {
                    code: codes::E009_SCHEMA_INVALID,
                    message: format!(
                        "`result` may only be referenced from postconditions, not {}s",
                        kind.as_str()
                    ),
                    clause_id: None,
                });
                None
            }
        }
        ContractExpr::Var(name) => match param_types.get(name.as_str()) {
            Some(ty) => Some((*ty).clone()),
            None => {
                issues.push(ContractValidationIssue {
                    code: codes::E009_SCHEMA_INVALID,
                    message: format!("Unknown contract variable '{name}' in {}", kind.as_str()),
                    clause_id: None,
                });
                None
            }
        },
        ContractExpr::Const(literal) => Some(literal_type(literal)),
        ContractExpr::Unary { op, expr } => {
            infer_unary_type(*op, expr, kind, param_types, return_type, issues)
        }
        ContractExpr::Binary { op, left, right } => {
            infer_binary_type(*op, left, right, kind, param_types, return_type, issues)
        }
        ContractExpr::Nary { op, args } => {
            infer_nary_type(*op, args, kind, param_types, return_type, issues)
        }
    }
}

fn infer_unary_type(
    op: ContractOperator,
    expr: &ContractExpr,
    kind: ClauseKind,
    param_types: &HashMap<&str, &DuumbiType>,
    return_type: &DuumbiType,
    issues: &mut Vec<ContractValidationIssue>,
) -> Option<DuumbiType> {
    let operand = infer_expr_type(expr, kind, param_types, return_type, issues)?;
    match op {
        ContractOperator::Not if operand == DuumbiType::Bool => Some(DuumbiType::Bool),
        ContractOperator::Not => {
            push_type_issue(
                issues,
                "not operand must be bool",
                &DuumbiType::Bool,
                &operand,
            );
            None
        }
        ContractOperator::Length if supports_length(&operand) => Some(DuumbiType::I64),
        ContractOperator::Length => {
            issues.push(ContractValidationIssue {
                code: codes::E001_TYPE_MISMATCH,
                message: format!("length operand must be string, json, or array; found {operand}"),
                clause_id: None,
            });
            None
        }
        other => {
            issues.push(ContractValidationIssue {
                code: codes::E009_SCHEMA_INVALID,
                message: format!("Operator '{other:?}' is not a unary contract operator"),
                clause_id: None,
            });
            None
        }
    }
}

fn infer_binary_type(
    op: ContractOperator,
    left: &ContractExpr,
    right: &ContractExpr,
    kind: ClauseKind,
    param_types: &HashMap<&str, &DuumbiType>,
    return_type: &DuumbiType,
    issues: &mut Vec<ContractValidationIssue>,
) -> Option<DuumbiType> {
    let left_type = infer_expr_type(left, kind, param_types, return_type, issues)?;
    let right_type = infer_expr_type(right, kind, param_types, return_type, issues)?;
    match op {
        ContractOperator::Eq | ContractOperator::Ne => {
            if types_compatible_for_equality(&left_type, &right_type) {
                Some(DuumbiType::Bool)
            } else {
                push_type_issue(
                    issues,
                    "equality operands must have compatible types",
                    &left_type,
                    &right_type,
                );
                None
            }
        }
        ContractOperator::Lt
        | ContractOperator::Le
        | ContractOperator::Gt
        | ContractOperator::Ge => {
            if numeric_pair(&left_type, &right_type) {
                Some(DuumbiType::Bool)
            } else {
                issues.push(ContractValidationIssue {
                    code: codes::E001_TYPE_MISMATCH,
                    message: format!(
                        "ordered comparison operands must be numeric; found {left_type} and {right_type}"
                    ),
                    clause_id: None,
                });
                None
            }
        }
        ContractOperator::Add
        | ContractOperator::Sub
        | ContractOperator::Mul
        | ContractOperator::Div
        | ContractOperator::Rem => {
            if numeric_pair(&left_type, &right_type) {
                if left_type == DuumbiType::F64 || right_type == DuumbiType::F64 {
                    Some(DuumbiType::F64)
                } else {
                    Some(DuumbiType::I64)
                }
            } else {
                issues.push(ContractValidationIssue {
                    code: codes::E001_TYPE_MISMATCH,
                    message: format!(
                        "arithmetic operands must be numeric; found {left_type} and {right_type}"
                    ),
                    clause_id: None,
                });
                None
            }
        }
        other => {
            issues.push(ContractValidationIssue {
                code: codes::E009_SCHEMA_INVALID,
                message: format!("Operator '{other:?}' is not a binary contract operator"),
                clause_id: None,
            });
            None
        }
    }
}

fn infer_nary_type(
    op: ContractOperator,
    args: &[ContractExpr],
    kind: ClauseKind,
    param_types: &HashMap<&str, &DuumbiType>,
    return_type: &DuumbiType,
    issues: &mut Vec<ContractValidationIssue>,
) -> Option<DuumbiType> {
    match op {
        ContractOperator::And | ContractOperator::Or => {
            if args.is_empty() {
                issues.push(ContractValidationIssue {
                    code: codes::E009_SCHEMA_INVALID,
                    message: format!("Operator '{op:?}' requires at least one argument"),
                    clause_id: None,
                });
                return None;
            }
            for arg in args {
                let Some(arg_type) = infer_expr_type(arg, kind, param_types, return_type, issues)
                else {
                    continue;
                };
                if arg_type != DuumbiType::Bool {
                    push_type_issue(
                        issues,
                        "boolean operator operands must be bool",
                        &DuumbiType::Bool,
                        &arg_type,
                    );
                    return None;
                }
            }
            Some(DuumbiType::Bool)
        }
        other => {
            issues.push(ContractValidationIssue {
                code: codes::E009_SCHEMA_INVALID,
                message: format!("Operator '{other:?}' is not a variadic contract operator"),
                clause_id: None,
            });
            None
        }
    }
}

fn literal_type(literal: &ContractLiteral) -> DuumbiType {
    match literal {
        ContractLiteral::Bool(_) => DuumbiType::Bool,
        ContractLiteral::I64(_) => DuumbiType::I64,
        ContractLiteral::F64(_) => DuumbiType::F64,
        ContractLiteral::String(_) => DuumbiType::String,
        ContractLiteral::Json(_) | ContractLiteral::Null => DuumbiType::Json,
    }
}

fn numeric_pair(left: &DuumbiType, right: &DuumbiType) -> bool {
    matches!(left, DuumbiType::I64 | DuumbiType::F64)
        && matches!(right, DuumbiType::I64 | DuumbiType::F64)
}

fn supports_length(ty: &DuumbiType) -> bool {
    matches!(
        ty,
        DuumbiType::String | DuumbiType::Json | DuumbiType::Array(_)
    )
}

fn types_compatible_for_equality(left: &DuumbiType, right: &DuumbiType) -> bool {
    left == right || numeric_pair(left, right)
}

fn push_type_issue(
    issues: &mut Vec<ContractValidationIssue>,
    message: impl Into<String>,
    expected: &DuumbiType,
    found: &DuumbiType,
) {
    issues.push(ContractValidationIssue {
        code: codes::E001_TYPE_MISMATCH,
        message: format!("{}; expected {}, found {}", message.into(), expected, found),
        clause_id: None,
    });
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
