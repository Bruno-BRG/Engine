# Contributing to Nexo

Thanks for helping build Nexo. This guide covers setup, conventions and how work flows from the backlog to a merged pull request.

## Setup

1. Install Rust (stable), then `rustup target add wasm32-unknown-unknown`.
2. Install `wasm-bindgen-cli` with the exact version pinned in `Cargo.toml`: `cargo install wasm-bindgen-cli --version 0.2.129 --locked`.
3. Install Node 20+ and pnpm.
4. Windows: Visual Studio Build Tools with "Desktop development with C++" and WebView2. Linux: the Tauri system packages (WebKitGTK). macOS: Xcode command line tools.
5. `cd editor && pnpm install && pnpm dev`.

## Everyday commands

| Command | What it does |
|---|---|
| `pnpm dev` | Desktop app with hot reload (rebuilds the WASM runtime first) |
| `pnpm dev:web` | UI in the browser; projects are kept in localStorage |
| `pnpm check` | Svelte + TypeScript type check |
| `cargo test --workspace` | Rust tests |
| `cargo clippy --workspace` | Rust lints |
| `pnpm wasm` | Rebuild only the WASM runtime |

## Picking work

All planned work lives in [docs/backlog](docs/backlog/README.md), one file per topic. Each item has an ID (for example `PHY-012`), a priority (P0/P1/P2) and a reference to how Unity, Godot or GameMaker solve it.

1. Pick an unchecked item, preferably P0 in the current phase.
2. Open an issue or draft PR named after the ID: `PHY-012: CollisionShape2D gizmo`.
3. Check the box in the backlog file in the same PR that completes it.

## Code style

- **Rust:** `rustfmt` defaults, no `unwrap()` in editor/runtime paths that can fail at runtime; return `Result` with a readable message.
- **TypeScript/Svelte:** Svelte 5 runes, strict TypeScript, Prettier (single quotes, 120 columns).
- **UI text:** never hardcode user-facing strings in components once i18n lands (EDT-040); use translation keys.
- **Design:** follow the tokens in `editor/src/styles/tokens.css` and [docs/05-editor-ui.md](docs/05-editor-ui.md). No new colors or radii without updating the tokens.
- **Language:** code, comments, commit messages and docs in English.

## Commits and pull requests

- Conventional commits: `feat(editor): ...`, `fix(render): ...`, `docs: ...`, `refactor(core): ...`.
- Keep PRs focused on one backlog item or one coherent change.
- The PR description says what changed for the user, how it was tested, and includes a screenshot or GIF for any UI change.
- CI must pass: `pnpm check`, `cargo test`, `cargo clippy -- -D warnings`.

## Architecture decisions

Changes that affect the engine's structure (file formats, runtime model, public node API) need an ADR in [docs/adr](docs/adr/). Copy the format of an existing ADR: context, decision, consequences.

## Reporting bugs

Include the Nexo version (status bar), OS, the graphics backend shown in the status bar (WebGPU/WebGL2), steps to reproduce and, when possible, the project folder or the scene `.ron` file.
