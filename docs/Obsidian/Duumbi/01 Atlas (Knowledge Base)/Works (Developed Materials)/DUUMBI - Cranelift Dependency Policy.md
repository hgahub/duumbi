---
tags:
  - project/duumbi
  - doc/technical
  - risk/dependency
status: active
created: 2026-03-14
updated: 2026-03-14
---
# Cranelift Dependency Management Policy

> This document defines how the DUUMBI project manages its critical dependency on the Cranelift compiler backend. It applies to all phases that modify or extend the codegen pipeline.

← Back: [[DUUMBI Technical Architecture Map]]

---

## Risk Assessment

| Factor | Current State | Risk Level |
|---|---|---|
| Cranelift version | ~0.129.x (pre-1.0) | **High** — any 0.x bump may break API |
| Cranelift governance | Wasmtime project (Bytecode Alliance) | Medium — active development, but WASM-focused priorities |
| DUUMBI's Cranelift surface area | codegen, frontend, module, object crates | **High** — deep integration across 4 crates |
| Breaking change history | Multiple API changes between 0.95→0.129 | **Documented** — happens regularly |
| DUUMBI-specific risk | Phase 9a heap management + Phase 13 telemetry instrumentation expand the surface area significantly | **Critical after Phase 9a** |

---

## Mitigation Strategy

### 1. Version Pinning (Immediate)

Pin exact Cranelift versions in `Cargo.toml` — never use `^` or `~` ranges:

```toml
# PINNED — do NOT use ranges. See docs/cranelift-policy.md before upgrading.
cranelift-codegen = "=0.129.0"
cranelift-frontend = "=0.129.0"
cranelift-module = "=0.129.0"
cranelift-object = "=0.129.0"
```

Update only deliberately, never automatically. Every Cranelift version bump requires:
1. Read the Cranelift CHANGELOG for the target version
2. Run full DUUMBI test suite against the new version in a branch
3. Fix any API breakage
4. Document changes in this file (see Upgrade Log below)
5. Update the pin in a dedicated PR with `cranelift-upgrade` label

### 2. Abstraction Layer (Phase 9a)

Introduce a `CodegenBackend` trait that abstracts the Cranelift-specific API:

```rust
/// Abstraction over compiler backends.
/// Current: Cranelift. Future: LLVM (via inkwell).
pub trait CodegenBackend {
    fn compile_function(&mut self, func: &FunctionDef) -> Result<Vec<u8>>;
    fn emit_object(&self, path: &Path) -> Result<()>;
    fn target_triple(&self) -> &str;
}

pub struct CraneliftBackend { /* cranelift-specific state */ }
impl CodegenBackend for CraneliftBackend { /* ... */ }

// Future:
// pub struct LlvmBackend { /* inkwell state */ }
// impl CodegenBackend for LlvmBackend { /* ... */ }
```

This serves two purposes:
- **Cranelift upgrades** become localized: only `CraneliftBackend` implementation changes, the rest of DUUMBI is unaffected
- **LLVM fallback** (Phase 15a) becomes architecturally feasible without major refactoring

The abstraction layer should be introduced at the start of Phase 9a, before adding heap management codegen.

### 3. CI Canary (Phase 9)

Add a CI job that tests against Cranelift `main` branch (nightly):

```yaml
# .github/workflows/cranelift-canary.yml
name: Cranelift Canary
on:
  schedule:
    - cron: '0 6 * * 1'  # Every Monday 6 AM
jobs:
  canary:
    runs-on: ubuntu-latest
    continue-on-error: true  # Don't block PRs, just report
    steps:
      - uses: actions/checkout@v4
      - name: Patch Cranelift to main
        run: |
          # Override pinned versions with git main
          cargo update -p cranelift-codegen --precise <latest-from-main>
      - name: Build + Test
        run: cargo test --all
      - name: Report
        if: failure()
        run: |
          echo "::warning::Cranelift main broke DUUMBI. Upcoming breaking change detected."
          # Optionally: create a GitHub issue
```

This gives early warning (days/weeks) before a breaking Cranelift release reaches crates.io.

### 4. Cranelift Feature Freeze Points

At certain milestones, the Cranelift version is frozen and not upgraded until the next milestone:

| Milestone | Freeze Period | Rationale |
|---|---|---|
| Phase 9a start | Freeze until Phase 9a complete | Heap management + ownership codegen is too sensitive for mid-phase Cranelift changes |
| Phase 9 benchmarks | Freeze during benchmark stabilization | Benchmark reproducibility requires stable codegen |
| Phase 13 telemetry | Freeze during instrumentation work | Trace injection into compiled binary is deeply Cranelift-coupled |

Between freeze points, Cranelift upgrades follow the deliberate upgrade process (Section 1).

### 5. LLVM Escape Hatch (Phase 15a)

The `CodegenBackend` trait (Section 2) makes LLVM a viable alternative:
- `inkwell` crate provides Rust bindings to LLVM
- LLVM is post-1.0 with strong backward compatibility
- Trade-off: slower compile times, but more optimization passes and wider target support

This is not planned before Phase 15, but the architectural preparation starts in Phase 9a.

---

## Upgrade Log

| Date | From | To | Breaking Changes | PR |
|---|---|---|---|---|
| 2026-02 | — | 0.129.0 | Initial version (Phase 0) | #5 |
| — | — | — | — | — |

---

## Related

- [[DUUMBI - Phase 9a - Type System Completion]] — heaviest Cranelift codegen expansion
- [[DUUMBI - Phase 13 - Self-Healing & Telemetry]] — trace instrumentation in compiled binary
- [[DUUMBI - Tools and Components]] — current crate versions
- [[DUUMBI Technical Architecture Map]] — system overview
- [[DUUMBI - PRD]] — Section 11.1 "Future: LLVM via inkwell"
