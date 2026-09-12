/** Vault-side wake-up: inspect changed notes; dispatch metadata only, never note content. */
import fs from 'node:fs';
import path from 'node:path';
import { execFileSync } from 'node:child_process';
import { pathToFileURL } from 'node:url';
import { hasStatus, INBOX } from './contract.mjs';

export function capturedChanges({ root, before }) {
  const git = (args) => execFileSync('git', args, { cwd: root, encoding: 'utf8' });
  if (!/^[a-f0-9]{40,64}$/.test(before || '')) throw new Error('Invalid base SHA');
  const args = /^0+$/.test(before) ? ['ls-files', '-z', '--', INBOX]
    : ['diff', '--name-only', '-z', '--diff-filter=AMR', before, 'HEAD', '--', INBOX];
  return git(args).split('\0').filter(Boolean).filter((file) => {
    if (!file.startsWith(INBOX) || !file.endsWith('.md')) return false;
    const absolute = path.join(root, file);
    if (!fs.existsSync(absolute) || fs.lstatSync(absolute).isSymbolicLink()) return false;
    if (!fs.realpathSync(absolute).startsWith(fs.realpathSync(root) + path.sep + INBOX)) return false;
    return hasStatus(fs.readFileSync(absolute, 'utf8'), 'captured');
  });
}

export async function notifyCaptured({ count, token, target = 'hgahub/duumbi', fetchImpl = fetch, dryRun = false }) {
  if (!count || dryRun) return { dispatched: false, candidates: count, dryRun };
  if (!token) throw new Error('Set DUUMBI_INTAKE_DISPATCH_TOKEN in the vault repository');
  if (!/^[A-Za-z0-9_.-]+\/[A-Za-z0-9_.-]+$/.test(target)) throw new Error('Invalid target repository');
  const response = await fetchImpl(`https://api.github.com/repos/${target}/dispatches`, {
    method: 'POST', signal: AbortSignal.timeout(30000), headers: { Authorization: `Bearer ${token}`, Accept: 'application/vnd.github+json', 'Content-Type': 'application/json', 'X-GitHub-Api-Version': '2022-11-28' },
    body: JSON.stringify({ event_type: 'duumbi-inbox-captured' }),
  });
  if (response.status !== 204) throw new Error(`Intake dispatch failed (HTTP ${response.status}); hourly sweep remains available`);
  return { dispatched: true, candidates: count };
}

if (import.meta.url === pathToFileURL(process.argv[1] || '').href) {
  try {
    const count = capturedChanges({ root: process.cwd(), before: process.env.INTAKE_BEFORE_SHA }).length;
    console.log(JSON.stringify(await notifyCaptured({ count, token: process.env.DUUMBI_INTAKE_DISPATCH_TOKEN, target: process.env.DUUMBI_SOURCE_REPOSITORY || 'hgahub/duumbi', dryRun: process.env.INTAKE_DRY_RUN === 'true' })));
  } catch (error) {
    console.error(error.status !== undefined ? 'Cannot inspect vault changes; see checkout/base configuration' : error.message);
    process.exitCode = 1;
  }
}
