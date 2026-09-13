// Shared validation for Slack submissions and the canonical GitHub writer.
function decisionDetails(raw) {
  const text = (key) => typeof raw[key] === 'string' ? raw[key].trim() : '';
  const details = {
    rationale: text('rationale'),
    clarification_question: text('clarification_question'),
    clarification_owner: text('clarification_owner').replace(/^@/, ''),
  };
  const errors = {};
  if (['reject', 'needs-clarification'].includes(raw.decision)) {
    if (!details.rationale || details.rationale.length > 2000) errors.rationale = 'Provide a rationale (1–2000 characters).';
  }
  if (raw.decision === 'needs-clarification') {
    if (!details.clarification_question || details.clarification_question.length > 2000) errors.clarification_question = 'Provide the blocking question (1–2000 characters).';
    if (!/^[a-z\d](?:[a-z\d-]{0,37}[a-z\d])?$/i.test(details.clarification_owner) || details.clarification_owner.includes('--')) errors.clarification_owner = 'Enter the responsible person’s GitHub username.';
  }
  return { details, errors };
}

function stage5CommentLines(decision, details) {
  // Quote user text so a multiline rationale cannot introduce decision headers.
  const quote = (value) => String(value).split(/\r?\n/).map((line) => `> ${line}`).join('\n');
  return [
    '**Rationale:**', quote(details.rationale),
    decision === 'needs-clarification' ? `**Clarification owner:** @${details.clarification_owner}` : null,
    decision === 'needs-clarification' ? '**Remaining open questions:**' : '**Remaining open questions:** not recorded by this decision',
    decision === 'needs-clarification' ? quote(details.clarification_question) : null,
    decision === 'needs-clarification' ? '**Continuation:** The owner answers in this issue with an `@Clarification` comment. Synthesis is advisory; a new human acceptance decision is required.' : null,
  ].filter(Boolean);
}
module.exports = { decisionDetails, stage5CommentLines };
