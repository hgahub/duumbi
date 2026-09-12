import test from "node:test";
import assert from "node:assert/strict";
import { readIntake, withIntake, hasStatus, replaceEnrichment, intakeExcerpt } from "./contract.mjs";

test("frontmatter is authoritative, body text and old markers cannot select a note", () => {
  assert.equal(hasStatus("# A note\nintake_status: captured", "captured"), false);
  assert.equal(hasStatus("---\nintake_status: needs_clarification\n---\nintake_status: captured", "captured"), false);
  assert.equal(hasStatus("---\nintake_status: captured\n---\nduumbi/status/processed", "captured"), true);
  for (const value of ["---\nintake_status: wrong\n---\nX", "---\nintake_status: captured\nintake_status: ready_for_triage\n---\nX", "---\nintake_status: captured"]) {
    assert.throws(() => readIntake(value));
    assert.equal(hasStatus(value, "captured"), false);
  }
});

test("bounded model context includes the latest clarification answer after long preparation", () => {
  const text = "---\nintake_status: captured\n---\n# Original problem\n" + "Earlier preparation. ".repeat(1000) + "\n## User clarifications\nDesktop users only.";
  const excerpt = intakeExcerpt(text);
  assert.ok(excerpt.length <= 9000);
  assert.equal(hasStatus(excerpt, "captured"), true);
  assert.ok(excerpt.includes("Original problem"));
  assert.ok(excerpt.includes("Desktop users only."));
});

test("metadata edits preserve unrelated properties, original content and generated block replacement", () => {
  const original = "---\ntags:\n  - my-tag\nsource: grok\nintake_owner: 'H. Gabor'\n---\n# Original\n\nUser's exact idea.";
  const first = replaceEnrichment(withIntake(original, { intake_status: "needs_clarification" }), "## Questions\n- Which user?");
  const answered = first + "\n## User clarification\nDesktop users.\n";
  const second = replaceEnrichment(withIntake(answered, { intake_status: "ready_for_triage" }), "## Prepared\nDesktop users.");
  assert.equal(readIntake(second).source, "grok");
  assert.equal(readIntake(second).intake_owner, "H. Gabor");
  assert.ok(second.includes("User's exact idea."));
  assert.ok(second.includes("## User clarification\nDesktop users."));
  assert.ok(second.includes("  - my-tag"));
  assert.equal(second.includes("Which user?"), false);
  assert.equal(second.split("<!-- duumbi-enrichment:start -->").length, 2);
});
