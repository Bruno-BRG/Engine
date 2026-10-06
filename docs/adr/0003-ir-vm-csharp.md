# ADR 0003: Grafos compilam para IR; C# é saída gerada

**Status:** aceito

## Contexto

A lógica visual precisa de hot reload instantâneo e rodar em desktop e web. O usuário quer C# como representação textual.

## Decisão

Grafos e folhas de eventos compilam para um IR tipado executado pela `nexo-vm` em Rust. Um gerador produz C# legível a partir do mesmo IR. Scripts C# escritos à mão chegam numa fase posterior, sobre a mesma API.

## Consequências

- Hot reload sem compilador externo e sem runtime .NET nos jogos.
- Exportação web e mobile simples.
- A API da engine precisa ser desenhada desde o início pensando em C# (nomes, tipos, eventos), para que o código gerado seja natural.
