//! Equivalence grouping and replay agreement metrics.

use std::collections::BTreeMap;

use serde::{Deserialize, Serialize};

/// Agreement metric for one equivalence tier.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
#[serde(tag = "status", rename_all = "snake_case")]
pub enum AgreementRate {
    /// Agreement could be computed from one or more comparable attempts.
    Available {
        /// Largest equivalence group divided by comparable attempts.
        rate: f64,
        /// Number of attempts with comparable evidence.
        comparable_attempt_count: usize,
        /// Size of the dominant equivalence group.
        largest_equivalence_group_count: usize,
        /// Dominant equivalence key.
        dominant_key: String,
    },
    /// Agreement could not be computed.
    Unavailable {
        /// Stable reason why no metric was available.
        reason: String,
        /// Number of attempts with comparable evidence.
        comparable_attempt_count: usize,
    },
}

/// Computes largest-group agreement for optional equivalence keys.
///
/// `None` values are treated as non-comparable attempts. When no comparable
/// attempts exist, the function returns [`AgreementRate::Unavailable`] rather
/// than NaN or infinity.
#[must_use]
pub fn largest_group_agreement<I, S>(values: I, unavailable_reason: &str) -> AgreementRate
where
    I: IntoIterator<Item = Option<S>>,
    S: AsRef<str>,
{
    let mut groups: BTreeMap<String, usize> = BTreeMap::new();
    for value in values.into_iter().flatten() {
        *groups.entry(value.as_ref().to_string()).or_default() += 1;
    }

    let comparable_attempt_count = groups.values().sum();
    if comparable_attempt_count == 0 {
        return AgreementRate::Unavailable {
            reason: unavailable_reason.to_string(),
            comparable_attempt_count,
        };
    }

    let (dominant_key, largest_equivalence_group_count) = groups
        .into_iter()
        .max_by(|left, right| left.1.cmp(&right.1).then_with(|| right.0.cmp(&left.0)))
        .expect("invariant: comparable_attempt_count > 0 means groups is non-empty");
    let rate = largest_equivalence_group_count as f64 / comparable_attempt_count as f64;

    AgreementRate::Available {
        rate,
        comparable_attempt_count,
        largest_equivalence_group_count,
        dominant_key,
    }
}

/// Evaluates an optional CI threshold against one agreement metric.
#[must_use]
pub fn threshold_passes(metric: &AgreementRate, threshold: Option<f64>) -> bool {
    let Some(threshold) = threshold else {
        return true;
    };
    match metric {
        AgreementRate::Available { rate, .. } => *rate >= threshold,
        AgreementRate::Unavailable { .. } => false,
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn largest_group_agreement_uses_dominant_group() {
        let metric = largest_group_agreement(
            [Some("a"), Some("b"), Some("a"), None],
            "no comparable graph evidence",
        );

        assert_eq!(
            metric,
            AgreementRate::Available {
                rate: 2.0 / 3.0,
                comparable_attempt_count: 3,
                largest_equivalence_group_count: 2,
                dominant_key: "a".to_string(),
            }
        );
    }

    #[test]
    fn zero_comparable_attempts_are_unavailable() {
        let metric =
            largest_group_agreement([Option::<&str>::None], "no comparable graph evidence");

        assert_eq!(
            metric,
            AgreementRate::Unavailable {
                reason: "no comparable graph evidence".to_string(),
                comparable_attempt_count: 0,
            }
        );
        let json = serde_json::to_string(&metric).expect("metric serializes");
        assert!(json.contains("\"status\":\"unavailable\""));
        assert!(!json.contains("NaN"));
        assert!(!json.contains("inf"));
    }

    #[test]
    fn unavailable_metric_fails_explicit_threshold_only() {
        let metric = AgreementRate::Unavailable {
            reason: "no comparable graph evidence".to_string(),
            comparable_attempt_count: 0,
        };

        assert!(threshold_passes(&metric, None));
        assert!(!threshold_passes(&metric, Some(1.0)));
    }
}
