# Nexo Engine

> Nome provisório. Uma game engine 2D/3D focada em **programação visual**, com um editor bonito o bastante para dar vontade de abrir todo dia.

Nexo é uma engine desktop construída com **Tauri 2** (editor) e **Rust** (núcleo e runtime). A forma principal de criar lógica de jogo é visual: grafos de nós e folhas de eventos que representam o mesmo código por baixo. Quando for preciso ver ou exportar a lógica como texto, ela é traduzida para **C#**.

## Princípios

1. **Visual primeiro.** Tudo que um jogo precisa deve ser possível sem digitar código.
2. **Bonito e rápido.** A interface é produto, não detalhe. 60 fps no editor, resposta instantânea a cada clique.
3. **Ver o resultado na hora.** Play-in-editor com hot reload da lógica visual, sem recompilar.
4. **Sem caixa-preta.** Cada grafo pode ser visto como C# legível. Quem aprende no visual aprende a programar.
5. **Começar pequeno.** 2D excelente antes de 3D ambicioso.

## Rodando o editor

Pré-requisitos: Rust (com o target `wasm32-unknown-unknown`), `wasm-bindgen-cli` na mesma versão do crate (0.2.129), Node 20+ e pnpm. No Windows também é preciso o Visual Studio Build Tools com C++ e o WebView2 (já vem no Windows 11).

```powershell
rustup target add wasm32-unknown-unknown
cargo install wasm-bindgen-cli --version 0.2.129 --locked
cd editor
pnpm install
pnpm dev          # app desktop (Tauri) com hot reload
pnpm dev:web      # só a interface, no navegador em http://localhost:1420
pnpm build        # instalador do app desktop
```

O comando `dev` compila o runtime para WebAssembly antes de subir a interface. No modo navegador os projetos ficam no `localStorage`, o que serve para testar a interface sem o app desktop.

## Estado atual (Fase 0)

- Editor desktop em Tauri 2 com barra de título própria, tela inicial com modelos e projetos recentes.
- Viewport rodando o runtime Rust em WASM com WebGPU (WebGL2 como reserva): grade, moldura da tela do jogo, sprites procedurais com sombra e contorno de seleção.
- Hierarquia com filtro, renomear, ocultar, travar e reordenar arrastando; Inspector com campos de arrastar valor e expressões (`32*4`); cores; behaviors Girar, Flutuar e Pulsar.
- Jogar/parar dentro do viewport com Depurador de valores ao vivo; painel C# gerado ao vivo a partir do objeto selecionado.
- Paleta de comandos (Ctrl+K), atalhos, desfazer/refazer ilimitado, salvamento automático em `cenas/principal.scene.ron`, tema claro e escuro.

## Documentação

| Documento | Conteúdo |
|---|---|
| [01 Visão do produto](docs/01-visao.md) | Público, proposta de valor, diferenciais |
| [02 Pesquisa de engines](docs/02-pesquisa-engines.md) | Unity, Unreal, Godot, GameMaker, Construct 3, GDevelop e o que trazer de cada |
| [03 Arquitetura](docs/03-arquitetura.md) | Tauri, Rust, renderização, ECS, formato de projeto, exportação |
| [04 Programação visual](docs/04-programacao-visual.md) | Modelo de grafos, folhas de eventos, IR, VM, geração de C# |
| [05 Interface e UX](docs/05-interface-ux.md) | Design system, layout do editor, interações, acessibilidade |
| [06 Funcionalidades](docs/06-funcionalidades.md) | Lista completa de módulos e prioridade de cada um |
| [07 Roadmap](docs/07-roadmap.md) | Fases, entregas e critérios de pronto |
| [Decisões (ADRs)](docs/adr/) | Registro das decisões técnicas e seus motivos |

## Estrutura planejada do repositório

```
nexo/
├─ editor/            # App Tauri: frontend (SvelteKit + TS) e backend (Rust)
│  ├─ src/            # UI do editor
│  └─ src-tauri/      # Comandos Tauri, ponte com o núcleo
├─ crates/
│  ├─ nexo-core/      # ECS, cena, assets, tipos base
│  ├─ nexo-render/    # Renderização com wgpu (nativo e WebGPU)
│  ├─ nexo-graph/     # Modelo de grafos, validação, compilação para IR
│  ├─ nexo-vm/        # Máquina virtual que executa o IR
│  ├─ nexo-csharp/    # Gerador de C# a partir do IR
│  ├─ nexo-physics/   # Integração com Rapier 2D/3D
│  ├─ nexo-audio/     # Áudio (kira)
│  └─ nexo-runtime/   # Player standalone usado nos jogos exportados
├─ docs/
└─ examples/          # Projetos de exemplo (plataforma, top-down, puzzle)
```
