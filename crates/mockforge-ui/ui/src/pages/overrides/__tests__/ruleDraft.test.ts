import { describe, expect, it } from 'vitest';
import type { OverrideRule } from '../../../services/api/overrides';
import { emptyDraft, fromDraft, toDraft } from '../ruleDraft';

const rule: OverrideRule = {
  name: 'VIP tier',
  enabled: true,
  targets: ['operation:getUser', 'tag:Users'],
  patch: [
    { op: 'replace', path: '/tier', value: 'gold' },
    { op: 'remove', path: '/debug' },
  ],
  when: 'header[x-tier]=vip',
  mode: 'merge',
  post_templating: true,
};

describe('rule drafts', () => {
  it('round-trips a rule through the editor state', () => {
    expect(fromDraft(toDraft(rule))).toEqual({ rule });
  });

  it('omits empty optional fields', () => {
    const draft = { ...toDraft(rule), name: '  ', when: '' };
    const result = fromDraft(draft);
    expect('rule' in result && result.rule).toEqual(
      expect.not.objectContaining({ name: expect.anything(), when: expect.anything() }),
    );
  });

  it('explains what is wrong before the server does', () => {
    const base = toDraft(rule);
    expect(fromDraft(emptyDraft())).toEqual({ error: 'Add at least one target.' });
    expect(fromDraft({ ...base, targets: 'users' })).toEqual({
      error: 'Target "users" must start with operation:, tag:, path:, or regex:, or be *.',
    });
    expect(fromDraft({ ...base, patch: [] })).toEqual({ error: 'Add at least one patch operation.' });
    expect(fromDraft({ ...base, patch: [{ op: 'add', path: 'tier', value: '1' }] })).toEqual({
      error: 'Patch 1: path must start with "/" (or be empty for the whole body).',
    });
    expect(fromDraft({ ...base, patch: [{ op: 'add', path: '/tier', value: 'gold' }] })).toEqual({
      error: 'Patch 1: value is not valid JSON. Quote strings, e.g. "gold".',
    });
  });

  it('accepts the wildcard target and a whole-body replace', () => {
    const result = fromDraft({ ...emptyDraft(), targets: '*', patch: [{ op: 'replace', path: '', value: '[]' }] });
    expect(result).toEqual({
      rule: expect.objectContaining({ targets: ['*'], patch: [{ op: 'replace', path: '', value: [] }] }),
    });
  });
});
