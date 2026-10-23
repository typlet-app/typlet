// SPDX-License-Identifier: GPL-3.0-or-later
// Trigger problems for the snippet being edited, computed live while typing.
// Same rules as `find_conflicts` in typlet-core (matcher.rs), which reports
// them for the whole library.

export type TriggerIssue =
  | { kind: "empty" }
  | { kind: "spaces" }
  | { kind: "too_long" }
  | { kind: "symbols_only" }
  | { kind: "duplicate"; other: string }
  | { kind: "shadowed_by"; other: string }
  | { kind: "shadows"; other: string };

/** `true` if typing `long` passes a point where `short` already expands. */
function fires_early(short: string, long: string): boolean {
  if (short.length >= long.length) return false;
  for (let end = short.length; end < long.length; end++) {
    if (long.slice(0, end).endsWith(short)) return true;
  }
  return false;
}

/**
 * Checks `trigger` against the rules and the other enabled triggers.
 * `others` maps trigger → label of the other snippets.
 */
export function triggerIssues(trigger: string, others: Map<string, string>): TriggerIssue[] {
  if (trigger === "") return [{ kind: "empty" }];
  if (/\s/.test(trigger)) return [{ kind: "spaces" }];
  if ([...trigger].length > 64) return [{ kind: "too_long" }];
  if (!/[\p{L}\p{N}]/u.test(trigger)) return [{ kind: "symbols_only" }];
  const issues: TriggerIssue[] = [];
  for (const [other, label] of others) {
    if (other === trigger) issues.push({ kind: "duplicate", other: label });
    else if (fires_early(other, trigger)) issues.push({ kind: "shadowed_by", other });
    else if (fires_early(trigger, other)) issues.push({ kind: "shadows", other });
  }
  return issues;
}

/** Issues that prevent saving. */
export function blocksSaving(issue: TriggerIssue): boolean {
  return issue.kind === "empty" || issue.kind === "spaces" || issue.kind === "too_long";
}
