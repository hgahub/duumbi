import { routingSchema, researchSchema, validateRouting, validateResearch } from './routing.mjs';
import { hash, productSchema, technicalSchema, reviewSchema, validateProduct, validateTechnical, validateReview } from './contract.mjs';
export const MODELS = { normal: 'gpt-5.6-sol', escalation: 'gpt-6-astra' };
// deps.model gets a fresh CLI session; successful checkpoints survive retry/reboot.
export async function generate(job, deps) {
  job.calls ??= {}; job.children ??= {};
  const prefix = job.routingVersion ? `a${job.attempt || 1}-` : '';
  const now = deps.now || Date.now;
  const started = now();
  const maxCalls = job.maxCalls || 16;
  const maxMs = job.maxMs || 90 * 60 * 1000;
  const save = () => deps.save(job);
  async function call(key, stage, schema, payload, model, verify) {
    key = prefix + key;
    if (job.calls[key]?.result) return job.calls[key].result;
    if (job.calls[key]) throw new Error(`Uncertain/interrupted model call ${key}; use explicit retry after checking quota and logs`);
    if (Object.keys(job.calls).filter((k) => k.startsWith(prefix)).length + (job.failedCalls || []).filter((c) => c.attempt === (job.attempt || 1)).length >= maxCalls || Object.values(job.calls).filter((c) => c.attempt === (job.attempt || 1)).reduce((ms, c) => ms + (c.finishedAt ? Date.parse(c.finishedAt) - Date.parse(c.startedAt) : 0), 0) >= maxMs || now() - started >= maxMs) throw Object.assign(new Error('Autonomous budget exhausted; continue interactively'), { handoff: true });
    payload = structuredClone({ ...payload, research: job.research || [], fixedProduct: job.fixedProduct || null });
    job.calls[key] = { attempt: job.attempt || 1, stage, model, effort: 'high', inputHash: hash(payload), startedAt: new Date().toISOString() }; await save();
    // Never silently retry timeouts, exhausted quota, model unavailability, or malformed output.
    const result = verify(await deps.model({ key, stage, schema, payload, model }));
    job.calls[key].result = result; job.calls[key].finishedAt = new Date().toISOString(); await save();
    return result;
  }
  async function research(question, key) {
    job.research ??= [];
    const existing = job.research.find((r) => r.key === key);
    if (existing) return existing.outcome === 'ready' ? null : { status: 'interactive', stage: 1, question: existing.question };
    if (job.research.length >= 2) return { status: 'interactive', stage: 1, question: `Research budget exhausted: ${question}` };
    const r = await call(key, 1, researchSchema, { context: job.context, question }, MODELS.normal, validateResearch);
    const recordedAt = new Date().toISOString();
    job.research.push({ ...r, key, recordedAt, hash: hash(r) }); await save();
    if (r.outcome !== 'ready') return { status: 'interactive', stage: 1, question: r.question };
    return null;
  }
  async function gate(draftStage, reviewStage, kind, schema, verify) {
    for (let round = 0; round <= 2; round++) {
      const previous = job.calls[`${prefix}${reviewStage}-${round - 1}`]?.result;
      const model = round > 0 || job.product?.complexity === 'high' ? MODELS.escalation : MODELS.normal;
      let draft = verify(await call(`${draftStage}-${round}`, draftStage, schema, { context: job.context, product: kind === 'technical' ? job.product : null, previousDraft: round ? job[kind] : null, findings: previous || null }, model, verify));
      if (draft.outcome === 'needs_research' && job.routingVersion) {
        const stop = await research(draft.question, `research-${draftStage}-${round}`);
        if (stop) return stop;
        draft = verify(await call(`${draftStage}-${round}-researched`, draftStage, schema, { context: job.context, product: kind === 'technical' ? job.product : null, previousDraft: draft, findings: previous || null }, model, verify));
      }
      if (draft.outcome === 'needs_research') return { status: 'interactive', stage: draftStage, question: draft.question };
      if (kind === 'product' && job.fixedProduct && draft.outcome === 'ready' && hash(draft) !== hash(job.fixedProduct)) return { status: 'interactive', stage: draftStage, question: 'Previously approved product/decomposition would change. Reconcile existing units and obtain new acceptance for changed scope.' };
      job[kind] = draft; await save();
      if (draft.outcome === 'needs_clarification') return { status: 'needs_clarification', stage: draftStage, question: draft.question };
      const reviewModel = draft.complexity === 'high' ? MODELS.escalation : model;
      const review = validateReview(await call(`${reviewStage}-${round}`, reviewStage, reviewSchema, { context: job.context, product: kind === 'technical' ? job.product : draft, technical: kind === 'technical' ? draft : null }, reviewModel, validateReview));
      if (review.decision === 'approve') { job[`gate${reviewStage}`] = { ...review, artifactHash: hash(draft), model: reviewModel, round }; await save(); return null; }
      if (review.decision === 'needs_research') {
        if (job.routingVersion) { const stop = await research(review.question, `research-${reviewStage}-${round}`); if (stop) return stop; continue; }
        return { status: 'interactive', stage: reviewStage, question: review.question };
      }
      if (review.decision === 'needs_clarification') return { status: 'needs_clarification', stage: reviewStage, question: review.question };
    }
    return { status: 'review_blocked', stage: reviewStage, question: 'Two correction rounds exhausted; owner must resolve recorded findings.' };
  }
  try {
  if (job.routingVersion && !job.routing) {
    job.routing = await call('route', 0, routingSchema, { context: job.context }, MODELS.normal, validateRouting); await save();
  }
  if (job.routing?.route === 'research' && !job.research?.length) { const stop = await research(job.routing.question, 'research-initial'); if (stop) return stop; }
  if (['interactive', 'scope_change'].includes(job.routing?.route)) return { status: 'interactive', stage: 0, question: job.routing.question };
  if (!job.gate7) { const stop = await gate(6, 7, 'product', productSchema, validateProduct); if (stop) return stop; }
  // Allocate child issues only after the decomposition passed the independent product gate.
  await deps.children(job); await save();
  if (!job.gate9) { const stop = await gate(8, 9, 'technical', technicalSchema, (t) => validateTechnical(t, job.product)); if (stop) return stop; }
  return { status: 'reviewed' };
  } catch (error) { if (error.handoff) return { status: 'interactive', stage: 0, question: error.message }; throw error; }
}
