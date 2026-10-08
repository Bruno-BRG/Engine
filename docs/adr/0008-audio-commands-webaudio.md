# ADR 0008: The core emits audio commands, and WebAudio plays them in the editor

**Status:** accepted

## Context

Phase 2 brings audio players (AUD-001), buses with effects and a mixer (AUD-003), and visual nodes that play sound (AUD-004). The Phase 0 docs named kira as the audio library. The runtime runs in WebAssembly inside the editor's webview (ADR 0002, ADR 0006), and that webview already has the WebAudio API. The native export runtime (EXP-001, winit and wgpu) will need its own audio backend later.

## Decision

- `nexo-core` (`audio.rs`) never produces sound. During play it keeps the state of each player (what is playing, its bus, its spatial attenuation) and queues `AudioCommand` values. `SceneEditor::audio_commands()` returns the queue as JSON every frame.
- The engine in `editor/src/lib/audio/engine.ts` executes the commands with WebAudio. Each bus is a chain of gain nodes with the four effects: reverb (a convolver), delay, low-pass filter and compressor.
- The wire format is one object per command. `type` is snake_case, and the fields are camelCase (`rename_all` and `rename_all_fields` in serde). Volumes are decibels, where 0 is the original level and -80 or less is silence. Node and asset identifiers are 16-digit hexadecimal UIDs.
- The engine reports back with `audio_finished(node)`, when a sound ends on its own or cannot play. Without that, the core would think the sound still plays.
- The core computes spatial attenuation for 2D players from the distance to the current camera, and sends `set_attenuation` only when it changes by more than 0.05 dB.
- Bus settings (volume, mute, effects) are saved in `settings.ron`, in snake_case, through the mixer. Bus commands sent during play change the sound, not the file. The solo state is per session and is not saved.

## Consequences

- Sound plays only in the editor for now. The export runtime will need a native backend that handles the same commands. The command type is the seam, so that backend can change (kira is the likely choice) without touching the nodes or the core.
- Core tests cover what the commands say, not what the sound is. Listening checks stay manual.
- The persisted format and the wire format use different casing on purpose: files stay readable, and the TypeScript side matches the wire format exactly. `editor/src/lib/audio/types.ts` documents both.
