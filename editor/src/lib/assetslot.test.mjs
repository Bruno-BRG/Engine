// Tests of the rule for dropping a Project asset on an Inspector resource slot (INS-003). Run from editor/ with:
//   node --test src/lib/assetslot.test.mjs
// Node 24 loads the .ts module directly (type stripping), so there is no build step and no dependency.
import assert from 'node:assert/strict';
import { test } from 'node:test';
import { checkSlotDrop } from './assetslot.ts';

test('a texture with a UID fills a texture slot, and the UID is what gets recorded', () => {
  assert.deepEqual(checkSlotDrop('texture', { name: 'hero.png', kind: 'texture', uid: 'a1' }), { ok: true, uid: 'a1' });
});

test('an audio with a UID fills an audio slot', () => {
  assert.deepEqual(checkSlotDrop('audio', { name: 'pulo.ogg', kind: 'audio', uid: 'b2' }), { ok: true, uid: 'b2' });
});

test('an audio dropped on a texture slot is refused, in Portuguese', () => {
  assert.deepEqual(checkSlotDrop('texture', { name: 'pulo.ogg', kind: 'audio', uid: 'b2' }), {
    ok: false,
    message: '“pulo.ogg” é um áudio. O campo de textura aceita só texturas.',
  });
});

test('a texture dropped on an audio slot is refused, in Portuguese', () => {
  assert.deepEqual(checkSlotDrop('audio', { name: 'hero.png', kind: 'texture', uid: 'a1' }), {
    ok: false,
    message: '“hero.png” é uma textura. O campo de áudio aceita só áudios.',
  });
});

test('folders, scenes, scripts, fonts and other files are refused by both slots', () => {
  const cases = [
    ['folder', 'uma pasta'],
    ['scene', 'uma cena'],
    ['script', 'um script'],
    ['font', 'uma fonte'],
    ['other', 'um arquivo'],
  ];
  for (const [kind, noun] of cases) {
    for (const slot of ['texture', 'audio']) {
      const r = checkSlotDrop(slot, { name: 'x', kind, uid: null });
      assert.equal(r.ok, false, `${kind} into ${slot}`);
      assert.match(r.message, new RegExp(`é ${noun}\.`), `${kind} into ${slot}`);
    }
  }
});

test('an asset without UID is refused even when its type fits', () => {
  const r = checkSlotDrop('texture', { name: 'novo.png', kind: 'texture', uid: null });
  assert.equal(r.ok, false);
  assert.match(r.message, /^“novo\.png” ainda não tem identificador\./);
});
