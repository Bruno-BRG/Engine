# 02. Pesquisa das principais engines

Resumo do que cada engine faz bem, o que faz mal e o que o Nexo deve trazer.

## Unity

**Pontos fortes:** ecossistema enorme, Inspector flexível, prefabs com variantes e overrides, Asset Store, exportação para quase tudo, C# como linguagem. Visual Scripting (antigo Bolt) e Shader Graph existem, mas são secundários.

**Pontos fracos:** editor lento para abrir e recompilar, interface datada, programação visual com desempenho ruim e pouco integrada, histórico de mudanças polêmicas de licença.

**Trazer:** Inspector com componentes, prefabs com overrides, sistema de pacotes, C# como linguagem de saída, Profiler.

## Unreal Engine

**Pontos fortes:** Blueprints são a referência de programação visual em jogos: eventos, funções, macros, interfaces, variáveis tipadas, comentários, depuração com fluxo animado. Material Editor e Niagara também são nodais. Sequencer para cinemáticas.

**Pontos fracos:** pesada, foco em 3D realista, Blueprints grandes viram "espaguete", difícil comparar versões de grafos no Git.

**Trazer:** modelo de Blueprint (eventos, execução por pinos brancos, dados por pinos coloridos), depuração visual, nós reroute, colapsar seleção em função, busca contextual ao arrastar um fio. **Melhorar:** diff legível (formato texto) e organização automática dos grafos.

## Godot

**Pontos fortes:** leve, open source, tudo é nó e cena, cenas aninhadas como prefabs, sinais (eventos entre nós), editor de animação que anima qualquer propriedade, TileMap muito bom, exportação simples. O VisualScript foi removido no Godot 4 por pouco uso, o que é uma lição importante.

**Pontos fracos:** interface funcional mas sem charme, 3D atrás da concorrência.

**Trazer:** cenas aninhadas, sinais, animação de qualquer propriedade, formato de arquivo em texto. **Lição:** programação visual falha quando é só uma tradução 1:1 do código textual. Ela precisa de abstrações de alto nível (comportamentos prontos, eventos de jogo) para ser mais rápida que digitar.

## GameMaker

**Pontos fortes:** fluxo objeto → eventos (Create, Step, Draw, Collision) muito fácil de entender, editor de sprites e salas embutidos, GML Visual com blocos, exportação 2D excelente.

**Pontos fracos:** GML Visual fica limitado em projetos grandes, interface confusa em alguns painéis.

**Trazer:** modelo de eventos por objeto (Criar, A cada frame, Desenhar, Colisão com X), editor de sprites embutido, salas com camadas.

## Construct 3

**Pontos fortes:** folhas de eventos (condições à esquerda, ações à direita) são a forma mais acessível de lógica já criada para jogos. Behaviors prontos (Plataforma, 8 direções, Bala, Sólido, Arrastar) eliminam 80% do trabalho. Roda no navegador, preview instantâneo, depurador visual.

**Pontos fracos:** 2D apenas, assinatura obrigatória, folhas de eventos longas ficam difíceis de navegar.

**Trazer:** folhas de eventos, behaviors prontos, preview instantâneo, depurador que mostra variáveis e objetos ao vivo.

## GDevelop

**Pontos fortes:** open source, eventos estilo Construct, extensões criadas pela comunidade *com eventos* (comportamentos visuais reutilizáveis), exportação com um clique.

**Trazer:** extensões e behaviors escritos na própria linguagem visual, publicáveis numa loja.

## Outras referências

- **Defold:** runtime minúsculo e builds rápidos.
- **Bevy:** ECS moderno em Rust, inspiração para o núcleo.
- **Figma, Linear, Blender 4:** referências de interface (velocidade, atalhos, painéis flexíveis).
- **Scratch:** blocos para quem nunca programou; inspiração para tooltips e onboarding.

## Matriz de funcionalidades

| Funcionalidade | Unity | Unreal | Godot | GameMaker | Construct | **Nexo** |
|---|---|---|---|---|---|---|
| Programação visual principal | Não | Sim | Não | Parcial | Sim | **Sim** |
| Folhas de eventos | Não | Não | Não | Não | Sim | **Sim** |
| Grafos de nós | Sim | Sim | Removido | Blocos | Não | **Sim** |
| Behaviors prontos | Não | Parcial | Não | Não | Sim | **Sim** |
| Visualização como código | Não | Não | - | Sim (GML) | Não | **C#** |
| Prefabs/cenas aninhadas | Sim | Sim | Sim | Parcial | Parcial | **Sim** |
| Editor de sprites embutido | Não | Não | Não | Sim | Sim | **Sim** |
| Tilemap | Sim | Parcial | Sim | Sim | Sim | **Sim** |
| Animação de qualquer propriedade | Sim | Sim | Sim | Parcial | Parcial | **Sim** |
| Shader nodal | Sim | Sim | Sim | Não | Não | **Fase 4** |
| Exportação web | Sim | Não | Sim | Sim | Sim | **Sim** |
| Editor leve | Não | Não | Sim | Sim | Sim | **Sim** |

## Conclusões

1. Programação visual só funciona quando tem **abstrações de jogo** (behaviors, eventos prontos) e não apenas operações de baixo nível.
2. Iniciantes preferem **eventos**; projetos grandes precisam de **funções e grafos**. Oferecer os dois sobre o mesmo modelo resolve.
3. **Depuração visual** é o que separa uma ferramenta de brinquedo de uma ferramenta profissional.
4. **Formato em texto** e diff legível são obrigatórios para trabalho em equipe.
