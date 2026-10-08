<script lang="ts">
  // Ícone por tipo de nó (EDT-003). Cada família tem uma cor (--node-*, styles/tokens.css), como na tabela
  // da seção 3.5 de docs/08: 2D azul, física laranja, UI verde, áudio amarelo e script roxo (lógica e tempo).
  import {
    Box,
    Move,
    Image as ImageIcon,
    Camera,
    Zap,
    Square,
    Circle,
    User,
    SquareDashed,
    Hexagon,
    Triangle,
    Crosshair,
    ScanLine,
    Pin,
    Route,
    Activity,
    Volume2,
    Speaker,
    Timer as TimerIcon,
    Spline,
  } from '@lucide/svelte';
  let { kind, size = 13 }: { kind: string; size?: number } = $props();

  /** Ícone de cada tipo de nó. Tipo fora da lista usa a caixa genérica. */
  const ICON = {
    Node: Box,
    Node2D: Move,
    Sprite2D: ImageIcon,
    Camera2D: Camera,
    Path2D: Spline,
    StaticBody2D: Square,
    RigidBody2D: Circle,
    CharacterBody2D: User,
    Area2D: SquareDashed,
    CollisionShape2D: Hexagon,
    CollisionPolygon2D: Triangle,
    RayCast2D: Crosshair,
    ShapeCast2D: ScanLine,
    PinJoint2D: Pin,
    GrooveJoint2D: Route,
    DampedSpringJoint2D: Activity,
    AudioStreamPlayer: Volume2,
    AudioStreamPlayer2D: Speaker,
    Behavior: Zap,
    Timer: TimerIcon,
  } as const;

  /** Cor de cada família. `ui` fica reservada para os nós de interface, que ainda não existem. */
  const COR = {
    '2d': 'var(--node-object)',
    fisica: 'var(--node-physics)',
    ui: 'var(--node-ui)',
    audio: 'var(--node-audio)',
    script: 'var(--node-script)',
    neutro: 'var(--text-dim)',
  } as const;

  /** Família de cada tipo com cor própria. O que não está aqui fica neutro (ex.: Node). */
  const FAMILIA: Record<string, keyof typeof COR> = {
    Node2D: '2d',
    Sprite2D: '2d',
    Camera2D: '2d',
    Path2D: '2d',
    StaticBody2D: 'fisica',
    RigidBody2D: 'fisica',
    CharacterBody2D: 'fisica',
    Area2D: 'fisica',
    CollisionShape2D: 'fisica',
    CollisionPolygon2D: 'fisica',
    RayCast2D: 'fisica',
    ShapeCast2D: 'fisica',
    PinJoint2D: 'fisica',
    GrooveJoint2D: 'fisica',
    DampedSpringJoint2D: 'fisica',
    AudioStreamPlayer: 'audio',
    AudioStreamPlayer2D: 'audio',
    Behavior: 'script',
    Timer: 'script',
  };

  const Icon = $derived(ICON[kind as keyof typeof ICON] ?? Box);
  const family = $derived(COR[FAMILIA[kind] ?? 'neutro']);
</script>

<span class="node-icon" style="color: {family}"><Icon {size} /></span>

<style>
  .node-icon {
    display: inline-flex;
    flex: none;
    vertical-align: middle;
  }
</style>
