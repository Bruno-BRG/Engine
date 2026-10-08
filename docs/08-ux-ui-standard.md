# 08. UX & UI standard

**Status:** in force for the editor.
**Source of truth:** [editor/src/styles/tokens.css](../editor/src/styles/tokens.css) (colors, radii, type scale, motion) and [editor/src/lib/palette.ts](../editor/src/lib/palette.ts) (object colors). This document records the decisions and the values. When they disagree, fix both in the same pull request.

Related: [05 Editor UI & UX](05-editor-ui.md) (principles and layout), [ADR 0005](adr/0005-unity-style-editor.md) (Unity-style visuals), [CONTRIBUTING](../CONTRIBUTING.md) (code style).

---

## 1. Rules that apply everywhere

1. **No raw colors in components.** Every color comes from a token. If no token fits, add one to `tokens.css` and a row to this document in the same change.
2. **A color means one thing.** Status colors mean state. Node-family colors mean the kind of node. Pin colors mean the data type. The accent means focus and selection. Do not reuse a color for another meaning.
3. **Square structure, small radii.** No pills, no glossy gradients, no glows. Shadows only on floating elements (menus, overlays, nodes, cards).
4. **Dense by default.** 12 px text, 20–22 px rows and headers, 24 px buttons.
5. **Everything reachable from the keyboard.** Every action has a command in the Ctrl+K palette (principle 4 in [05](05-editor-ui.md)).
6. **Motion answers an action.** Short transitions (120–200 ms). No decorative loops, except the flowing execution wires and the live dot.
7. **Sentence case, active verbs.** "Salvar cena", "Abrir pasta do projeto". No all-caps labels.

---

## 2. Layout

| Region | Size | Contents |
|---|---|---|
| Title bar | 30 px tall | Menu bar (Arquivo, Editar, Node, Assets, Janela, Ajuda), project name, Play, Pause and Step centered, layout selector, command search, window controls |
| Hierarchy | 180–420 px wide | Scene tree, filter, add-object menu |
| Center | fills | Workspace tabs (22 px), then the viewport or the workspace view |
| Inspector | 240–480 px wide | Properties of the selected object, or of the scene when nothing is selected |
| Bottom dock | 120–520 px tall | Tabs: Assets, Console, Depurador, C# |
| Status bar | 22 px tall | Save state, object count, last action, FPS while playing, graphics backend, zoom, version |

- Regions are separated by a 2 px gap in `--bg-0`. The splitter is 2 px and turns accent-colored on hover.
- **Workspaces** are Cena, Jogo, Lógica, Animação and Sprite, opened with Alt+1 to Alt+5. They are tabs above the viewport, the same way Unity puts Scene and Game above its view. Cena edits the scene. Jogo shows the scene camera at the chosen aspect (16:9, 4:3 or free) and scale. Lógica, Animação and Sprite are phase placeholders ([WorkspacePreview](../editor/src/lib/components/WorkspacePreview.svelte)).
- Panels can be hidden: Ctrl+B (Hierarchy), Ctrl+Alt+B (Inspector), Ctrl+J (bottom dock).

---

## 3. Color

### 3.1 Surfaces (dark theme, default)

| Token | Value | Used for |
|---|---|---|
| `--bg-0` | `#191919` | Window, gaps between regions, wells |
| `--bg-1` | `#383838` | Panel bodies, selected tab, inspector content |
| `--bg-toolbar` | `#282828` | Title bar, panel headers, tab strips, status bar |
| `--bg-2` | `#2a2a2a` | Fields, menus, nodes |
| `--bg-3` | `#454545` | Buttons, keyboard keys |
| `--bg-overlay` | `rgb(40 40 40 / 0.92)` | Floating panels over the viewport |
| `--bg-hover` | `rgb(255 255 255 / 0.06)` | Hover |
| `--bg-active` | `rgb(255 255 255 / 0.10)` | Pressed |
| `--line` | `#232323` | 1 px borders and separators |
| `--line-soft` | `#1e1e1e` | Softer separators inside panels |
| `--selection` | `#2c5d87` | Selected row when its list has focus |
| `--selection-unfocused` | `#4d4d4d` | Selected row when its list has no focus |
| `--sel-text` | `#ffffff` | Text on a selection |
| `--play-tint` | `#3b3b4a` | Panel background while the game plays |
| `--scrim` | `rgb(5 6 10 / 0.5)` | Behind dialogs and the command palette |
| `--canvas-dot` | `rgb(255 255 255 / 0.05)` | Dots on the logic canvas |
| `--texture-line` | `rgb(255 255 255 / 0.02)` | Placeholder stripes on empty cards |

### 3.2 Text

| Token | Value | Used for |
|---|---|---|
| `--text` | `#d2d2d2` | Primary text, values, active items |
| `--text-dim` | `#a6a6a6` | Labels, secondary text |
| `--text-faint` | `#a4a4a4` | Placeholders and hints only |

`--text-dim` and `--text-faint` are almost the same value on purpose: both had to pass contrast. Use `--text-dim` for labels and `--text-faint` for placeholders.

### 3.3 Accent and focus

| Token | Value | Used for |
|---|---|---|
| `--accent` | `#3a79bb` | Primary buttons, focused field border, drop indicator, object label in the viewport |
| `--accent-hi` | `#4d8fd6` | Icons of the active tool or tab. **Icons only, never text** |
| `--accent-soft` | `rgb(58 121 187 / 0.22)` | Badge background, active icon button, drag highlight |
| `--accent-ring` | `rgb(58 121 187 / 0.70)` | Keyboard focus outline, tag borders |

### 3.4 Status

| Token | Value | Meaning |
|---|---|---|
| `--success` | `#5fb36a` | Saved, playing (FPS), live debugger dot |
| `--warning` | `#e0b04a` | Unsaved changes |
| `--danger` | `#f28288` | Errors: console error count, error states, close button hover |
| `--info` | `#70b8ee` | Neutral notices |

Status colors are for dots, icons and short text. They are never large fills. Status text must sit on the surfaces listed in 3.9.

### 3.5 Node families (visual programming)

One color per family. The families follow the table in [05](05-editor-ui.md) (2D blue, physics orange, UI green, audio yellow).

| Token | Value | Family | Example |
|---|---|---|---|
| `--node-event` | `#c9606e` | Events | "Ao pressionar ação" |
| `--node-flow` | `#d2d2d2` | Flow control (neutral) | Reserved for branches and loops |
| `--node-data` | `#4fb0a5` | Data and math | "Força do pulo" |
| `--node-object` | `#4f8fd6` | 2D objects and transforms | Reserved |
| `--node-physics` | `#e08a3c` | Physics | "Aplicar impulso" |
| `--node-ui` | `#6bbf59` | UI | Reserved |
| `--node-audio` | `#e5c152` | Audio | "Tocar som" |
| `--node-script` | `#9a7fd1` | Script and timers (purple) | "Repetir a cada 1 s" |

### 3.6 Pin and wire types

| Token | Value | Shape |
|---|---|---|
| `--type-exec` | `#d2d2d2` | Triangle (execution flow) |
| `--type-bool` | `#c9606e` | Circle ring |
| `--type-int` | `#4fb0a5` | Circle ring |
| `--type-float` | `#6bbf59` | Circle ring |
| `--type-text` | `#c77dba` | Circle ring |
| `--type-vec` | `#e5c152` | Circle ring |
| `--type-object` | `#4f8fd6` | Circle ring |
| `--type-color` | `#9a7fd1` | Circle ring |

Some hues repeat between node families and pin types (red, teal, blue, yellow). The shapes differ: pins are triangles or rings, and node headers are filled by family. If users confuse them in practice, change the pin hues first.

### 3.7 Code syntax (C# view and Depurador)

| Token | Value | Token kind |
|---|---|---|
| `--code-keyword` | `#569cd6` | Keywords |
| `--code-type` | `#4ec9b0` | Types |
| `--code-fn` | `#dcdcaa` | Functions and methods |
| `--code-string` | `#ce9178` | Strings |
| `--code-number` | `#b5cea8` | Numbers |

Comments use `--text-faint` in italics.

### 3.8 Object palette (project content)

These are the colors objects can take. The list lives in [palette.ts](../editor/src/lib/palette.ts). Index 0 is the default for new objects.

| Index | Name | Hex |
|---|---|---|
| 0 | Blue (default) | `#4f8fd6` |
| 1 | Teal | `#4fb0a5` |
| 2 | Green | `#6bbf59` |
| 3 | Yellow | `#e5c152` |
| 4 | Orange | `#e08a3c` |
| 5 | Red | `#c9606e` |
| 6 | Violet | `#9a7fd1` |
| 7 | Gray | `#9a9a9a` |
| 8 | White | `#d2d2d2` |

Each object stores its own hex in the scene file. Changing the palette does not recolor objects that already exist.

### 3.9 Contrast (measured)

Ratios use the WCAG 2.1 relative-luminance formula. Text needs 4.5:1. Graphics and icons need 3:1.

| Foreground | Background | Ratio | Result |
|---|---|---|---|
| `--text` `#d2d2d2` | panel `#383838` | 7.76 | Pass |
| `--text-dim` `#a6a6a6` | window `#191919` | 7.22 | Pass |
| `--text-dim` `#a6a6a6` | panel `#383838` | 4.82 | Pass |
| `--text-faint` `#a4a4a4` | panel `#383838` | 4.70 | Pass |
| `--text` `#d2d2d2` | toolbar `#282828` | 9.75 | Pass |
| `--text` `#d2d2d2` | button `#454545` | 6.34 | Pass |
| `--sel-text` `#ffffff` | selection `#2c5d87` | 6.95 | Pass |
| white | accent `#3a79bb` | 4.53 | Pass |
| `--success` `#5fb36a` | panel `#383838` | 4.55 | Pass |
| `--warning` `#e0b04a` | panel `#383838` | 5.85 | Pass |
| `--danger` `#f28288` | panel `#383838` | 4.65 | Pass |
| `--info` `#70b8ee` | panel `#383838` | 5.46 | Pass |
| Pin and node colors | field `#2a2a2a` | 3.7–9.5 | Pass (graphics, 3:1) |

**Rules that follow from the table:**
- Status colors and `--text-dim` or `--text-faint` text do not go on `--bg-3` (buttons). On a button, labels use `--text`.
- Icons and graphics need 3:1 on `--bg-2`. `--accent-hi` is used for icons only, because as text it fails on `--bg-1`.
- Disabled controls (45 % opacity) are exempt.

### 3.10 Deliberate exceptions

- **Brand mark.** [Logo.svelte](../editor/src/lib/components/Logo.svelte) keeps its own gradient (`#9A82FF` to `#5132E8`). It is a brand asset, not interface color.
- **Sample scene art.** The templates in [templates.ts](../editor/src/lib/templates.ts) use literal dark tones for floors, platforms, the arena, trees and the board. Their bright object colors come from the object palette.

---

## 4. Typography

- **Fonts:** Inter Variable for the interface (`--font-ui`), JetBrains Mono Variable for values, code and value chips (`--font-mono`).
- **Scale** (tokens): 11 px for labels, status bar and badges; **12 px as the default** for rows, tabs, buttons, menus and fields; 13 px for subheadings; 15 px for card and dialog titles; 18 px and 24 px for the start screen.
- **Weights:** 400 for body text; 500 for buttons, tabs and labels; 600 only for section bands and panel titles.
- **Case:** sentence case. The app still has two all-caps labels (see section 15).
- **Numbers:** monospace in fields and value chips; tabular figures in the status bar.
- **Minimum size:** 11 px. Nothing smaller is allowed in the interface.

---

## 5. Shape, borders and elevation

| Radius | Use |
|---|---|
| `0` | Panels, tab strips, title bar, status bar, viewport, scroll areas |
| `3px` (`--r-ctl`) | Buttons, fields, menu items, tree buttons, badges, value chips, nodes, icon tiles, segmented controls, tags |
| `4px` (`--r-dlg`) | Dialogs, cards, toasts, floating overlay cards |
| `50%` | Color dots and palette swatches only |

- **Pills (99 px) are not used.**
- **Borders:** 1 px in `--line`. No double borders.
- **Shadows:** `--shadow-pop` for menus, `--shadow-float` for viewport overlays, `--shadow-card` for nodes and cards. Panels are flat.
- **Gradients:** not used in interface chrome. The exceptions are the faint accent glow behind the start screen title, the diagonal placeholder stripes on empty cards (`--texture-line`), and the brand mark.

---

## 6. Sizes and density

| Element | Size |
|---|---|
| Title bar | 30 px |
| Workspace tabs, panel header, bottom-dock tab strip, section band | 22 px |
| Status bar | 22 px |
| Button | 24 px (title bar Play: 22 px) |
| Icon button | 22 px (18 px inside panel headers) |
| Text field | 20 px |
| Numeric field (Inspector) | 18 px |
| Hierarchy row | 20 px |
| Menu item | 24 px |
| Color swatch | 28 px square |
| Palette swatch | 18 px circle |

- **Padding:** panel content 8 px horizontal; section content 8 px.
- **Tree indentation:** 14 px base. Doc 05 specifies 16 px per level once nesting exists.
- **Icons** (Lucide, `@lucide/svelte`): 12 px in rows, 13 px in tabs and menus, 14–15 px in headers and buttons, 22 px for empty-state illustrations.
- **Scrollbars:** 10 px wide, thumb in `--bg-3`.

---

## 7. Components

**Buttons**
- Default: `--bg-3` background, 1 px `--line` border, `--text` label, 24 px tall.
- Primary: `--accent` background, white label, border in accent. Hover lightens to `--accent-hi`. Used for one main action per dialog.
- Ghost: no background or border, `--text-dim` label. Hover shows `--bg-hover`.
- Pressed: `--bg-active`, plus a 1 px downward shift.
- Disabled: 45 % opacity, no pointer events.

**Fields**
- Text field: `--bg-2`, 1 px `--line` border, 20 px tall. Hover shows `--text-faint` border. Focus shows an `--accent` border (no glow).
- Numeric field: a label cell (drag to scrub) plus a monospace value. Shift multiplies the step by 10, Alt by 0.1. Enter commits, Escape cancels, arrows step by 1.

**Menus and popovers**
- `--bg-2`, `--shadow-pop`, 3 px inner padding, 24 px items. Hover shows the selection color with `--sel-text`. Click outside closes.
- **Menu bar:** Arquivo, Editar, Node, Assets, Janela, Ajuda. Each item comes from the command list in [commands.ts](../editor/src/lib/commands.ts), so the menu, the Ctrl+K palette and the shortcuts always agree. Checkable items (panels, workspaces) show a check mark for the current state. A click opens a menu; while one is open, hovering another menu switches to it; Escape closes.

**Tabs**
- Workspace tabs: active tab on `--bg-1` with `--text`; inactive `--text-dim`; hover `--bg-hover`.
- Bottom-dock tabs and panel headers follow the same pattern.

**Hierarchy rows**
- 20 px. Hover shows `--bg-hover`. Selected shows `--selection-unfocused`, and `--selection` while the list has focus.
- Hidden objects are drawn at 40 % opacity. Lock and visibility buttons (16 px) appear on hover, and stay visible when on.

**Inspector**
- Object header: 20 px color tile, name field, visibility and lock buttons.
- Sections are 22 px bands in `--bg-toolbar`, with a chevron that rotates when collapsed. Content has 8 px side padding and a 64 px label column.
- The shape selector is a segmented control on `--bg-2`, with the active segment in the selection color.
- With nothing selected, the Inspector shows scene properties and an empty-state message.

**Dialogs**
- Modal, `--scrim` behind, `--r-dlg` radius, Escape closes. Modals are used only for flows that need input (creating a project). Other feedback uses toasts (decision D9).

**Toasts**
- Short confirmations and notices, not blocking. Use the matching status color for the leading icon.

**Empty states**
- A title, one line on what to do, and the shortcut or action. Examples: "Sua cena está vazia. Arraste uma forma do painel Assets para cá, ou pressione Ctrl K.", "Nada selecionado. Clique num objeto no viewport ou na hierarquia para editar suas propriedades."

**Viewport overlays**
- Tool strip, top-left, `--bg-overlay`, 3 px radius, 1 px `--line` border.
- Selection label above the selected object: accent background, white text, name and size.
- "Jogando" badge at the top center while playing, with the Esc hint.
- Empty-scene hint at the bottom center.

**Status bar**
- Left: save state (dot plus text), object count, last action. Right: FPS while playing, graphics backend, zoom, version and platform.
- Save state is always shown as text next to its dot, never as the dot alone.

---

## 8. States and interaction

| State | Look |
|---|---|
| Default | Token defaults |
| Hover | `--bg-hover` overlay |
| Pressed | `--bg-active` overlay, 1 px down shift on buttons |
| Selected | `--selection` (focused list) or `--selection-unfocused` |
| Keyboard focus | 2 px outline in `--accent-ring`, 1 px offset (`:focus-visible` only) |
| Disabled | 45 % opacity, no pointer events |
| Playing | Play button in `--selection`; panels tinted with `--play-tint`; viewport badge |

**Motion**
- Durations: 120 ms for hover and color, 200 ms for popovers, 180 ms for section slides.
- Easing: `cubic-bezier(0.2, 0.8, 0.2, 1)` everywhere.
- Reduced motion: every transition is cut to 0.01 ms when the OS asks for it ([global.css](../editor/src/styles/global.css)).

**Play mode**
- F5 or Ctrl+P starts and stops. Escape stops while playing.
- **Pause** (F6 or the Pause button) freezes game time. The viewport badge changes to "Pausado", and the status bar shows 0 fps.
- **Step** (F10 or the Step button) advances exactly one frame of 1/60 s, and the Depurador updates right away. If the game is stopped, Step starts it paused, as Unity does.
- While playing, edit commands are disabled: add, duplicate, delete, undo and redo. Hierarchy and Inspector edit buttons are disabled too.

**Autosave**
- The scene saves 1.5 seconds after the last change ([Editor.svelte](../editor/src/lib/components/Editor.svelte)). The status bar shows "Alterações não salvas", then "Salvando…", then "Salvo agora" or "Salvo há N min".

**Hierarchy**
- Click selects. Double-click, F2 or Enter renames. Arrow keys move the selection. Drag reorders, with an accent line at the drop point.

**Viewport**
- V selects and moves. H is the hand tool. Holding Space is a temporary hand, and middle or right drag always pans.

---

## 9. Keyboard shortcuts

The source of truth is [commands.ts](../editor/src/lib/commands.ts). While typing in a text field, shortcuts are ignored, except the global ones (Ctrl+K, Ctrl+S, F5, Ctrl+P).

| Shortcut | Action |
|---|---|
| Ctrl+K | Command palette |
| Ctrl+S | Save scene |
| F5, Ctrl+P | Play or stop |
| F6 | Pause or resume the game (only while playing) |
| F10 | Step one frame of 1/60 s. If stopped, starts the game paused |
| Ctrl+C | Copy the selected object |
| Ctrl+X | Cut the selected object (undo puts it back) |
| Ctrl+V | Paste a copy 32 px away from the original, and select it |
| Escape | Clear selection, or stop play |
| Ctrl+Z | Undo |
| Ctrl+Shift+Z, Ctrl+Y | Redo |
| Ctrl+D | Duplicate selected object |
| Delete, Backspace | Delete selected object |
| Q | Hand tool |
| W | Move tool |
| E | Rotate tool |
| R | Scale tool |
| T | Rect tool |
| F | Frame selection |
| Home | Frame the game screen |
| Ctrl+1 | Zoom 100 % |
| Alt+1 to Alt+5 | Cena, Jogo, Lógica, Animação, Sprite |
| Ctrl+B | Show or hide Hierarchy |
| Ctrl+Alt+B | Show or hide Inspector |
| Ctrl+J | Show or hide bottom dock |
| F2, Enter | Rename (Hierarchy, when focused) |

---

## 10. Visual programming (Lógica)

Status: Phase 2. The current view is a static mock, and it defines the look that the real graph editor will follow.

- **Canvas:** `--bg-0` with `--canvas-dot` dots on a 22 px grid. No gradients.
- **Nodes:** 200 px wide, 3 px radius, `--bg-2` body, 1 px `--line` border, `--shadow-card`.
- **Node header:** 4 px by 8 px padding, weight 500, `--text` label. The background is the family color mixed 30 % into `--bg-2`, with a 1 px bottom border. Flat, no gradient.
- **Pin rows:** 4 px by 8 px padding, label in `--text-dim`, values in a monospace chip (`--bg-0`, 1 px `--line`, 3 px radius).
- **Pins:** execution pins are white triangles (`--type-exec`). Data pins are 9 px rings in their type color.
- **Wires:** 2 px. Execution wires are dashed (6 px dash, 8 px gap) in white and animate forward over 1.2 s. Data wires are solid in their type color. Curves are cubic Bézier.
- **Naming:** event nodes start with "Ao" ("Ao pressionar ação"). Action nodes use an imperative verb ("Aplicar impulso"). Data nodes use a noun ("Força do pulo").

---

## 11. Iconography

- One icon library: Lucide through `@lucide/svelte`. No emoji and no other icon sets.
- One icon per action. Icons are `--text-dim` by default, `--text` on hover, `--accent-hi` when active.
- Node families may tint their icon with the family color.
- Icons are not copied from Unity (ADR 0005).

---

## 12. Copy and language

- **Current state:** the interface is in Brazilian Portuguese. Strings are hard-coded, and `index.html` declares `lang="pt-BR"`.
- **Target:** English as the default language, with Portuguese as a translation. Every string goes through i18n keys (task EDT-040, see [05](05-editor-ui.md) and [CONTRIBUTING](../CONTRIBUTING.md)).
- **Until EDT-040 lands:** new strings follow the existing Portuguese copy, and extraction into keys happens as one separate task.
- **Voice:** plain verbs, sentence case, no filler. Buttons name the result ("Salvar cena", "Criar projeto"). The same action keeps the same name everywhere.
- **Errors:** say what happened and how to fix it, in the interface's voice. Example: "Não foi possível iniciar o viewport", followed by the technical reason.
- **Empty states** invite an action (see section 7).

**Glossary**

| Term | Meaning |
|---|---|
| Cena | Scene: the tree of objects in the working level |
| Hierarquia | Hierarchy panel: the scene tree |
| Inspector | Properties of the selected object or the scene |
| Assets | Shapes and imported files |
| Console | Log messages |
| Depurador | Debugger: values while playing |
| C# | Generated C# code (read-only) |
| Lógica | Visual logic workspace (node graphs) |
| Workspace | One of Cena, Lógica, Animação, Sprite |
| Behaviors | Ready-made behaviors on objects (kept as the English term) |
| Nó, pino | Node and pin, in the logic workspace |

---

## 13. Accessibility

- **Contrast:** section 3.9. Text 4.5:1, graphics 3:1.
- **Focus:** a visible 2 px outline on keyboard focus (`:focus-visible`).
- **Keyboard:** every action is reachable from the keyboard, and the Ctrl+K palette covers all commands.
- **Not color alone:** pins differ by shape (triangle versus ring). Hidden objects use opacity and the eye icon. Save state has a text label.
- **Motion:** respects the OS reduced-motion setting.
- **Scale:** doc 05 asks for 80–200 % UI scale. Not implemented yet (section 15).

---

## 14. Decision log

| # | Decision | Reason |
|---|---|---|
| D1 | Unity-style dark theme and layout | Users come from Unity ([ADR 0005](adr/0005-unity-style-editor.md)) |
| D2 | Workspace tabs above the viewport | Same place as Unity's Scene and Game tabs |
| D3 | One token source, no raw colors in components | Keeps the look consistent and easy to change |
| D4 | Node-family colors from the table in 05 | One color per kind of node, as in Godot and Unity |
| D5 | Object palette of nine colors, blue as default | Replaces the Phase 0 purple, which was the old accent |
| D6 | Status and pin colors muted | Pass contrast on panels while staying readable |
| D7 | Radii 0, 3 and 4 px; no pills | Square, dense, Unity-like |
| D8 | Text colors raised to pass AA on panels | Accessibility goal in 05 |
| D9 | Modals only for flows that need input; other feedback in toasts | Keeps the flow fast (principle 3 in 05) |
| D10 | Canvas dots and dashed execution wires kept | Decided in ADR 0005 |
| D11 | Flat node headers, no gradients | Decorative gradients were removed from the chrome |
| D12 | Alternative purple theme not kept | Not implemented; doc 05 listed it as optional. Revisit if asked |
| D13 | Menus are built from the command list | One source for menu, palette and shortcuts, so they cannot drift apart |
| D14 | Step on a stopped game starts it paused | Same as Unity: Step is for stepping through a game, not for running it |
| D15 | Scene state lives in nexo-core, behind WebAssembly ([ADR 0006](adr/0006-scene-state-in-core.md)) | Edits are synchronous, with no IPC per drag; one model for the viewport, the Logic VM and the exporter |
| D16 | Tool keys are Q, W, E, R and T, as in Unity | Users come from Unity. H was the Phase 0 hand key and is gone |
| D17 | The Jogo tab draws the scene camera, with aspect presets | Matches Unity's Game view and the Camera2D node |
| D18 | Native file drop is off (`dragDropEnabled: false`) | On Windows, Tauri's drop handler blocks HTML5 drag inside the webview, which hierarchy reordering needs. Files are imported with the Importar button for now |

---

## 15. Known gaps

These are known deviations from this standard. Close them as separate tasks.

1. **All-caps labels:** the start screen section titles ("COMECE COM UM MODELO", "RECENTES") and the Inspector group label ("Chegam na Fase 2") use `text-transform: uppercase`. Switch to sentence case.
2. **Viewport background:** `#0E0F13`, set in the renderer clear color ([lib.rs:275](../crates/nexo-render/src/lib.rs:275)). The same renderer is used by exported games, so changing it needs a decision: an editor-only background, or a new default for games too.
3. **Default object colors in existing projects:** saved objects keep their colors. Only new objects use the new palette.
4. **Light theme:** tokens exist and text passes contrast, but node and pin hues are not checked on light surfaces. Yellow and the neutral flow color fail 3:1 on white. The light theme needs its own node tuning.
5. **Language:** UI strings are hard-coded Portuguese (section 12).
6. **Menus:** Build is a disabled placeholder until Phase 3. Project Settings and Preferences from [05](05-editor-ui.md) are missing (P1).
7. **Layouts:** the layout selector has four presets. Saving custom layouts (EDT-015) is P1.
8. **UI scale:** 80–200 % scaling from [05](05-editor-ui.md) is not implemented.
9. **Local padding:** some Inspector menus and the start screen use paddings that differ from section 6.
10. **Hierarchy guides:** nesting works and the rows indent by level, but the indent guide lines from section 6 are not drawn yet.

11. **File drop from Explorer:** dragging files from the system into the Project panel is off for now (D18). Use the Importar button.
12. **Texture size:** a sprite keeps its size when a texture is assigned, because the image size is only known after decoding. The size is not set from the image yet.

---

## 16. Changing this standard

1. Edit [tokens.css](../editor/src/styles/tokens.css) and, for object colors, [palette.ts](../editor/src/lib/palette.ts).
2. Update the matching table in this document in the same change.
3. Recheck contrast for any text or graphic color you changed (section 3.9 lists the pairs and the rule).
4. Run `pnpm check` in `editor/`.
5. Add a decision row (section 14). Changes that affect file formats, the runtime or the node API also need an ADR, as [CONTRIBUTING](../CONTRIBUTING.md) requires.
