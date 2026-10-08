<script lang="ts">
  // Ícone por tipo de nó (EDT-003, família de nós). Cada tipo tem um ícone e uma cor de família.
  import { Box, Move, Image as ImageIcon, Camera, Zap, Square, Circle, User, SquareDashed, Hexagon } from '@lucide/svelte';
  let { kind, size = 13 }: { kind: string; size?: number } = $props();
  const ICON = {
    Node: Box,
    Node2D: Move,
    Sprite2D: ImageIcon,
    Camera2D: Camera,
    Behavior: Zap,
    StaticBody2D: Square,
    RigidBody2D: Circle,
    CharacterBody2D: User,
    Area2D: SquareDashed,
    CollisionShape2D: Hexagon,
  } as const;
  const Icon = $derived(ICON[kind as keyof typeof ICON] ?? Box);
  const PHYSICS = ['StaticBody2D', 'RigidBody2D', 'CharacterBody2D', 'Area2D', 'CollisionShape2D'];
  const family = $derived(
    kind === 'Behavior' ? 'var(--node-event)'
    : kind === 'Camera2D' ? 'var(--node-audio)'
    : kind === 'Sprite2D' ? 'var(--node-object)'
    : kind === 'Node2D' ? 'var(--node-flow)'
    : PHYSICS.includes(kind) ? 'var(--node-physics)'
    : 'var(--text-dim)',
  );
</script>

<span class="node-icon" style="color: {family}"><Icon {size} /></span>

<style>
  .node-icon {
    display: inline-flex;
    flex: none;
    vertical-align: middle;
  }
</style>
