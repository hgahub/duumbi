import { hash, validate } from './contract.mjs';
const str = { type: 'string' };
const object = (properties) => ({ type: 'object', properties, required: Object.keys(properties), additionalProperties: false });
export const routingSchema = object({ route: { type: 'string', enum: ['autonomous', 'research', 'interactive', 'scope_change'] }, rationale: str, question: str });
export const researchSchema = object({ outcome: { type: 'string', enum: ['ready', 'interactive'] }, summary: str, question: str, sources: { type: 'array', items: object({ url: str, retrieved_on: str, summary: str }) } });
export function validateRouting(value) {
  validate(value, routingSchema);
  if (!value.rationale.trim() || (value.route !== 'autonomous' && !value.question.trim())) throw new Error('Routing requires rationale and actionable question');
  return value;
}
export function validateResearch(value) {
  validate(value, researchSchema);
  if (!value.summary.trim() || (value.outcome === 'ready' && !value.sources.length) || (value.outcome === 'interactive' && !value.question.trim())) throw new Error('Research requires evidence or a concrete unresolved question');
  for (const s of value.sources) {
    const u = new URL(s.url);
    if (u.protocol !== 'https:' || u.username || u.password || !/^\d{4}-\d{2}-\d{2}$/.test(s.retrieved_on) || !s.summary.trim()) throw new Error('Invalid research source');
  }
  return value;
}
export function continueAttempt(job, answer, permission) {
  const attempt = job.attempt || 1;
  if (!['interactive', 'needs_clarification', 'review_blocked', 'needs_research'].includes(job.status) || job.head || job.pr) throw new Error('Only an unpublished interactive job can continue');
  if (!['admin', 'maintain', 'write'].includes(permission) || answer.user?.type === 'Bot') throw new Error('Continuation requires a human repository writer');
  const prefix = `DUUMBI_SPEC_CONTINUE_V1 ${job.event.issue} ${job.event.decision} ${attempt}`;
  const header = `${prefix}\nScope: unchanged\n`;
  if (job.routing?.route === 'scope_change') throw new Error('Changed scope requires renewed Stage 5 acceptance');
  if (!answer.body?.startsWith(header) || !answer.body.slice(header.length).trim()) throw new Error('Continuation must target the current attempt and confirm unchanged scope');
  if (Object.values(job.calls || {}).some((c) => !c.result)) throw new Error('Uncertain model call remains; inspect before continuation');
  job.attempts ??= [];
  job.attempts.push(JSON.parse(JSON.stringify({ attempt, status: job.status, context: job.context, product: job.product, technical: job.technical, gate7: job.gate7, gate9: job.gate9, research: job.research, question: job.question, inputVersion: job.inputVersion })));
  // Once allocated, child scope is immutable. A new product may not invalidate existing children.
  if (job.gate7) job.fixedProduct ??= job.product;
  job.attempt = attempt + 1;
  job.context = { ...job.context, previousAnswers: [...(job.context.previousAnswers || []), ...(job.context.continuation ? [job.context.continuation] : [])], previousResearch: [...(job.context.previousResearch || []), ...(job.research || [])], continuation: { id: answer.id, author: answer.user.login, body: answer.body } };
  job.inputVersion = hash({ baseSha: job.baseSha, vaultSha: job.vaultSha, accepted: job.inputHash, answer: job.context.continuation });
  for (const key of ['product','technical','gate7','gate9','research','question','routing','handoff']) delete job[key];
  job.status = 'running';
  return job;
}
export function handoff(job) {
  const attempt = job.attempt || 1;
  const issue = `https://github.com/hgahub/duumbi/issues/${job.event.issue}`;
  const artifacts = [['Product draft', job.product?.product], ['Technical draft', job.technical?.technical]].filter(([, text]) => text).map(([name, text]) => `### ${name} (excerpt, up to 12000 characters)\n${text.slice(0, 12000)}`).join('\n\n');
  return `## Specification interactive handoff\nOwner: @${job.context.issue.assignees?.[0]?.login || 'hgahub'}\nIssue: ${issue}\nJob: ${job.key}; attempt: ${attempt}; stage: ${job.stage}\nReason: ${job.question}\n\nCompleted product review: ${job.gate7 ? 'yes' : 'no'}; technical review: ${job.gate9 ? 'yes' : 'no'}.\nEvidence: local job directory, attempt-prefixed result/events files and handoff.md; source ${job.baseSha}, planning ${job.vaultSha}.\n\n${artifacts}\n\n### Copyable interactive prompt\nContinue specification only for ${issue}. Read the accepted issue, owner decisions and this handoff. Resolve: ${job.question}\nPreserve scope and existing artifacts. Research public official documentation when needed. Do not merge, implement, delete checkpoints or bypass gates. Record your proposed answer on the issue for a human repository writer to confirm.\n\n### Return to the worker\nIf scope is unchanged, a human repository writer posts:\n\n\`\`\`text\nDUUMBI_SPEC_CONTINUE_V1 ${job.event.issue} ${job.event.decision} ${attempt}\nScope: unchanged\n<answer and evidence links>\n\`\`\`\n\nThen explicitly run: node scripts/spec-automation/run.mjs continue ${job.event.issue} ${job.event.decision} COMMENT_ID\nChanged scope requires renewed Stage 5 acceptance and reconciliation instead. The queue must not automatically retry this handoff.\n`;
}
