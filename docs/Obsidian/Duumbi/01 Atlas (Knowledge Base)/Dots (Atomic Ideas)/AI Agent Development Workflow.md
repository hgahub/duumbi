---
tags:
  - project/duumbi
  - concept/development
status: final
created: 2026-03-12
updated: 2026-03-12
related_maps:
  - "[[DUUMBI Technical Architecture Map]]"
---
# AI Agent Development Workflow

This note describes the recommended workflow for using AI agents (Claude Code or similar) to develop DUUMBI features. Based on practical experience from MVP development.

## Standard Task Cycle

1. **Select** the next task from the active milestone
2. **Clarify** intent and plan the approach
3. **Design** test plan before implementation
4. **Implement** the feature
5. **Validate** with `duumbi check` (schema + type + reference integrity)
6. **Test** with `cargo test` and manual verification
7. **Review** using a code review agent — fix findings
8. **Evaluate** from a user perspective — is the feature useful and productive?
9. **Update** CLAUDE.md and memory with learnings
10. **Iterate** — go through the cycle again with accumulated knowledge

## CLAUDE.md Configuration

The `CLAUDE.md` file in the project root configures AI agent behavior:
- JSON-LD schema validation rules
- Forbidden patterns and coding conventions
- Test execution hooks
- Architectural decisions and context

## Key Principle

> Planning, decomposition, and clarity dominate everything else. The quality of AI-generated code is determined by how clearly the task is structured, not by which tool you use.

## Related

- [[AI Agent Architecture]] — the technical agent system
- [[Intent-Driven Development]] — the structured intent workflow
- [[DUUMBI - Task List]] — the atomic implementation checklist
