//! Deterministic type-driven value generation.

use serde_json::json;

use crate::types::DuumbiType;

use super::value::PropertyValue;

/// Settings controlling deterministic property value generation.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct GeneratorSettings {
    /// Global seed for deterministic generation.
    pub seed: u64,
    /// Number of values requested per top-level type.
    pub cases: u32,
    /// Maximum generated collection length.
    pub max_array_len: usize,
}

impl Default for GeneratorSettings {
    fn default() -> Self {
        Self {
            seed: 0,
            cases: 64,
            max_array_len: 8,
        }
    }
}

/// Unsupported generator result with a stable reason string.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct UnsupportedGenerator {
    /// Type that could not be generated.
    pub ty: DuumbiType,
    /// Stable unsupported reason for evidence.
    pub reason: &'static str,
}

/// Generates deterministic values for a DUUMBI type.
///
/// Boundary values are emitted before seeded random values. The sequence is
/// stable for a given generator implementation version, seed, type, and
/// settings.
pub fn generate_values(
    ty: &DuumbiType,
    settings: &GeneratorSettings,
) -> Result<Vec<PropertyValue>, UnsupportedGenerator> {
    let target = settings.cases.max(1) as usize;
    let mut rng = DeterministicRng::new(settings.seed ^ stable_type_seed(ty));
    let mut values = generate_seed_values(ty, settings, &mut rng, target)?;
    values.truncate(target);
    Ok(values)
}

fn generate_seed_values(
    ty: &DuumbiType,
    settings: &GeneratorSettings,
    rng: &mut DeterministicRng,
    target: usize,
) -> Result<Vec<PropertyValue>, UnsupportedGenerator> {
    match ty {
        DuumbiType::I64 => Ok(generate_i64(rng, target)),
        DuumbiType::F64 => Ok(generate_f64(rng, target)),
        DuumbiType::Bool => Ok(generate_bool(target)),
        DuumbiType::String => Ok(generate_string(rng, target)),
        DuumbiType::Json => Ok(generate_json(rng, target)),
        DuumbiType::Array(inner) => generate_array(inner, settings, rng, target),
        DuumbiType::Option(inner) => generate_option(inner, settings, rng, target),
        DuumbiType::Result(ok, err) => generate_result(ok, err, settings, rng, target),
        DuumbiType::Struct(name) => Err(unsupported(
            ty,
            if name.is_empty() {
                "unsupported_struct_name_missing"
            } else {
                "unsupported_struct_field_metadata_missing"
            },
        )),
        DuumbiType::Void => Err(unsupported(ty, "unsupported_void_parameter")),
        DuumbiType::TcpSocket => Err(unsupported(ty, "unsupported_resource_tcp_socket")),
        DuumbiType::TcpListener => Err(unsupported(ty, "unsupported_resource_tcp_listener")),
        DuumbiType::HttpServer => Err(unsupported(ty, "unsupported_resource_http_server")),
        DuumbiType::HttpResponse => Err(unsupported(ty, "unsupported_resource_http_response")),
        DuumbiType::DbConnection => Err(unsupported(ty, "unsupported_resource_db_connection")),
        DuumbiType::DbRows => Err(unsupported(ty, "unsupported_resource_db_rows")),
        DuumbiType::Ref(_) | DuumbiType::RefMut(_) => {
            Err(unsupported(ty, "unsupported_reference_generation"))
        }
    }
}

fn generate_i64(rng: &mut DeterministicRng, target: usize) -> Vec<PropertyValue> {
    let mut values = vec![
        PropertyValue::I64(0),
        PropertyValue::I64(1),
        PropertyValue::I64(-1),
        PropertyValue::I64(i64::MIN),
        PropertyValue::I64(i64::MIN + 1),
        PropertyValue::I64(i64::MAX),
    ];
    while values.len() < target {
        values.push(PropertyValue::I64(rng.next_i64()));
    }
    values
}

fn generate_f64(rng: &mut DeterministicRng, target: usize) -> Vec<PropertyValue> {
    let mut values = vec![
        PropertyValue::F64(0.0),
        PropertyValue::F64(1.0),
        PropertyValue::F64(-1.0),
        PropertyValue::F64(0.5),
        PropertyValue::F64(-0.5),
    ];
    while values.len() < target {
        values.push(PropertyValue::F64(rng.next_f64()));
    }
    values
}

fn generate_bool(target: usize) -> Vec<PropertyValue> {
    [PropertyValue::Bool(false), PropertyValue::Bool(true)]
        .into_iter()
        .cycle()
        .take(target)
        .collect()
}

fn generate_string(rng: &mut DeterministicRng, target: usize) -> Vec<PropertyValue> {
    let mut values = vec![
        PropertyValue::String(String::new()),
        PropertyValue::String("a".to_string()),
        PropertyValue::String(" ".to_string()),
        PropertyValue::String("0".to_string()),
    ];
    while values.len() < target {
        values.push(PropertyValue::String(rng.next_ascii_string(8)));
    }
    values
}

fn generate_json(rng: &mut DeterministicRng, target: usize) -> Vec<PropertyValue> {
    let mut values = vec![
        PropertyValue::Json(serde_json::Value::Null),
        PropertyValue::Json(json!(true)),
        PropertyValue::Json(json!(0)),
        PropertyValue::Json(json!("")),
        PropertyValue::Json(json!([])),
    ];
    while values.len() < target {
        values.push(PropertyValue::Json(json!(rng.next_ascii_string(6))));
    }
    values
}

fn generate_array(
    inner: &DuumbiType,
    settings: &GeneratorSettings,
    rng: &mut DeterministicRng,
    target: usize,
) -> Result<Vec<PropertyValue>, UnsupportedGenerator> {
    let inner_values = generate_seed_values(inner, settings, rng, target.max(1))?;
    let max_len = settings.max_array_len.min(3);
    let mut values = vec![PropertyValue::Array(Vec::new())];
    for len in 1..=max_len {
        let items = inner_values.iter().take(len).cloned().collect();
        values.push(PropertyValue::Array(items));
    }
    while values.len() < target {
        let len = rng.next_usize(settings.max_array_len.saturating_add(1));
        let items = (0..len)
            .map(|idx| inner_values[idx % inner_values.len()].clone())
            .collect();
        values.push(PropertyValue::Array(items));
    }
    Ok(values)
}

fn generate_option(
    inner: &DuumbiType,
    settings: &GeneratorSettings,
    rng: &mut DeterministicRng,
    target: usize,
) -> Result<Vec<PropertyValue>, UnsupportedGenerator> {
    let inner_values = generate_seed_values(inner, settings, rng, target.max(1))?;
    let mut values = vec![PropertyValue::Option(None)];
    values.extend(
        inner_values
            .into_iter()
            .map(|value| PropertyValue::Option(Some(Box::new(value)))),
    );
    values.truncate(target);
    Ok(values)
}

fn generate_result(
    ok: &DuumbiType,
    err: &DuumbiType,
    settings: &GeneratorSettings,
    rng: &mut DeterministicRng,
    target: usize,
) -> Result<Vec<PropertyValue>, UnsupportedGenerator> {
    let ok_values = generate_seed_values(ok, settings, rng, target.max(1))?;
    let err_values = generate_seed_values(err, settings, rng, target.max(1))?;
    let mut values = Vec::with_capacity(target);
    for value in ok_values {
        values.push(PropertyValue::ResultOk(Box::new(value)));
        if values.len() >= target {
            return Ok(values);
        }
    }
    for value in err_values {
        values.push(PropertyValue::ResultErr(Box::new(value)));
        if values.len() >= target {
            return Ok(values);
        }
    }
    Ok(values)
}

fn unsupported(ty: &DuumbiType, reason: &'static str) -> UnsupportedGenerator {
    UnsupportedGenerator {
        ty: ty.clone(),
        reason,
    }
}

fn stable_type_seed(ty: &DuumbiType) -> u64 {
    let text = ty.to_string();
    let mut hash = 0xcbf2_9ce4_8422_2325_u64;
    for byte in text.as_bytes() {
        hash ^= u64::from(*byte);
        hash = hash.wrapping_mul(0x1000_0000_01b3);
    }
    hash
}

#[derive(Debug, Clone)]
struct DeterministicRng {
    state: u64,
}

impl DeterministicRng {
    fn new(seed: u64) -> Self {
        Self { state: seed }
    }

    fn next_u64(&mut self) -> u64 {
        self.state = self.state.wrapping_add(0x9e37_79b9_7f4a_7c15);
        let mut value = self.state;
        value = (value ^ (value >> 30)).wrapping_mul(0xbf58_476d_1ce4_e5b9);
        value = (value ^ (value >> 27)).wrapping_mul(0x94d0_49bb_1331_11eb);
        value ^ (value >> 31)
    }

    fn next_i64(&mut self) -> i64 {
        i64::from_ne_bytes(self.next_u64().to_ne_bytes())
    }

    fn next_f64(&mut self) -> f64 {
        let bounded = (self.next_u64() % 20_001) as i64 - 10_000;
        bounded as f64 / 100.0
    }

    fn next_usize(&mut self, upper_exclusive: usize) -> usize {
        if upper_exclusive == 0 {
            0
        } else {
            (self.next_u64() as usize) % upper_exclusive
        }
    }

    fn next_ascii_string(&mut self, max_len: usize) -> String {
        let len = self.next_usize(max_len.saturating_add(1));
        (0..len)
            .map(|_| {
                let offset = self.next_usize(26) as u8;
                char::from(b'a' + offset)
            })
            .collect()
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn i64_generation_is_reproducible_for_seed() {
        let settings = GeneratorSettings {
            seed: 717,
            cases: 8,
            max_array_len: 4,
        };
        let first = generate_values(&DuumbiType::I64, &settings).expect("i64 supported");
        let second = generate_values(&DuumbiType::I64, &settings).expect("i64 supported");
        assert_eq!(first, second);
        assert_eq!(first[0], PropertyValue::I64(0));
        assert_eq!(first.len(), 8);
    }

    #[test]
    fn i64_generation_includes_exact_minimum_boundary() {
        let values = generate_values(
            &DuumbiType::I64,
            &GeneratorSettings {
                seed: 717,
                cases: 6,
                max_array_len: 4,
            },
        )
        .expect("i64 supported");

        assert!(values.contains(&PropertyValue::I64(i64::MIN)));
    }

    #[test]
    fn bool_generation_covers_both_values() {
        let values = generate_values(
            &DuumbiType::Bool,
            &GeneratorSettings {
                seed: 717,
                cases: 4,
                max_array_len: 4,
            },
        )
        .expect("bool supported");
        assert_eq!(
            values,
            vec![
                PropertyValue::Bool(false),
                PropertyValue::Bool(true),
                PropertyValue::Bool(false),
                PropertyValue::Bool(true),
            ]
        );
    }

    #[test]
    fn arrays_are_bounded_by_settings() {
        let values = generate_values(
            &DuumbiType::Array(Box::new(DuumbiType::I64)),
            &GeneratorSettings {
                seed: 717,
                cases: 6,
                max_array_len: 2,
            },
        )
        .expect("array<i64> supported");
        assert!(values.iter().all(|value| match value {
            PropertyValue::Array(items) => items.len() <= 2,
            _ => false,
        }));
    }

    #[test]
    fn option_generation_includes_none_and_some() {
        let values = generate_values(
            &DuumbiType::Option(Box::new(DuumbiType::Bool)),
            &GeneratorSettings {
                seed: 717,
                cases: 3,
                max_array_len: 4,
            },
        )
        .expect("option<bool> supported");
        assert!(matches!(values[0], PropertyValue::Option(None)));
        assert!(matches!(values[1], PropertyValue::Option(Some(_))));
    }

    #[test]
    fn runtime_resources_are_unsupported_with_reason() {
        let err = generate_values(
            &DuumbiType::DbConnection,
            &GeneratorSettings {
                seed: 717,
                cases: 3,
                max_array_len: 4,
            },
        )
        .unwrap_err();
        assert_eq!(err.reason, "unsupported_resource_db_connection");
    }
}
