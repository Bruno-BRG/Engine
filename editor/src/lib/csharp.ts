// Gera C# legível a partir da cena. Na Fase 2 o gerador passa a partir do IR da lógica visual
// (crate nexo-csharp); aqui ele já mostra a forma da API que os nós vão expor.

import type { NodeView, Outline } from './state/scene.svelte';

const SHAPES: Record<string, string> = { rect: 'RoundRect', circle: 'Circle', diamond: 'Diamond' };

export function className(name: string) {
  const clean = name
    .normalize('NFD')
    .replace(/[\u0300-\u036f]/g, '')
    .replace(/[^A-Za-z0-9 ]+/g, ' ')
    .trim()
    .split(/\s+/)
    .map((w) => w.charAt(0).toUpperCase() + w.slice(1))
    .join('');
  return /^[A-Za-z_]/.test(clean) ? clean : 'No' + clean;
}

const f = (n: number) => {
  const r = Math.round(n * 100) / 100;
  return (Number.isInteger(r) ? r.toFixed(0) : String(r)) + 'f';
};

const hex = (c: string) => c.toUpperCase().replace('#', '');
const num = (v: unknown) => (typeof v === 'number' ? v : 0);
const vec = (v: unknown): [number, number] => {
  const a = Array.isArray(v) ? v : [0, 0];
  return [num(a[0]), num(a[1])];
};

/** Classe C# para um nó. Comportamentos são os nós filhos do tipo Behavior. */
export function nodeToCSharp(n: NodeView, all: NodeView[]): string {
  const cls = className(n.name);
  const p = n.props;
  const [px, py] = vec(p.position);
  const [sw, sh] = vec(p.size);
  const behaviors = n.children
    .map((id) => all.find((x) => x.uid === id))
    .filter((x): x is NodeView => !!x && x.kind === 'Behavior');
  const kindOf = (b: NodeView) => (b.props.kind as string) ?? 'spin';
  const bob = behaviors.find((b) => kindOf(b) === 'float');
  const pulse = behaviors.find((b) => kindOf(b) === 'pulse');
  const L: string[] = [];
  L.push('using Nexo;');
  L.push('');
  L.push('// Gerado pelo Nexo a partir do nó "' + n.name + '". Somente leitura.');
  L.push('public partial class ' + cls + ' : NexoBehaviour');
  L.push('{');
  if (bob) L.push('    private float _baseY;');
  if (pulse) L.push('    private Vector2 _baseSize;');
  if (bob || pulse) L.push('');
  L.push('    protected override void OnStart()');
  L.push('    {');
  L.push('        Transform.Position = new Vector2(' + f(px) + ', ' + f(py) + ');');
  if (n.kind === 'Sprite2D') {
    L.push('        Transform.Size = new Vector2(' + f(sw) + ', ' + f(sh) + ');');
    L.push('        Sprite.Shape = Shape.' + (SHAPES[(p.shape as string) ?? 'rect'] ?? 'RoundRect') + ';');
    L.push('        Sprite.Color = Color.FromHex("' + hex(String(p.modulate ?? '#ffffff')) + '");');
  }
  L.push('        Transform.Rotation = ' + f(num(p.rotation)) + ';');
  if (p.visible === false) L.push('        Visible = false;');
  if (bob) L.push('        _baseY = Transform.Position.Y;');
  if (pulse) L.push('        _baseSize = Transform.Size;');
  L.push('    }');
  if (behaviors.length) {
    L.push('');
    L.push('    protected override void OnUpdate(float dt)');
    L.push('    {');
    behaviors.forEach((b, i) => {
      if (i) L.push('');
      const speed = num(b.props.speed);
      if (kindOf(b) === 'spin') {
        L.push('        // Behavior: Girar');
        L.push('        Transform.Rotation += ' + f(speed) + ' * dt;');
      } else if (kindOf(b) === 'float') {
        L.push('        // Behavior: Flutuar');
        L.push('        Transform.Position.Y = _baseY + MathF.Sin(Time.Elapsed * ' + f(speed) + ' * MathF.Tau) * 14f;');
      } else {
        L.push('        // Behavior: Pulsar');
        L.push('        var scale = 1f + MathF.Sin(Time.Elapsed * ' + f(speed) + ' * MathF.Tau) * 0.08f;');
        L.push('        Transform.Size = _baseSize * scale;');
      }
    });
    L.push('    }');
  }
  L.push('}');
  return L.join('\n');
}

/** Classe C# da cena: cria um nó para cada filho direto da raiz. */
export function sceneToCSharp(s: Outline): string {
  const L: string[] = [];
  const roots = s.nodes.filter((n) => n.parent == null);
  L.push('using Nexo;');
  L.push('');
  L.push('// Gerado pelo Nexo a partir da cena "' + s.name + '". Somente leitura.');
  L.push('public partial class ' + className(s.name) + 'Scene : NexoScene');
  L.push('{');
  L.push('    public override Vector2 Screen => new(' + f(s.screen[0]) + ', ' + f(s.screen[1]) + ');');
  L.push('');
  L.push('    protected override void Build()');
  L.push('    {');
  const children = roots.flatMap((r) => r.children.map((id) => s.nodes.find((n) => n.uid === id)).filter((n): n is NodeView => !!n));
  if (!children.length) L.push('        // A cena ainda não tem nós filhos.');
  for (const n of children) if (n.kind !== 'Behavior') L.push('        Spawn<' + className(n.name) + '>();');
  L.push('    }');
  L.push('}');
  return L.join('\n');
}

const KEYWORDS = new Set(['using', 'public', 'private', 'partial', 'class', 'protected', 'override', 'void', 'float', 'new', 'var', 'return', 'if', 'else', 'false', 'true', 'this']);

export type Token = { t: 'kw' | 'type' | 'str' | 'num' | 'com' | 'fn' | 'txt'; v: string };

/** Destaque de sintaxe simples para o painel C#. */
export function highlight(line: string): Token[] {
  const out: Token[] = [];
  const re = /(\/\/.*$)|("(?:[^"\\]|\\.)*")|(\b\d+(?:\.\d+)?f?\b)|([A-Za-z_][A-Za-z0-9_]*)|(\s+)|(.)/g;
  let m: RegExpExecArray | null;
  while ((m = re.exec(line))) {
    if (m[1]) out.push({ t: 'com', v: m[1] });
    else if (m[2]) out.push({ t: 'str', v: m[2] });
    else if (m[3]) out.push({ t: 'num', v: m[3] });
    else if (m[4]) {
      const w = m[4];
      const next = line[re.lastIndex];
      out.push({ t: KEYWORDS.has(w) ? 'kw' : next === '(' ? 'fn' : /^[A-Z]/.test(w) ? 'type' : 'txt', v: w });
    } else out.push({ t: 'txt', v: m[0] });
  }
  return out;
}
