# 07. Roadmap

Cada fase termina com algo que dá para usar e mostrar.

## Fase 0: Fundação

**Status:** concluída no Windows. Falta testar macOS e Linux.

- Repositório com workspace Cargo e app Tauri 2 + Svelte 5.
- Design system: tokens, componentes base (botão, campo, painel, árvore, menu, toast), tema escuro.
- Shell do editor: painéis encaixáveis, Ctrl+K, tela inicial.
- Prova técnica do viewport: `nexo-runtime` em WASM desenhando sprites via WebGPU dentro do Tauri, a 60 fps, nas três plataformas desktop.

**Pronto quando:** o editor abre em menos de 2 s, mostra um sprite girando no viewport e já parece bonito.

## Fase 1: Cena

- ECS, cena, serialização em RON, assets com UUID.
- Hierarquia, Inspector, gizmos, desfazer/refazer.
- Play/stop no viewport.

**Pronto quando:** dá para montar uma fase com sprites e salvar/abrir sem perda.

## Fase 2: Lógica visual

- Modelo de grafo, editor de grafos em canvas, IR e VM.
- Folhas de eventos sobre o mesmo modelo.
- Física 2D, entrada, áudio, behaviors Plataforma, 8 direções, Sólido, Câmera.
- Painel C# ao vivo.
- Depuração com fios animados e valores nos pinos.

**Pronto quando:** alguém que nunca programou faz um jogo de plataforma com moedas em 30 minutos seguindo o tutorial.

## Fase 3: Primeiro lançamento (alpha pública)

- Exportação desktop e web.
- Tilemap, animação de sprites, partículas, UI do jogo.
- Prefabs com overrides, nós latentes, máquina de estados.
- Três projetos de exemplo e documentação de todos os nós.

**Pronto quando:** um jogo completo pequeno é publicado no itch.io feito só com o Nexo.

## Fase 4: Profundidade (1.0)

- Behaviors do usuário, ejetar para C#, profiler, diff de grafos, luz 2D, pós-processamento, i18n, editor de shaders.

## Fase 5: Expansão

- Scripts C# à mão, mobile, 3D inicial, loja de extensões, multiplayer.
