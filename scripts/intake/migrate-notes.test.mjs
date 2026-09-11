import test from "node:test";
import assert from "node:assert/strict";
import fs from "node:fs";
import os from "node:os";
import path from "node:path";
import { migrateNotes } from "./migrate-notes.mjs";
import { readIntake, INBOX } from "./contract.mjs";

test("explicit migration preserves content, skips ambiguous notes and is idempotent", () => {
  const vault = fs.mkdtempSync(path.join(os.tmpdir(), "intake-migrate-test-"));
  const root = path.join(vault, INBOX);
  fs.mkdirSync(root, { recursive: true });
  const fixtures = {
    "raw.md": "# Raw\nAn idea.\n",
    "duplicate.md": "# Legacy\nduumbi-inbox-enrichment:v1\n- Status: duplicate candidate\n",
    "waiting.md": "# Legacy\nduumbi/status/processed\n- Status: needs clarification\n",
    "ambiguous.md": "# Already processed\nduumbi/status/processed\n",
    "current.md": "---\nintake_status: captured\nintake_id: existing\nsource: grok\nintake_owner: other\n---\n# Current\n",
  };
  try {
    for (const [name, text] of Object.entries(fixtures)) fs.writeFileSync(path.join(root, name), text);
    const preview = migrateNotes({ vault, owner: "hgahub" });
    assert.equal(preview.length, 4);
    assert.ok(preview.every((row) => row.applied === false));
    for (const [name, text] of Object.entries(fixtures)) assert.equal(fs.readFileSync(path.join(root, name), "utf8"), text);
    const applied = migrateNotes({ vault, owner: "hgahub", apply: true });
    assert.equal(applied.filter((row) => row.applied).length, 3);
    for (const [name, status] of [["raw.md", "captured"], ["duplicate.md", "ready_for_triage"], ["waiting.md", "needs_clarification"]]) {
      const content = fs.readFileSync(path.join(root, name), "utf8");
      const metadata = readIntake(content);
      assert.equal(metadata.intake_status, status);
      assert.equal(metadata.intake_owner, "hgahub");
      assert.match(metadata.intake_id, /^[a-f0-9-]{36}$/);
      assert.ok(content.endsWith(fixtures[name]));
    }
    assert.equal(fs.readFileSync(path.join(root, "current.md"), "utf8"), fixtures["current.md"]);
    assert.equal(fs.readFileSync(path.join(root, "ambiguous.md"), "utf8"), fixtures["ambiguous.md"]);
    assert.deepEqual(migrateNotes({ vault, owner: "hgahub", apply: true }).map((row) => row.status), ["review_required"]);
  } finally { fs.rmSync(vault, { recursive: true, force: true }); }
});
