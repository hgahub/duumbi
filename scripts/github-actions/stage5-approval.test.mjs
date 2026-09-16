import test from 'node:test';
import assert from 'node:assert/strict';
import fs from 'node:fs';
import path from 'node:path';
import { createRequire } from 'node:module';
const require = createRequire(import.meta.url);
const root = path.resolve(import.meta.dirname, '../..');
const yaml = fs.readFileSync(path.join(root, '.github/workflows/stage-approval.yml'), 'utf8');
const script = yaml.split('          script: |\n')[1].split('\n      - name:')[0].split('\n').map((line) => line.replace(/^ {12}/, '')).join('\n');
const execute = new (Object.getPrototypeOf(async function() {}).constructor)('github', 'context', 'core', 'require', 'process', 'fetch', script);

async function runDecision(input, options = {}) {
  const calls = [], comments = options.comments || [];
  const labels = options.labels || ['needs-human-review', 'needs-clarification'];
  const issue = { number: 123, title: 'Test issue', state: 'open', labels, ...options.issue };
  const issues = {
    get: async () => ({ data: issue }),
    listComments: () => {},
    createComment: async ({ body }) => { calls.push(['comment', body]); const c = { id: comments.length + 100, body, html_url: 'https://github.com/hgahub/duumbi/issues/123#issuecomment-1' }; comments.push(c); return { data: c }; },
    update: async (args) => { calls.push(['update', args]); },
    addLabels: async (args) => calls.push(['add', args.labels]),
    removeLabel: async (args) => calls.push(['remove', args.name]),
  };
  const summary = new Proxy({}, { get: (_, key) => key === "then" ? undefined : () => summary });
  const failures = [];
  await execute({ rest: { issues, users: { getByUsername: async () => { if (options.missingOwner) throw new Error("404"); return { data: { type: "User" } }; } } }, paginate: async () => comments }, {
    eventName: options.manual ? 'workflow_dispatch' : 'repository_dispatch',
    payload: options.manual ? { inputs: { stage: '5', issue_number: 123, ...input } } : { client_payload: { stage: '5', issue_number: 123, ...input } },
    actor: 'hgahub', repo: { owner: 'hgahub', repo: 'duumbi' },
  }, { info() {}, warning() {}, setFailed: (s) => failures.push(s), summary }, require, { env: { GITHUB_WORKSPACE: root, ...(options.env || {}) } }, async (url, request) => { calls.push(['slack', JSON.parse(request.body)]); return { json: async () => ({ ok: true }) }; });
  return { calls, failures, comments };
}

test('Stage 5 validates mandatory details before any GitHub writes on dispatch and manual paths', async () => {
  for (const manual of [false, true]) for (const input of [{ decision: 'reject' }, { decision: 'needs-clarification', rationale: 'Unclear' }]) {
    const result = await runDecision(input, { manual });
    assert.equal(result.failures.length, 1);
    assert.deepEqual(result.calls, []);
  }
});
test('Stage 5 clarification records actual rationale, question and owner, preserves review label', async () => {
  const result = await runDecision({ decision: 'needs-clarification', rationale: 'Scope is missing', clarification_question: 'Which provider?', clarification_owner: '@hgahub' });
  assert.deepEqual(result.failures, []);
  const body = result.calls.find(([kind]) => kind === 'comment')[1];
  assert.match(body, /Scope is missing/);
  assert.match(body, /Which provider\?/);
  assert.match(body, /Clarification owner:\*\* @hgahub/);
  assert.doesNotMatch(body, /Remaining open questions:\*\* none/);
  assert.equal(result.calls.some(([kind]) => kind === 'update' || kind === 'remove'), false);
});
test('Stage 5 Reject records rationale and closes; Accept clears clarification label', async () => {
  const reject = await runDecision({ decision: 'reject', rationale: 'Outside scope' });
  assert.deepEqual(reject.failures, []);
  assert.equal(reject.calls.find(([kind]) => kind === 'update')[1].state, 'closed');
  assert.match(reject.comments[0].body, /Outside scope/);
  const accept = await runDecision({ decision: 'approve' });
  assert.deepEqual(accept.failures, []);
  assert.ok(accept.calls.some(([kind, name]) => kind === 'remove' && name === 'needs-clarification'));
});
test('clarification rounds retain distinct evidence while identical dispatch retries reuse it', async () => {
  const input = { decision: 'needs-clarification', rationale: 'Scope', clarification_question: 'Which provider?', clarification_owner: 'hgahub', decision_id: 'V1' };
  const first = await runDecision(input);
  const retry = await runDecision(input, { comments: first.comments });
  assert.equal(retry.calls.filter(([kind]) => kind === 'comment').length, 0);
  const next = await runDecision({ ...input, clarification_question: 'Which user?', decision_id: 'V2' }, { comments: first.comments });
  assert.equal(next.calls.filter(([kind]) => kind === 'comment').length, 1);
});
test('new stale decisions fail before writes after human acceptance', async () => {
  const result = await runDecision({ decision: 'reject', rationale: 'Old button' }, { labels: ['accepted'] });
  assert.equal(result.failures.length, 1);
  assert.deepEqual(result.calls, []);
});

test('retrying an old clarification form cannot roll an accepted issue backward', async () => {
  const input = { decision: 'needs-clarification', rationale: 'Scope', clarification_question: 'Which provider?', clarification_owner: 'hgahub', decision_id: 'V1' };
  const first = await runDecision(input);
  const retry = await runDecision(input, { comments: first.comments, labels: ['accepted', 'needs-spec'] });
  assert.deepEqual(retry.failures, []);
  assert.deepEqual(retry.calls, []);
});

test('unknown clarification owner fails before recording a decision', async () => {
  const result = await runDecision({ decision: 'needs-clarification', rationale: 'Scope', clarification_question: 'Which provider?', clarification_owner: 'missing' }, { missingOwner: true });
  assert.equal(result.failures.length, 1);
  assert.deepEqual(result.calls, []);
});


test('Stage 5 Accept stores the Desktop prompt on GitHub and sends it once without a worker event', async () => {
  for (const manual of [false, true]) {
    const env = { SLACK_BOT_TOKEN: 'fake-test-only', SLACK_REVIEW_CHANNEL_ID: 'test-channel', INPUT_SLACK_RESPONSE_URL: 'https://example.test/response' };
    const accepted = await runDecision({ decision: 'approve' }, { env, manual, labels: ['needs-human-review', 'spec-automation'] });
    assert.ok(accepted.calls.some(([kind, name]) => kind === 'remove' && name === 'spec-automation'));
    assert.deepEqual(accepted.failures, []);
    const messages = accepted.calls.filter(([kind]) => kind === 'slack');
    assert.equal(messages.length, 1);
    assert.equal(messages[0][1].channel, 'test-channel');
    const prompt = accepted.comments[0].body.split('```text\n')[1].split('```')[0].trim();
    assert.match(prompt, /duumbi-spec-desktop/);
    assert.match(prompt, /Do not merge, start implementation or Stage 10/);
    assert.ok(messages[0][1].text.includes(prompt));
    assert.doesNotMatch(messages[0][1].text, /DUUMBI_SPEC_EVENT|duumbi-spec-autopilot/);
    const replay = await runDecision({ decision: 'approve' }, { env, manual, comments: accepted.comments, labels: ['accepted', 'needs-spec'] });
    assert.deepEqual(replay.calls, []);
  }
});
test('Stage 5 Desktop handoff uses response URL when channel credentials are absent', async () => {
  const accepted = await runDecision({ decision: 'approve' }, { env: { INPUT_SLACK_RESPONSE_URL: 'https://example.test/response' } });
  const messages = accepted.calls.filter(([kind]) => kind === 'slack');
  assert.equal(messages.length, 1);
  assert.match(messages[0][1].text, /duumbi-spec-desktop/);
  assert.equal(messages[0][1].replace_original, false);
});
test('non-accept decisions never emit a specification prompt or worker event', async () => {
  const env = { SLACK_BOT_TOKEN: 'fake-test-only', SLACK_REVIEW_CHANNEL_ID: 'test-channel' };
  for (const decision of ['reject', 'needs-clarification']) {
    const result = await runDecision({ decision, rationale: 'Requires owner attention', clarification_question: 'Which scope?', clarification_owner: 'hgahub' }, { env });
    assert.deepEqual(result.failures, []);
    assert.doesNotMatch(result.comments[0].body, /duumbi-spec-desktop/);
    assert.doesNotMatch(result.calls.find(([kind]) => kind === 'slack')[1].text, /DUUMBI_SPEC_EVENT|duumbi-spec-desktop/);
  }
});
