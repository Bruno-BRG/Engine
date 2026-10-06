# Project browser and assets

Unity-style Project window over the project folder on disk.

Legend: **P0** next phase · **P1** before 1.0 · **P2** after 1.0. References: **U** Unity, **G** Godot, **GM** GameMaker.

- [ ] **AST-001** (P0) Project panel: folder tree on the left, asset grid on the right, breadcrumbs, size slider (grid ↔ list). *U: Project window, two-column layout.*
- [ ] **AST-002** (P0) Create menu: Folder, Scene, Visual Script, Physics Material, Shape, Input Action, Audio Bus.
- [ ] **AST-003** (P0) Import PNG/JPG/WAV/OGG/TTF with background processing into `.nexo/cache`; thumbnails.
- [ ] **AST-004** (P0) File watcher: hot reload when a file changes outside the editor.
- [ ] **AST-005** (P0) Rename/move keeps references (UIDs in sidecar `.import` files). *G: .import files and UIDs.*
- [ ] **AST-006** (P1) Search by name, type and label; favorites.
- [ ] **AST-007** (P1) Sprite sheet slicing (grid and automatic) and import settings (filter, pixels per unit).
- [ ] **AST-008** (P1) Aseprite import with tags as animations.
- [ ] **AST-009** (P1) Dependency view: where is this asset used.
- [ ] **AST-010** (P2) Asset packages (import/export) and future asset store.
