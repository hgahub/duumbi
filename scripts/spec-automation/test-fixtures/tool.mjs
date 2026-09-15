// Fake external tools for the worker's subprocess integration tests. Never used by production.
import fs from 'node:fs';
import path from 'node:path';
const file = process.env.SPEC_TEST_DB;
if (!file) throw new Error('Test fixture requires SPEC_TEST_DB');
const db = JSON.parse(fs.readFileSync(file));
const args = process.argv.slice(2), bin = path.basename(process.argv[1]);
const input = fs.readFileSync(0, 'utf8');
const save = () => fs.writeFileSync(file, JSON.stringify(db));
const out = (v) => process.stdout.write(typeof v === 'string' ? v : JSON.stringify(v));
db.operations.push({ bin, args });
if (bin === 'codex') {
  if (args[0] === 'login') out('Logged in using ChatGPT');
  else if (args.includes('--help')) out('--ignore-user-config --ignore-rules --output-schema --ephemeral');
  else {
    const stage = Number(input.match(/Stage: (\d+)/)[1]);
    db.models.push({ stage, model: args[args.indexOf('--model') + 1] });
    if (db.failModel) { save(); process.stderr.write('subscription quota exhausted'); process.exit(1); }
    const product = { outcome: 'ready', question: '', complexity: 'normal', rationale: 'One unit', product: '# Product', decomposition: '# One unit', units: [{ key: 'core', title: 'Core', product: '# Product', dependencies: [] }] };
    const technical = { outcome: 'ready', question: '', technical: '# Technical', units: [{ key: 'core', technical: '# Technical' }] };
    if (db.split) {
      product.units.push({ key: 'ui', title: 'UI', product: '# UI product', dependencies: ['core'] });
      technical.units.push({ key: 'ui', technical: '# UI technical' });
    }
    const result = stage === 0 ? { route: db.route || 'autonomous', rationale: 'Assessed accepted scope', question: db.route ? 'Which behavior?' : '' } : stage === 6 ? product : stage === 8 ? technical : { decision: 'approve', rationale: 'Verified', findings: [], question: '' };
    fs.writeFileSync(args[args.indexOf('--output-last-message') + 1], JSON.stringify(result)); out('{"type":"turn.completed","usage":{"input_tokens":10,"output_tokens":10}}\n');
  }
} else if (bin === 'git') {
  if (args[0] === 'clone') fs.mkdirSync(args.at(-1), { recursive: true });
  if (args[0] === 'rev-parse') out(process.cwd().endsWith('/publish') ? 'headsha\n' : 'basesha\n');
  if (args[0] === 'push') {
    db.remoteHead = 'headsha';
    if (db.failPushOnce) { db.failPushOnce = false; save(); process.stderr.write('lost push response'); process.exit(1); }
  }
} else if (bin === 'gh') {
  if (args.includes('--slurp')) { process.stderr.write('unknown flag: --slurp'); process.exit(1); }
  if (args.includes('--paginate') && db.failPagination) { save(); process.stderr.write('pagination failure before job checkpoint'); process.exit(1); }
  if (args[0] === 'auth') out('Authenticated');
  else if (args.includes('--include') && args.includes('user')) out(`HTTP/2 200 OK\r\nX-OAuth-Scopes: ${db.noProjectScope ? 'repo, read:project' : 'repo, project'}\r\n\r\n{}`);
  else {
    const route = args.find((a) => a === 'graphql' || a.startsWith('repos/'));
    const body = input ? JSON.parse(input) : undefined;
    const apiPath = route.replace('repos/hgahub/duumbi', '').split('?')[0];
    let result;
    const number = Number(apiPath.match(/^\/issues\/(\d+)/)?.[1]);
    const target = number === 123 ? db.issue : db.children?.find((c) => c.number === number);
    if (route === 'graphql') {
      if (body.query.includes('reviewThreads')) result = { data: { repository: { pullRequest: { reviewThreads: { nodes: db.unresolved ? [{ isResolved: false }] : [], pageInfo: { hasNextPage: false } } } } } };
      else if (body.query.startsWith('query')) result = { data: { user: { projectV2: { id: 'project', viewerCanUpdate: !db.readOnlyProject, fields: { nodes: [{ id: 'status', name: 'Status', options: ['Spec Needed', 'Technical Spec Needed', 'Technical Spec Review', 'Needs Clarification', 'Ready for Build', 'In Progress'].map((name) => ({ name, id: name })) }] } } }, repository: { issue: { id: 'issue', projectItems: { nodes: [{ id: 'item', project: { id: 'project' } }] } } } } };
      else { db.status = body.variables.o; result = { data: {} }; }
    } else if (apiPath.startsWith('/collaborators/')) result = { permission: 'write' };
    else if (apiPath === '') result = { full_name: 'hgahub/duumbi' };
    else if (apiPath === '/issues') {
      db.children ??= [];
      if (body) db.children.push({ id: 200 + db.children.length, number: 200 + db.children.length, title: body.title, body: body.body, labels: [], state: 'open' });
      result = body ? db.children.at(-1) : [db.issue, ...db.children];
    }
    else if (/^\/issues\/\d+$/.test(apiPath)) result = target;
    else if (/^\/issues\/\d+\/comments$/.test(apiPath)) {
      db.childComments ??= {};
      const comments = number === 123 ? db.comments : (db.childComments[number] ??= []);
      if (body) comments.push({ id: comments.length + 100, body: body.body }); result = body ? comments.at(-1) : comments;
    }
    else if (apiPath === '/labels') { if (body) db.labels.push(body); result = body || db.labels; }
    else if (/^\/issues\/\d+\/labels$/.test(apiPath)) { if (body) for (const name of body.labels) if (!target.labels.some((l) => l.name === name)) target.labels.push({ name }); result = target.labels; }
    else if (/^\/issues\/\d+\/labels\//.test(apiPath)) { target.labels = target.labels.filter((l) => l.name !== decodeURIComponent(apiPath.split('/').at(-1))); result = null; }
    else if (apiPath === '/issues/123/sub_issues') { db.linked ??= []; if (body) db.linked.push(db.children.find((c) => c.id === body.sub_issue_id)); result = db.linked; }
    else if (apiPath.startsWith('/git/matching-refs/')) result = [];
    else if (apiPath === '/git/refs') { db.claim = body.ref; result = {}; }
    else if (apiPath.startsWith('/compare/')) result = { status: 'ahead', files: db.sourceDrift ? [{ filename: 'src/main.rs' }] : [] };
    else if (apiPath === '/commits/main') result = { sha: 'basesha' };
    else if (apiPath === '/pulls') {
      if (body) db.pr = { number: 500, html_url: 'https://github.com/hgahub/duumbi/pull/500', head: { sha: 'headsha' }, base: { ref: 'main', repo: { full_name: 'hgahub/duumbi' } }, merged: false, merge_commit_sha: 'mergesha' };
      result = body ? db.pr : db.pr ? [db.pr] : [];
    } else if (apiPath === '/pulls/500') result = db.pr;
    else if (apiPath === '/pulls/500/files') {
      const job = JSON.parse(fs.readFileSync(path.join(process.env.DUUMBI_SPEC_STATE, 'v1-123-42/job.json')));
      result = Object.keys(job.files).map((filename) => ({ filename, status: 'added' }));
      if (db.extraFile) result.push({ filename: 'src/main.rs', status: 'modified' });
    } else if (apiPath === '/pulls/500/reviews') result = db.badReview ? [{ id: 1, user: { login: 'reviewer' }, state: 'CHANGES_REQUESTED' }] : [];
    else if (apiPath.endsWith('/check-runs')) result = { check_runs: [{ id: 1, app: { id: 1 }, name: 'CI', status: 'completed', conclusion: db.badCI ? 'failure' : 'success' }] };
    else if (apiPath.endsWith('/statuses')) result = [];
    else if (apiPath.startsWith('/contents/')) result = { content: Buffer.from(db.badArtifact ? 'changed' : fs.readFileSync(path.join(process.env.DUUMBI_SPEC_STATE, 'v1-123-42/publish', apiPath.slice('/contents/'.length)))).toString('base64') };
    else throw new Error(`Unhandled fake API: ${route}`);
    out(args.includes('--paginate') ? `${JSON.stringify(result)}\n` : result === null ? '' : result);
  }
} else throw new Error(`Unexpected test binary: ${bin}`);
save();
