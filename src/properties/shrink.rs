//! Deterministic shrink candidate generation for property values.

use std::collections::BTreeMap;

use super::value::PropertyValue;

/// Produces deterministic simpler candidates for a failing value.
#[must_use]
pub fn shrink_candidates(value: &PropertyValue) -> Vec<PropertyValue> {
    match value {
        PropertyValue::I64(value) => shrink_i64(*value),
        PropertyValue::F64(value) => shrink_f64(*value),
        PropertyValue::Bool(true) => vec![PropertyValue::Bool(false)],
        PropertyValue::Bool(false) => Vec::new(),
        PropertyValue::String(value) => shrink_string(value),
        PropertyValue::Json(value) => shrink_json(value),
        PropertyValue::Array(values) => shrink_array(values),
        PropertyValue::Struct { name, fields } => shrink_struct(name, fields),
        PropertyValue::Option(Some(value)) => {
            let mut candidates = vec![PropertyValue::Option(None)];
            candidates.extend(
                shrink_candidates(value)
                    .into_iter()
                    .map(|value| PropertyValue::Option(Some(Box::new(value)))),
            );
            candidates
        }
        PropertyValue::Option(None) => Vec::new(),
        PropertyValue::ResultOk(value) => shrink_candidates(value)
            .into_iter()
            .map(|value| PropertyValue::ResultOk(Box::new(value)))
            .collect(),
        PropertyValue::ResultErr(value) => shrink_candidates(value)
            .into_iter()
            .map(|value| PropertyValue::ResultErr(Box::new(value)))
            .collect(),
    }
}

fn shrink_i64(value: i64) -> Vec<PropertyValue> {
    if value == 0 {
        return Vec::new();
    }
    unique(vec![
        PropertyValue::I64(0),
        PropertyValue::I64(value / 2),
        PropertyValue::I64(value.signum()),
    ])
}

fn shrink_f64(value: f64) -> Vec<PropertyValue> {
    if value == 0.0 {
        return Vec::new();
    }
    let sign = if value.is_sign_negative() { -1.0 } else { 1.0 };
    unique(vec![
        PropertyValue::F64(0.0),
        PropertyValue::F64(value / 2.0),
        PropertyValue::F64(sign),
    ])
}

fn shrink_string(value: &str) -> Vec<PropertyValue> {
    if value.is_empty() {
        return Vec::new();
    }
    let mut candidates = vec![PropertyValue::String(String::new())];
    let half_len = value.chars().count() / 2;
    if half_len > 0 {
        candidates.push(PropertyValue::String(
            value.chars().take(half_len).collect(),
        ));
    }
    if value != "a" {
        candidates.push(PropertyValue::String("a".to_string()));
    }
    unique(candidates)
}

fn shrink_json(value: &serde_json::Value) -> Vec<PropertyValue> {
    match value {
        serde_json::Value::Null => Vec::new(),
        serde_json::Value::Bool(true) => vec![PropertyValue::Json(serde_json::Value::Bool(false))],
        serde_json::Value::Bool(false) => Vec::new(),
        serde_json::Value::Number(number) => {
            if let Some(value) = number.as_i64() {
                shrink_i64(value)
                    .into_iter()
                    .filter_map(|value| match value {
                        PropertyValue::I64(value) => {
                            Some(PropertyValue::Json(serde_json::json!(value)))
                        }
                        _ => None,
                    })
                    .collect()
            } else {
                vec![PropertyValue::Json(serde_json::json!(0))]
            }
        }
        serde_json::Value::String(value) => shrink_string(value)
            .into_iter()
            .filter_map(|value| match value {
                PropertyValue::String(value) => Some(PropertyValue::Json(serde_json::json!(value))),
                _ => None,
            })
            .collect(),
        serde_json::Value::Array(values) => {
            let arrays = shrink_array(
                &values
                    .iter()
                    .cloned()
                    .map(PropertyValue::Json)
                    .collect::<Vec<_>>(),
            );
            arrays
                .into_iter()
                .filter_map(|value| match value {
                    PropertyValue::Array(values) => {
                        Some(PropertyValue::Json(serde_json::Value::Array(
                            values
                                .into_iter()
                                .filter_map(|value| match value {
                                    PropertyValue::Json(value) => Some(value),
                                    _ => None,
                                })
                                .collect(),
                        )))
                    }
                    _ => None,
                })
                .collect()
        }
        serde_json::Value::Object(_) => vec![PropertyValue::Json(serde_json::json!({}))],
    }
}

fn shrink_array(values: &[PropertyValue]) -> Vec<PropertyValue> {
    if values.is_empty() {
        return Vec::new();
    }
    let mut candidates = vec![PropertyValue::Array(Vec::new())];
    let half_len = values.len() / 2;
    if half_len > 0 {
        candidates.push(PropertyValue::Array(
            values.iter().take(half_len).cloned().collect(),
        ));
    }
    for (index, value) in values.iter().enumerate() {
        for shrunk in shrink_candidates(value) {
            let mut next = values.to_vec();
            next[index] = shrunk;
            candidates.push(PropertyValue::Array(next));
        }
    }
    unique(candidates)
}

fn shrink_struct(name: &str, fields: &BTreeMap<String, PropertyValue>) -> Vec<PropertyValue> {
    let mut candidates = Vec::new();
    for (field_name, field_value) in fields {
        for shrunk in shrink_candidates(field_value) {
            let mut next = fields.clone();
            next.insert(field_name.clone(), shrunk);
            candidates.push(PropertyValue::Struct {
                name: name.to_string(),
                fields: next,
            });
        }
    }
    unique(candidates)
}

fn unique(candidates: Vec<PropertyValue>) -> Vec<PropertyValue> {
    let mut unique = Vec::new();
    for candidate in candidates {
        if !unique.contains(&candidate) {
            unique.push(candidate);
        }
    }
    unique
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn i64_shrinks_toward_zero() {
        assert_eq!(
            shrink_candidates(&PropertyValue::I64(10)),
            vec![
                PropertyValue::I64(0),
                PropertyValue::I64(5),
                PropertyValue::I64(1),
            ]
        );
    }

    #[test]
    fn minimal_i64_has_no_candidates() {
        assert!(shrink_candidates(&PropertyValue::I64(0)).is_empty());
    }

    #[test]
    fn array_shrinks_length_then_elements() {
        let candidates = shrink_candidates(&PropertyValue::Array(vec![
            PropertyValue::I64(4),
            PropertyValue::I64(2),
        ]));
        assert_eq!(candidates[0], PropertyValue::Array(Vec::new()));
        assert!(candidates.contains(&PropertyValue::Array(vec![PropertyValue::I64(4)])));
        assert!(candidates.contains(&PropertyValue::Array(vec![
            PropertyValue::I64(0),
            PropertyValue::I64(2),
        ])));
    }

    #[test]
    fn option_some_shrinks_to_none_first() {
        let candidates = shrink_candidates(&PropertyValue::Option(Some(Box::new(
            PropertyValue::Bool(true),
        ))));
        assert_eq!(candidates[0], PropertyValue::Option(None));
    }
}
