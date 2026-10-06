# Nexo Engine

> Nome provisório. Uma game engine 2D/3D focada em **programação visual**, com um editor bonito o bastante para dar vontade de abrir todo dia.

Nexo é uma engine desktop construída com **Tauri 2** (editor) e **Rust** (núcleo e runtime). A forma principal de criar lógica de jogo é visual: grafos de nós e folhas de eventos que representam o mesmo código por baixo. Quando for preciso ver ou exportar a lógica como texto, ela é traduzida para **C#**.

## Princípios

1. **Visual primeiro.** Tudo que um jogo precisa deve ser possível sem digitar código.
2. **Bonito e rápido.** A interface é produto, não detalhe. 60 fps no editor, resposta instantânea a cada clique.
3. **Ver o resultado na hora.** Play-in-editor com hot reload da lógica visual, sem recompilar.
4. **Sem caixa-preta.** Cada grafo pode ser visto como C# legível. Quem aprende no visual aprende a programar.
5. **Começar pequeno.** 2D excelente antes de 3D ambicioso.

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
