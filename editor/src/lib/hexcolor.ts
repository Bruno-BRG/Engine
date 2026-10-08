// Check of the hex text typed in the Inspector color field (INS-003). Same rule as `normalize_hex` in
// crates/nexo-core/src/scene.rs: a `#` followed by 3, 6 or 8 hexadecimal digits. The engine refuses anything else,
// so the field must not send it.

const HEX_COLOR = /^#(?:[0-9a-fA-F]{3}|[0-9a-fA-F]{6}|[0-9a-fA-F]{8})$/;

/** The color text without the spaces around it, or null when the engine would refuse it. */
export function hexColorOrNull(text: string): string | null {
  const t = text.trim();
  return HEX_COLOR.test(t) ? t : null;
}
