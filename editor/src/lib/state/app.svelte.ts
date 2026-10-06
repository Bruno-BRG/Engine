import type { ProjectInfo } from '../platform';

export type Workspace = 'cena' | 'logica' | 'animacao' | 'sprite';
export type BottomTab = 'assets' | 'console' | 'depurador' | 'csharp';
export type Tool = 'select' | 'hand';

const saved = JSON.parse(localStorage.getItem('nexo.ui') ?? '{}');

class AppState {
  project = $state<ProjectInfo | null>(null);
  workspace = $state<Workspace>('cena');
  bottomTab = $state<BottomTab>(saved.bottomTab ?? 'assets');
  tool = $state<Tool>('select');
  playing = $state(false);
  paletteOpen = $state(false);
  theme = $state<'dark' | 'light'>(saved.theme ?? 'dark');
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
    localStorage.setItem(
      'nexo.ui',
      JSON.stringify({ theme: this.theme, panels: this.panels, sizes: this.sizes, bottomTab: this.bottomTab }),
    );
  }
}

export const app = new AppState();
