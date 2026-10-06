// Execução simplificada da cena no play-in-editor (Fase 0).
// Na Fase 2 isto é substituído pela nexo-vm executando a lógica visual.
import type { SceneObject } from './scene.svelte';

class PlayRuntime {
  objects: SceneObject[] = [];
  #base: SceneObject[] = [];
  time = 0;
  /** Cópia reduzida para o painel Depurador, atualizada algumas vezes por segundo. */
  live = $state.raw<SceneObject[]>([]);
  #sinceLive = 0;

  start(objects: SceneObject[]) {
    this.#base = structuredClone(objects);
    this.objects = structuredClone(objects);
    this.time = 0;
    this.live = this.objects.map((o) => ({ ...o }));
  }

  step(dt: number) {
    this.time += dt;
    const t = this.time;
    for (let i = 0; i < this.objects.length; i++) {
      const base = this.#base[i];
      const o = this.objects[i];
      for (const b of base.behaviors) {
        switch (b.kind) {
          case 'girar':
            o.rotation = (o.rotation + b.speed * dt) % 360;
            break;
          case 'flutuar':
            o.y = base.y + Math.sin(t * b.speed * Math.PI * 2) * 14;
            break;
          case 'pulsar': {
            const s = 1 + Math.sin(t * b.speed * Math.PI * 2) * 0.08;
            o.w = base.w * s;
            o.h = base.h * s;
            break;
          }
        }
      }
    }
    this.#sinceLive += dt;
    if (this.#sinceLive > 0.1) {
      this.#sinceLive = 0;
      this.live = this.objects.map((o) => ({ ...o }));
    }
  }

  stop() {
    this.objects = [];
    this.#base = [];
    this.live = [];
  }
}

export const runtime = new PlayRuntime();
