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
