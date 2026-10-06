# 01. Visão do produto

## O problema

Quem quer fazer jogo sem saber programar hoje escolhe entre ferramentas fáceis com teto baixo (Construct, GDevelop) e ferramentas poderosas com curva íngreme (Unity, Unreal, Godot). O Blueprint da Unreal prova que programação visual escala para jogos grandes, mas vem preso a uma engine pesada e voltada a 3D AAA. O GameMaker tem programação visual, mas ela é tratada como cidadã de segunda classe frente ao GML.

## A proposta

Nexo é a engine em que **a programação visual é o caminho principal**, com o mesmo cuidado que outras engines dão às suas linguagens de script. Ela combina:

- a acessibilidade das **folhas de eventos** do Construct e GDevelop ("quando isso acontecer, faça aquilo");
- o poder dos **grafos de nós** do Blueprint (fluxo, funções, variáveis tipadas, reutilização);
- uma ponte para código real: todo comportamento pode ser visto como **C#**.

## Público

| Perfil | O que precisa |
|---|---|
| Iniciante e estudante | Fazer o primeiro jogo em uma tarde, sem travar em sintaxe |
| Artista e game designer | Prototipar mecânicas sozinho, sem depender de programador |
| Dev indie | Lançar jogo 2D completo em PC, web e mobile |
| Educador | Ensinar lógica com algo visual que vira código de verdade |

## Diferenciais

1. **Duas visões, um código.** O mesmo comportamento pode ser editado como folha de eventos ou como grafo. Iniciantes começam na folha e migram para o grafo quando quiserem.
2. **Depuração visual de primeira.** Fios acendem quando o fluxo passa, valores aparecem em cima dos pinos, breakpoints em nós, histórico de execução por frame.
3. **C# ao vivo.** Painel lateral mostra o C# equivalente do grafo selecionado, atualizado a cada alteração.
4. **Editor bonito de verdade.** Design system próprio, animações suaves, tema claro e escuro, comando rápido (Ctrl+K) para tudo.
5. **Leve.** Instalador pequeno graças ao Tauri; abre em segundos.

## O que Nexo não é (por enquanto)

Uma engine 3D AAA. O 3D entra depois de o 2D estar maduro, começando por cenas simples e jogos estilizados.
