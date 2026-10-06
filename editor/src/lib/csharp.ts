// Gera C# legível a partir da cena. Na Fase 2 o gerador passa a partir do IR da lógica visual
// (crate nexo-csharp); aqui ele já mostra a forma da API que os nós vão expor.
import type { SceneObject, SceneData } from './state/scene.svelte';

const SHAPES = ['RoundRect', 'Circle', 'Diamond'];

export function className(name: string) {
  const clean = name
    .normalize('NFD')
    .replace(/[\u0300-\u036f]/g, '')
    .replace(/[^A-Za-z0-9 ]+/g, ' ')
    .trim()
    .split(/\s+/)
    .map((w) => w.charAt(0).toUpperCase() + w.slice(1))
    .join('');
  return /^[A-Za-z_]/.test(clean) ? clean : 'Objeto' + clean;
}

const f = (n: number) => {
  const r = Math.round(n * 100) / 100;
  return (Number.isInteger(r) ? r.toFixed(0) : String(r)) + 'f';
};

export function objectToCSharp(o: SceneObject): string {
  const cls = className(o.name);
  const L: string[] = [];
  L.push('using Nexo;');
  L.push('');
  L.push('// Gerado pelo Nexo a partir do objeto "' + o.name + '". Somente leitura.');
  L.push('public partial class ' + cls + ' : NexoBehaviour');
  L.push('{');
  const bob = o.behaviors.find((b) => b.kind === 'flutuar');
  const pulse = o.behaviors.find((b) => b.kind === 'pulsar');
  if (bob) L.push('    private float _baseY;');
  if (pulse) L.push('    private Vector2 _baseSize;');
  if (bob || pulse) L.push('');
  L.push('    protected override void OnStart()');
  L.push('    {');
  L.push('        Transform.Position = new Vector2(' + f(o.x) + ', ' + f(o.y) + ');');
  L.push('        Transform.Size = new Vector2(' + f(o.w) + ', ' + f(o.h) + ');');
  L.push('        Transform.Rotation = ' + f(o.rotation) + ';');
  L.push('        Sprite.Shape = Shape.' + SHAPES[o.shape] + ';');
  L.push('        Sprite.Color = Color.FromHex("' + o.color.toUpperCase() + '");');
  if (!o.visible) L.push('        Visible = false;');
  if (bob) L.push('        _baseY = Transform.Position.Y;');
  if (pulse) L.push('        _baseSize = Transform.Size;');
  L.push('    }');
  if (o.behaviors.length) {
    L.push('');
    L.push('    protected override void OnUpdate(float dt)');
    L.push('    {');
    o.behaviors.forEach((b, i) => {
      if (i) L.push('');
      if (b.kind === 'girar') {
        L.push('        // Behavior: Girar');
        L.push('        Transform.Rotation += ' + f(b.speed) + ' * dt;');
      } else if (b.kind === 'flutuar') {
        L.push('        // Behavior: Flutuar');
        L.push('        Transform.Position.Y = _baseY + MathF.Sin(Time.Elapsed * ' + f(b.speed) + ' * MathF.Tau) * 14f;');
      } else {
        L.push('        // Behavior: Pulsar');
        L.push('        var scale = 1f + MathF.Sin(Time.Elapsed * ' + f(b.speed) + ' * MathF.Tau) * 0.08f;');
        L.push('        Transform.Size = _baseSize * scale;');
      }
    });
    L.push('    }');
  }
  L.push('}');
  return L.join('\n');
}

export function sceneToCSharp(s: SceneData): string {
  const L: string[] = [];
  L.push('using Nexo;');
  L.push('');
  L.push('// Gerado pelo Nexo a partir da cena "' + s.name + '". Somente leitura.');
  L.push('public partial class ' + className(s.name) + 'Scene : NexoScene');
  L.push('{');
  L.push('    public override Vector2 Screen => new(' + f(s.screen[0]) + ', ' + f(s.screen[1]) + ');');
  L.push('');
  L.push('    protected override void Build()');
  L.push('    {');
  if (!s.objects.length) L.push('        // A cena ainda não tem objetos.');
  for (const o of s.objects) L.push('        Spawn<' + className(o.name) + '>();');
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
