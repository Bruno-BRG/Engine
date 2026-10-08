// Texturas do projeto no viewport. Cada UID ocupa um slot, e o slot fica com a mesma textura enquanto o asset existir.
// O viewport chama `slotOf` ao desenhar (0 enquanto a textura não carregou) e `sync` no quadro, para enviar os pixels.
// Quando o arquivo muda no disco, a textura é recarregada no mesmo slot (AST-004).
import { assets, messageOf, type AssetsEvent } from './assets.svelte';
import { log } from './log.svelte';

interface Entry {
  uid: string;
  /** Slot no viewport. 0 enquanto nenhum slot foi reservado. */
  slot: number;
  /** Os pixels já estão no viewport. Só então `slotOf` devolve o slot. */
  uploaded: boolean;
  state: 'loading' | 'ready' | 'failed';
  w: number;
  h: number;
  /** RGBA aguardando envio ao viewport. */
  pixels: Uint8Array | null;
  /** Sobe a cada recarga, para descartar decodificações antigas. */
  gen: number;
}

/** Interface de texturas do editor WASM (o viewport). */
export interface TextureSink {
  upload_texture(slot: number, width: number, height: number, rgba: Uint8Array): void;
  remove_texture(slot: number): void;
}

/** Imagens decodificadas ao mesmo tempo. */
const MAX_DECODE = 3;

type Ctx2D = CanvasRenderingContext2D | OffscreenCanvasRenderingContext2D;

function context2d(w: number, h: number): Ctx2D | null {
  if (typeof OffscreenCanvas !== 'undefined') return new OffscreenCanvas(w, h).getContext('2d');
  const canvas = document.createElement('canvas');
  canvas.width = w;
  canvas.height = h;
  return canvas.getContext('2d');
}

/** Decodifica a imagem em RGBA sem pré-multiplicar (o que o shader espera). */
async function decode(bytes: ArrayBuffer): Promise<{ w: number; h: number; rgba: Uint8Array }> {
  const bmp = await createImageBitmap(new Blob([bytes]));
  try {
    const w = bmp.width;
    const h = bmp.height;
    const ctx = context2d(w, h);
    if (!ctx) throw new Error('canvas 2D indisponível');
    ctx.drawImage(bmp, 0, 0);
    const data = ctx.getImageData(0, 0, w, h).data;
    return { w, h, rgba: new Uint8Array(data.buffer, data.byteOffset, data.byteLength) };
  } finally {
    bmp.close();
  }
}

class TexturesState {
  /** Sobe quando algo visível muda (textura pronta, enviada ou removida). Quem renderiza lê este número. */
  version = $state(0);

  #entries = new Map<string, Entry>();
  #queue: string[] = [];
  #active = 0;
  #free: number[] = [];
  #next = 1;
  /** Slots a liberar no próximo sync. Libera antes dos envios, porque um slot livre pode ser reaproveitado. */
  #removeQueue: number[] = [];
  #pending = new Set<string>();
  #assetsVersion = -1;

  /** Slot da textura, ou 0 se ela ainda não está no viewport. Pede o carregamento na primeira vez. */
  slotOf(uid: string): number {
    const e = this.#entries.get(uid);
    if (e) return e.uploaded ? e.slot : 0;
    if (assets.byUid(uid)) {
      this.#entries.set(uid, { uid, slot: 0, uploaded: false, state: 'loading', w: 0, h: 0, pixels: null, gen: 0 });
      this.#queue.push(uid);
      this.#pump();
    }
    return 0;
  }

  /** Tamanho em pixels da textura já decodificada, ou null. */
  dims(uid: string): { w: number; h: number } | null {
    void this.version;
    const e = this.#entries.get(uid);
    return e && e.state === 'ready' ? { w: e.w, h: e.h } : null;
  }

  /**
   * Envia ao viewport o que mudou: libera slots de assets apagados e sobe os pixels prontos.
   * Sem nada pendente, custa só uma comparação de números.
   */
  sync(vp: TextureSink): void {
    if (assets.version !== this.#assetsVersion) {
      this.#assetsVersion = assets.version;
      for (const [uid, e] of this.#entries) {
        if (!assets.byUid(uid)) this.#drop(uid, e);
      }
    }
    for (const slot of this.#removeQueue.splice(0)) {
      try {
        vp.remove_texture(slot);
      } catch (e) {
        log.error(`Textura: não foi possível liberar o slot ${slot}: ${messageOf(e)}`);
      }
    }
    if (this.#pending.size === 0) return;
    let uploaded = false;
    for (const uid of this.#pending) {
      this.#pending.delete(uid);
      const e = this.#entries.get(uid);
      if (!e || !e.pixels) continue;
      try {
        vp.upload_texture(e.slot, e.w, e.h, e.pixels);
        e.uploaded = true;
        uploaded = true;
      } catch (err) {
        log.error(`Textura: o viewport não aceitou a imagem: ${messageOf(err)}`);
      }
      e.pixels = null;
    }
    if (uploaded) this.version++;
  }

  /** Esquece todas as texturas (troca de projeto). Os slots são liberados no próximo sync. */
  reset(): void {
    for (const e of this.#entries.values()) {
      if (e.slot && e.uploaded) this.#removeQueue.push(e.slot);
    }
    this.#entries.clear();
    this.#queue = [];
    this.#pending.clear();
    this.#free = [];
    this.#next = 1;
    this.version++;
  }

  /** Recebe as mudanças de assets: troca de projeto, ou arquivos de textura alterados. */
  onAssetsEvent(ev: AssetsEvent): void {
    if (ev.type === 'project') {
      this.reset();
      return;
    }
    for (const rel of ev.paths) {
      const ref = assets.byRel(rel);
      if (ref?.kind === 'texture') this.#reload(ref.uid);
    }
  }

  #reload(uid: string): void {
    const e = this.#entries.get(uid);
    // Sem entrada, a próxima leitura já pega o arquivo novo.
    if (!e) return;
    e.gen++;
    if (!e.uploaded) e.state = 'loading';
    this.#queue.push(uid);
    this.#pump();
  }

  #drop(uid: string, e: Entry): void {
    this.#entries.delete(uid);
    this.#pending.delete(uid);
    if (e.slot) {
      if (e.uploaded) this.#removeQueue.push(e.slot);
      this.#free.push(e.slot);
    }
    this.version++;
  }

  #allocSlot(): number {
    return this.#free.pop() ?? this.#next++;
  }

  #pump(): void {
    while (this.#active < MAX_DECODE && this.#queue.length > 0) {
      const uid = this.#queue.shift() as string;
      this.#active++;
      void this.#decodeUid(uid).finally(() => {
        this.#active--;
        this.#pump();
      });
    }
  }

  async #decodeUid(uid: string): Promise<void> {
    const e = this.#entries.get(uid);
    if (!e) return;
    const gen = e.gen;
    const ref = assets.byUid(uid);
    if (!ref) {
      this.#drop(uid, e);
      return;
    }
    try {
      const img = await decode(await assets.readBytes(ref.rel));
      // Descartada se a textura saiu do projeto ou foi recarregada enquanto decodificava.
      if (this.#entries.get(uid) !== e || e.gen !== gen) return;
      e.w = img.w;
      e.h = img.h;
      e.pixels = img.rgba;
      e.state = 'ready';
      if (!e.slot) e.slot = this.#allocSlot();
      this.#pending.add(uid);
      this.version++;
    } catch (err) {
      if (this.#entries.get(uid) !== e || e.gen !== gen) return;
      log.error(`Não foi possível carregar a textura ${ref.rel}: ${messageOf(err)}`);
      // Recarga com erro mantém a imagem antiga. Primeira carga com erro fica no slot 0.
      if (!e.uploaded) e.state = 'failed';
      this.version++;
    }
  }
}

export const textures = new TexturesState();

// Acompanha o projeto desde o carregamento deste módulo.
assets.subscribe((ev) => textures.onAssetsEvent(ev));
