# GameMaker

Reference for the **logic workflow**: objects that contain events, scripts as assets in a browser, approachable visual code.

## Version context

- LTS 2026.0 ([blog](https://gamemaker.io/en/blog/lts-2026-release), [release notes](https://releases.gamemaker.io/release-notes/2026/0)): package manager, new code editor, visual UI editing with UI layers and flex panels, visual particle editor.
- 2026.100 adds horizontal and vertical workspace splits ([release notes](https://releases.gamemaker.io/release-notes/2026/100)).
- The new GMRT runtime is in beta; the roadmap includes JavaScript/TypeScript and C# as interoperable languages ([spring 2026 update](https://gamemaker.io/en/blog/update-spring-2026)).
- GML Visual still exists with no major redesign announced.

## Event model

Each object has a list of **Events**: **Create** runs once when the instance is created, **Step** runs every frame, **Collision** runs when colliding with a specific object. Each event holds GML Code or GML Visual. (The source page for this summary came back truncated in the first pass; confirm with the manual and add the link, DOC-004.)

## What Nexo copies

- Workspace with an **asset browser** listing scripts (Script 1, Script 2...).
- Script editor showing the **events list**; selecting an event opens its code.
- Free navigation on the workspace canvas (pan and zoom), which the Phase 0 Logic preview lacks.
- Right-click to add things.

Nexo mapping: [visual-scripting](../backlog/visual-scripting.md) VIS-001, VIS-002, VIS-010, VIS-011.

## What we do better

- Node graphs with typed pins instead of stacked blocks, plus live debugging on the graph.
- Generated C# for every script.
