#!/usr/bin/env node
/** Publish one captured note using an isolated clone and compare-and-swap checks. */
import fs from "node:fs";
import os from "node:os";
import path from "node:path";
import { execFileSync } from "node:child_process";
import { pathToFileURL } from "node:url";
import { INBOX, readIntake } from "./contract.mjs";

function git(cwd, args) {
  return execFileSync("git", args, { cwd, encoding: "utf8", stdio: ["ignore", "pipe", "pipe"] }).trim();
}

function authorConfig(root, key, fallback) {
  try { return git(root, ["config", "--get", key]) || fallback; }
  catch (error) { if (error.status === 1) return fallback; throw error; }
}

export function syncNote({ vault, note, expectedBlob }) {
  if (!expectedBlob || !/^(absent|[a-f0-9]{40}|[a-f0-9]{64})$/.test(expectedBlob)) {
    throw new Error("Specify --expected-blob absent for new notes, or the remote blob read before editing");
  }
  const root = fs.realpathSync(vault);
  if (git(root, ["rev-parse", "--show-toplevel"]) !== root) throw new Error("--vault must be the repository root");
  if (!note.startsWith(INBOX) || path.posix.normalize(note) !== note || !note.endsWith(".md")) throw new Error("Only a relative Inbox Markdown path can be synchronized");
  const local = fs.realpathSync(path.join(root, note));
  const inbox = fs.realpathSync(path.join(root, INBOX));
  if (!local.startsWith(inbox + path.sep)) throw new Error("Note resolves outside Inbox");
  const text = fs.readFileSync(local, "utf8");
  const metadata = readIntake(text);
  if (!(metadata.intake_status === "captured" || (metadata.intake_status === "needs_clarification" && expectedBlob !== "absent")) || !metadata.intake_id || !metadata.source || !metadata.intake_owner) {
    throw new Error("Sync requires a captured note (or an existing clarification), intake_id, source, and intake_owner");
  }
  const localBlob = execFileSync("git", ["hash-object", "--stdin"], { cwd: root, input: text, encoding: "utf8" }).trim();
  const remote = git(root, ["remote", "get-url", "origin"]);
  const dir = fs.mkdtempSync(path.join(os.tmpdir(), "duumbi-intake-sync-"));
  try {
    git(dir, ["clone", "--quiet", "--no-checkout", "--", remote, "repo"]);
    const repo = path.join(dir, "repo");
    git(repo, ["config", "user.name", authorConfig(root, "user.name", "DUUMBI Intake")]);
    git(repo, ["config", "user.email", authorConfig(root, "user.email", "intake@users.noreply.github.com")]);
    for (let attempt = 0; attempt < 2; attempt++) {
      git(repo, ["fetch", "--quiet", "origin", "main"]);
      git(repo, ["checkout", "--quiet", "--detach", "-f", "FETCH_HEAD"]);
      const remoteFiles = git(repo, ["ls-tree", "-r", "--name-only", "HEAD", "--", INBOX, "Duumbi/05 Archive/Processed Inbox/"]).split("\n").filter(Boolean);
      const remotePath = path.join(repo, note);
      // Reject symlink components in the remote checkout before writing.
      let component = repo;
      for (const part of note.split("/")) {
        component = path.join(component, part);
        let stat;
        try { stat = fs.lstatSync(component); } catch (error) { if (error.code !== "ENOENT") throw error; }
        if (stat?.isSymbolicLink()) throw new Error("Remote note path contains a symlink");
      }
      if (fs.existsSync(remotePath) && fs.readFileSync(remotePath, "utf8") === text) {
        return { status: "synced", commit: git(repo, ["rev-parse", "HEAD"]), note, alreadyPresent: true };
      }
      for (const file of remoteFiles) {
        const contents = git(repo, ["show", `HEAD:${file}`]);
        if (file !== note && readIntake(contents).intake_id === metadata.intake_id) throw new Error("This intake_id already exists under another Inbox filename");
      }
      const treeLine = git(repo, ["ls-tree", "HEAD", "--", note]);
      const blob = treeLine ? treeLine.split(/\s+/)[2] : "absent";
      if (blob !== "absent" && readIntake(fs.readFileSync(remotePath, "utf8")).intake_id !== metadata.intake_id) throw new Error("Do not replace the identity of an existing note");
      if (blob !== expectedBlob) throw new Error("Remote note changed; keep local draft and reconcile it before retrying");
      fs.mkdirSync(path.dirname(remotePath), { recursive: true });
      fs.writeFileSync(remotePath, text);
      git(repo, ["add", "--", note]);
      git(repo, ["commit", "--quiet", "-m", "📝 docs: capture intake note"]);
      const commit = git(repo, ["rev-parse", "HEAD"]);
      try {
        git(repo, ["push", "origin", "HEAD:main"]);
      } catch {
        // Refresh once: an unrelated push may be rebased; same-note edits fail above.
        if (attempt === 0) continue;
        // Resolve an uncertain second push by checking the remote content.
      }
      git(repo, ["fetch", "--quiet", "origin", "main"]);
      if (git(repo, ["rev-parse", `FETCH_HEAD:${note}`]) !== localBlob) throw new Error("Remote content verification failed; local draft retained");
      return { status: "synced", commit, note, alreadyPresent: false };
    }
    throw new Error("Push not verified; local draft retained");
  } finally {
    fs.rmSync(dir, { recursive: true, force: true });
  }
}

if (import.meta.url === pathToFileURL(process.argv[1] || "").href) {
  const args = Object.fromEntries(Array.from({ length: Math.floor((process.argv.length - 2) / 2) }, (_, i) => [process.argv[2 + i * 2], process.argv[3 + i * 2]]));
  try {
    console.log(JSON.stringify(syncNote({ vault: args["--vault"], note: args["--note"], expectedBlob: args["--expected-blob"] })));
  } catch (error) {
    // Git stderr can contain remote credentials; show only controlled errors.
    console.error(error.status !== undefined ? "Git synchronization failed; local note retained. Inspect repository access and remote changes." : error.message);
    process.exitCode = 1;
  }
}
