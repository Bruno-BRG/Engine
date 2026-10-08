// Rule for dropping a Project asset on a resource slot of the Inspector (INS-003). A slot takes only the asset
// type it names, and the drop only records the asset UID. The file on disk is never touched.
import type { AssetKind } from './state/assets.svelte';

/** Resource slots of the Inspector that accept an asset dropped from the Project. */
export type SlotKind = 'texture' | 'audio';

export type SlotDrop = { ok: true; uid: string } | { ok: false; message: string };

/** Noun phrases with their article, used in the refusal messages (Portuguese UI text). */
const KIND_NOUN: Record<AssetKind, string> = {
  folder: 'uma pasta',
  texture: 'uma textura',
  audio: 'um áudio',
  font: 'uma fonte',
  scene: 'uma cena',
  script: 'um script',
  other: 'um arquivo',
};

const SLOT_TEXT: Record<SlotKind, { field: string; accepts: string }> = {
  texture: { field: 'de textura', accepts: 'texturas' },
  audio: { field: 'de áudio', accepts: 'áudios' },
};

/**
 * Says whether the Project item `asset` can fill a slot of kind `slot`. `uid` is null for an item the engine
 * has not imported yet. Returns the UID to record, or the message to show to the user.
 */
export function checkSlotDrop(slot: SlotKind, asset: { name: string; kind: AssetKind; uid: string | null }): SlotDrop {
  if (asset.kind !== slot) {
    const text = SLOT_TEXT[slot];
    const noun = KIND_NOUN[asset.kind] ?? 'um arquivo';
    return { ok: false, message: `“${asset.name}” é ${noun}. O campo ${text.field} aceita só ${text.accepts}.` };
  }
  if (asset.uid === null) {
    return {
      ok: false,
      message: `“${asset.name}” ainda não tem identificador. Atualize a lista no painel Projeto e tente de novo.`,
    };
  }
  return { ok: true, uid: asset.uid };
}
