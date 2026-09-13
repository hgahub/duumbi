import fs from 'node:fs';
import path from 'node:path';
import { readIntake } from '../intake/contract.mjs';
import { createGithubApi, validateProjectStatusConfig, statusNameForProjectItem, archiveProcessedInboxNotes, commitVaultChanges, HUMAN_ACCEPTANCE_STATUS, HUMAN_ACCEPTANCE_LABEL } from './triage-queue-refill.mjs';

export function validateIntakeId(value) {
  if (typeof value !== 'string' || !/^[A-Za-z0-9][A-Za-z0-9._:-]{0,127}$/.test(value)) throw new Error('intake_id must be 1–128 letters, digits, dots, colons, underscores or hyphens');
  return value;
}
function markdownFiles(root) {
  if (!fs.existsSync(root)) return [];
  return fs.readdirSync(root, { withFileTypes: true }).flatMap((entry) => {
    const file = path.join(root, entry.name);
    return entry.isDirectory() ? markdownFiles(file) : entry.isFile() && entry.name.endsWith('.md') ? [file] : [];
  });
}
export function findIntake(vaultRoot, id) {
  validateIntakeId(id);
  const roots = ['Duumbi/00 Inbox (ToProcess)', 'Duumbi/05 Archive/Processed Inbox'];
  const matches = roots.flatMap((dir, index) => markdownFiles(path.join(vaultRoot, dir)).flatMap((file) => {
    const text = fs.readFileSync(file, 'utf8');
    // Invalid frontmatter fails closed; ambiguous identity must never select an arbitrary note.
    const metadata = readIntake(text);
    return metadata.intake_id === id ? [{ file, text, metadata, archived: index === 1, relative: path.relative(vaultRoot, file).split(path.sep).join('/') }] : [];
  }));
  if (matches.length !== 1) throw new Error(matches.length ? 'Duplicate intake_id in vault; reconcile the notes first' : 'intake_id not found in Inbox or Processed Inbox');
  return matches[0];
}
async function linkedIssues(api, tag, source) {
  const found = [];
  for (let page = 1;; page++) {
    const batch = await api.rest('GET', `/repos/${api.owner}/${api.repo}/issues?state=all&per_page=100&page=${page}`);
    for (const issue of batch) if (!issue.pull_request && (issue.body?.includes(tag) || (source && issue.body?.includes(source)))) found.push(issue);
    if (batch.length < 100) return found;
  }
}
export async function promoteIntake({ intakeId, workspace = process.cwd(), env = process.env, api, git } = {}) {
  const id = validateIntakeId(intakeId);
  const vaultRoot = path.join(workspace, 'duumbi-vault');
  if (!fs.existsSync(path.join(vaultRoot, 'Duumbi'))) throw new Error('Vault checkout missing');
  const note = findIntake(vaultRoot, id);
  const tag = `<!-- duumbi-intake-stage5:${id} -->`;
  if (!note.archived && note.metadata.intake_status !== 'ready_for_triage') throw new Error(`Note is ${note.metadata.intake_status || 'missing status'}, not ready_for_triage`);
  if (note.archived && note.metadata.intake_status !== 'triaged') throw new Error('Archived note has inconsistent status');
  const matches = await linkedIssues(api, tag, note.archived ? null : note.relative);
  if (matches.length > 1) throw new Error('Multiple GitHub issues reference this intake; reconcile before retry');
  let issue = matches[0];
  if (note.archived) {
    // Scheduled triage uses source-path references rather than the targeted marker.
    const link = (note.text.includes('## Triage result') ? note.text.split('## Triage result').at(-1) : '').match(/https:\/\/github\.com\/([^/]+\/[^/]+)\/issues\/(\d+)/g)?.filter((url) => url.startsWith(`https://github.com/${api.owner}/${api.repo}/issues/`)).at(-1);
    if (!issue && link) issue = await api.rest('GET', `/repos/${api.owner}/${api.repo}/issues/${link.split('/').at(-1)}`);
    if (!issue) throw new Error('Processed note has no verifiable issue link');
    return { status: 'already_processed', intake_id: id, issue_url: issue.html_url, issue_number: issue.number };
  }
  const project = await api.loadProject(env.DUUMBI_PROJECT_OWNER || api.owner, Number(env.DUUMBI_PROJECT_NUMBER), env.DUUMBI_PROJECT_OWNER_TYPE || 'user');
  if (!project) throw new Error('DUUMBI project not found');
  validateProjectStatusConfig(project);
  let item = issue && project.items.nodes.find((i) => i.content?.url === issue.html_url);
  if (issue) {
    const labels = issue.labels.map((l) => typeof l === 'string' ? l : l.name);
    if (issue.state !== 'open' || labels.some((l) => ['accepted', 'tech-spec-approved', 'product-spec-approved', 'needs-clarification'].includes(l))) throw new Error('Linked issue is closed or already downstream; no rollback to Stage 5');
    if (item && !['Todo', HUMAN_ACCEPTANCE_STATUS].includes(statusNameForProjectItem(item))) throw new Error('Linked issue is not Todo or Needs Human Acceptance');
    if (!item && !issue.body.includes(tag)) throw new Error('Existing issue is outside the target project; inspect manually');
  } else {
    const title = (note.text.match(/^#\s+(.+)$/m)?.[1] || path.basename(note.file, '.md')).slice(0, 240);
    const body = `${tag}\n## Developer-selected intake\n\n**Intake ID:** ${id}\n**Source:** ${note.relative}\n\nThis enriched note was explicitly selected for Stage 5 Human Acceptance. It is not accepted for specification or implementation.\n\n${note.text}`;
    if (body.length > 60000) throw new Error('Note is too large for an issue; shorten it without discarding requirements');
    // The marker is included in the initial create: a lost response can be recovered by listing issues.
    issue = await api.createIssue(title, body);
  }
  if (!item) { const itemId = await api.addIssueToProject(project.id, issue.node_id); if (!itemId) throw new Error('Issue could not be added to project'); item = { id: itemId }; }
  await api.updateProjectStatus(project, item.id, HUMAN_ACCEPTANCE_STATUS);
  // Add label after status so the existing Stage 5 notifier sees the completed queue entry.
  await api.addIssueLabel(issue.number, HUMAN_ACCEPTANCE_LABEL);
  const archived = archiveProcessedInboxNotes({ workspace, sourceLinks: [note.relative], issueNumber: issue.number, issueUrl: issue.html_url, decision: { action: 'route_existing_issue', rationale: `Developer selected intake_id ${id}`, source_links: [note.relative] } });
  const commit = commitVaultChanges({ vaultRoot, archivedInboxNotes: archived, ...(git ? { git } : {}) });
  return { status: 'needs_human_acceptance', intake_id: id, issue_url: issue.html_url, issue_number: issue.number, vault_commit: commit };
}
export async function runTargetedIntake({ context, core, env = process.env, workspace = process.cwd(), fetchImpl = fetch }) {
  const raw = context.eventName === 'repository_dispatch' ? context.payload.client_payload : context.payload.inputs;
  let result;
  try {
    const intakeId = validateIntakeId(raw?.intake_id);
    if (!env.GH_PROJECT_PAT || !Number.isInteger(Number(env.DUUMBI_PROJECT_NUMBER)) || Number(env.DUUMBI_PROJECT_NUMBER) < 1) throw new Error('GH_PROJECT_PAT and DUUMBI_PROJECT_NUMBER are required');
    const api = createGithubApi({ fetchImpl, token: env.GH_PROJECT_PAT, ...context.repo });
    result = await promoteIntake({ intakeId, workspace, env, api });
    await core.summary.addHeading('Developer-selected intake → Stage 5').addRaw(JSON.stringify(result, null, 2)).write();
  } catch (error) { result = { status: 'failed', error: error.message }; core.setFailed(error.message); }
  fs.writeFileSync(path.join(workspace, 'targeted-intake-result.json'), JSON.stringify(result));
  return result;
}
