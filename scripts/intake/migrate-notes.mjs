#!/usr/bin/env node
/** Explicit migration: preview by default; apply changes only with --apply. */
import fs from "node:fs";
import path from "node:path";
import { randomUUID } from "node:crypto";
import { pathToFileURL } from "node:url";
import { INBOX, readIntake, withIntake } from "./contract.mjs";

export function migrateNotes({ vault, owner, apply = false }) {
  if (!vault) throw new Error("A vault repository root is required");
  if (!owner) throw new Error("An Inbox owner is required");
  const root = path.join(vault, INBOX);
  const results = [];
  function visit(dir) {
    for (const entry of fs.readdirSync(dir, { withFileTypes: true })) {
      const file = path.join(dir, entry.name);
      if (entry.isDirectory()) { visit(file); continue; }
      if (!entry.isFile() || !entry.name.endsWith(".md")) continue;
      const text = fs.readFileSync(file, "utf8");
      try {
        if (readIntake(text).intake_status) continue;
        const processed = /duumbi-inbox-enrichment:v1|duumbi\/status\/processed/.test(text);
        const legacy = [...text.matchAll(/^- Status:\s*(.+)$/gm)].at(-1)?.[1]?.trim().replace(/ /g, "_");
        const known = ["ready_for_triage", "duplicate_candidate", "no_action_candidate", "needs_clarification"];
        if (processed && !known.includes(legacy)) {
          results.push({ path: path.relative(vault, file), status: "review_required", applied: false });
          continue;
        }
        const status = !processed ? "captured" : legacy === "needs_clarification" ? legacy : "ready_for_triage";
        const metadata = readIntake(text);
        const updates = {
          intake_id: metadata.intake_id || randomUUID(), intake_status: status,
          source: metadata.source || (/^- (Source|Surface): Codex\s*$/m.test(text) ? "codex" : "obsidian"),
          intake_owner: metadata.intake_owner || owner, intake_updated_at: new Date().toISOString(),
        };
        if (processed) updates.enrichment_result = legacy;
        if (apply) fs.writeFileSync(file, withIntake(text, updates));
        results.push({ path: path.relative(vault, file), status, applied: apply });
      } catch {
        results.push({ path: path.relative(vault, file), status: "review_required", applied: false });
      }
    }
  }
  visit(root);
  return results;
}
if (import.meta.url === pathToFileURL(process.argv[1] || "").href) {
  const args = process.argv.slice(2);
  const option = (key) => args.includes(key) ? args[args.indexOf(key) + 1] : undefined;
  try {
    console.log(JSON.stringify(migrateNotes({ vault: option("--vault"), owner: option("--owner"), apply: args.includes("--apply") }), null, 2));
  } catch (error) { console.error(error.message); process.exitCode = 1; }
}
