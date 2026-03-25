//! Agent templates defining behavior, tools, and constraints for each role.
//!
//! Five seed templates are embedded in the binary and loaded at startup.
//! Custom templates can be added to `.duumbi/knowledge/agent-templates/`.

use std::path::Path;

use serde::{Deserialize, Serialize};

// ---------------------------------------------------------------------------
// Types
// ---------------------------------------------------------------------------

/// Role an agent template fulfills in a team.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Serialize, Deserialize)]
#[serde(rename_all = "lowercase")]
pub enum AgentRole {
    /// Decomposes tasks into sub-tasks.
    Planner,
    /// Generates or modifies graph nodes.
    Coder,
    /// Reviews generated code for correctness.
    Reviewer,
    /// Verifies tests pass and outputs match expectations.
    Tester,
    /// Repairs validation failures.
    Repair,
}

/// An agent template defining behavior, tools, and constraints.
///
/// Templates are stored as JSON-LD nodes with `@type = "duumbi:AgentTemplate"`.
/// Five seed templates are embedded in the binary; users can add custom ones.
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct AgentTemplate {
    /// JSON-LD type tag.
    #[serde(rename = "@type")]
    pub node_type: String,

    /// Unique identifier (e.g., `"duumbi:template/coder"`).
    #[serde(rename = "@id")]
    pub id: String,

    /// Display name.
    #[serde(rename = "duumbi:name")]
    pub name: String,

    /// Agent role.
    #[serde(rename = "duumbi:role")]
    pub role: AgentRole,

    /// System prompt for this agent type.
    #[serde(rename = "duumbi:systemPrompt")]
    pub system_prompt: String,

    /// Available tool names this agent can use.
    #[serde(rename = "duumbi:tools")]
    pub tools: Vec<String>,

    /// Task specialisations (e.g., `"create"`, `"modify"`, `"fix"`).
    #[serde(rename = "duumbi:specialization")]
    pub specialization: Vec<String>,

    /// Max token budget per call.
    #[serde(rename = "duumbi:tokenBudget")]
    pub token_budget: usize,

    /// Template version (SemVer).
    #[serde(rename = "duumbi:templateVersion")]
    pub template_version: String,
}

// ---------------------------------------------------------------------------
// Seed templates
// ---------------------------------------------------------------------------

/// Return the five built-in seed templates embedded in the binary.
///
/// Always returns exactly 5 entries covering all [`AgentRole`] variants.
#[must_use]
pub fn seed_templates() -> Vec<AgentTemplate> {
    vec![
        AgentTemplate {
            node_type: "duumbi:AgentTemplate".to_string(),
            id: "duumbi:template/planner".to_string(),
            name: "Planner".to_string(),
            role: AgentRole::Planner,
            system_prompt: "You are a task planner. Decompose the user request into an ordered list of atomic sub-tasks. Each sub-task must be small enough for a single agent call. Output JSON-LD patch plans — do not write code yourself.".to_string(),
            tools: vec![],
            specialization: vec!["plan".to_string(), "decompose".to_string()],
            token_budget: 2048,
            template_version: "1.0.0".to_string(),
        },
        AgentTemplate {
            node_type: "duumbi:AgentTemplate".to_string(),
            id: "duumbi:template/coder".to_string(),
            name: "Coder".to_string(),
            role: AgentRole::Coder,
            system_prompt: "You are a code generation agent. Given a task description and the current semantic graph, emit the minimal set of graph patch operations required. Use only the available tools. Prefer small, focused patches.".to_string(),
            tools: vec![
                "add_function".to_string(),
                "add_block".to_string(),
                "add_op".to_string(),
                "modify_op".to_string(),
                "remove_node".to_string(),
                "set_edge".to_string(),
                "replace_block".to_string(),
            ],
            specialization: vec!["create".to_string(), "modify".to_string()],
            token_budget: 4096,
            template_version: "1.0.0".to_string(),
        },
        AgentTemplate {
            node_type: "duumbi:AgentTemplate".to_string(),
            id: "duumbi:template/reviewer".to_string(),
            name: "Reviewer".to_string(),
            role: AgentRole::Reviewer,
            system_prompt: "You are a code review agent. Inspect the semantic graph and identify type errors, missing return ops, orphan references, and structural issues. Report findings as structured JSON — do not modify the graph.".to_string(),
            tools: vec![],
            specialization: vec!["review".to_string(), "validate".to_string()],
            token_budget: 2048,
            template_version: "1.0.0".to_string(),
        },
        AgentTemplate {
            node_type: "duumbi:AgentTemplate".to_string(),
            id: "duumbi:template/tester".to_string(),
            name: "Tester".to_string(),
            role: AgentRole::Tester,
            system_prompt: "You are a test verification agent. Given acceptance criteria and test cases, confirm that the compiled binary output matches the expected values. Report pass/fail for each test case.".to_string(),
            tools: vec![],
            specialization: vec!["test".to_string(), "verify".to_string()],
            token_budget: 2048,
            template_version: "1.0.0".to_string(),
        },
        AgentTemplate {
            node_type: "duumbi:AgentTemplate".to_string(),
            id: "duumbi:template/repair".to_string(),
            name: "Repair".to_string(),
            role: AgentRole::Repair,
            system_prompt: "You are an error repair agent. Given validation errors or test failures, apply the minimal patch to fix the reported issues. Focus on one error at a time. Do not introduce unrelated changes.".to_string(),
            tools: vec![
                "add_function".to_string(),
                "add_block".to_string(),
                "add_op".to_string(),
                "modify_op".to_string(),
                "remove_node".to_string(),
                "set_edge".to_string(),
                "replace_block".to_string(),
            ],
            specialization: vec!["fix".to_string(), "repair".to_string()],
            token_budget: 4096,
            template_version: "1.0.0".to_string(),
        },
    ]
}

// ---------------------------------------------------------------------------
// TemplateStore
// ---------------------------------------------------------------------------

/// Storage directory for agent templates relative to the workspace root.
const TEMPLATE_DIR: &str = ".duumbi/knowledge/agent-templates";

/// Manages agent templates, merging embedded seeds with user-defined overrides.
///
/// On load, embedded seeds are used as the baseline. Any templates found on
/// disk with matching `@id` values override the seed. Additional disk-only
/// templates are appended.
pub struct TemplateStore {
    templates: Vec<AgentTemplate>,
}

impl TemplateStore {
    /// Load templates from disk, falling back to embedded seeds for any missing entry.
    ///
    /// Files that fail to parse are silently skipped so a single corrupt file
    /// cannot block the entire load.
    #[must_use]
    pub fn load(workspace: &Path) -> Self {
        let mut templates = seed_templates();

        let dir = workspace.join(TEMPLATE_DIR);
        if let Ok(entries) = std::fs::read_dir(&dir) {
            for entry in entries.flatten() {
                let path = entry.path();
                if path.extension().and_then(|e| e.to_str()) != Some("json") {
                    continue;
                }
                if let Ok(content) = std::fs::read_to_string(&path)
                    && let Ok(tpl) = serde_json::from_str::<AgentTemplate>(&content)
                {
                    // Override seed with same id, or push new template.
                    if let Some(pos) = templates.iter().position(|t| t.id == tpl.id) {
                        templates[pos] = tpl;
                    } else {
                        templates.push(tpl);
                    }
                }
            }
        }

        TemplateStore { templates }
    }

    /// Get all available templates.
    #[must_use]
    pub fn templates(&self) -> &[AgentTemplate] {
        &self.templates
    }

    /// Find template by role, returning the first match.
    #[must_use]
    pub fn find_by_role(&self, role: AgentRole) -> Option<&AgentTemplate> {
        self.templates.iter().find(|t| t.role == role)
    }

    /// Persist the built-in seed templates to the workspace knowledge directory.
    ///
    /// Creates parent directories as needed. Existing files are overwritten.
    pub fn save_seeds(workspace: &Path) -> std::io::Result<()> {
        let dir = workspace.join(TEMPLATE_DIR);
        std::fs::create_dir_all(&dir)?;
        for tpl in seed_templates() {
            let name = tpl
                .id
                .split('/')
                .next_back()
                .unwrap_or("template")
                .to_string();
            let path = dir.join(format!("{name}.json"));
            let json = serde_json::to_string_pretty(&tpl).map_err(std::io::Error::other)?;
            std::fs::write(path, json)?;
        }
        Ok(())
    }
}

// ---------------------------------------------------------------------------
// Tests
// ---------------------------------------------------------------------------

#[cfg(test)]
mod tests {
    use super::*;
    use tempfile::TempDir;

    #[test]
    fn seed_templates_returns_five() {
        let seeds = seed_templates();
        assert_eq!(seeds.len(), 5, "expected exactly 5 seed templates");
    }

    #[test]
    fn all_roles_covered() {
        let seeds = seed_templates();
        let roles: std::collections::HashSet<_> = seeds.iter().map(|t| t.role).collect();
        assert!(roles.contains(&AgentRole::Planner));
        assert!(roles.contains(&AgentRole::Coder));
        assert!(roles.contains(&AgentRole::Reviewer));
        assert!(roles.contains(&AgentRole::Tester));
        assert!(roles.contains(&AgentRole::Repair));
    }

    #[test]
    fn serialization_roundtrip() {
        for tpl in seed_templates() {
            let json = serde_json::to_string(&tpl).expect("serialise");
            let back: AgentTemplate = serde_json::from_str(&json).expect("deserialise");
            assert_eq!(tpl.id, back.id);
            assert_eq!(tpl.role, back.role);
            assert_eq!(tpl.token_budget, back.token_budget);
            assert_eq!(tpl.specialization, back.specialization);
        }
    }

    #[test]
    fn find_by_role_coder() {
        let store = TemplateStore {
            templates: seed_templates(),
        };
        let coder = store
            .find_by_role(AgentRole::Coder)
            .expect("coder template");
        assert_eq!(coder.role, AgentRole::Coder);
        assert!(!coder.tools.is_empty(), "coder must have tools");
    }

    #[test]
    fn find_by_role_missing_returns_none() {
        // Store with only planner — no coder.
        let planner = seed_templates()
            .into_iter()
            .find(|t| t.role == AgentRole::Planner)
            .unwrap();
        let store = TemplateStore {
            templates: vec![planner],
        };
        assert!(store.find_by_role(AgentRole::Coder).is_none());
    }

    #[test]
    fn load_falls_back_to_seeds_when_dir_absent() {
        let tmp = TempDir::new().expect("tmp dir");
        let store = TemplateStore::load(tmp.path());
        assert_eq!(store.templates().len(), 5);
    }

    #[test]
    fn save_seeds_creates_files() {
        let tmp = TempDir::new().expect("tmp dir");
        TemplateStore::save_seeds(tmp.path()).expect("save");
        let dir = tmp.path().join(TEMPLATE_DIR);
        let count = std::fs::read_dir(&dir)
            .expect("read dir")
            .filter_map(|e| e.ok())
            .filter(|e| e.path().extension().and_then(|x| x.to_str()) == Some("json"))
            .count();
        assert_eq!(count, 5);
    }

    #[test]
    fn load_disk_overrides_seed() {
        let tmp = TempDir::new().expect("tmp dir");
        TemplateStore::save_seeds(tmp.path()).expect("save seeds");

        // Override coder token budget on disk.
        let dir = tmp.path().join(TEMPLATE_DIR);
        let coder_path = dir.join("coder.json");
        let mut coder: AgentTemplate =
            serde_json::from_str(&std::fs::read_to_string(&coder_path).expect("read"))
                .expect("parse");
        coder.token_budget = 9999;
        std::fs::write(
            &coder_path,
            serde_json::to_string_pretty(&coder).expect("ser"),
        )
        .expect("write");

        let store = TemplateStore::load(tmp.path());
        let loaded = store.find_by_role(AgentRole::Coder).expect("coder");
        assert_eq!(loaded.token_budget, 9999);
    }

    #[test]
    fn coder_and_repair_have_mutation_tools() {
        let seeds = seed_templates();
        for role in [AgentRole::Coder, AgentRole::Repair] {
            let tpl = seeds.iter().find(|t| t.role == role).expect("template");
            assert!(
                tpl.tools.contains(&"add_function".to_string()),
                "{role:?} must have add_function tool"
            );
        }
    }

    #[test]
    fn planner_reviewer_tester_have_no_tools() {
        let seeds = seed_templates();
        for role in [AgentRole::Planner, AgentRole::Reviewer, AgentRole::Tester] {
            let tpl = seeds.iter().find(|t| t.role == role).expect("template");
            assert!(
                tpl.tools.is_empty(),
                "{role:?} should have no tools but has {:?}",
                tpl.tools
            );
        }
    }
}
