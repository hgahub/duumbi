//! Source-backed Tier 1 publish matrix for DUUMBI-381.
//!
//! The matrix is implementation evidence, not a registry policy engine. It
//! records which modules are ready to package after local verification and
//! which modules must stay deferred to an upstream issue.

/// Release-readiness state for a Tier 1 stdlib module.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum PublishMatrixState {
    /// The module has source in this repository and can be packaged after local
    /// verification succeeds.
    PublishableAfterVerify,
    /// The module is blocked by an upstream issue/spec/implementation gate and
    /// must not be published by DUUMBI-381.
    DeferredUpstream,
}

impl PublishMatrixState {
    /// Returns the stable evidence label used in specs and issue reports.
    #[must_use]
    pub const fn as_str(self) -> &'static str {
        match self {
            Self::PublishableAfterVerify => "publishable-after-verify",
            Self::DeferredUpstream => "deferred-upstream",
        }
    }
}

/// One source-backed Tier 1 publish-matrix row.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct PublishMatrixEntry {
    /// Scoped registry module name.
    pub module: &'static str,
    /// Current release-readiness state.
    pub state: PublishMatrixState,
    /// Local graph source backing this row, when present in this repository.
    pub source_graph: Option<&'static str>,
    /// Upstream GitHub issue that owns unresolved work, if any.
    pub upstream_issue: Option<u32>,
    /// Human-readable basis for the row.
    pub evidence: &'static str,
}

/// DUUMBI-381 Tier 1 source-backed publish matrix.
pub const TIER1_PUBLISH_MATRIX: &[PublishMatrixEntry] = &[
    PublishMatrixEntry {
        module: "@duumbi/stdlib-math",
        state: PublishMatrixState::PublishableAfterVerify,
        source_graph: Some("stdlib/math.jsonld"),
        upstream_issue: None,
        evidence: "Core source module exists and default init uses it.",
    },
    PublishMatrixEntry {
        module: "@duumbi/stdlib-io",
        state: PublishMatrixState::PublishableAfterVerify,
        source_graph: Some("stdlib/io.jsonld"),
        upstream_issue: None,
        evidence: "Core source module exists and default init uses it.",
    },
    PublishMatrixEntry {
        module: "@duumbi/stdlib-lang",
        state: PublishMatrixState::PublishableAfterVerify,
        source_graph: Some("stdlib/lang.jsonld"),
        upstream_issue: None,
        evidence: "Core source module exists and default init uses it.",
    },
    PublishMatrixEntry {
        module: "@duumbi/stdlib-string",
        state: PublishMatrixState::PublishableAfterVerify,
        source_graph: Some("stdlib/string.jsonld"),
        upstream_issue: None,
        evidence: "Core source module exists and default init uses it.",
    },
    PublishMatrixEntry {
        module: "@duumbi/stdlib-file",
        state: PublishMatrixState::PublishableAfterVerify,
        source_graph: Some("stdlib/file.jsonld"),
        upstream_issue: Some(378),
        evidence: "#378 completed Stage 12 and source module exists.",
    },
    PublishMatrixEntry {
        module: "@duumbi/stdlib-json",
        state: PublishMatrixState::PublishableAfterVerify,
        source_graph: Some("stdlib/json.jsonld"),
        upstream_issue: Some(379),
        evidence: "#379 completed Stage 12 and source module exists.",
    },
    PublishMatrixEntry {
        module: "@duumbi/stdlib-net",
        state: PublishMatrixState::PublishableAfterVerify,
        source_graph: Some("stdlib/net.jsonld"),
        upstream_issue: Some(379),
        evidence: "#379 completed Stage 12 and source module exists.",
    },
    PublishMatrixEntry {
        module: "@duumbi/stdlib-server",
        state: PublishMatrixState::DeferredUpstream,
        source_graph: None,
        upstream_issue: Some(381),
        evidence: "DUUMBI-381 owns the server module; it remains deferred until implementation evidence exists.",
    },
    PublishMatrixEntry {
        module: "@duumbi/stdlib-http",
        state: PublishMatrixState::DeferredUpstream,
        source_graph: None,
        upstream_issue: Some(380),
        evidence: "#380 owns HTTP client behavior and publication readiness.",
    },
    PublishMatrixEntry {
        module: "@duumbi/stdlib-tls",
        state: PublishMatrixState::DeferredUpstream,
        source_graph: None,
        upstream_issue: Some(380),
        evidence: "#380 must decide whether TLS is separate or HTTP behavior.",
    },
    PublishMatrixEntry {
        module: "@duumbi/stdlib-db",
        state: PublishMatrixState::DeferredUpstream,
        source_graph: None,
        upstream_issue: Some(380),
        evidence: "#380 owns database behavior and publication readiness.",
    },
];

/// Returns the DUUMBI-381 Tier 1 publish matrix.
#[must_use]
pub const fn tier1_publish_matrix() -> &'static [PublishMatrixEntry] {
    TIER1_PUBLISH_MATRIX
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::collections::BTreeSet;
    use std::path::Path;

    fn entry(module: &str) -> PublishMatrixEntry {
        *tier1_publish_matrix()
            .iter()
            .find(|row| row.module == module)
            .expect("matrix row must exist")
    }

    #[test]
    fn ready_modules_are_source_backed_publishable_after_verify() {
        for module in [
            "@duumbi/stdlib-math",
            "@duumbi/stdlib-io",
            "@duumbi/stdlib-lang",
            "@duumbi/stdlib-string",
            "@duumbi/stdlib-file",
            "@duumbi/stdlib-json",
            "@duumbi/stdlib-net",
        ] {
            let row = entry(module);
            assert_eq!(row.state, PublishMatrixState::PublishableAfterVerify);
            assert!(
                row.source_graph.is_some(),
                "{module} must cite a source graph"
            );
        }
    }

    #[test]
    fn cited_source_graphs_exist() {
        let repo_root = Path::new(env!("CARGO_MANIFEST_DIR"));
        for row in tier1_publish_matrix() {
            if let Some(source_graph) = row.source_graph {
                assert!(
                    repo_root.join(source_graph).exists(),
                    "{} cites missing graph source {source_graph}",
                    row.module
                );
            }
        }
    }

    #[test]
    fn issue_380_owned_modules_remain_deferred() {
        for module in [
            "@duumbi/stdlib-http",
            "@duumbi/stdlib-tls",
            "@duumbi/stdlib-db",
        ] {
            let row = entry(module);
            assert_eq!(row.state, PublishMatrixState::DeferredUpstream);
            assert_eq!(row.upstream_issue, Some(380));
            assert_eq!(row.source_graph, None);
        }
    }

    #[test]
    fn server_module_remains_deferred_until_issue_381_implementation() {
        let row = entry("@duumbi/stdlib-server");
        assert_eq!(row.state, PublishMatrixState::DeferredUpstream);
        assert_eq!(row.upstream_issue, Some(381));
        assert_eq!(row.source_graph, None);
    }

    #[test]
    fn matrix_has_no_duplicate_modules() {
        let mut modules = BTreeSet::new();
        for row in tier1_publish_matrix() {
            assert!(
                modules.insert(row.module),
                "duplicate row for {}",
                row.module
            );
        }
    }

    #[test]
    fn state_labels_match_spec_terms() {
        assert_eq!(
            PublishMatrixState::PublishableAfterVerify.as_str(),
            "publishable-after-verify"
        );
        assert_eq!(
            PublishMatrixState::DeferredUpstream.as_str(),
            "deferred-upstream"
        );
    }
}
