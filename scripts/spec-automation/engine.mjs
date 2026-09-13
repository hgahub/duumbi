import { hash, productSchema, technicalSchema, reviewSchema, validateProduct, validateTechnical, validateReview } from './contract.mjs';
export const MODELS = { normal: 'gpt-5.6-sol', escalation: 'gpt-6-astra' };
// deps.model gets a fresh CLI session; successful checkpoints survive retry/reboot.
export async function generate(job, deps) {
  job.calls ??= {}; job.children ??= {};
  const save = () => deps.save(job);
  async function call(key, stage, schema, payload, model, verify) {
    if (job.calls[key]?.result) return job.calls[key].result;
    if (job.calls[key]) throw new Error(`Uncertain/interrupted model call ${key}; use explicit retry after checking quota and logs`);
    job.calls[key] = { stage, model, effort: 'high', inputHash: hash(payload), startedAt: new Date().toISOString() }; await save();
    // Never silently retry timeouts, exhausted quota, model unavailability, or malformed output.
    const result = verify(await deps.model({ key, stage, schema, payload, model }));
    job.calls[key].result = result; job.calls[key].finishedAt = new Date().toISOString(); await save();
    return result;
  }
  async function gate(draftStage, reviewStage, kind, schema, verify) {
    for (let round = 0; round <= 2; round++) {
      const previous = job.calls[`${reviewStage}-${round - 1}`]?.result;
      const model = round > 0 || job.product?.complexity === 'high' ? MODELS.escalation : MODELS.normal;
      const draft = verify(await call(`${draftStage}-${round}`, draftStage, schema, { context: job.context, product: kind === 'technical' ? job.product : null, previousDraft: round ? job[kind] : null, findings: previous || null }, model, verify));
      job[kind] = draft; await save();
      if (draft.outcome === 'needs_clarification') return { status: 'needs_clarification', stage: draftStage, question: draft.question };
      const reviewModel = draft.complexity === 'high' ? MODELS.escalation : model;
      const review = validateReview(await call(`${reviewStage}-${round}`, reviewStage, reviewSchema, { context: job.context, product: kind === 'technical' ? job.product : draft, technical: kind === 'technical' ? draft : null }, reviewModel, validateReview));
      if (review.decision === 'approve') { job[`gate${reviewStage}`] = { ...review, artifactHash: hash(draft), model: reviewModel, round }; await save(); return null; }
      if (review.decision === 'needs_clarification') return { status: 'needs_clarification', stage: reviewStage, question: review.question };
    }
    return { status: 'review_blocked', stage: reviewStage, question: 'Two correction rounds exhausted; owner must resolve recorded findings.' };
  }
  if (!job.gate7) { const stop = await gate(6, 7, 'product', productSchema, validateProduct); if (stop) return stop; }
  // Allocate child issues only after the decomposition passed the independent product gate.
  await deps.children(job); await save();
  if (!job.gate9) { const stop = await gate(8, 9, 'technical', technicalSchema, (t) => validateTechnical(t, job.product)); if (stop) return stop; }
  return { status: 'reviewed' };
}
