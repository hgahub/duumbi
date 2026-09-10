//! Offline authoring → native service → repair/evidence checks for DUUMBI-780.

use std::future::Future;
use std::path::Path;
use std::pin::Pin;
use std::process::Command;
use std::sync::Arc;
use std::sync::atomic::{AtomicUsize, Ordering};

use duumbi::agents::{AgentError, LlmProvider};
use duumbi::bench::process::ProcessStage;
use duumbi::bench::runner::{BenchmarkConfig, run_benchmark_with_provider_factory};
use duumbi::config::{ProviderConfig, ProviderKind, ProviderRole};
use duumbi::patch::PatchOp;
use serde_json::{Value, json};

const REFERENCE: &str = include_str!("../examples/flagship-http-sqlite-json/graph/main.jsonld");

struct FixtureProvider {
    calls: Arc<AtomicUsize>,
    behavior: FixtureBehavior,
}

#[derive(Clone, Copy)]
enum FixtureBehavior {
    Pass,
    Repair,
    Unrepaired,
    AuthoringFailure,
}

fn field(node: &str, key: &str, value: Value) -> PatchOp {
    PatchOp::ModifyOp {
        node_id: node.into(),
        field: key.into(),
        value,
    }
}

fn body_prefix(service: &str) -> Value {
    json!(format!(
        "{{\"service\":\"{service}\",\"route\":\"/facts\",\"count\":"
    ))
}

fn native_fixture(port: u16, service: &str) -> Value {
    let mut graph: Value = serde_json::from_str(REFERENCE).expect("reference JSON");
    let ops = graph["duumbi:functions"][0]["duumbi:blocks"][0]["duumbi:ops"]
        .as_array_mut()
        .expect("ops");
    ops.retain(|op| {
        !matches!(
            op["@id"].as_str(),
            Some("duumbi:main/main/entry/fact_value" | "duumbi:main/main/entry/insert_param_push")
        )
    });
    for op in ops.iter_mut() {
        match op["@id"].as_str().expect("id") {
            "duumbi:main/main/entry/port" => op["duumbi:value"] = json!(port),
            "duumbi:main/main/entry/body_prefix" => op["duumbi:value"] = body_prefix(service),
            "duumbi:main/main/entry/insert_sql" => {
                *op = json!({"@type":"duumbi:ResultUnwrap", "@id":"duumbi:main/main/entry/insert_sql", "duumbi:operand":{"@id":"duumbi:main/main/entry/sql_input"}, "duumbi:resultType":"string"})
            }
            _ => {}
        }
    }
    let index = ops
        .iter()
        .position(|op| op["@id"] == "duumbi:main/main/entry/insert_sql")
        .expect("insert");
    ops.insert(index, json!({"@type":"duumbi:ReadLine", "@id":"duumbi:main/main/entry/sql_input", "duumbi:resultType":"result<string,string>"}));
    // The authoring validator requires explicit ResultIsOk evidence, unlike
    // the legacy reference's direct build path.
    let mut guarded = Vec::new();
    for op in ops.drain(..) {
        if op["@type"] == "duumbi:ResultUnwrap" {
            guarded.push(json!({"@type":"duumbi:ResultIsOk", "@id":format!("{}/checked", op["@id"].as_str().expect("id")), "duumbi:operand":op["duumbi:operand"].clone(), "duumbi:resultType":"bool"}));
        }
        guarded.push(op);
    }
    *ops = guarded;
    graph
}

impl LlmProvider for FixtureProvider {
    fn name(&self) -> &str {
        "offline-process-fixture"
    }
    fn call_with_tools<'a>(
        &'a self,
        _system: &'a str,
        message: &'a str,
    ) -> Pin<Box<dyn Future<Output = Result<Vec<PatchOp>, AgentError>> + Send + 'a>> {
        self.calls.fetch_add(1, Ordering::SeqCst);
        let result = if matches!(self.behavior, FixtureBehavior::AuthoringFailure) {
            Err(AgentError::NoToolCalls)
        } else if message.contains("The following test cases FAILED") {
            if matches!(self.behavior, FixtureBehavior::Unrepaired) {
                return Box::pin(async { Err(AgentError::NoToolCalls) });
            }
            if message.contains("\"@id\": \"duumbi:main/main\"") {
                Ok(vec![field(
                    "duumbi:main/main/entry/body_prefix",
                    "duumbi:value",
                    body_prefix("scaled-http-sqlite-json"),
                )])
            } else {
                Err(AgentError::NoToolCalls)
            }
        } else if message.contains("This is a standalone module") {
            Ok(vec![PatchOp::AddFunction {
                function: json!({"@type":"duumbi:Function", "@id":"duumbi:services/facts/helper", "duumbi:name":"helper", "duumbi:returnType":"i64", "duumbi:blocks":[{"@type":"duumbi:Block", "@id":"duumbi:services/facts/helper/entry", "duumbi:label":"entry", "duumbi:ops":[{"@type":"duumbi:Const", "@id":"duumbi:services/facts/helper/entry/zero", "duumbi:value":0}, {"@type":"duumbi:Return", "@id":"duumbi:services/facts/helper/entry/ret", "duumbi:operand":{"@id":"duumbi:services/facts/helper/entry/zero"}}]}]}),
            }])
        } else {
            let port: u16 = message
                .split("TCP port ")
                .nth(1)
                .expect("process contract in authoring prompt")
                .split('.')
                .next()
                .expect("port")
                .parse()
                .expect("numeric port");
            let graph = native_fixture(
                port,
                if matches!(
                    self.behavior,
                    FixtureBehavior::Repair | FixtureBehavior::Unrepaired
                ) {
                    "wrong-service"
                } else {
                    "scaled-http-sqlite-json"
                },
            );
            Ok(vec![
                field(
                    "duumbi:main",
                    "duumbi:imports",
                    graph["duumbi:imports"].clone(),
                ),
                field(
                    "duumbi:main/main",
                    "duumbi:blocks",
                    graph["duumbi:functions"][0]["duumbi:blocks"].clone(),
                ),
            ])
        };
        Box::pin(async move { result })
    }
    fn call_with_tools_streaming<'a>(
        &'a self,
        system: &'a str,
        message: &'a str,
        _on_text: &'a (dyn Fn(&str) + Send + Sync),
    ) -> Pin<Box<dyn Future<Output = Result<Vec<PatchOp>, AgentError>> + Send + 'a>> {
        self.call_with_tools(system, message)
    }
}

fn initialize(path: &Path) -> anyhow::Result<()> {
    let output = Command::new(env!("CARGO_BIN_EXE_duumbi"))
        .arg("init")
        .current_dir(path)
        .output()?;
    anyhow::ensure!(
        output.status.success(),
        "init failed: {}",
        String::from_utf8_lossy(&output.stderr)
    );
    Ok(())
}

fn provider_config() -> ProviderConfig {
    ProviderConfig {
        provider: ProviderKind::OpenAI,
        role: ProviderRole::Primary,
        api_key_env: "DUUMBI_780_OFFLINE_FIXTURE".into(),
        model: Some("fixture".into()),
        base_url: None,
        timeout_secs: None,
        key_storage: Default::default(),
        auth_token_env: None,
    }
}

async fn benchmark(behavior: FixtureBehavior) -> duumbi::bench::report::BenchmarkResult {
    let needs_repair = matches!(behavior, FixtureBehavior::Repair);
    let artifacts = tempfile::tempdir().expect("artifacts");
    let calls = Arc::new(AtomicUsize::new(0));
    let config = BenchmarkConfig {
        attempts: 1,
        providers: vec![provider_config()],
        showcase_filter: Some(vec!["scaled_http_sqlite_json".into()]),
        provider_filter: None,
        suite_filter: None,
        smoke: false,
        artifact_dir: artifacts.path().into(),
        keep_workspaces: false,
        capture_model_io: false,
    };
    let results = run_benchmark_with_provider_factory(&config, initialize, |_| {
        Ok(Arc::new(FixtureProvider {
            calls: Arc::clone(&calls),
            behavior,
        }))
    })
    .await
    .expect("benchmark");
    assert!(
        calls.load(Ordering::SeqCst) > 0,
        "process rows must invoke authoring"
    );
    let result = results.into_iter().next().expect("result");
    if matches!(behavior, FixtureBehavior::Unrepaired) {
        let log = result
            .artifact_paths
            .iter()
            .find(|p| p.ends_with("execute.log"))
            .map(|p| std::fs::read_to_string(artifacts.path().join(p)).expect("execute log"))
            .expect("retained execute log");
        assert!(log.contains("@duumbi/stdlib-server@1.0.0 added from the workspace cache"));
        assert!(log.contains("I_EXTERNAL_VERIFICATION"));
        assert!(!log.contains("E_NO_TEST_CASES"));
    }
    if matches!(behavior, FixtureBehavior::Pass | FixtureBehavior::Repair) {
        if !result.success {
            let log = result
                .artifact_paths
                .iter()
                .find(|p| {
                    Path::new(p)
                        .file_name()
                        .is_some_and(|name| name == "execute.log")
                })
                .and_then(|p| std::fs::read_to_string(artifacts.path().join(p)).ok())
                .unwrap_or_default();
            panic!("benchmark failed: {:?}\n{log}", result.error_category);
        }
        let evidence = result.evidence.as_ref().expect("process evidence");
        assert_eq!(evidence.status, "passed");
        let path = evidence
            .artifact_path
            .as_ref()
            .expect("durable process artifact even on success");
        let persisted: Vec<duumbi::bench::process::ProcessEvidence> = serde_json::from_slice(
            &std::fs::read(artifacts.path().join(path)).expect("retained file"),
        )
        .expect("evidence JSON");
        assert_eq!(persisted.len(), if needs_repair { 2 } else { 1 });
        let pass = persisted.last().expect("pass");
        assert!(pass.build.reaped);
        assert_eq!(pass.scenarios.len(), 2);
        for scenario in &pass.scenarios {
            assert!(scenario.assertions_passed);
            assert_eq!(scenario.process.exit_code, Some(0));
            assert!(scenario.process.reaped);
            assert!(!scenario.process.killed);
            assert!(
                std::net::TcpStream::connect(("127.0.0.1", scenario.port)).is_err(),
                "child listener must be gone"
            );
        }
        if let Some(path) = std::env::var_os("DUUMBI_780_EVIDENCE_DIR") {
            let path = Path::new(&path);
            std::fs::create_dir_all(path).expect("manual artifact dir");
            for relative in &result.artifact_paths {
                let target = path.join(relative);
                std::fs::create_dir_all(target.parent().expect("artifact parent"))
                    .expect("artifact directory");
                std::fs::copy(artifacts.path().join(relative), target)
                    .expect("export retained evidence");
            }
            std::fs::write(
                path.join(if needs_repair {
                    "repaired.json"
                } else {
                    "first-pass.json"
                }),
                serde_json::to_vec_pretty(&result).expect("JSON"),
            )
            .expect("manual evidence");
        }
    }
    result
}

#[tokio::test]
async fn benchmark_authors_builds_and_retains_sqlite_process_evidence() {
    let result = benchmark(FixtureBehavior::Pass).await;
    assert_eq!(result.first_pass_success, Some(true));
    assert!(!result.repair_attempted);
    assert_eq!((result.tests_passed, result.tests_total), (1, 1));
}

#[tokio::test]
async fn process_mismatch_enters_normal_repair_and_reverifies() {
    let result = benchmark(FixtureBehavior::Repair).await;
    assert_eq!(result.first_pass_success, Some(false));
    assert!(result.repair_attempted && result.repair_applied);
    assert_eq!(result.repair_success, Some(true));
    assert_eq!(
        result.evidence.expect("evidence").process[0]
            .failure
            .as_ref()
            .expect("initial mismatch")
            .stage,
        ProcessStage::Assertion
    );
}

#[tokio::test]
async fn authoring_failure_is_preserved_and_process_is_not_run() {
    let result = benchmark(FixtureBehavior::AuthoringFailure).await;
    assert!(!result.success);
    assert_eq!(result.executed, Some(true));
    assert_eq!(result.evidence.expect("evidence").status, "not_run");
    assert_ne!(
        result.error_category,
        Some(duumbi::bench::report::ErrorCategory::EvidenceRequired)
    );
}

#[tokio::test]
async fn determinism_replay_runs_the_same_process_contract() {
    use duumbi::determinism::runner::{ReplayConfig, run_replay_with_provider_factory};
    let artifacts = tempfile::tempdir().expect("artifacts");
    let calls = Arc::new(AtomicUsize::new(0));
    let config = ReplayConfig {
        run_id: "duumbi-780-offline".into(),
        attempts: 2,
        providers: vec![provider_config()],
        showcase_filter: Some(vec!["scaled_http_sqlite_json".into()]),
        provider_filter: None,
        suite_filter: None,
        smoke: false,
        artifact_dir: artifacts.path().into(),
        started_at: "2026-09-10T00:00:00Z".into(),
        source_commit: "offline-fixture".into(),
        provider_source: "injected".into(),
        keep_workspaces: false,
        capture_model_io: false,
    };
    let report = run_replay_with_provider_factory(&config, initialize, |_| {
        Ok(Box::new(FixtureProvider {
            calls: Arc::clone(&calls),
            behavior: FixtureBehavior::Pass,
        }))
    })
    .await
    .expect("replay");
    assert!(calls.load(Ordering::SeqCst) >= 2);
    let attempt = &report.attempts[0];
    assert!(attempt.success, "{attempt:#?}");
    assert_eq!(attempt.executed, Some(true));
    assert_eq!(
        attempt
            .benchmark_evidence
            .as_ref()
            .expect("evidence")
            .status,
        "passed"
    );
    assert_eq!(attempt.tests_total, 1);
    assert!(attempt.dominant_error_code.is_none());
    assert_eq!(report.attempts.len(), 2);
    let second = &report.attempts[1];
    assert!(second.success, "{second:#?}");
    assert!(attempt.intent_spec_hash.is_some());
    assert_eq!(attempt.intent_spec_hash, second.intent_spec_hash);
    assert!(attempt.final_graph_semantic_hash.is_some());
    assert_eq!(
        attempt.final_graph_semantic_hash,
        second.final_graph_semantic_hash
    );
    assert_eq!(
        attempt.final_graph_exact_hash,
        second.final_graph_exact_hash
    );
    for row in &report.attempts {
        assert!(
            row.behavior_signature
                .as_deref()
                .expect("signature")
                .ends_with(";process=passed")
        );
    }
    if let Some(path) = std::env::var_os("DUUMBI_780_EVIDENCE_DIR") {
        std::fs::create_dir_all(&path).expect("manual artifact directory");
        std::fs::write(
            Path::new(&path).join("replay.json"),
            serde_json::to_vec_pretty(&report).expect("replay JSON"),
        )
        .expect("manual replay evidence");
    }
}

#[tokio::test]
async fn unrepaired_json_mismatch_remains_a_product_logic_failure() {
    let result = benchmark(FixtureBehavior::Unrepaired).await;
    assert!(!result.success);
    assert!(result.repair_attempted && !result.repair_applied);
    assert_eq!(result.repair_success, Some(false));
    assert_eq!(
        result.root_cause,
        Some(duumbi::intent::taxonomy::RootCauseClass::ProductLogicMismatch)
    );
    assert_eq!(
        result.error_category,
        Some(duumbi::bench::report::ErrorCategory::LogicError)
    );
    assert_eq!(
        result.dominant_error_code.as_deref(),
        Some("process_assertion")
    );
    assert_eq!(result.evidence.expect("process evidence").status, "failed");
}
