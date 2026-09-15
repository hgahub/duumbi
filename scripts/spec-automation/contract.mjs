import { createHash } from 'node:crypto';
export const REPO = 'hgahub/duumbi';
export const hash = (value) => createHash('sha256').update(typeof value === 'string' ? value : JSON.stringify(value)).digest('hex');
export const marker = (key) => `<!-- duumbi-spec-job:${key} -->`;
export function event(input) {
  if (input.version !== 1 || input.repo !== REPO || !Number.isSafeInteger(input.issue) || input.issue < 1 || !Number.isSafeInteger(input.decision) || input.decision < 1) throw new Error('Invalid Stage 5 event');
  return { version: 1, repo: REPO, issue: input.issue, decision: input.decision };
}
export function accepted(issue, comments, input, expectedHash) {
  const labels = issue.labels.map((l) => typeof l === 'string' ? l : l.name);
  const decisions = comments.filter((c) => c.user?.login === 'github-actions[bot]' && /^<!-- duumbi-stage5-decision:[a-f0-9]{64} -->$/m.test(c.body));
  const latest = decisions.sort((a, b) => a.id - b.id).at(-1);
  if (issue.state !== 'open' || !labels.includes('accepted') || labels.includes('needs-clarification') || latest?.id !== input.decision || !/^\*\*Decision:\*\* Accept\s*$/m.test(latest.body)) throw new Error('Current human acceptance is missing or superseded');
  const digest = hash({ title: issue.title, body: issue.body, decision: latest.body });
  if (expectedHash && digest !== expectedHash) throw new Error('Accepted input changed; a new human acceptance is required');
  return digest;
}
const str = { type: 'string' };
const array = (items) => ({ type: 'array', items });
const object = (properties) => ({ type: 'object', properties, required: Object.keys(properties), additionalProperties: false });
export const productSchema = object({
  outcome: { enum: ['ready', 'needs_clarification', 'needs_research'], type: 'string' }, question: str,
  complexity: { enum: ['normal', 'high'], type: 'string' }, rationale: str,
  product: str, decomposition: str,
  units: array(object({ key: str, title: str, product: str, dependencies: array(str) })),
});
export const technicalSchema = object({ outcome: { enum: ['ready', 'needs_clarification', 'needs_research'], type: 'string' }, question: str, technical: str, units: array(object({ key: str, technical: str })) });
export const reviewSchema = object({ decision: { enum: ['approve', 'revise', 'needs_clarification', 'needs_research'], type: 'string' }, rationale: str, findings: array(str), question: str });
// Validate every field locally as well: output-schema is a model constraint, not a trust boundary.
export function validate(value, schema, location = 'result') {
  if (schema.type === 'object') {
    if (!value || Array.isArray(value) || typeof value !== 'object' || Object.keys(value).some((k) => !Object.hasOwn(schema.properties, k))) throw new Error(`Invalid ${location}`);
    for (const k of schema.required) validate(value[k], schema.properties[k], `${location}.${k}`);
  } else if (schema.type === 'array') {
    if (!Array.isArray(value) || value.length > 20) throw new Error(`Invalid ${location}`);
    value.forEach((v, i) => validate(v, schema.items, `${location}[${i}]`));
  } else if (typeof value !== 'string' || value.length > 150000 || (schema.enum && !schema.enum.includes(value))) throw new Error(`Invalid ${location}`);
  return value;
}
export function validateProduct(p) {
  validate(p, productSchema);
  if (p.outcome !== 'ready') { if (!p.question.trim()) throw new Error('Clarification requires a question'); return p; }
  if (!p.product.trim() || !p.decomposition.trim() || !p.rationale.trim() || !p.units.length) throw new Error('Empty product or decomposition');
  const keys = new Set(p.units.map((u) => u.key));
  if (keys.size !== p.units.length) throw new Error('Duplicate unit key');
  for (const u of p.units) {
    if (!/^[a-z][a-z0-9-]{0,39}$/.test(u.key) || !u.title.trim() || !u.product.trim() || u.dependencies.some((d) => !keys.has(d))) throw new Error('Invalid implementation unit');
  }
  const done = new Set(), visiting = new Set();
  function visit(key) {
    if (visiting.has(key)) throw new Error('Dependency cycle');
    if (done.has(key)) return;
    visiting.add(key); p.units.find((u) => u.key === key).dependencies.forEach(visit); visiting.delete(key); done.add(key);
  }
  keys.forEach(visit);
  return p;
}
export function validateTechnical(t, p) {
  validate(t, technicalSchema);
  if (t.outcome !== 'ready') { if (!t.question.trim()) throw new Error('Clarification requires a question'); return t; }
  if (!t.technical.trim() || t.units.length !== p.units.length || new Set(t.units.map((u) => u.key)).size !== t.units.length || t.units.some((u) => !u.technical.trim() || !p.units.some((v) => v.key === u.key))) throw new Error('Technical coverage does not match approved units');
  return t;
}
export function validateReview(r) {
  validate(r, reviewSchema);
  if (!r.rationale.trim() || (r.decision === 'approve' && (r.findings.length || r.question.trim())) || (r.decision === 'revise' && !r.findings.length) || (['needs_clarification', 'needs_research'].includes(r.decision) && !r.question.trim())) throw new Error('Inconsistent review verdict');
  return r;
}
export function specFiles(job) {
  validateProduct(job.product); validateTechnical(job.technical, job.product);
  if (job.gate7 && job.gate7.artifactHash !== hash(job.product) || job.gate9 && job.gate9.artifactHash !== hash(job.technical)) throw new Error('Artifact changed after its review');
  if (job.product.units.length > 1 && job.product.units.some((u) => !Number.isSafeInteger(job.children[u.key]) || job.children[u.key] < 1)) throw new Error('Missing child issue mapping');
  const root = `specs/DUUMBI-${job.event.issue}`;
  const files = { [`${root}/PRODUCT.md`]: job.product.product, [`${root}/TECHNICAL.md`]: job.technical.technical, [`${root}/DECOMPOSITION.md`]: job.product.decomposition };
  if (job.product.units.length > 1) for (const unit of job.product.units) {
    const dir = `specs/DUUMBI-${job.children[unit.key]}`;
    files[`${dir}/PRODUCT.md`] = unit.product;
    files[`${dir}/TECHNICAL.md`] = job.technical.units.find((u) => u.key === unit.key).technical;
  }
  const research = [...(job.context?.previousResearch || []), ...(job.research || [])];
  if (research.length) files[`${root}/RESEARCH.md`] = research.map((r) => `## ${r.recordedAt}\n${r.summary}\n\n${r.sources.map((s) => `- [Source](${s.url}) (${s.retrieved_on}): ${s.summary}`).join('\n')}`).join('\n\n');
  // One execution issue uses the top-level pair; require the model to keep unit and aggregate consistent in review.
  return files;
}
