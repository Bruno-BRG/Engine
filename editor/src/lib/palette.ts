// Paleta de cores dos objetos do projeto (conteúdo, não interface).
// Mesmos tons das famílias de nó e dos tipos de pino em styles/tokens.css: mantenha os dois alinhados.
// Ordem importa: o índice 0 é a cor padrão de objeto novo. Padrão em docs/08-ux-ui-standard.md.
export const OBJECT_PALETTE = [
  '#4f8fd6', // 0 azul (padrão)
  '#4fb0a5', // 1 turquesa
  '#6bbf59', // 2 verde
  '#e5c152', // 3 amarelo
  '#e08a3c', // 4 laranja
  '#c9606e', // 5 vermelho
  '#9a7fd1', // 6 violeta
  '#9a9a9a', // 7 cinza
  '#d2d2d2', // 8 branco
] as const;

export const DEFAULT_OBJECT_COLOR = OBJECT_PALETTE[0];
