//! Dynamic agent team assembler.
//!
//! Maps a [`TaskProfile`] to an [`AgentTeam`] using a static 9-row lookup
//! table.  All logic is deterministic — no LLM calls, no I/O.

use crate::agents::analyzer::{Complexity, Risk, Scope, TaskProfile, TaskType};
use crate::agents::template::{AgentRole, TemplateStore};

// ---------------------------------------------------------------------------
// Types
// ---------------------------------------------------------------------------

/// How a team of agents should be executed.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum ExecutionStrategy {
    /// Single agent runs alone.
    Sequential,
    /// Agents run in chain: output of one feeds the next.
    Pipeline,
    /// Multiple agents run concurrently on different modules, then merge.
    Parallel,
}

/// A configured team of agents ready for execution.
#[derive(Debug, Clone)]
pub struct AgentTeam {
    /// Ordered list of agent roles in execution order.
    pub agents: Vec<AgentRole>,
    /// How to execute this team.
    pub strategy: ExecutionStrategy,
    /// Number of parallel coders (only meaningful for [`ExecutionStrategy::Parallel`]).
    pub parallel_coders: usize,
}

// ---------------------------------------------------------------------------
// Assembly logic
// ---------------------------------------------------------------------------

/// Assembles an agent team from a task profile using the lookup table.
///
/// The `store` parameter is accepted so callers can verify that required
/// templates are available; the current implementation does not filter on
/// availability but future versions may.
///
/// # Lookup table (9 rows, evaluated top-to-bottom, first match wins)
///
/// | Complexity | TaskType | Scope | Risk | Team | Strategy |
/// |------------|----------|-------|------|------|----------|
/// | Simple | Create | Single | Low | [Coder] | Sequential |
/// | Simple | Modify | Single | Low | [Coder] | Sequential |
/// | Any | Test | Any | Any | [Tester] | Sequential |
/// | Moderate | Create | Single | Any | [Planner, Coder, Tester] | Pipeline |
/// | Moderate | Create | Multi | Any | [Planner, Coder(N), Tester] | Parallel |
/// | Moderate | Modify | Any | Medium/High | [Planner, Coder, Reviewer, Tester] | Pipeline |
/// | Complex | Any | Multi | Any | [Planner, Coder(N), Reviewer, Tester] | Parallel |
/// | Any | Refactor | Any | Any | [Planner, Coder, Reviewer, Tester] | Pipeline |
/// | Any | Fix | Any | Any | [Coder] | Sequential |
/// | (default) | | | | [Coder] | Sequential |
#[must_use]
pub fn assemble(profile: &TaskProfile, _store: &TemplateStore) -> AgentTeam {
    // Row 3: Test — fast path, any complexity/scope/risk.
    if profile.task_type == TaskType::Test {
        return AgentTeam {
            agents: vec![AgentRole::Tester],
            strategy: ExecutionStrategy::Sequential,
            parallel_coders: 1,
        };
    }

    // Row 8: Refactor — full pipeline regardless of other dimensions.
    if profile.task_type == TaskType::Refactor {
        return AgentTeam {
            agents: vec![
                AgentRole::Planner,
                AgentRole::Coder,
                AgentRole::Reviewer,
                AgentRole::Tester,
            ],
            strategy: ExecutionStrategy::Pipeline,
            parallel_coders: 1,
        };
    }

    // Row 9: Fix — fast path single coder.
    if profile.task_type == TaskType::Fix {
        return AgentTeam {
            agents: vec![AgentRole::Coder],
            strategy: ExecutionStrategy::Sequential,
            parallel_coders: 1,
        };
    }

    // Row 1: Simple + Create + Single + Low.
    if profile.complexity == Complexity::Simple
        && profile.task_type == TaskType::Create
        && profile.scope == Scope::SingleModule
        && profile.risk == Risk::Low
    {
        return AgentTeam {
            agents: vec![AgentRole::Coder],
            strategy: ExecutionStrategy::Sequential,
            parallel_coders: 1,
        };
    }

    // Row 2: Simple + Modify + Single + Low.
    if profile.complexity == Complexity::Simple
        && profile.task_type == TaskType::Modify
        && profile.scope == Scope::SingleModule
        && profile.risk == Risk::Low
    {
        return AgentTeam {
            agents: vec![AgentRole::Coder],
            strategy: ExecutionStrategy::Sequential,
            parallel_coders: 1,
        };
    }

    // Row 4: Moderate + Create + Single.
    if profile.complexity == Complexity::Moderate
        && profile.task_type == TaskType::Create
        && profile.scope == Scope::SingleModule
    {
        return AgentTeam {
            agents: vec![AgentRole::Planner, AgentRole::Coder, AgentRole::Tester],
            strategy: ExecutionStrategy::Pipeline,
            parallel_coders: 1,
        };
    }

    // Row 5: Moderate + Create + Multi.
    if profile.complexity == Complexity::Moderate
        && profile.task_type == TaskType::Create
        && profile.scope == Scope::MultiModule
    {
        return AgentTeam {
            agents: vec![AgentRole::Planner, AgentRole::Coder, AgentRole::Tester],
            strategy: ExecutionStrategy::Parallel,
            parallel_coders: 2,
        };
    }

    // Row 6: Moderate + Modify + Medium/High risk.
    if profile.complexity == Complexity::Moderate
        && profile.task_type == TaskType::Modify
        && matches!(profile.risk, Risk::Medium | Risk::High)
    {
        return AgentTeam {
            agents: vec![
                AgentRole::Planner,
                AgentRole::Coder,
                AgentRole::Reviewer,
                AgentRole::Tester,
            ],
            strategy: ExecutionStrategy::Pipeline,
            parallel_coders: 1,
        };
    }

    // Row 7: Complex + Multi.
    if profile.complexity == Complexity::Complex && profile.scope == Scope::MultiModule {
        return AgentTeam {
            agents: vec![
                AgentRole::Planner,
                AgentRole::Coder,
                AgentRole::Reviewer,
                AgentRole::Tester,
            ],
            strategy: ExecutionStrategy::Parallel,
            parallel_coders: 2,
        };
    }

    // Default fallback: single Coder, sequential.
    AgentTeam {
        agents: vec![AgentRole::Coder],
        strategy: ExecutionStrategy::Sequential,
        parallel_coders: 1,
    }
}

// ---------------------------------------------------------------------------
// Tests
// ---------------------------------------------------------------------------

#[cfg(test)]
mod tests {
    use super::*;
    use crate::agents::analyzer::{Complexity, Risk, Scope, TaskProfile, TaskType};
    use crate::agents::template::TemplateStore;
    use tempfile::TempDir;

    fn make_store() -> (TempDir, TemplateStore) {
        let tmp = TempDir::new().expect("tmp dir");
        let store = TemplateStore::load(tmp.path());
        (tmp, store)
    }

    fn profile(
        complexity: Complexity,
        task_type: TaskType,
        scope: Scope,
        risk: Risk,
    ) -> TaskProfile {
        TaskProfile {
            complexity,
            task_type,
            scope,
            risk,
        }
    }

    // Row 1: simple+create+single+low → [Coder], Sequential
    #[test]
    fn row1_simple_create_single_low() {
        let (_tmp, store) = make_store();
        let p = profile(
            Complexity::Simple,
            TaskType::Create,
            Scope::SingleModule,
            Risk::Low,
        );
        let team = assemble(&p, &store);
        assert_eq!(team.agents, vec![AgentRole::Coder]);
        assert_eq!(team.strategy, ExecutionStrategy::Sequential);
    }

    // Row 2: simple+modify+single+low → [Coder], Sequential
    #[test]
    fn row2_simple_modify_single_low() {
        let (_tmp, store) = make_store();
        let p = profile(
            Complexity::Simple,
            TaskType::Modify,
            Scope::SingleModule,
            Risk::Low,
        );
        let team = assemble(&p, &store);
        assert_eq!(team.agents, vec![AgentRole::Coder]);
        assert_eq!(team.strategy, ExecutionStrategy::Sequential);
    }

    // Row 3: any+test+*+* → [Tester], Sequential
    #[test]
    fn row3_test_any_dimensions() {
        let (_tmp, store) = make_store();
        for complexity in [
            Complexity::Simple,
            Complexity::Moderate,
            Complexity::Complex,
        ] {
            for scope in [Scope::SingleModule, Scope::MultiModule] {
                let p = profile(complexity, TaskType::Test, scope, Risk::Low);
                let team = assemble(&p, &store);
                assert_eq!(team.agents, vec![AgentRole::Tester]);
                assert_eq!(team.strategy, ExecutionStrategy::Sequential);
            }
        }
    }

    // Row 4: moderate+create+single+* → [Planner, Coder, Tester], Pipeline
    #[test]
    fn row4_moderate_create_single() {
        let (_tmp, store) = make_store();
        let p = profile(
            Complexity::Moderate,
            TaskType::Create,
            Scope::SingleModule,
            Risk::Low,
        );
        let team = assemble(&p, &store);
        assert_eq!(
            team.agents,
            vec![AgentRole::Planner, AgentRole::Coder, AgentRole::Tester]
        );
        assert_eq!(team.strategy, ExecutionStrategy::Pipeline);
        assert_eq!(team.parallel_coders, 1);
    }

    // Row 5: moderate+create+multi+* → [Planner, Coder, Tester], Parallel
    #[test]
    fn row5_moderate_create_multi() {
        let (_tmp, store) = make_store();
        let p = profile(
            Complexity::Moderate,
            TaskType::Create,
            Scope::MultiModule,
            Risk::Medium,
        );
        let team = assemble(&p, &store);
        assert_eq!(
            team.agents,
            vec![AgentRole::Planner, AgentRole::Coder, AgentRole::Tester]
        );
        assert_eq!(team.strategy, ExecutionStrategy::Parallel);
        assert!(team.parallel_coders >= 2);
    }

    // Row 6: moderate+modify+*+medium → [Planner, Coder, Reviewer, Tester], Pipeline
    #[test]
    fn row6_moderate_modify_medium_risk() {
        let (_tmp, store) = make_store();
        let p = profile(
            Complexity::Moderate,
            TaskType::Modify,
            Scope::SingleModule,
            Risk::Medium,
        );
        let team = assemble(&p, &store);
        assert_eq!(
            team.agents,
            vec![
                AgentRole::Planner,
                AgentRole::Coder,
                AgentRole::Reviewer,
                AgentRole::Tester
            ]
        );
        assert_eq!(team.strategy, ExecutionStrategy::Pipeline);
    }

    // Row 6: moderate+modify+*+high risk also hits the pipeline path
    #[test]
    fn row6_moderate_modify_high_risk() {
        let (_tmp, store) = make_store();
        let p = profile(
            Complexity::Moderate,
            TaskType::Modify,
            Scope::MultiModule,
            Risk::High,
        );
        let team = assemble(&p, &store);
        assert_eq!(
            team.agents,
            vec![
                AgentRole::Planner,
                AgentRole::Coder,
                AgentRole::Reviewer,
                AgentRole::Tester
            ]
        );
        assert_eq!(team.strategy, ExecutionStrategy::Pipeline);
    }

    // Row 7: complex+*+multi → [Planner, Coder, Reviewer, Tester], Parallel
    #[test]
    fn row7_complex_multi() {
        let (_tmp, store) = make_store();
        let p = profile(
            Complexity::Complex,
            TaskType::Create,
            Scope::MultiModule,
            Risk::High,
        );
        let team = assemble(&p, &store);
        assert_eq!(
            team.agents,
            vec![
                AgentRole::Planner,
                AgentRole::Coder,
                AgentRole::Reviewer,
                AgentRole::Tester
            ]
        );
        assert_eq!(team.strategy, ExecutionStrategy::Parallel);
    }

    // Row 8: *+refactor+*+* → [Planner, Coder, Reviewer, Tester], Pipeline
    #[test]
    fn row8_refactor_any_dimensions() {
        let (_tmp, store) = make_store();
        for complexity in [
            Complexity::Simple,
            Complexity::Moderate,
            Complexity::Complex,
        ] {
            let p = profile(
                complexity,
                TaskType::Refactor,
                Scope::SingleModule,
                Risk::Low,
            );
            let team = assemble(&p, &store);
            assert_eq!(
                team.agents,
                vec![
                    AgentRole::Planner,
                    AgentRole::Coder,
                    AgentRole::Reviewer,
                    AgentRole::Tester
                ]
            );
            assert_eq!(team.strategy, ExecutionStrategy::Pipeline);
        }
    }

    // Row 9: *+fix+*+* → [Coder], Sequential
    #[test]
    fn row9_fix_fast_path() {
        let (_tmp, store) = make_store();
        for risk in [Risk::Low, Risk::Medium, Risk::High] {
            let p = profile(Complexity::Complex, TaskType::Fix, Scope::MultiModule, risk);
            let team = assemble(&p, &store);
            assert_eq!(team.agents, vec![AgentRole::Coder]);
            assert_eq!(team.strategy, ExecutionStrategy::Sequential);
        }
    }

    // Default fallback: complex+modify+single+low → [Coder], Sequential
    #[test]
    fn default_fallback_single_coder() {
        let (_tmp, store) = make_store();
        let p = profile(
            Complexity::Complex,
            TaskType::Modify,
            Scope::SingleModule,
            Risk::Low,
        );
        let team = assemble(&p, &store);
        assert_eq!(team.agents, vec![AgentRole::Coder]);
        assert_eq!(team.strategy, ExecutionStrategy::Sequential);
    }

    // Verify parallel_coders=1 for sequential teams.
    #[test]
    fn sequential_teams_have_one_coder() {
        let (_tmp, store) = make_store();
        let p = profile(
            Complexity::Simple,
            TaskType::Fix,
            Scope::SingleModule,
            Risk::Low,
        );
        let team = assemble(&p, &store);
        assert_eq!(team.parallel_coders, 1);
    }
}
