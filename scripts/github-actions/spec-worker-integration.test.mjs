import test from 'node:test';
import assert from 'node:assert/strict';
import fs from 'node:fs/promises';
import path from 'node:path';
import os from 'node:os';
import { command } from '../spec-automation/run.mjs';
const root = path.resolve(import.meta.dirname, '../..');
async function fixture(t) {
  const dir = await fs.mkdtemp(path.join(os.tmpdir(), 'spec-worker-'));
  t.after(() => fs.rm(dir, { recursive: true, force: true }));
  const bin = path.join(dir, 'bin'); await fs.mkdir(bin);
  for (const tool of ['gh', 'git', 'codex']) {
    // Wrapper preserves a tool-specific argv[1] while running the real Node executable.
    await fs.writeFile(path.join(bin, tool), `#!${process.execPath}\n` + await fs.readFile(path.join(root, 'scripts/spec-automation/test-fixtures/tool.mjs'), 'utf8'), { mode: 0o755 });
  }
  const file = path.join(dir, 'db.json');
  const initial = { operations: [], models: [], labels: [], issue: { title: 'Feature', body: 'Scope', state: 'open', labels: [{ name: 'accepted' }, { name: 'needs-spec' }], assignees: [] }, comments: [{ id: 42, user: { login: 'github-actions[bot]' }, body: `<!-- duumbi-stage5-decision:${'a'.repeat(64)} -->\n**Decision:** Accept` }] };
  await fs.writeFile(file, JSON.stringify(initial));
  const env = { ...process.env, PATH: `${bin}:${process.env.PATH}`, SPEC_TEST_DB: file, DUUMBI_PROJECT_NUMBER: '1', DUUMBI_SPEC_STATE: path.join(dir, 'state') };
  delete env.OPENAI_API_KEY; delete env.CODEX_API_KEY;
  const read = async () => JSON.parse(await fs.readFile(file, 'utf8'));
  const edit = async (patch) => fs.writeFile(file, JSON.stringify({ ...await read(), ...patch }));
  const run = (mode) => command(process.execPath, [path.join(root, 'scripts/spec-automation/run.mjs'), mode, '123', '42'], { env });
  return { read, edit, run, dir };
}
test('real worker subprocess publishes once, waits for human merge, then finalizes idempotently', async (t) => {
  const f = await fixture(t);
  assert.match(await f.run('check'), /Preflight passed/);
  assert.equal((await f.read()).models.length, 0);
  assert.match(await f.run('run'), /Spec PR/);
  const published = await f.read(); assert.deepEqual(published.models.map((m) => m.stage), [6, 7, 8, 9]);
  assert.equal(published.status, 'Technical Spec Review');
  assert.match(await f.run('run'), /Duplicate event/);
  await assert.rejects(f.run('finalize'), /not been merged/);
  await f.edit({ pr: { ...published.pr, merged: true } });
  assert.match(await f.run('finalize'), /Complete/);
  const final = await f.read(); assert.equal(final.status, 'Ready for Build'); assert.equal(final.models.length, 4);
  assert.ok(final.issue.labels.some((l) => l.name === 'tech-spec-approved'));
  assert.equal(final.issue.state, 'open');
  const writes = final.comments.length; await f.run('finalize'); assert.equal((await f.read()).comments.length, writes);
});
test('ambiguous push resumes the exact reviewed commit without another model call', async (t) => {
  const f = await fixture(t); await f.edit({ failPushOnce: true });
  await assert.rejects(f.run('run'), /lost push response/);
  assert.match(await f.run('run'), /Duplicate event/);
  await f.run('resume'); assert.equal((await f.read()).models.length, 4);
  assert.equal((await f.read()).pr.head.sha, 'headsha');
});
test('finalizer rejects changed head, non-spec files, failed CI, unresolved reviews and changed artifacts', async (t) => {
  const f = await fixture(t); await f.run('run'); const original = (await f.read()).pr;
  for (const [flag, error] of [['sourceDrift', /Source changed/], ['extraFile', /spec-only/], ['badCI', /CI is not green/], ['badReview', /Blocking review/], ['unresolved', /Unresolved/], ['badArtifact', /artifact differs/]]) {
    await f.edit({ pr: { ...original, merged: true }, [flag]: true });
    await assert.rejects(f.run('finalize'), error);
    assert.notEqual((await f.read()).status, 'Ready for Build');
    await f.edit({ [flag]: false });
  }
  await f.edit({ pr: { ...original, merged: true, head: { sha: 'unreviewed' } } });
  await assert.rejects(f.run('finalize'), /merged unchanged/);
});
test('changed human input and an existing worker lock prevent model starts', async (t) => {
  const f = await fixture(t); await fs.mkdir(path.join(f.dir, 'state/worker.lock'), { recursive: true });
  await assert.rejects(f.run('run'), /Worker already running/); assert.equal((await f.read()).models.length, 0);
  await fs.rm(path.join(f.dir, 'state/worker.lock'), { recursive: true });
  await f.run('run'); const db = await f.read(); await f.edit({ issue: { ...db.issue, body: 'Expanded scope' } });
  await assert.rejects(f.run('finalize'), /Accepted input changed/);
});


test('split work creates linked children once and finalizes parent as coordination only', async (t) => {
  const f = await fixture(t); await f.edit({ split: true, failPushOnce: true });
  await assert.rejects(f.run('run'), /lost push response/);
  await f.run('resume'); const published = await f.read();
  assert.equal(published.children.length, 2); assert.equal(published.linked.length, 2);
  await f.edit({ pr: { ...published.pr, merged: true } }); await f.run('finalize');
  const final = await f.read();
  assert.ok(final.issue.labels.some((l) => l.name === 'spec-coordinator'));
  assert.ok(!final.issue.labels.some((l) => l.name === 'tech-spec-approved'));
  assert.ok(final.children.every((c) => c.labels.some((l) => l.name === 'tech-spec-approved')));
  assert.equal(final.models.length, 4);
});


test('durable queue retains a busy event, drains later, and deduplicates repeated delivery', async (t) => {
  const f = await fixture(t); await fs.mkdir(path.join(f.dir, 'state/worker.lock'), { recursive: true });
  assert.match(await f.run('enqueue'), /worker busy/);
  assert.equal((await f.read()).models.length, 0);
  await fs.rm(path.join(f.dir, 'state/worker.lock'), { recursive: true });
  await f.run('drain'); assert.equal((await f.read()).models.length, 4);
  await f.run('enqueue'); await f.run('drain'); assert.equal((await f.read()).models.length, 4);
});
test('queue records quota failure for attention and never automatically retries it', async (t) => {
  const f = await fixture(t); await f.edit({ failModel: true });
  await assert.rejects(f.run('enqueue'), /quota exhausted/);
  await f.run('drain'); await f.run('enqueue');
  assert.equal((await f.read()).models.length, 1);
  const record = JSON.parse(await fs.readFile(path.join(f.dir, 'state/queue/run-123-42.json'), 'utf8'));
  assert.equal(record.status, 'attention');
});
