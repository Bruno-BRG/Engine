# Visual scripting

Nexo's core. GameMaker workflow (scripts as assets made of events), Blueprint-style graphs, IR + VM, generated C#. Spec: [04-visual-programming.md](../04-visual-programming.md).

Legend: **P0** next phase · **P1** before 1.0 · **P2** after 1.0. References: **U** Unity, **G** Godot, **GM** GameMaker.

## Logic workspace (GameMaker style)

- [x] **VIS-001** (P0) Script list panel: Script 1, Script 2... create, rename, duplicate, delete, folders; also listed in the Project browser. *GM: Asset Browser.* *Status (Fase 2): feito: lista de scripts com criar, renomear, duplicar e excluir. Os scripts ficam em scripts/ e aparecem também no painel Projeto.*
- [ ] **VIS-002** (P0) Events panel per script: add event menu (On Ready, On Process, On Physics Process, On Collision with..., On Input Action, On Signal, On Draw, On Destroy, On Timer). *GM: Object events Create/Step/Collision/Draw.* *Status (Fase 2): parcial: OnReady, OnProcess e OnPhysicsProcess disparam na reprodução, além dos eventos de estado. Ficam desabilitados no menu Adicionar evento: colisão, entrada, sinal, desenho, destruir e temporizador. As ligações de sinal (SCN-034) já são salvas no nó emissor, no formato 3, com o nome do método no alvo: falta o evento Ao receber sinal e o despacho.*
- [x] **VIS-003** (P0) "Used by" list: which nodes use the script; attach to selected node. *Status (Fase 2): feito: o painel Usado por lista os nós que usam o script, e o botão anexa o script ao nó selecionado.*
- [x] **VIS-004** (P0) Keep the Phase 0 look: dotted background, animated dashed execution wires. *Status (Fase 2): feito: fundo pontilhado e fios de execução tracejados animados. Não verificado no navegador.*

## Canvas

- [x] **VIS-010** (P0) Infinite canvas: pan with middle mouse, Space+drag, or left-drag on empty background; zoom with wheel around cursor; Home to frame all. *Status (Fase 2): feito: pan com botão do meio, Espaço ou arrastar o fundo; zoom com a roda em torno do cursor; Home enquadra tudo.*
- [x] **VIS-011** (P0) Right-click opens the node search menu (categories + fuzzy search, EN/PT); dropping a wire on empty space opens it filtered by pin type. *Status (Fase 2): feito: menu com categorias e busca difusa em PT e EN; soltar um fio em espaço vazio abre a busca já filtrada pelo tipo do pino.*
- [x] **VIS-012** (P0) Box selection, move, delete, copy/paste (also between scripts), undo/redo. *Status (Fase 2): feito: seleção por caixa, mover, excluir, copiar e colar (também entre scripts), desfazer e refazer.*
- [x] **VIS-013** (P0) Typed pins with colors and shapes; connection validation with clear errors. *Status (Fase 2): feito: pinos tipados com cores e formas; ligação recusada mostra o motivo e a sugestão no toast, no título do pino e no aviso do nó, inclusive saída com saída, entrada com entrada e entrada obrigatória sem fio. O realce do arrasto usa a mesma regra da ligação (tipos, entrada ocupada, ciclo). Soltar um fio numa recusa mantém o fio antigo. Verificado: `cargo test --workspace` (282 passam), clippy dos crates tocados, svelte-check, e o arrasto no navegador com eventos de ponteiro disparados por JS num harness temporário (Tauri falso). Não verificado: arrasto com mouse real, captura visual do realce (conferido pelo estilo computado), tema claro, pinos de reroute, desfazer da troca de fio; `store.connect` não tem teste automatizado, porque o TS não tem runner.*
- [ ] **VIS-014** (P1) Comment boxes, reroute nodes, auto-align, minimap, semantic zoom. *Status (Fase 2): parcial: caixas de comentário e nós de reroute na interface. Faltam alinhamento automático e minimapa.*
- [ ] **VIS-015** (P1) Collapse selection to function; macros. *Status (Fase 2): parcial: colapsar a seleção em função nas fronteiras de fluxo. Macros pendentes.*

## Language

- [x] **VIS-020** (P0) Graph model in `nexo-graph` with serialization (`scripts/*.graph.ron`). *Status (Fase 2): feito em `nexo-graph`: fios, tipos de pino, remoção de nós e fios, ids estáveis (não reaproveitados), caixas de comentário, reroute, e validação completa (`Graph::validate`) que a compilação também usa.*
- [ ] **VIS-021** (P0) Node library v1: flow (Branch, Sequence, For Each, While), math, compare, Vector2, string, Print, Get/Set property of any node (from type metadata), Get Node, Instance Scene, Destroy, Input action pressed, Move and Slide, Play sound, Timer start. *Status (Fase 2): parcial: 54 nós da biblioteca, com fluxo (For Each, While, Branch, Sequence, Wait, Tween, Move To, Await Signal, Timer), matemática, comparação, vetor 2D, texto, Print/Warn/Error, entrada, Get Node, Instance Scene, Destroy, Move and Slide, áudio e funções/estados. Get/Set de propriedade lê e grava só decimais: falta metadado de tipo por nó para outros tipos.*
- [ ] **VIS-022** (P0) Variables (script-level, exported to Inspector, local) with types and defaults. *Status (Fase 2): parcial: variáveis com tipo, padrão validado e marcação de exportação no modelo. Falta mostrá-las no Inspector (interface).*
- [x] **VIS-023** (P0) Compile to IR, run in `nexo-vm`, hot reload during play. *Status (Fase 2): feito: o IR roda a cada quadro pelo Host (host.rs) e recarrega a quente durante o Jogar. Ao Jogar, o editor lê os scripts anexados e chama set_scripts.*
- [ ] **VIS-024** (P0) C# generation from IR, shown in a side panel. *Status (Fase 2): parcial: `generate_csharp` em `nexo-graph/src/csharp.rs`, função pura e determinística sobre a IR. Saltos viram `goto`. Falta o painel (interface).*
- [x] **VIS-025** (P1) Latent nodes: Wait, Tween, Move To, Await signal (coroutines in IR).
- [ ] **VIS-026** (P1) Functions with inputs/outputs; custom signals; calling other scripts. *Status (Fase 2): parcial: sinais (emitir e aguardar) e funções com retorno, embutidas no lugar da chamada; entradas e saídas passam por variáveis de nome combinado. Faltam pinos de entrada e saída por função e chamar outro script pelo nome.*
- [x] **VIS-027** (P1) State machine graph.
- [ ] **VIS-028** (P2) Event sheets (Construct style) over the same model.
- [ ] **VIS-029** (P2) Publish a script as a reusable behavior with Inspector parameters.

## Debugging

- [ ] **VIS-030** (P1) Live execution highlight on wires; pin value on hover. *Status (Fase 2): parcial: o realce segue os fios de fluxo que de fato rodaram, não o nó de origem, então um ramo não tomado fica apagado. O rastro é um resumo por leitura (nós, fios percorridos e último valor de cada nó, texto cortado em 64 caracteres), lido a 10 Hz só com o Depurador observando. O fio quente anima em 0,4 s e apaga em 300 ms. O valor no hover já existia e não mudou. Verificado: testes em Rust (`nexo-graph`: laço de 1.000 voltas com 3 nós e 2 fios, ramo falso, ramo verdadeiro vazio, cadeia por Sequence, fim de programa, chamada de função, rastro desligado, corte de texto; `nexo-runtime-web`: formato do JSON) e, num teste em Node, a lógica de realce do store. Não verificado: o realce no canvas e o valor no hover no app, porque o store de Lógica exige o app desktop. Limites: ids reaproveitados (DBG-003) podem acender outro nó por até 300 ms; a entrada de um While acende a cada volta; fios de dados não acendem.*
- [ ] **VIS-031** (P1) Breakpoints, step, instance picker. *Status (Fase 2): implementado, sem verificação no app desktop (os scripts exigem o Tauri, então não rodam no navegador). Pontos de parada (F9) por instância, Continuar, Próxima operação (uma operação, que cruza eventos e quadros) e seletor de instância, ligados ao Rust por `set_breakpoints`, `debug_continue`, `debug_step`, `debug_state` e `debug_trace`. Pausa só a instância: o jogo segue. Testes em Rust cobrem o agendador, a VM e a pausa e o rastro da instância (scripts.rs); as exportações wasm (debug.rs) e a interface não têm teste automatizado.*
- [ ] **VIS-032** (P2) Per-frame execution history.
