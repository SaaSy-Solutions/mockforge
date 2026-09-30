import type { OverrideRule, PatchOp } from '../../services/api/overrides';

/** Editable form state for one patch operation. `value` is JSON text. */
export interface PatchDraft {
  op: PatchOp['op'];
  path: string;
  value: string;
}

/** Editable form state for one rule. `targets` holds one target per line. */
export interface RuleDraft {
  name: string;
  enabled: boolean;
  targets: string;
  mode: OverrideRule['mode'];
  when: string;
  postTemplating: boolean;
  patch: PatchDraft[];
}

const TARGET_PREFIXES = ['operation:', 'tag:', 'path:', 'regex:'];

export function emptyDraft(): RuleDraft {
  return {
    name: '',
    enabled: true,
    targets: '',
    mode: 'replace',
    when: '',
    postTemplating: false,
    patch: [{ op: 'replace', path: '', value: '' }],
  };
}

export function toDraft(rule: OverrideRule): RuleDraft {
  return {
    name: rule.name ?? '',
    enabled: rule.enabled,
    targets: rule.targets.join('\n'),
    mode: rule.mode,
    when: rule.when ?? '',
    postTemplating: rule.post_templating,
    patch: rule.patch.map((op) => ({
      op: op.op,
      path: op.path,
      value: op.op === 'remove' ? '' : JSON.stringify(op.value, null, 2),
    })),
  };
}

/**
 * Turn a draft into a rule, or explain what is wrong with it. The server
 * validates again; this catches mistakes before a round trip.
 */
export function fromDraft(draft: RuleDraft): { rule: OverrideRule } | { error: string } {
  const targets = draft.targets
    .split('\n')
    .map((t) => t.trim())
    .filter(Boolean);
  if (targets.length === 0) return { error: 'Add at least one target.' };
  const badTarget = targets.find((t) => t !== '*' && !TARGET_PREFIXES.some((p) => t.startsWith(p)));
  if (badTarget) {
    return { error: `Target "${badTarget}" must start with operation:, tag:, path:, or regex:, or be *.` };
  }
  if (draft.patch.length === 0) return { error: 'Add at least one patch operation.' };

  const patch: PatchOp[] = [];
  for (const [index, op] of draft.patch.entries()) {
    const path = op.path.trim();
    if (path !== '' && !path.startsWith('/')) {
      return { error: `Patch ${index + 1}: path must start with "/" (or be empty for the whole body).` };
    }
    if (op.op === 'remove') {
      patch.push({ op: 'remove', path });
      continue;
    }
    try {
      patch.push({ op: op.op, path, value: JSON.parse(op.value) });
    } catch {
      return { error: `Patch ${index + 1}: value is not valid JSON. Quote strings, e.g. "gold".` };
    }
  }

  const rule: OverrideRule = {
    enabled: draft.enabled,
    targets,
    patch,
    mode: draft.mode,
    post_templating: draft.postTemplating,
  };
  if (draft.name.trim()) rule.name = draft.name.trim();
  if (draft.when.trim()) rule.when = draft.when.trim();
  return { rule };
}
