// Miniaturas do painel: os bytes do arquivo viram um blob URL, cacheado por caminho.
// O <img> decodifica a imagem. O cache é descartado quando o arquivo muda no disco (AST-004).
import { assets } from '$lib/state/assets.svelte';

const MIME: Record<string, string> = {
  png: 'image/png',
  jpg: 'image/jpeg',
  jpeg: 'image/jpeg',
  webp: 'image/webp',
  gif: 'image/gif',
  bmp: 'image/bmp',
};

const cache = new Map<string, Promise<string | null>>();

/** Versão por caminho: muda quando o arquivo muda, e a miniatura é pedida de novo. */
export const thumbGen = $state<Record<string, number>>({});

function forget(rel: string): void {
  const url = cache.get(rel);
  cache.delete(rel);
  url?.then((u) => {
    if (u) URL.revokeObjectURL(u);
  });
}

/** URL da miniatura do arquivo `rel`, ou null se não der para ler. */
export function thumbUrl(rel: string): Promise<string | null> {
  let url = cache.get(rel);
  if (!url) {
    const ext = rel.slice(rel.lastIndexOf('.') + 1).toLowerCase();
    url = assets
      .readBytes(rel)
      .then((bytes) => URL.createObjectURL(new Blob([bytes], { type: MIME[ext] ?? '' })))
      .catch(() => null);
    cache.set(rel, url);
  }
  return url;
}

assets.subscribe((ev) => {
  if (ev.type === 'project') {
    for (const rel of [...cache.keys()]) forget(rel);
    return;
  }
  for (const rel of ev.paths) {
    forget(rel);
    thumbGen[rel] = (thumbGen[rel] ?? 0) + 1;
  }
});
