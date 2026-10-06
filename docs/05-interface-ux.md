# 05. Interface e UX

A meta: a pessoa abrir o Nexo e sentir vontade de criar. Referências de qualidade: Figma, Linear, Arc, Blender 4, Raycast.

## Princípios de design

1. **Calma.** Fundo neutro, cor usada só para significado (seleção, tipos, erros). O jogo é o protagonista.
2. **Rápido de verdade.** Toda ação responde em menos de 100 ms. Animações de 120 a 200 ms, nunca bloqueando a próxima ação.
3. **Descobrível.** Tudo tem nome, atalho e aparece no **Ctrl+K**.
4. **Direto.** Arrastar asset para a cena cria o objeto. Arrastar behavior para o objeto adiciona. Clicar duas vezes num objeto abre sua lógica.
5. **Perdoa erros.** Desfazer ilimitado, salvamento automático, histórico de versões local.

## Layout padrão

```
┌───────────────────────────────────────────────────────────────┐
│ ◆ Nexo   MeuJogo ▾   [Cena] [Lógica] [Animação] [Sprite]  ▶ ⏸ │
├──────────┬─────────────────────────────────────┬──────────────┤
│ Cena     │                                     │ Inspector    │
│ ▸ Mundo  │          Viewport / Grafo           │ Transform    │
│  ▸ Jogador│                                    │ Sprite       │
│  ▸ Moedas│                                     │ Plataforma   │
│          │                                     │ + Behavior   │
├──────────┴─────────────────────────────────────┴──────────────┤
│ Assets   │ Console │ Depurador │ C#                            │
└───────────────────────────────────────────────────────────────┘
```

- **Workspaces** no topo (Cena, Lógica, Animação, Sprite, UI) reorganizam os painéis para cada tarefa, como no Blender.
- Painéis encaixáveis, destacáveis em janela separada (útil com dois monitores) e layouts salvos.
- Modo foco (F11 / Shift+Espaço) esconde tudo menos a área principal.

## Design system

**Tipografia:** Inter para a interface, JetBrains Mono para código e valores. Escala 11/12/13/15/18/24.

**Espaçamento:** grade de 4 px. Painéis com 8 ou 12 px de respiro.

**Cantos:** 6 px em controles, 10 px em painéis e nós, 14 px em diálogos.

**Tema escuro (padrão):**

| Token | Valor | Uso |
|---|---|---|
| bg-0 | #0E0F13 | fundo da janela |
| bg-1 | #15171D | painéis |
| bg-2 | #1C1F27 | campos, nós |
| line | #2A2E39 | bordas |
| text | #E8EAF0 | texto principal |
| text-dim | #8A90A2 | rótulos |
| accent | #7C5CFF | seleção, foco, botão primário |
| success | #3DDC97 | play, ok |
| warning | #FFB84D | avisos |
| danger | #FF5C7A | erros |

Tema claro equivalente e cor de destaque configurável. Ícones em traço único (Lucide como base, ícones próprios para tipos de objeto).

**Movimento:** curvas `cubic-bezier(0.2, 0.8, 0.2, 1)`. Painéis deslizam, menus surgem com leve escala, nós novos "assentam" no grafo. Respeita "reduzir movimento" do sistema.

## Detalhes que fazem diferença

- Campos numéricos com **arrastar para alterar** (scrub), aceitando expressões (`32*4`).
- Seletor de cor com conta-gotas que pega cor do viewport.
- Miniaturas animadas dos sprites no navegador de assets.
- Tela inicial com projetos recentes em cards com captura de tela e modelos ("Plataforma", "Top-down", "Puzzle", "Vazio").
- Notificações discretas (toasts) em vez de diálogos modais.
- Barra de status mostra fps do jogo, memória e estado do hot reload.
- Play abre o jogo **dentro do viewport** em menos de 1 s; alterações feitas durante o play podem ser mantidas ou descartadas ao parar.

## Onboarding

- Tutorial interativo de 10 minutos: criar um personagem que pula e coleta moedas.
- Dicas contextuais na primeira vez que cada painel é aberto.
- Cada nó tem descrição, exemplo animado e link para a documentação.

## Acessibilidade e idioma

- Interface em português e inglês desde o início (i18n com chaves, nada hardcoded).
- Contraste AA, navegação completa por teclado, escala da UI de 80% a 200%.
- Cores de tipos com formas diferentes nos pinos para daltônicos.
