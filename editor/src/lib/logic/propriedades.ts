// Propriedades do registro para os nós de propriedade da lógica (VIS-021): as opções de tipo e de nome que a
// interface mostra, e o padrão de cada propriedade. As regras (o tipo do pino, a validação) estão em nexo-graph;
// aqui só se monta a lista que o usuário escolhe.

import type { NodeType, PropDef } from '$lib/state/scene.svelte';
import type { Value } from './model';

/** Nós que leem ou gravam uma propriedade do registro. Os pinos "type" e "name" deles são escolhas. */
export const NOS_DE_PROPRIEDADE = ['GetProperty', 'SetProperty'];

export const ehNoDePropriedade = (kind: string): boolean => NOS_DE_PROPRIEDADE.includes(kind);

/** Propriedades de um tipo: as dele e as herdadas (cadeia `base`), na ordem do registro. Sem repetir nomes. */
export function propriedadesDoTipo(tipos: NodeType[], tipo: string): PropDef[] {
  const out: PropDef[] = [];
  const vistos = new Set<string>();
  let atual = tipos.find((t) => t.name === tipo);
  while (atual) {
    for (const p of atual.props) {
      if (!vistos.has(p.name)) {
        vistos.add(p.name);
        out.push(p);
      }
    }
    const pai: string | null = atual.base;
    atual = pai ? tipos.find((t) => t.name === pai) : undefined;
  }
  return out;
}

/** Propriedade de um tipo pelo nome, ou undefined quando o tipo não a tem. */
export function propriedadeDoTipo(tipos: NodeType[], tipo: string, nome: string): PropDef | undefined {
  return propriedadesDoTipo(tipos, tipo).find((p) => p.name === nome);
}

/**
 * Valor padrão de uma propriedade no formato do pino (o mesmo que o grafo usa quando o pino não tem valor).
 * Null quando a propriedade não tem pino (lista de pontos).
 */
export function valorPadrao(def: PropDef): Value | null {
  switch (def.kind) {
    case 'float':
      return { Float: Number(def.default) };
    case 'int':
    case 'mask':
      return { Int: Math.round(Number(def.default)) };
    case 'bool':
      return { Bool: def.default === true };
    case 'str':
    case 'enum':
    case 'color':
    case 'uid':
      return { String: typeof def.default === 'string' ? def.default : '' };
    case 'vec2': {
      const par = Array.isArray(def.default) ? def.default : [];
      return { Vec2: [Number(par[0] ?? 0), Number(par[1] ?? 0)] };
    }
    default:
      return null;
  }
}
