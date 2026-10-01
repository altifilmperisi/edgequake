/**
 * @implements SPEC-102 / FEAT-102 — resolver + hex contract
 */
import { describe, expect, it } from 'bun:test';
import {
  ENTITY_TYPE_COLORS,
  MAX_ENTITY_TYPE_COLORS,
  canonicalizeEntityTypeHex,
  getEntityTypeColor,
  isValidEntityTypeHex,
  mergeEntityTypeColorMap,
  normalizeEntityTypeKey,
  resolveEntityTypeColor,
  stripDefaultOverrides,
} from './entity-type-colors';

describe('normalizeEntityTypeKey', () => {
  it('uppercases and replaces spaces/hyphens', () => {
    expect(normalizeEntityTypeKey('person')).toBe('PERSON');
    expect(normalizeEntityTypeKey(' natural-object ')).toBe('NATURAL_OBJECT');
  });
});

describe('isValidEntityTypeHex / canonicalizeEntityTypeHex', () => {
  it('accepts #RGB and #RRGGBB', () => {
    expect(isValidEntityTypeHex('#0f0')).toBe(true);
    expect(isValidEntityTypeHex('#00FF00')).toBe(true);
    expect(canonicalizeEntityTypeHex('#0f0')).toBe('#00ff00');
    expect(canonicalizeEntityTypeHex('#AABBCC')).toBe('#aabbcc');
  });

  it('rejects invalid hex', () => {
    expect(isValidEntityTypeHex('#gg0000')).toBe(false);
    expect(isValidEntityTypeHex('')).toBe(false);
    expect(isValidEntityTypeHex('#12345')).toBe(false);
    expect(isValidEntityTypeHex('red')).toBe(false);
    expect(canonicalizeEntityTypeHex('#gg0000')).toBeNull();
  });
});

describe('resolveEntityTypeColor', () => {
  it('override > default > DEFAULT', () => {
    expect(
      resolveEntityTypeColor('PERSON', { PERSON: '#112233' }),
    ).toBe('#112233');
    expect(resolveEntityTypeColor('PERSON')).toBe(ENTITY_TYPE_COLORS.PERSON);
    expect(resolveEntityTypeColor('ZZZ_UNKNOWN')).toBe(
      ENTITY_TYPE_COLORS.DEFAULT,
    );
    expect(resolveEntityTypeColor(undefined)).toBe(ENTITY_TYPE_COLORS.DEFAULT);
  });

  it('normalizes override keys and expands shorthand', () => {
    expect(resolveEntityTypeColor('person', { person: '#0f0' })).toBe(
      '#00ff00',
    );
  });

  it('covers Rust default_entity_types beyond legacy palette', () => {
    for (const t of [
      'CREATURE',
      'METHOD',
      'CONTENT',
      'DATA',
      'ARTIFACT',
      'NATURALOBJECT',
      'OTHER',
    ]) {
      expect(resolveEntityTypeColor(t)).not.toBe(ENTITY_TYPE_COLORS.DEFAULT);
      expect(resolveEntityTypeColor(t)).toBe(ENTITY_TYPE_COLORS[t]);
    }
  });

  it('getEntityTypeColor aliases resolve without overrides', () => {
    expect(getEntityTypeColor('drawing')).toBe(ENTITY_TYPE_COLORS.DRAWING);
  });
});

describe('resolveEntityTypeShape (SPEC-155 colour-blind)', () => {
  it('returns stable known shapes', async () => {
    const { resolveEntityTypeShape } = await import('./entity-type-colors');
    expect(resolveEntityTypeShape('PERSON')).toBe('circle');
    expect(resolveEntityTypeShape('ORGANIZATION')).toBe('square');
    expect(resolveEntityTypeShape('LOCATION')).toBe('diamond');
  });

  it('hashes unknown types into a distinct shape (not always circle)', async () => {
    const { resolveEntityTypeShape } = await import('./entity-type-colors');
    const a = resolveEntityTypeShape('ZZZ_UNKNOWN_ALPHA');
    const b = resolveEntityTypeShape('ZZZ_UNKNOWN_BETA');
    expect(a).toBeTruthy();
    expect(b).toBeTruthy();
    // Same key is stable
    expect(resolveEntityTypeShape('ZZZ_UNKNOWN_ALPHA')).toBe(a);
  });
});

describe('mergeEntityTypeColorMap / stripDefaultOverrides', () => {
  it('merges, caps, and strips defaults', () => {
    const merged = mergeEntityTypeColorMap({
      person: '#Abc',
      INVALID: 'nope',
      DEFAULT: '#000000',
    });
    expect(merged).toEqual({ PERSON: '#aabbcc' });

    const stripped = stripDefaultOverrides({
      PERSON: ENTITY_TYPE_COLORS.PERSON,
      MACHINE: '#112233',
    });
    expect(stripped).toEqual({ MACHINE: '#112233' });
  });

  it('respects max fifty entries', () => {
    const input: Record<string, string> = {};
    for (let i = 0; i < 60; i++) {
      input[`TYPE_${i}`] = '#abcdef';
    }
    expect(Object.keys(mergeEntityTypeColorMap(input)).length).toBe(
      MAX_ENTITY_TYPE_COLORS,
    );
  });
});
