import test from 'node:test';
import assert from 'node:assert/strict';
import fs from 'node:fs';
import os from 'node:os';
import path from 'node:path';
import { promoteIntake, findIntake, validateIntakeId } from './targeted-intake-triage.mjs';
function fixture(t) {
  const workspace = fs.mkdtempSync(path.join(os.tmpdir(), 'target-intake-'));
  t.after(() => fs.rmSync(workspace, { recursive: true, force: true }));
  const vault = path.join(workspace, 'duumbi-vault'), inbox = path.join(vault, 'Duumbi/00 Inbox (ToProcess)'); fs.mkdirSync(inbox, { recursive: true });
  const note = path.join(inbox, 'Idea.md');
  const write = (status = 'ready_for_triage') => fs.writeFileSync(note, `---\nintake_id: idea-123\nintake_status: ${status}\n---\n# Selected idea\n\nEnriched scope.\n`);
  write();
  const issues = [], writes = [];
  const project = { id: 'project', fields: { nodes: [{ id: 'status', name: 'Status', options: [{ id: 'human', name: 'Needs Human Acceptance' }] }] }, items: { nodes: [] } };
  const api = { owner: 'hgahub', repo: 'duumbi', rest: async () => issues,
    loadProject: async () => project,
    createIssue: async (title, body) => { const issue = { number: 100, node_id: 'node-100', state: 'open', labels: [], title, body, html_url: 'https://github.com/hgahub/duumbi/issues/100' }; issues.push(issue); writes.push('create'); return issue; },
    addIssueToProject: async () => { project.items.nodes.push({ id: 'item-100', content: { url: issues[0].html_url }, fieldValues: { nodes: [] } }); writes.push('project'); return 'item-100'; },
    updateProjectStatus: async () => { project.items.nodes[0].fieldValues.nodes = [{ field: { name: 'Status' }, name: 'Needs Human Acceptance' }]; writes.push('status'); },
    addIssueLabel: async (_, name) => { issues[0].labels = [{ name }]; writes.push('label'); },
  };
  const run = () => promoteIntake({ intakeId: 'idea-123', workspace, env: { DUUMBI_PROJECT_NUMBER: '4' }, api, git: (args) => args[0] === 'status' ? 'M Idea.md' : 'sha' });
  return { workspace, vault, inbox, note, write, issues, writes, api, run };
}
test('exact ready note becomes one Stage 5 issue, archives, and repeats without writes', async (t) => {
  const f = fixture(t); const first = await f.run();
  assert.equal(first.status, 'needs_human_acceptance'); assert.equal(f.issues.length, 1);
  assert.deepEqual(f.writes, ['create', 'project', 'status', 'label']);
  assert.match(f.issues[0].body, /Enriched scope/); assert.equal(fs.existsSync(f.note), false);
  const archived = findIntake(f.vault, 'idea-123'); assert.equal(archived.metadata.intake_status, 'triaged');
  const repeat = await f.run(); assert.equal(repeat.status, 'already_processed'); assert.equal(f.writes.length, 4);
});
test('wrong status, missing or ambiguous ID fail before GitHub writes', async (t) => {
  const f = fixture(t);
  for (const status of ['captured', 'needs_clarification', 'triaged']) { f.write(status); await assert.rejects(f.run(), /not ready_for_triage/); }
  f.write(); fs.copyFileSync(f.note, path.join(f.inbox, 'Duplicate.md')); await assert.rejects(f.run(), /Duplicate intake_id/);
  assert.deepEqual(f.writes, []); assert.throws(() => findIntake(f.vault, 'missing'), /not found/);
  for (const id of ['', '../file', 'one two', 'a\n', 'a'.repeat(129)]) assert.throws(() => validateIntakeId(id));
});
test('lost create response recovers the same issue without duplication', async (t) => {
  const f = fixture(t); const create = f.api.createIssue;
  f.api.createIssue = async (...args) => { await create(...args); throw new Error('lost response'); };
  await assert.rejects(f.run(), /lost response/); assert.equal(fs.existsSync(f.note), true);
  const next = await f.run(); assert.equal(next.issue_number, 100); assert.equal(f.issues.length, 1);
});
test('partial queue write retries same issue and does not archive on failure', async (t) => {
  const f = fixture(t); const label = f.api.addIssueLabel;
  f.api.addIssueLabel = async () => { throw new Error('label denied'); };
  await assert.rejects(f.run(), /label denied/); assert.equal(fs.existsSync(f.note), true);
  f.api.addIssueLabel = label; await f.run(); assert.equal(f.issues.length, 1);
});
test('downstream or closed issue is never rolled back to Stage 5', async (t) => {
  const f = fixture(t); await f.api.createIssue('Existing', '<!-- duumbi-intake-stage5:idea-123 -->');
  f.issues[0].labels = [{ name: 'accepted' }];
  await assert.rejects(f.run(), /no rollback/); assert.deepEqual(f.writes, ['create']);
});

test('target selection searches beyond the scheduled sweep batch and file caps', (t) => {
  const f = fixture(t);
  for (let i = 0; i < 205; i++) fs.writeFileSync(path.join(f.inbox, `000-${i}.md`), `---\nintake_id: other-${i}\nintake_status: ready_for_triage\n---\n# Other\n`);
  assert.equal(findIntake(f.vault, 'idea-123').file, f.note);
});
