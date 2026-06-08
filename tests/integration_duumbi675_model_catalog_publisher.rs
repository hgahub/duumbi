use std::fs;
use std::process::Command;

#[test]
fn duumbi675_model_catalog_publisher_binary_emits_deterministic_catalog_artifacts() {
    let binary = env!("CARGO_BIN_EXE_duumbi-model-catalog-publisher");
    let temp = tempfile::tempdir().expect("temp dir");
    let first = temp.path().join("first");
    let second = temp.path().join("second");
    let evidence = temp.path().join("evidence").join("run-evidence.json");
    let input = "tests/fixtures/model_catalog/publisher_valid.json";

    run_publisher(binary, input, &first, Some(&evidence));
    run_publisher(binary, input, &second, None);

    let first_catalog = fs::read(first.join("model-catalog.v1.json")).expect("first catalog");
    let second_catalog = fs::read(second.join("model-catalog.v1.json")).expect("second catalog");
    let first_checksum =
        fs::read_to_string(first.join("model-catalog.v1.sha256")).expect("first checksum");
    let second_checksum =
        fs::read_to_string(second.join("model-catalog.v1.sha256")).expect("second checksum");
    let evidence_body = fs::read_to_string(evidence).expect("evidence");

    assert_eq!(first_catalog, second_catalog);
    assert_eq!(first_checksum, second_checksum);
    assert!(first_checksum.ends_with("  model-catalog.v1.json\n"));
    assert!(evidence_body.contains("workflowRunUrl"));
    assert!(
        !String::from_utf8(first_catalog)
            .expect("utf8 catalog")
            .contains("workflowRunUrl")
    );
}

#[test]
fn duumbi675_model_catalog_publisher_binary_fails_closed_for_unavailable_provider_without_fallback()
{
    let binary = env!("CARGO_BIN_EXE_duumbi-model-catalog-publisher");
    let temp = tempfile::tempdir().expect("temp dir");
    let output = Command::new(binary)
        .arg("--input")
        .arg("tests/fixtures/model_catalog/publisher_unavailable.json")
        .arg("--out-dir")
        .arg(temp.path().join("out"))
        .output()
        .expect("publisher runs");

    assert!(!output.status.success());
    let stderr = String::from_utf8_lossy(&output.stderr);
    assert!(stderr.contains("provider discovery unavailable without valid fallback"));
}

#[test]
fn duumbi675_model_catalog_publisher_docs_cover_v1_contract() {
    let docs = fs::read_to_string("docs/provider-catalog.md").expect("provider catalog docs");
    for expected in [
        "| Anthropic | `ANTHROPIC_API_KEY` | `anthropic` |",
        "| OpenAI | `OPENAI_API_KEY` | `openai` |",
        "| xAI | `XAI_API_KEY` | `xai` |",
        "| MiniMax | `MINIMAX_API_KEY` | `minimax` |",
        "| DeepSeek | `DEEPSEEK_API_KEY` | `deepseek` |",
        "| Alibaba Cloud Model Studio (Qwen) | `DASHSCOPE_API_KEY` | `qwen` |",
        "| Moonshot AI (Kimi) | `MOONSHOT_API_KEY` | `moonshot` |",
        "| Zhipu AI (GLM) | `ZHIPUAI_API_KEY` | `zhipu` |",
        "| Google Gemini | `GEMINI_API_KEY` | `gemini` |",
        "`~/.duumbi/model-catalog/current.json`",
        "approve, skip, remind later, or disable",
        "must not overwrite provider credentials",
        "Public Catalog Publication Handoff",
        "0e80f66921d283cca8185a358f485004721dbc513e54c5fbe91f37895268107e",
        "model-catalog/v1/model-catalog.v1.json",
        "model-catalog/v1/model-catalog.v1.sha256",
        "hgahub/duumbi-web",
        "verify both public URLs after deployment before Stage 12 closure",
    ] {
        assert!(docs.contains(expected), "missing docs text: {expected}");
    }
    assert!(!docs.contains("| OpenRouter |"));
    assert!(!docs.contains("provider = \"openrouter\""));
    assert!(!docs.contains("provider = \"grok\""));
}

#[test]
fn duumbi675_model_catalog_publisher_workflow_runs_generator_without_publication() {
    let workflow = fs::read_to_string(".github/workflows/model-catalog-publisher.yml")
        .expect("publisher workflow");

    assert!(workflow.contains("workflow_dispatch:"));
    assert!(workflow.contains("schedule:"));
    assert!(workflow.contains("cargo run --bin duumbi-model-catalog-publisher"));
    assert!(workflow.contains("--input tests/fixtures/model_catalog/publisher_valid.json"));
    assert!(workflow.contains("--evidence-out .tmp/model-catalog/run-evidence.json"));
    assert!(workflow.contains("actions/upload-artifact@v4"));
    assert!(!workflow.contains("contents: write"));
    assert!(!workflow.contains("duumbi-web"));
}

#[test]
fn duumbi675_model_catalog_publisher_studio_uses_accepted_provider_list() {
    let script =
        fs::read_to_string("crates/duumbi-studio/src/script/studio.js").expect("studio script");

    for key in [
        "anthropic",
        "openai",
        "xai",
        "minimax",
        "deepseek",
        "qwen",
        "moonshot",
        "zhipu",
        "gemini",
    ] {
        assert!(
            script.contains(&format!("{key}:")),
            "missing provider default for {key}"
        );
        assert!(
            script.contains(&format!("'{key}'")),
            "missing provider option for {key}"
        );
    }
    let provider_names = script
        .lines()
        .find(|line| line.contains("var PROVIDER_NAMES ="))
        .expect("provider names");
    assert!(!provider_names.contains("grok"));
    assert!(!provider_names.contains("openrouter"));
}

#[test]
fn duumbi675_studio_exposes_catalog_update_controls() {
    let lib = fs::read_to_string("crates/duumbi-studio/src/lib.rs").expect("studio lib");
    let script =
        fs::read_to_string("crates/duumbi-studio/src/script/studio.js").expect("studio script");

    for route in [
        r#"/api/settings/catalog"#,
        r#"/api/settings/catalog/check"#,
        r#"/api/settings/catalog/approve"#,
        r#"/api/settings/catalog/skip"#,
        r#"/api/settings/catalog/remind"#,
        r#"/api/settings/catalog/disable"#,
    ] {
        assert!(lib.contains(route), "missing Studio catalog route {route}");
    }

    for token in [
        "Model Catalog Updates",
        "checkCatalogUpdate",
        "approveCatalogUpdate",
        "skipCatalogUpdate",
        "remindCatalogUpdate",
        "disableCatalogUpdate",
        "cancelCatalogUpdate",
        "Check for a catalog update before approving.",
        "Catalog review canceled. Active catalog unchanged.",
    ] {
        assert!(
            script.contains(token),
            "missing Studio catalog UI token {token}"
        );
    }

    assert!(lib.contains("studio_catalog_status_payload"));
    assert!(lib.contains("lastFailure"));
    assert!(lib.contains("Catalog approval requires a reviewed hash."));
    assert!(lib.contains("Catalog update approval failed"));
    assert!(lib.contains("Catalog hash skipped"));
}

fn run_publisher(
    binary: &str,
    input: &str,
    out_dir: &std::path::Path,
    evidence_out: Option<&std::path::Path>,
) {
    let mut command = Command::new(binary);
    command
        .arg("--input")
        .arg(input)
        .arg("--out-dir")
        .arg(out_dir);
    if let Some(evidence_out) = evidence_out {
        command.arg("--evidence-out").arg(evidence_out);
    }
    let output = command.output().expect("publisher runs");
    assert!(
        output.status.success(),
        "publisher failed: {}",
        String::from_utf8_lossy(&output.stderr)
    );
}
