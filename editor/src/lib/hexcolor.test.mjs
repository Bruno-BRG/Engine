// Tests of the hex color check of the Inspector color field (INS-003). Run from editor/ with:
//   node --test src/lib/hexcolor.test.mjs
// Node 24 loads the .ts module directly (type stripping), so there is no build step and no dependency.
import assert from 'node:assert/strict';
import { test } from 'node:test';
import { hexColorOrNull } from './hexcolor.ts';

test('accepts #rgb, #rrggbb and #rrggbbaa, in either case', () => {
  assert.equal(hexColorOrNull('#fff'), '#fff');
  assert.equal(hexColorOrNull('#4F8FD6'), '#4F8FD6');
  assert.equal(hexColorOrNull('#4f8fd680'), '#4f8fd680');
});

test('trims the spaces around the text', () => {
  assert.equal(hexColorOrNull('  #abc  '), '#abc');
});

test('rejects color names and text without the hash', () => {
  assert.equal(hexColorOrNull('red'), null);
  assert.equal(hexColorOrNull('4f8fd6'), null);
});

test('rejects the wrong number of digits', () => {
  assert.equal(hexColorOrNull('#'), null);
  assert.equal(hexColorOrNull('#ff'), null);
  assert.equal(hexColorOrNull('#fffff'), null);
  assert.equal(hexColorOrNull('#fffffff'), null);
  assert.equal(hexColorOrNull('#fffffffff'), null);
});

test('rejects characters that are not hex digits, and inner spaces', () => {
  assert.equal(hexColorOrNull('#ggg'), null);
  assert.equal(hexColorOrNull('#12345g'), null);
  assert.equal(hexColorOrNull('#ff ff'), null);
});

test('rejects empty and blank text', () => {
  assert.equal(hexColorOrNull(''), null);
  assert.equal(hexColorOrNull('   '), null);
});
