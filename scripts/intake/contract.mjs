/** Shared, flat YAML frontmatter contract for DUUMBI intake notes. */
export const INBOX = "Duumbi/00 Inbox (ToProcess)/";
export const STATUSES = new Set(["captured", "needs_clarification", "ready_for_triage", "triaged"]);
export const FIELDS = new Set([
  "intake_id", "intake_status", "source", "intake_owner", "intake_owner_slack_id",
  "intake_updated_at", "enrichment_result", "enriched_at",
]);

export function frontmatter(text) {
  const value = String(text).replace(/\r\n/g, "\n");
  if (!value.startsWith("---\n")) return { lines: [], body: value };
  const end = value.indexOf("\n---\n", 3);
  if (end < 0) throw new Error("Unterminated intake frontmatter");
  return { lines: value.slice(4, end).split("\n"), body: value.slice(end + 5) };
}

function scalar(value) {
  if (value.startsWith('"')) return JSON.parse(value);
  if (value.startsWith("'")) {
    if (!value.endsWith("'")) throw new Error("Invalid quoted intake field");
    return value.slice(1, -1).replace(/''/g, "'");
  }
  if (!/^[a-zA-Z0-9_@. /:-]+$/.test(value)) throw new Error("Intake fields must be flat string scalars");
  return value.trim();
}

export function readIntake(text) {
  const { lines } = frontmatter(text);
  const result = {};
  for (const line of lines) {
    const match = /^([a-z_]+):\s*(.*?)\s*$/.exec(line);
    if (!match || !FIELDS.has(match[1])) continue;
    if (Object.hasOwn(result, match[1])) throw new Error(`Duplicate intake field: ${match[1]}`);
    result[match[1]] = scalar(match[2]);
    if (typeof result[match[1]] !== "string") throw new Error("Intake fields must be strings");
  }
  if (result.intake_status && !STATUSES.has(result.intake_status)) throw new Error("Unknown intake_status");
  return result;
}

export function withIntake(text, updates) {
  readIntake(text);
  const { lines, body } = frontmatter(text);
  for (const [key, value] of Object.entries(updates)) {
    if (!FIELDS.has(key) || typeof value !== "string") throw new Error(`Invalid intake update: ${key}`);
  }
  const kept = lines.filter((line) => !Object.keys(updates).some((key) => line.startsWith(`${key}:`)));
  const next = [...kept, ...Object.entries(updates).map(([k, v]) => `${k}: ${JSON.stringify(v)}`)];
  const output = `---\n${next.join("\n")}\n---\n${body}`;
  readIntake(output);
  return output;
}

export function hasStatus(text, status) {
  try { return readIntake(text).intake_status === status; } catch { return false; }
}

/** Bound model context without dropping clarification answers appended at the end. */
export function intakeExcerpt(text, maxLength = 9000) {
  const value = String(text);
  if (value.length <= maxLength) return value;
  const marker = "\n\n[Middle omitted for context budget; inspect the source note if needed.]\n\n";
  const side = Math.floor((maxLength - marker.length) / 2);
  return value.slice(0, side) + marker + value.slice(-side);
}

export const ENRICHMENT_START = "<!-- duumbi-enrichment:start -->";
export const ENRICHMENT_END = "<!-- duumbi-enrichment:end -->";

/** Replace only the generated block; preserve original capture and clarification answers. */
export function replaceEnrichment(text, block) {
  const start = text.indexOf(ENRICHMENT_START);
  const end = text.indexOf(ENRICHMENT_END);
  if ((start < 0) !== (end < 0) || (start >= 0 && end < start)) throw new Error("Broken enrichment block");
  const generated = `${ENRICHMENT_START}\n${block}\n${ENRICHMENT_END}`;
  return start < 0
    ? `${text.trimEnd()}\n\n${generated}\n`
    : `${text.slice(0, start)}${generated}${text.slice(end + ENRICHMENT_END.length)}`;
}
