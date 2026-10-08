# 05. Editor UI & UX

Target: **Unity's visual language** (neutral grays, dense, square, organized) with **Godot's node workflow**. See [research/unity.md](research/unity.md) for the reference breakdown.

> The full standard (every color, size, component, shortcut and decision, with measured contrast) lives in [08-ux-ui-standard.md](08-ux-ui-standard.md). The token table below is the first draft; where they differ, 08 and `editor/src/styles/tokens.css` win.

## Principles

1. **Familiar.** Someone coming from Unity finds Hierarchy, Scene/Game, Inspector, Project and Console where they expect them.
2. **Dense and organized.** Small paddings, 20–22 px rows, clear section headers, no decorative curves.
3. **Fast.** Every action responds in under 100 ms; animations are short and never block input.
4. **Discoverable.** Everything has a menu entry, a shortcut and a Ctrl+K command.
5. **Forgiving.** Unlimited undo, autosave, local history.

## Default layout

```
┌ File Edit Assets Node Window Help ────────── ▶ ❚❚ ▶| ─────────── Layout ▾ ┐
├──────────────┬─────────────────────────────────────┬──────────────────────┤
│ Hierarchy    │ [# Scene] [▶ Game] [◇ Logic]        │ Inspector            │
│ + ▾  search  │  tool strip: hand move rotate scale │ ☑ Player   Tag Layer │
│ ▾ Level      │                                     │ ▾ Transform          │
│   ▾ Player   │            viewport                 │ ▾ CharacterBody2D    │
│     Sprite   │                                     │ ▾ CollisionShape2D   │
│     Shape    │                                     │ ▾ Script             │
│   Coin       │                                     │ [ Add Component ]    │
├──────────────┴──────────────┬──────────────────────┴──────────────────────┤
│ Project | Console | Animation | Audio Mixer                               │
│ ▸ Assets   │ grid of assets with thumbnails, size slider, breadcrumbs     │
└─────────────────────────────┴─────────────────────────────────────────────┘
```

## Design tokens (Unity-like dark theme)

| Token | Value | Use |
|---|---|---|
| bg-window | #191919 | behind panels, tab strip background |
| bg-panel | #383838 | panel body |
| bg-toolbar | #282828 | toolbars, headers |
| bg-field | #2A2A2A | input fields |
| bg-hover | #454545 | hovered rows/buttons |
| selection | #2C5D87 | selected row (focused) |
| selection-unfocused | #4D4D4D | selected row (panel not focused) |
| border | #232323 | 1 px separators |
| text | #D2D2D2 | main text |
| text-dim | #9A9A9A | labels |
| accent | #3A79BB | focus ring, primary buttons |
| play-tint | #3B3B4A | editor tint while playing (configurable) |

- **Radius:** 2–3 px on controls, 0 on panels and tabs.
- **Type:** Inter 12 px for UI (11 px labels), JetBrains Mono 11 px for values and code.
- **Rows:** 20 px tree rows, 18 px inspector fields, 16 px indent per level.
- **Icons:** 16 px, one color per node family (2D blue, physics orange, UI green, audio yellow), Godot-style.
- A light theme with the same structure; the current purple accent remains available as an alternative theme.

## Interaction details to keep

Scrub numeric fields by dragging the label, math expressions in fields, eyedropper colors, Ctrl+K palette, toasts instead of modals, animated framing of the camera.

## Accessibility and language

English UI by default with Portuguese translation (i18n keys from day one), AA contrast, full keyboard navigation, UI scale 80–200 %, pin shapes in addition to colors.
