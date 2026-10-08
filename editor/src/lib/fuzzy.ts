// Busca difusa usada pela paleta de comandos e pelo menu de nós da Lógica. Fica num módulo próprio para
// que a Lógica não precise importar os comandos (que importam as ações e o resto do editor).

/** Busca difusa simples, sem acentos. Retorna -1 quando não casa. */
export function fuzzyScore(query: string, text: string): number {
  const norm = (s: string) => s.normalize('NFD').replace(/[̀-ͯ]/g, '').toLowerCase();
  const q = norm(query.trim());
  const s = norm(text);
  if (!q) return 0;
  const direct = s.indexOf(q);
  if (direct >= 0) return 1000 - direct;
  let score = 0;
  let pos = 0;
  let streak = 0;
  for (const ch of q) {
    if (ch === ' ') continue;
    const i = s.indexOf(ch, pos);
    if (i < 0) return -1;
    streak = i === pos ? streak + 1 : 0;
    score += 10 + streak * 5 - (i - pos);
    pos = i + 1;
  }
  return score;
}
