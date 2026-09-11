import test from "node:test";
import assert from "node:assert/strict";
import fs from "node:fs";
import os from "node:os";
import path from "node:path";
import { execFileSync } from "node:child_process";
import { syncNote } from "./sync-note.mjs";
import { withIntake, INBOX } from "./contract.mjs";

const git = (cwd, args) => execFileSync("git", args, { cwd, encoding: "utf8", stdio: ["ignore", "pipe", "pipe"] }).trim();
function setup() {
  const root = fs.mkdtempSync(path.join(os.tmpdir(), "intake-sync-test-"));
  const remote = path.join(root, "remote.git");
  git(root, ["init", "--bare", "--initial-branch=main", remote]);
  const vault = path.join(root, "duumbi-vault");
  git(root, ["clone", remote, vault]);
  git(vault, ["config", "user.name", "Test"]); git(vault, ["config", "user.email", "test@example.com"]);
  fs.mkdirSync(path.join(vault, INBOX), { recursive: true });
  fs.writeFileSync(path.join(vault, "README.md"), "Vault");
  git(vault, ["add", "."]); git(vault, ["commit", "-m", "init"]); git(vault, ["push", "origin", "main"]);
  const note = INBOX + "2026-09-12 - Test.md";
  const text = withIntake("# Test\n\nAn idea.\n", { intake_status: "captured", intake_id: "unique-test-id", source: "codex", intake_owner: "tester" });
  fs.writeFileSync(path.join(vault, note), text);
  return { root, remote, vault, note, text };
}

test("sync commits only the note, verifies remote and is idempotent", () => {
  const t = setup();
  try {
    fs.writeFileSync(path.join(t.vault, "unrelated.txt"), "Do not publish");
    git(t.vault, ["add", "unrelated.txt"]);
    const beforeIndex = git(t.vault, ["diff", "--cached"]);
    const result = syncNote({ vault: t.vault, note: t.note, expectedBlob: "absent" });
    assert.equal(result.status, "synced");
    assert.equal(git(t.remote, ["show", `main:${t.note}`]), t.text.trim());
    assert.throws(() => git(t.remote, ["show", "main:unrelated.txt"]));
    assert.equal(git(t.vault, ["diff", "--cached"]), beforeIndex);
    assert.equal(syncNote({ vault: t.vault, note: t.note, expectedBlob: "absent" }).alreadyPresent, true);
    const blob = git(t.remote, ["rev-parse", `main:${t.note}`]);
    fs.appendFileSync(path.join(t.vault, t.note), "\nClarification answer.\n");
    assert.throws(() => syncNote({ vault: t.vault, note: t.note, expectedBlob: "absent" }), /Remote note changed/);
    assert.equal(syncNote({ vault: t.vault, note: t.note, expectedBlob: blob }).status, "synced");
    assert.ok(fs.readFileSync(path.join(t.vault, t.note), "utf8").includes("Clarification answer"));
  } finally { fs.rmSync(t.root, { recursive: true, force: true }); }
});

test("sync rejects an existing ID under a new name and paths outside Inbox", () => {
  const t = setup();
  try {
    syncNote({ vault: t.vault, note: t.note, expectedBlob: "absent" });
    const other = INBOX + "another.md";
    fs.writeFileSync(path.join(t.vault, other), t.text);
    assert.throws(() => syncNote({ vault: t.vault, note: other, expectedBlob: "absent" }), /already exists/);
    assert.throws(() => syncNote({ vault: t.vault, note: "README.md", expectedBlob: "absent" }), /Only a relative Inbox/);
    assert.throws(() => syncNote({ vault: t.vault, note: INBOX + "../escape.md", expectedBlob: "absent" }), /Only a relative Inbox/);
  } finally { fs.rmSync(t.root, { recursive: true, force: true }); }
});

test('partial clarification sync preserves identity and refuses stale remote edits', () => {
  const t = setup();
  try {
    syncNote({ vault: t.vault, note: t.note, expectedBlob: 'absent' });
    const baseline = git(t.remote, ['rev-parse', `main:${t.note}`]);
    const waiting = withIntake(t.text + '\nPartial answer.\n', { intake_status: 'needs_clarification' });
    fs.writeFileSync(path.join(t.vault, t.note), waiting);
    assert.equal(syncNote({ vault: t.vault, note: t.note, expectedBlob: baseline }).status, 'synced');
    fs.writeFileSync(path.join(t.vault, t.note), t.text + '\nStale answer.\n');
    assert.throws(() => syncNote({ vault: t.vault, note: t.note, expectedBlob: baseline }), /Remote note changed/);
    assert.equal(git(t.remote, ['show', `main:${t.note}`]), waiting.trim());
    const latest = git(t.remote, ['rev-parse', `main:${t.note}`]);
    fs.writeFileSync(path.join(t.vault, t.note), withIntake(t.text, { intake_id: 'different-id' }));
    assert.throws(() => syncNote({ vault: t.vault, note: t.note, expectedBlob: latest }), /identity/);
  } finally { fs.rmSync(t.root, { recursive: true, force: true }); }
});
