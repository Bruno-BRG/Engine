import { emptyScene, newId, type SceneData, type SceneObject } from './state/scene.svelte';

export interface Template {
  id: string;
  name: string;
  description: string;
  build: () => SceneData;
}

type O = Partial<SceneObject> & Pick<SceneObject, 'name' | 'shape' | 'x' | 'y' | 'w' | 'h' | 'color'>;
const obj = (o: O): SceneObject => ({ id: newId(), rotation: 0, visible: true, locked: false, behaviors: [], ...o });

export const TEMPLATES: Template[] = [
  {
    id: 'plataforma',
    name: 'Plataforma',
    description: 'Personagem, plataformas, moedas e um inimigo.',
    build: () => ({
      ...emptyScene(),
      objects: [
        obj({ name: 'Chão', shape: 0, x: 640, y: 684, w: 1280, h: 72, color: '#343a58', locked: true }),
        obj({ name: 'Plataforma', shape: 0, x: 380, y: 500, w: 260, h: 32, color: '#4a5287' }),
        obj({ name: 'Plataforma 2', shape: 0, x: 860, y: 372, w: 300, h: 32, color: '#4a5287' }),
        obj({ name: 'Jogador', shape: 0, x: 180, y: 612, w: 56, h: 72, color: '#7c5cff' }),
        obj({ name: 'Moeda', shape: 2, x: 380, y: 436, w: 34, h: 34, color: '#ffd84d', behaviors: [{ kind: 'girar', speed: 120 }, { kind: 'flutuar', speed: 0.8 }] }),
        obj({ name: 'Moeda 2', shape: 2, x: 860, y: 308, w: 34, h: 34, color: '#ffd84d', behaviors: [{ kind: 'girar', speed: 120 }, { kind: 'flutuar', speed: 0.8 }] }),
        obj({ name: 'Moeda 3', shape: 2, x: 1110, y: 590, w: 34, h: 34, color: '#ffd84d', behaviors: [{ kind: 'girar', speed: 120 }, { kind: 'flutuar', speed: 0.8 }] }),
        obj({ name: 'Inimigo', shape: 1, x: 640, y: 616, w: 64, h: 64, color: '#ff5c7a', behaviors: [{ kind: 'pulsar', speed: 1.5 }] }),
      ],
    }),
  },
  {
    id: 'topdown',
    name: 'Top-down',
    description: 'Arena vista de cima com árvores, baú e inimigos.',
    build: () => ({
      ...emptyScene(),
      objects: [
        obj({ name: 'Arena', shape: 0, x: 640, y: 360, w: 1160, h: 620, color: '#1d3330', locked: true }),
        obj({ name: 'Árvore', shape: 1, x: 220, y: 180, w: 120, h: 120, color: '#2f8f6a' }),
        obj({ name: 'Árvore 2', shape: 1, x: 1060, y: 220, w: 140, h: 140, color: '#2f8f6a' }),
        obj({ name: 'Árvore 3', shape: 1, x: 300, y: 560, w: 100, h: 100, color: '#2f8f6a' }),
        obj({ name: 'Baú', shape: 0, x: 980, y: 540, w: 72, h: 52, color: '#ffb84d', behaviors: [{ kind: 'pulsar', speed: 1 }] }),
        obj({ name: 'Jogador', shape: 1, x: 640, y: 380, w: 60, h: 60, color: '#3ddc97' }),
        obj({ name: 'Inimigo', shape: 2, x: 820, y: 220, w: 56, h: 56, color: '#ff5c7a', behaviors: [{ kind: 'girar', speed: 90 }] }),
        obj({ name: 'Inimigo 2', shape: 2, x: 480, y: 280, w: 56, h: 56, color: '#ff5c7a', behaviors: [{ kind: 'girar', speed: -90 }] }),
      ],
    }),
  },
  {
    id: 'puzzle',
    name: 'Puzzle',
    description: 'Tabuleiro de peças coloridas para combinar.',
    build: () => {
      const colors = ['#7c5cff', '#3ddc97', '#ffb84d', '#ff5c7a', '#5cc8ff'];
      const objects: SceneObject[] = [obj({ name: 'Tabuleiro', shape: 0, x: 640, y: 360, w: 640, h: 540, color: '#1f2230', locked: true })];
      for (let r = 0; r < 5; r++)
        for (let c = 0; c < 6; c++) {
          const color = colors[(r * 7 + c * 3) % colors.length];
          objects.push(
            obj({
              name: 'Peça ' + (r * 6 + c + 1),
              shape: (r + c) % 4 === 0 ? 1 : 0,
              x: 640 - 250 + c * 100,
              y: 360 - 200 + r * 100,
              w: 80,
              h: 80,
              color,
              behaviors: (r * 6 + c) % 7 === 3 ? [{ kind: 'pulsar', speed: 1.2 }] : [],
            }),
          );
        }
      return { ...emptyScene(), objects };
    },
  },
  {
    id: 'vazio',
    name: 'Vazio',
    description: 'Uma cena em branco para começar do zero.',
    build: () => emptyScene(),
  },
];

export const templateById = (id: string) => TEMPLATES.find((t) => t.id === id) ?? TEMPLATES[3];
