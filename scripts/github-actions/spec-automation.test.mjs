import test from 'node:test';
import assert from 'node:assert/strict';
import { accepted, event, validateProduct, validateTechnical, validateReview, specFiles, hash } from '../spec-automation/contract.mjs';
import { generate, MODELS } from '../spec-automation/engine.mjs';
const product = () => ({ outcome: 'ready', question: '', complexity: 'normal', rationale: 'One cohesive capability', product: '# Product', decomposition: '# One unit', units: [{ key: 'core', title: 'Core', product: '# Product', dependencies: [] }] });
const technical = () => ({ outcome: 'ready', question: '', technical: '# Technical', units: [{ key: 'core', technical: '# Technical' }] });
const approve = () => ({ decision: 'approve', rationale: 'All criteria verified', findings: [], question: '' });
const input = { version: 1, repo: 'hgahub/duumbi', issue: 123, decision: 42 };
const decision = { id: 42, user: { login: 'github-actions[bot]' }, body: `<!-- duumbi-stage5-decision:${'a'.repeat(64)} -->\n**Decision:** Accept` };
const issue = { title: 'Feature', body: 'Scope', state: 'open', labels: [{ name: 'accepted' }] };
function fixture(answer = (c) => c.stage === 6 ? product() : c.stage === 8 ? technical() : approve()) {
  const calls = [], writes = [];
  const job = { event: input, context: { issue }, calls: {} };
  const deps = { save: async () => {}, children: async () => { writes.push('children'); }, model: async (c) => { calls.push(c); return answer(c); } };
  return { job, deps, calls, writes };
}
test('validates transport and current, trusted, unchanged human acceptance', () => {
  assert.deepEqual(event(input), input);
  for (const mutation of [{ repo: 'other/repo' }, { issue: '123' }, { decision: 0 }, { version: 2 }]) assert.throws(() => event({ ...input, ...mutation }));
  const digest = accepted(issue, [decision], input);
  assert.throws(() => accepted(issue, [{ ...decision, user: { login: 'attacker' } }], input));
  assert.throws(() => accepted({ ...issue, body: 'Different scope' }, [decision], input, digest));
  assert.throws(() => accepted(issue, [decision, { ...decision, id: 43, body: decision.body.replace('Accept', 'Reject') }], input));
  assert.throws(() => accepted({ ...issue, labels: ['accepted', 'needs-clarification'] }, [decision], input));
});
test('decomposition rejects traversal, duplicates, dangling dependencies and cycles', () => {
  for (const units of [[], [{ ...product().units[0], key: '../outside' }], [product().units[0], product().units[0]], [{ ...product().units[0], dependencies: ['missing'] }], [{ ...product().units[0], dependencies: ['core'] }]]) assert.throws(() => validateProduct({ ...product(), units }));
  assert.throws(() => validateTechnical({ ...technical(), units: [] }, product()));
  assert.throws(() => validateReview({ ...approve(), findings: ['Blocking issue'] }));
  assert.throws(() => validateReview({ ...approve(), decision: 'needs_clarification' }));
});
test('runs 6 → independent 7 → child allocation → 8 → independent 9, then stops', async () => {
  const f = fixture(); assert.equal((await generate(f.job, f.deps)).status, 'reviewed');
  assert.deepEqual(f.calls.map((c) => c.stage), [6, 7, 8, 9]);
  assert.ok(f.calls.every((c) => c.model === MODELS.normal));
  assert.equal(f.job.gate7.artifactHash, hash(f.job.product));
  assert.equal(f.job.gate9.artifactHash, hash(f.job.technical));
  assert.equal(f.calls[1].payload.previousDraft, undefined);
  assert.deepEqual(f.writes, ['children']);
  await generate(f.job, f.deps); assert.equal(f.calls.length, 4, 'resume must reuse successful model checkpoints');
});
test('two correction rounds are bounded and escalate to Astra', async () => {
  const f = fixture((c) => c.stage === 6 ? product() : { ...approve(), decision: 'revise', findings: ['Missing criterion'] });
  assert.equal((await generate(f.job, f.deps)).status, 'review_blocked');
  assert.deepEqual(f.calls.map((c) => c.stage), [6, 7, 6, 7, 6, 7]);
  assert.ok(f.calls.slice(2).every((c) => c.model === MODELS.escalation));
  assert.deepEqual(f.writes, []);
});
test('complex product escalates independent review and technical work', async () => {
  const f = fixture((c) => c.stage === 6 ? { ...product(), complexity: 'high' } : c.stage === 8 ? technical() : approve());
  await generate(f.job, f.deps);
  assert.equal(f.calls[0].model, MODELS.normal); assert.ok(f.calls.slice(1).every((c) => c.model === MODELS.escalation));
});
test('clarification stops before further work, preserving the actual question', async () => {
  const f = fixture((c) => c.stage === 6 ? product() : { ...approve(), decision: 'needs_clarification', question: 'Which deployment is supported?' });
  const result = await generate(f.job, f.deps);
  assert.equal(result.status, 'needs_clarification'); assert.match(result.question, /deployment/); assert.equal(f.calls.length, 2); assert.deepEqual(f.writes, []);
});
test('quota failure or malformed output cannot silently retry or fall back to API', async () => {
  for (const answer of [() => { throw new Error('quota exceeded'); }, () => ({ unexpected: 'not a spec' })]) {
    const f = fixture(answer);
    await assert.rejects(generate(f.job, f.deps));
    await assert.rejects(generate(f.job, f.deps), /Uncertain\/interrupted/);
    assert.equal(f.calls.length, 1);
  }
});
test('split package only writes fixed spec paths, with separate child artifacts', () => {
  const p = product(); p.units.push({ key: 'ui', title: 'UI', product: '# UI', dependencies: ['core'] });
  validateProduct(p);
  const files = specFiles({ event: input, product: p, technical: { ...technical(), units: [...technical().units, { key: 'ui', technical: '# UI tech' }] }, children: { core: 201, ui: 202 } });
  assert.deepEqual(Object.keys(files).sort(), ['specs/DUUMBI-123/DECOMPOSITION.md','specs/DUUMBI-123/PRODUCT.md','specs/DUUMBI-123/TECHNICAL.md','specs/DUUMBI-201/PRODUCT.md','specs/DUUMBI-201/TECHNICAL.md','specs/DUUMBI-202/PRODUCT.md','specs/DUUMBI-202/TECHNICAL.md'].sort());
});

test('legacy gh pagination preserves arrays and object envelopes across compact JSON pages', async () => {
  const { parsePages } = await import('../spec-automation/run.mjs');
  assert.deepEqual(parsePages('[{"id":1,"body":"line 1\\nline 2"}]\r\n[]\n[{"id":2}]\n'), [{ id: 1, body: 'line 1\nline 2' }, { id: 2 }]);
  assert.deepEqual(parsePages('{"check_runs":[{"id":1}]}\n{"check_runs":[{"id":2}]}\n').flatMap((p) => p.check_runs), [{ id: 1 }, { id: 2 }]);
  assert.deepEqual(parsePages('[]\n'), []);
  assert.throws(() => parsePages('[{"id":1}]\nnot json\n'));
  assert.throws(() => parsePages('null\n'));
  assert.throws(() => parsePages(''));
});

test('Project preflight checks writable project, required statuses and classic-token write scope', async () => {
  const { validateProjectAccess } = await import('../spec-automation/run.mjs');
  const project = { viewerCanUpdate: true, fields: { nodes: [{ name: 'Status', options: ['Technical Spec Needed','Technical Spec Review','Needs Clarification','Ready for Build','In Progress'].map((name) => ({ name })) }] } };
  assert.doesNotThrow(() => validateProjectAccess(project, 'X-OAuth-Scopes: repo, project\r\n'));
  assert.throws(() => validateProjectAccess(project, 'x-oauth-scopes: repo, read:project'), /requires project scope/);
  assert.throws(() => validateProjectAccess({ ...project, viewerCanUpdate: false }), /not writable/);
  assert.throws(() => validateProjectAccess({ ...project, fields: { nodes: [] } }), /Missing project status/);
});

test('routing researches before drafting and carries evidence into independent reviews', async () => {
  const f = fixture(c => c.stage === 0 ? { route: 'research', rationale: 'Need official evidence', question: 'Verify API' } : c.stage === 1 ? { outcome: 'ready', summary: 'Verified API', question: '', sources: [{ url: 'https://example.com/docs', retrieved_on: '2026-09-15', summary: 'API contract' }] } : c.stage === 6 ? product() : c.stage === 8 ? technical() : approve());
  f.job.routingVersion = 1;
  assert.equal((await generate(f.job, f.deps)).status, 'reviewed');
  assert.deepEqual(f.calls.map(c => c.stage), [0, 1, 6, 7, 8, 9]);
  assert.equal(f.calls.at(-1).payload.research.length, 1);
  assert.equal(f.job.calls['a1-9-0'].inputHash, hash(f.calls.at(-1).payload));
  assert.match(specFiles(f.job)['specs/DUUMBI-123/RESEARCH.md'], /example.com/);
  f.job.context.previousResearch = f.job.research; delete f.job.research;
  assert.match(specFiles(f.job)['specs/DUUMBI-123/RESEARCH.md'], /example.com/, 'continuation retains earlier sources in published evidence');
});
test('interactive routing stops before drafting; budget also returns handoff', async () => {
  const f = fixture(() => ({ route: 'interactive', rationale: 'Product choice', question: 'Which outcome?' }));
  f.job.routingVersion = 1;
  assert.equal((await generate(f.job, f.deps)).status, 'interactive');
  assert.deepEqual(f.calls.map(c => c.stage), [0]);
  assert.equal(f.writes.length, 0);
  const b = fixture(c => c.stage === 0 ? { route: 'autonomous', rationale: 'Clear', question: '' } : product());
  b.job.routingVersion = 1; b.job.maxCalls = 1;
  assert.equal((await generate(b.job, b.deps)).status, 'interactive');
  assert.equal(b.calls.length, 1);
});
test('continuation is scoped, permission checked and preserves previous attempts', async () => {
  const { continueAttempt } = await import('../spec-automation/routing.mjs');
  const job = { event: input, status: 'needs_clarification', context: { issue }, calls: {}, product: product(), gate7: { approved: true } };
  const answer = { id: 99, user: { login: 'owner', type: 'User' }, body: 'DUUMBI_SPEC_CONTINUE_V1 123 42 1\nScope: unchanged\nKeep the accepted behavior.' };
  assert.throws(() => continueAttempt(structuredClone(job), answer, 'read'));
  assert.throws(() => continueAttempt(structuredClone(job), { ...answer, body: answer.body.replace('42 1', '42 2') }, 'write'));
  assert.throws(() => continueAttempt({ ...job, routing: { route: 'scope_change' } }, answer, 'write'));
  continueAttempt(job, answer, 'write');
  assert.equal(job.attempt, 2); assert.equal(job.status, 'running');
  assert.equal(job.attempts[0].gate7.approved, true);
  assert.equal(job.gate7, undefined); assert.deepEqual(job.fixedProduct, product());
});
