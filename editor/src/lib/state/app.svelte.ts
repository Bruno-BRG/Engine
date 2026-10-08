import type { ProjectInfo } from '../platform';

export type Workspace = 'cena' | 'jogo' | 'logica' | 'animacao' | 'sprite';
export type BottomTab = 'assets' | 'console' | 'depurador' | 'csharp' | 'mixer';
/** Ferramentas de transformação, como no Unity (VPT-001). */
export type Tool = 'hand' | 'move' | 'rotate' | 'scale' | 'rect';

// Preferências de interface. localStorage pode falhar (modo privado, dados bloqueados).
function readUi(): Record<string, any> {
  try {
    return JSON.parse(localStorage.getItem('nexo.ui') ?? '{}');
  } catch {
    return {};
  }
}
const saved = readUi();

class AppState {
  project = $state<ProjectInfo | null>(null);
  workspace = $state<Workspace>('cena');
  bottomTab = $state<BottomTab>(saved.bottomTab ?? 'assets');
  tool = $state<Tool>('move');
  /** Snap de grade e de rotação (VPT-002). Ctrl inverte enquanto arrasta. */
  snap = $state(saved.snap ?? false);
  gridSize = $state<number>(saved.gridSize ?? 16);
  rotationSnap = $state<number>(saved.rotationSnap ?? 15);
  /** Aba Jogo: proporção da tela e escala de visualização (EDT-012). */
  gameAspect = $state<'16:9' | '4:3' | 'livre'>(saved.gameAspect ?? '16:9');
  gameScale = $state(saved.gameScale ?? 1);
  playing = $state(false);
  /** Pausa do jogo em execução: o tempo não avança até retomar ou dar um passo. */
  paused = $state(false);
  /** Pedido de um único quadro (Passo a passo), consumido pelo laço do viewport. */
  stepRequested = $state(false);
  /** Desenho de depuração da física (DBG-004). Só vale nesta sessão: não entra em `persistUi`. */
  physicsDebug = $state(false);
  paletteOpen = $state(false);
  theme = $state<'dark' | 'light'>(saved.theme ?? 'dark');
  layout = $state<'padrao' | 'cena' | 'programacao' | 'viewport'>(saved.layout ?? 'padrao');
  panels = $state<{ left: boolean; right: boolean; bottom: boolean }>(
    saved.panels ?? { left: true, right: true, bottom: true },
  );
  sizes = $state<{ left: number; right: number; bottom: number }>(
    saved.sizes ?? { left: 248, right: 300, bottom: 230 },
  );

  // Telemetria do viewport
  fps = $state(0);
  gpu = $state('');
  zoom = $state(1);

  // Persistência
  dirty = $state(false);
  saving = $state(false);
  lastSaved = $state<Date | null>(null);

  persistUi() {
    try {
      localStorage.setItem(
        'nexo.ui',
        JSON.stringify({
        theme: this.theme,
        panels: this.panels,
        sizes: this.sizes,
        bottomTab: this.bottomTab,
        snap: this.snap,
        gridSize: this.gridSize,
        rotationSnap: this.rotationSnap,
        gameAspect: this.gameAspect,
        gameScale: this.gameScale,
        layout: this.layout,
        }),
      );
    } catch {
      // Sem armazenamento disponível: as preferências valem só nesta sessão.
    }
  }
}

export const app = new AppState();
