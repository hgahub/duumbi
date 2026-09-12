import test from 'node:test';
import assert from 'node:assert/strict';
import fs from 'node:fs';
import os from 'node:os';
import path from 'node:path';
import { execFileSync } from 'node:child_process';
import { capturedChanges, notifyCaptured } from './vault-listener.mjs';
import { INBOX, withIntake } from './contract.mjs';

test('listener wakes on new capture and clarification, but not its own enrichment or deletion', () => {
  const root = fs.mkdtempSync(path.join(os.tmpdir(), 'vault-listener-'));
  const git = (...args) => execFileSync('git', args, { cwd: root, encoding: 'utf8', stdio: ['ignore','pipe','pipe'] }).trim();
  try {
    git('init'); git('config','user.name','Test'); git('config','user.email','test@example.com');
    fs.writeFileSync(path.join(root,'README.md'),'Vault');git('add','.');git('commit','-m','init');
    let before = git('rev-parse','HEAD');
    const note = INBOX + 'One.md'; fs.mkdirSync(path.join(root,INBOX),{recursive:true});
    const save = (status) => { fs.writeFileSync(path.join(root,note),withIntake('# Idea',{intake_status:status}));git('add','.');git('commit','-m',status); };
    save('captured');assert.deepEqual(capturedChanges({root,before}),[note]);
    before=git('rev-parse','HEAD');save('needs_clarification');assert.deepEqual(capturedChanges({root,before}),[]);
    before=git('rev-parse','HEAD');save('captured');assert.deepEqual(capturedChanges({root,before}),[note]);
    before=git('rev-parse','HEAD');save('ready_for_triage');assert.deepEqual(capturedChanges({root,before}),[]);
    before=git('rev-parse','HEAD');git('rm',note);git('commit','-m','archive');assert.deepEqual(capturedChanges({root,before}),[]);
  } finally { fs.rmSync(root,{recursive:true,force:true}); }
});

test('dispatch sends only a wake-up event and needs credentials only for real candidates', async () => {
  let calls=0;
  const fetchImpl=async (url,opts) => {calls++;assert.equal(url,'https://api.github.com/repos/hgahub/duumbi/dispatches');assert.deepEqual(JSON.parse(opts.body),{event_type:'duumbi-inbox-captured'});return {status:204};};
  assert.equal((await notifyCaptured({count:0,fetchImpl})).dispatched,false);
  assert.equal((await notifyCaptured({count:1,dryRun:true,fetchImpl})).dispatched,false);
  await assert.rejects(notifyCaptured({count:1,fetchImpl}),/TOKEN/);
  assert.equal((await notifyCaptured({count:6,token:'test',fetchImpl})).dispatched,true);
  assert.equal(calls,1);
  await assert.rejects(notifyCaptured({count:1,token:'test',fetchImpl:async()=>({status:403})}),/HTTP 403/);
});
