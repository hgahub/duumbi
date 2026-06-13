//! CLI adapter for semantic rewrite rules.

use std::fs;
use std::io::{self, Write as _};
use std::path::{Path, PathBuf};

use anyhow::{Context, Result};

use crate::rewrite::{
    ApplyMode, ApplyOptions, RewriteApplyPlan, RewriteEngine, RewriteMatch, RewritePreview,
};
use crate::snapshot;

use super::RewriteSubcommand;

/// Dispatches `duumbi rewrite` subcommands.
pub fn run_rewrite(subcommand: RewriteSubcommand, workspace: &Path) -> Result<()> {
    match subcommand {
        RewriteSubcommand::List { json } => run_list(json),
        RewriteSubcommand::Preview {
            module,
            rule,
            json,
            limit,
        } => run_preview(workspace, module.as_deref(), &rule, json, limit),
        RewriteSubcommand::Apply {
            module,
            rule,
            match_id,
            all,
            max_matches,
            yes,
            json,
        } => run_apply(
            workspace,
            module.as_deref(),
            &rule,
            match_id,
            all,
            max_matches,
            yes,
            json,
        ),
    }
}

fn run_list(json_output: bool) -> Result<()> {
    let rules = RewriteEngine::default().list_rules();
    if json_output {
        println!("{}", serde_json::to_string_pretty(&rules)?);
        return Ok(());
    }

    if rules.is_empty() {
        println!("No rewrite rules registered.");
        return Ok(());
    }

    for rule in rules {
        println!(
            "{}\n  category: {:?}\n  safety: {:?}\n  apply-capable: {}\n  preconditions: {}\n  effect: {}\n",
            rule.id,
            rule.category,
            rule.safety_class,
            rule.apply_capable,
            rule.preconditions,
            rule.effect_summary
        );
    }
    Ok(())
}

fn run_preview(
    workspace: &Path,
    module: Option<&str>,
    rule_id: &str,
    json_output: bool,
    limit: Option<usize>,
) -> Result<()> {
    let module_path = resolve_module_path(workspace, module);
    let (_, source) = read_module_source(&module_path)?;
    let preview = RewriteEngine::default()
        .preview_source(&source, rule_id, limit)
        .map_err(|err| anyhow::anyhow!("{err}"))?;

    if json_output {
        println!("{}", serde_json::to_string_pretty(&preview)?);
    } else {
        print_preview(&preview);
    }
    Ok(())
}

#[allow(clippy::too_many_arguments)]
fn run_apply(
    workspace: &Path,
    module: Option<&str>,
    rule_id: &str,
    match_id: Option<String>,
    all: bool,
    max_matches: Option<usize>,
    yes: bool,
    json_output: bool,
) -> Result<()> {
    if all == match_id.is_some() {
        anyhow::bail!("Specify exactly one of --match <match-id> or --all");
    }

    let module_path = resolve_module_path(workspace, module);
    let (source_str, source) = read_module_source(&module_path)?;
    let mode = if all {
        ApplyMode::All
    } else {
        ApplyMode::Match
    };
    let options = ApplyOptions {
        rule_id: rule_id.to_string(),
        module: None,
        mode,
        match_id,
        max_matches,
    };
    let outcome = RewriteEngine::default()
        .apply_to_source(&source, &options)
        .map_err(|err| anyhow::anyhow!("{err}"))?;

    if !yes {
        print_apply_plan(&outcome.plan);
        eprint!("Apply rewrite? [y/N] ");
        io::stderr().flush().ok();
        let mut input = String::new();
        io::stdin()
            .read_line(&mut input)
            .context("Failed to read confirmation")?;
        if !input.trim().eq_ignore_ascii_case("y") {
            if json_output {
                println!(
                    "{}",
                    serde_json::json!({
                        "status": "cancelled",
                        "message": "No rewrite applied"
                    })
                );
            } else {
                eprintln!("No rewrite applied.");
            }
            return Ok(());
        }
    }

    let snapshot_path = snapshot::save_snapshot(workspace, &source_str)
        .context("Failed to save rewrite undo snapshot")?;
    let patched_str = serde_json::to_string_pretty(&outcome.candidate_source)
        .context("Failed to serialize rewritten graph")?;
    fs::write(&module_path, patched_str).with_context(|| {
        format!(
            "Failed to write rewritten graph '{}'",
            module_path.display()
        )
    })?;

    if json_output {
        let mut value = serde_json::to_value(&outcome.plan)?;
        value["snapshotPath"] = serde_json::Value::String(snapshot_path.display().to_string());
        println!("{}", serde_json::to_string_pretty(&value)?);
    } else {
        print_apply_plan(&outcome.plan);
        println!("Snapshot: {}", snapshot_path.display());
        println!("Graph updated: {}", module_path.display());
    }

    Ok(())
}

fn resolve_module_path(workspace: &Path, module: Option<&str>) -> PathBuf {
    match module {
        None => workspace.join(".duumbi").join("graph").join("main.jsonld"),
        Some(name)
            if Path::new(name).extension().is_none()
                && !name.contains(std::path::MAIN_SEPARATOR) =>
        {
            workspace
                .join(".duumbi")
                .join("graph")
                .join(format!("{name}.jsonld"))
        }
        Some(path) => PathBuf::from(path),
    }
}

fn read_module_source(path: &Path) -> Result<(String, serde_json::Value)> {
    let source_str = fs::read_to_string(path)
        .with_context(|| format!("Failed to read module '{}'", path.display()))?;
    let source = serde_json::from_str(&source_str)
        .with_context(|| format!("Failed to parse module '{}' as JSON", path.display()))?;
    Ok((source_str, source))
}

fn print_preview(preview: &RewritePreview) {
    println!("Rule: {}", preview.rule.id);
    println!("Safety: {:?}", preview.rule.safety_class);
    println!("Matches: {}", preview.matches.len());
    println!(
        "Cost: considered={}, returned={}, truncated={}, patchOps={}",
        preview.cost.matches_considered,
        preview.cost.matches_returned,
        preview.cost.matches_truncated,
        preview.cost.patch_op_count
    );
    if preview.matches.is_empty() {
        println!("No matches.");
        return;
    }
    for matched in &preview.matches {
        print_match(matched);
    }
}

fn print_apply_plan(plan: &RewriteApplyPlan) {
    println!("Rule: {}", plan.rule.id);
    println!("Matches: {}", plan.match_ids.join(", "));
    println!(
        "Validation: ran={}, valid={}",
        plan.validation.ran, plan.validation.valid
    );
    println!(
        "Cost: returned={}, patchOps={}",
        plan.cost.matches_returned, plan.cost.patch_op_count
    );
    println!("{}", plan.operation_summary);
}

fn print_match(matched: &RewriteMatch) {
    println!("- {}", matched.match_id);
    println!("  primary: {}", matched.primary_node_id);
    println!("  touched: {}", matched.touched_node_ids.join(", "));
    println!("  effect: {}", matched.operation_summary);
    println!("  explanation: {}", matched.explanation);
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn resolves_default_module_to_workspace_main() {
        assert_eq!(
            resolve_module_path(Path::new("/tmp/workspace"), None),
            Path::new("/tmp/workspace")
                .join(".duumbi")
                .join("graph")
                .join("main.jsonld")
        );
    }

    #[test]
    fn resolves_named_module_to_workspace_graph_file() {
        assert_eq!(
            resolve_module_path(Path::new("/tmp/workspace"), Some("main")),
            Path::new("/tmp/workspace")
                .join(".duumbi")
                .join("graph")
                .join("main.jsonld")
        );
    }

    #[test]
    fn resolves_explicit_jsonld_path_as_path() {
        assert_eq!(
            resolve_module_path(Path::new("/tmp/workspace"), Some("fixtures/rewrite.jsonld")),
            Path::new("fixtures/rewrite.jsonld")
        );
    }
}
