# Other references

## Unreal Engine (Blueprints)

The reference for visual programming at scale: events, functions, macros, interfaces, typed variables, white execution pins and colored data pins, wire search filtered by pin type, collapse to function, execution flow animation while debugging. Weak spots: "spaghetti" graphs and hard-to-diff binary assets. Nexo keeps the model and fixes diffing (text format) and organization (auto layout).

## Construct 3

Event sheets (conditions on the left, actions on the right) are the most accessible logic format for games. Ready-made behaviors (Platform, 8 Direction, Bullet, Solid, Drag & Drop) remove most of the work. Nexo: event sheets as an alternative view (VIS-028), behaviors as built-in node types.

## GDevelop

Open source, Construct-style events, extensions built with events and shared in a store. Nexo: reusable behaviors published from scripts (VIS-029).

## Defold

Tiny runtime and fast builds; a good bar for export size.

## Bevy

Modern Rust ECS; inspiration for Nexo's internal runtime storage.

## UI references beyond engines

Figma, Linear and Blender 4 for speed, shortcuts and panel management.
