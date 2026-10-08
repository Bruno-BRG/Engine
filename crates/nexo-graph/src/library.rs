//! Biblioteca de nós (VIS-021). Cada tipo de nó é uma entrada de dados: o editor lista esta tabela no menu de
//! busca (VIS-011) e o compilador reconhece o nó pelo `kind`. Para acrescentar um nó, basta uma definição aqui
//! e um ramo em `ir.rs`. Os nós que dependem do mundo do jogo (cena, entrada, áudio, física) só descrevem a
//! forma: quem executa é o `Host` (ver `vm.rs`).

use crate::types::{PinType, Value};
use serde::Serialize;
use std::sync::OnceLock;

/// Pino de um nó. `ty` vazio quer dizer "o tipo que o próprio nó decide": o da variável nomeada (GetVariable e
/// SetVariable) ou o do fio que chega (Reroute). `default` vazio torna a entrada obrigatória: sem fio e sem
/// valor embutido, a validação falha.
#[derive(Clone, Debug, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct PinDef {
    pub name: &'static str,
    pub label: &'static str,
    pub ty: Option<PinType>,
    pub default: Option<Value>,
}

/// Um tipo de nó da biblioteca.
#[derive(Clone, Debug, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct NodeDef {
    /// Nome do tipo no grafo e no compilador: "Add", "OnProcess"...
    pub kind: &'static str,
    pub category: &'static str,
    pub label: &'static str,
    pub description: &'static str,
    /// Evento: começa um programa e não tem entrada de fluxo.
    pub event: bool,
    pub inputs: Vec<PinDef>,
    pub outputs: Vec<PinDef>,
}

impl PinDef {
    fn new(name: &'static str, label: &'static str, ty: PinType) -> Self {
        Self {
            name,
            label,
            ty: Some(ty),
            default: None,
        }
    }

    /// Pino cujo tipo segue a variável nomeada pelo próprio nó.
    fn of_variable(name: &'static str, label: &'static str) -> Self {
        Self {
            name,
            label,
            ty: None,
            default: None,
        }
    }

    /// Pino cujo tipo segue o fio que chega ao reroute (VIS-014). Como o de variável, não tem `ty`; a
    /// resolução é por fio, ver `Graph::tipo_do_reroute`.
    fn segue_o_fio(name: &'static str, label: &'static str) -> Self {
        Self {
            name,
            label,
            ty: None,
            default: None,
        }
    }

    /// Valor usado quando o pino não tem fio nem valor embutido.
    fn padrao(mut self, value: Value) -> Self {
        self.default = Some(value);
        self
    }
}

fn exec() -> PinDef {
    PinDef::new("exec", "Fluxo", PinType::Flow)
}

fn saida_fluxo(name: &'static str, label: &'static str) -> PinDef {
    PinDef::new(name, label, PinType::Flow)
}

fn decimal(name: &'static str, label: &'static str, padrao: f64) -> PinDef {
    PinDef::new(name, label, PinType::Float).padrao(Value::Float(padrao))
}

fn inteiro(name: &'static str, label: &'static str, padrao: i64) -> PinDef {
    PinDef::new(name, label, PinType::Int).padrao(Value::Int(padrao))
}

fn booleano(name: &'static str, label: &'static str, padrao: bool) -> PinDef {
    PinDef::new(name, label, PinType::Bool).padrao(Value::Bool(padrao))
}

fn texto(name: &'static str, label: &'static str, padrao: &'static str) -> PinDef {
    PinDef::new(name, label, PinType::String).padrao(Value::String(padrao.to_string()))
}

fn vetor(name: &'static str, label: &'static str, padrao: [f64; 2]) -> PinDef {
    PinDef::new(name, label, PinType::Vec2).padrao(Value::Vec2(padrao))
}

/// Referência a um nó da cena. Sem padrão: precisa de fio ou de id embutido.
fn no(name: &'static str, label: &'static str) -> PinDef {
    PinDef::new(name, label, PinType::Node)
}

/// Monta uma definição de nó. Evita repetir `NodeDef { ... }` em cada entrada da tabela.
fn def(
    kind: &'static str,
    category: &'static str,
    label: &'static str,
    description: &'static str,
    inputs: Vec<PinDef>,
    outputs: Vec<PinDef>,
) -> NodeDef {
    NodeDef {
        kind,
        category,
        label,
        description,
        event: false,
        inputs,
        outputs,
    }
}

fn evento(
    kind: &'static str,
    label: &'static str,
    description: &'static str,
    outputs: Vec<PinDef>,
) -> NodeDef {
    NodeDef {
        kind,
        category: "Eventos",
        label,
        description,
        event: true,
        inputs: vec![],
        outputs,
    }
}

fn build() -> Vec<NodeDef> {
    let decimal_resultado = || vec![PinDef::new("result", "Resultado", PinType::Float)];
    let bool_resultado = || vec![PinDef::new("result", "Resultado", PinType::Bool)];
    let vetor_resultado = || vec![PinDef::new("result", "Resultado", PinType::Vec2)];
    vec![
        // Eventos.
        evento(
            "OnReady",
            "Ao ficar pronto",
            "Roda uma vez, quando o nó entra na cena (On Ready).",
            vec![exec(), PinDef::new("self", "Este nó", PinType::Node)],
        ),
        evento(
            "OnProcess",
            "A cada quadro",
            "Roda a cada quadro (On Process). Fornece o delta, em segundos.",
            vec![
                exec(),
                PinDef::new("delta", "Delta (s)", PinType::Float),
                PinDef::new("self", "Este nó", PinType::Node),
            ],
        ),
        evento(
            "OnPhysicsProcess",
            "A cada quadro de física",
            "Roda a cada passo fixo da física (On Physics Process). Fornece o delta, em segundos.",
            vec![
                exec(),
                PinDef::new("delta", "Delta (s)", PinType::Float),
                PinDef::new("self", "Este nó", PinType::Node),
            ],
        ),
        // Fluxo.
        def(
            "Branch",
            "Fluxo",
            "Se (Branch)",
            "Segue por Verdadeiro ou por Falso, conforme a condição.",
            vec![exec(), booleano("condition", "Condição", false)],
            vec![
                saida_fluxo("true", "Verdadeiro"),
                saida_fluxo("false", "Falso"),
            ],
        ),
        def(
            "Sequence",
            "Fluxo",
            "Sequência",
            "Roda os dois caminhos de saída, um depois do outro.",
            vec![exec()],
            vec![
                saida_fluxo("then_0", "Passo 1"),
                saida_fluxo("then_1", "Passo 2"),
            ],
        ),
        def(
            "ForEach",
            "Fluxo",
            "Para cada (For Each)",
            "Repete o corpo de 0 até count - 1. O índice atual sai no pino Índice. Não há listas ainda.",
            vec![exec(), inteiro("count", "Quantidade", 0)],
            vec![
                saida_fluxo("body", "Corpo"),
                PinDef::new("index", "Índice", PinType::Int),
                saida_fluxo("done", "Fim"),
            ],
        ),
        def(
            "While",
            "Fluxo",
            "Enquanto (While)",
            "Repete o corpo enquanto a condição for verdadeira. Limitado pelo orçamento de passos da VM.",
            vec![exec(), booleano("condition", "Condição", false)],
            vec![saida_fluxo("body", "Corpo"), saida_fluxo("done", "Fim")],
        ),
        def(
            "Wait",
            "Fluxo",
            "Esperar (Wait)",
            "Pausa este caminho pelos segundos dados. O resto do jogo continua rodando.",
            vec![exec(), decimal("seconds", "Segundos", 1.0)],
            vec![saida_fluxo("exec", "Fluxo")],
        ),
        def(
            "Tween",
            "Fluxo",
            "Interpolar propriedade (Tween)",
            "Leva uma propriedade numérica do valor atual até o valor dado, em segundos. Pausa este caminho.",
            vec![
                exec(),
                no("node", "Nó"),
                texto("property", "Propriedade", "position_x"),
                decimal("to", "Até", 0.0),
                decimal("seconds", "Segundos", 1.0),
            ],
            vec![saida_fluxo("exec", "Fluxo")],
        ),
        def(
            "MoveTo",
            "Fluxo",
            "Mover até (Move To)",
            "Interpola a posição do nó até o ponto dado, em segundos. Pausa este caminho.",
            vec![
                exec(),
                no("node", "Nó"),
                vetor("target", "Destino", [0.0, 0.0]),
                decimal("seconds", "Segundos", 1.0),
            ],
            vec![saida_fluxo("exec", "Fluxo")],
        ),
        def(
            "AwaitSignal",
            "Fluxo",
            "Aguardar sinal (Await Signal)",
            "Pausa este caminho até outro caminho emitir o sinal com este nome.",
            vec![exec(), texto("name", "Sinal", "")],
            vec![saida_fluxo("exec", "Fluxo")],
        ),
        def(
            "EmitSignal",
            "Fluxo",
            "Emitir sinal (Emit Signal)",
            "Acorda os caminhos que aguardam o sinal com este nome, no próximo quadro.",
            vec![exec(), texto("name", "Sinal", "")],
            vec![saida_fluxo("exec", "Fluxo")],
        ),
        def(
            "TimerStart",
            "Fluxo",
            "Iniciar temporizador (Timer)",
            "Sai por Iniciado na hora. Depois dos segundos dados, roda o caminho Tempo esgotado.",
            vec![exec(), decimal("seconds", "Segundos", 1.0)],
            vec![
                saida_fluxo("started", "Iniciado"),
                saida_fluxo("timeout", "Tempo esgotado"),
            ],
        ),
        // Matemática.
        def(
            "Add",
            "Matemática",
            "Somar",
            "A + B, em decimal.",
            vec![decimal("a", "A", 0.0), decimal("b", "B", 0.0)],
            decimal_resultado(),
        ),
        def(
            "Subtract",
            "Matemática",
            "Subtrair",
            "A - B, em decimal.",
            vec![decimal("a", "A", 0.0), decimal("b", "B", 0.0)],
            decimal_resultado(),
        ),
        def(
            "Multiply",
            "Matemática",
            "Multiplicar",
            "A * B, em decimal.",
            vec![decimal("a", "A", 1.0), decimal("b", "B", 1.0)],
            decimal_resultado(),
        ),
        def(
            "Divide",
            "Matemática",
            "Dividir",
            "A / B, em decimal. Dividir por zero para o jogo com erro.",
            vec![decimal("a", "A", 0.0), decimal("b", "B", 1.0)],
            decimal_resultado(),
        ),
        // Comparação.
        def(
            "Greater",
            "Comparação",
            "Maior que",
            "Verdadeiro quando A é maior que B.",
            vec![decimal("a", "A", 0.0), decimal("b", "B", 0.0)],
            bool_resultado(),
        ),
        def(
            "Less",
            "Comparação",
            "Menor que",
            "Verdadeiro quando A é menor que B.",
            vec![decimal("a", "A", 0.0), decimal("b", "B", 0.0)],
            bool_resultado(),
        ),
        def(
            "Equal",
            "Comparação",
            "Igual a",
            "Verdadeiro quando A e B são iguais (decimais).",
            vec![decimal("a", "A", 0.0), decimal("b", "B", 0.0)],
            bool_resultado(),
        ),
        // Vetor 2D.
        def(
            "MakeVector2",
            "Vetor 2D",
            "Criar vetor",
            "Monta um vetor 2D com X e Y.",
            vec![decimal("x", "X", 0.0), decimal("y", "Y", 0.0)],
            vetor_resultado(),
        ),
        def(
            "AddVector2",
            "Vetor 2D",
            "Somar vetores",
            "A + B, componente a componente.",
            vec![vetor("a", "A", [0.0, 0.0]), vetor("b", "B", [0.0, 0.0])],
            vetor_resultado(),
        ),
        def(
            "ScaleVector2",
            "Vetor 2D",
            "Escalar vetor",
            "Multiplica o vetor pelo fator.",
            vec![
                vetor("vector", "Vetor", [0.0, 0.0]),
                decimal("scale", "Fator", 1.0),
            ],
            vetor_resultado(),
        ),
        def(
            "VectorLength",
            "Vetor 2D",
            "Comprimento do vetor",
            "Tamanho do vetor, em decimal.",
            vec![vetor("vector", "Vetor", [0.0, 0.0])],
            decimal_resultado(),
        ),
        // Texto e depuração.
        def(
            "Concat",
            "Texto",
            "Concatenar",
            "Junta os dois textos, A seguido de B.",
            vec![texto("a", "A", ""), texto("b", "B", "")],
            vec![PinDef::new("result", "Resultado", PinType::String)],
        ),
        def(
            "Print",
            "Depuração",
            "Imprimir",
            "Escreve um texto no console do editor, em nível informação.",
            vec![exec(), texto("msg", "Texto", "")],
            vec![exec()],
        ),
        def(
            "Warn",
            "Depuração",
            "Avisar",
            "Escreve um aviso no console (nível Warning).",
            vec![exec(), texto("msg", "Texto", "")],
            vec![exec()],
        ),
        def(
            "Error",
            "Depuração",
            "Registrar erro",
            "Escreve um erro no console (nível Error).",
            vec![exec(), texto("msg", "Texto", "")],
            vec![exec()],
        ),
        // Nós da cena.
        def(
            "GetProperty",
            "Propriedades",
            "Ler propriedade",
            "Lê uma propriedade numérica de qualquer nó.",
            vec![no("node", "Nó"), texto("name", "Propriedade", "")],
            vec![PinDef::new("value", "Valor", PinType::Float)],
        ),
        def(
            "SetProperty",
            "Propriedades",
            "Definir propriedade",
            "Grava uma propriedade numérica de qualquer nó.",
            vec![
                exec(),
                no("node", "Nó"),
                texto("name", "Propriedade", ""),
                decimal("value", "Valor", 0.0),
            ],
            vec![exec()],
        ),
        def(
            "GetNode",
            "Nó",
            "Obter nó pelo nome",
            "Acha um nó da cena pelo nome. Sem resultado, sai a referência vazia (id 0).",
            vec![texto("name", "Nome", "")],
            vec![no("node", "Nó")],
        ),
        def(
            "InstanceScene",
            "Nó",
            "Instanciar cena",
            "Cria uma cópia da cena no ponto dado. Sai com a referência do nó criado.",
            vec![
                exec(),
                texto("scene", "Cena", ""),
                vetor("position", "Posição", [0.0, 0.0]),
            ],
            vec![saida_fluxo("exec", "Fluxo"), no("instance", "Instância")],
        ),
        def(
            "Destroy",
            "Nó",
            "Destruir nó",
            "Remove o nó da cena.",
            vec![exec(), no("node", "Nó")],
            vec![exec()],
        ),
        // Entrada (INP-003).
        def(
            "ActionPressed",
            "Entrada",
            "Ação pressionada",
            "Verdadeiro enquanto a ação de entrada está pressionada.",
            vec![texto("action", "Ação", "")],
            vec![PinDef::new("pressed", "Pressionada", PinType::Bool)],
        ),
        def(
            "ActionJustPressed",
            "Entrada",
            "Ação acabou de ser pressionada",
            "Verdadeiro só no quadro em que a ação foi pressionada.",
            vec![texto("action", "Ação", "")],
            vec![PinDef::new(
                "pressed",
                "Acabou de ser pressionada",
                PinType::Bool,
            )],
        ),
        def(
            "ActionReleased",
            "Entrada",
            "Ação soltada",
            "Verdadeiro só no quadro em que a ação foi solta.",
            vec![texto("action", "Ação", "")],
            vec![PinDef::new("released", "Soltada", PinType::Bool)],
        ),
        def(
            "GetAxis",
            "Entrada",
            "Eixo",
            "-1 se a ação negativa está pressionada, 1 se a positiva, 0 com nenhuma ou ambas.",
            vec![
                texto("negative", "Negativa", ""),
                texto("positive", "Positiva", ""),
            ],
            vec![PinDef::new("value", "Valor", PinType::Float)],
        ),
        def(
            "GetVector",
            "Entrada",
            "Vetor de entrada",
            "Vetor das quatro ações, com comprimento no máximo 1.",
            vec![
                texto("left", "Esquerda", ""),
                texto("right", "Direita", ""),
                texto("up", "Cima", ""),
                texto("down", "Baixo", ""),
            ],
            vetor_resultado(),
        ),
        def(
            "MousePositionWorld",
            "Entrada",
            "Posição do mouse (mundo)",
            "Posição do ponteiro no mundo do jogo.",
            vec![],
            vec![PinDef::new("position", "Posição", PinType::Vec2)],
        ),
        def(
            "MousePositionScreen",
            "Entrada",
            "Posição do mouse (tela)",
            "Posição do ponteiro na tela, em pixels.",
            vec![],
            vec![PinDef::new("position", "Posição", PinType::Vec2)],
        ),
        // Corpo físico (host).
        def(
            "MoveAndSlide",
            "Física",
            "Mover e deslizar",
            "Move o corpo com a velocidade dada e para nas paredes. Sai dizendo se está no chão.",
            vec![
                exec(),
                no("node", "Corpo"),
                vetor("velocity", "Velocidade", [0.0, 0.0]),
            ],
            vec![exec(), PinDef::new("on_floor", "No chão", PinType::Bool)],
        ),
        // Áudio (AUD-004).
        def(
            "PlaySound",
            "Áudio",
            "Tocar som",
            "Toca o som no barramento dado, com o volume dado.",
            vec![
                exec(),
                texto("sound", "Som", ""),
                texto("bus", "Barramento", "Master"),
                decimal("volume", "Volume", 1.0),
            ],
            vec![exec()],
        ),
        def(
            "StopSound",
            "Áudio",
            "Parar som",
            "Para o som dado.",
            vec![exec(), texto("sound", "Som", "")],
            vec![exec()],
        ),
        def(
            "FadeSound",
            "Áudio",
            "Fade do som",
            "Leva o volume do som ao valor dado, em segundos.",
            vec![
                exec(),
                texto("sound", "Som", ""),
                decimal("volume", "Volume", 0.0),
                decimal("seconds", "Segundos", 1.0),
            ],
            vec![exec()],
        ),
        def(
            "SetBusVolume",
            "Áudio",
            "Volume do barramento",
            "Define o volume de um barramento de áudio.",
            vec![
                exec(),
                texto("bus", "Barramento", "Master"),
                decimal("volume", "Volume", 1.0),
            ],
            vec![exec()],
        ),
        // Variáveis.
        def(
            "GetVariable",
            "Variáveis",
            "Ler variável",
            "Lê uma variável do script. O nome é fixo no nó.",
            vec![texto("name", "Variável", "")],
            vec![PinDef::of_variable("value", "Valor")],
        ),
        def(
            "SetVariable",
            "Variáveis",
            "Definir variável",
            "Grava uma variável do script. O nome é fixo no nó.",
            vec![
                exec(),
                texto("name", "Variável", ""),
                PinDef::of_variable("value", "Valor"),
            ],
            vec![exec()],
        ),
        // Organização do canvas (VIS-014).
        def(
            "Reroute",
            "Organização",
            "Ponto de passagem (Reroute)",
            "Passa o fio adiante sem mudar nada. O tipo é o do fio que chega.",
            vec![PinDef::segue_o_fio("in", "Entrada")],
            vec![PinDef::segue_o_fio("out", "Saída")],
        ),
        // Funções (VIS-026). Entradas e saídas passam por variáveis do script de nome combinado.
        def(
            "Function",
            "Funções",
            "Função",
            "Define uma função com este nome. O corpo começa na saída Corpo. Não roda sozinha: use Chamar função.",
            vec![texto("name", "Nome", "")],
            vec![saida_fluxo("exec", "Corpo")],
        ),
        def(
            "CallFunction",
            "Funções",
            "Chamar função",
            "Roda o corpo da função com este nome no lugar da chamada. Quando o corpo termina, a chamada segue.",
            vec![exec(), texto("name", "Nome", "")],
            vec![exec()],
        ),
        def(
            "Return",
            "Funções",
            "Retornar",
            "Encerra a função atual. O que viria depois deste nó no corpo não roda.",
            vec![exec()],
            vec![],
        ),
        // Estados (VIS-027). O primeiro estado do grafo é o inicial.
        estado_evento(
            "State",
            "Estado",
            "Roda a cada quadro enquanto este estado está ativo.",
        ),
        estado_evento(
            "StateEnter",
            "Ao entrar no estado",
            "Roda uma vez quando o estado com este nome passa a ser o ativo.",
        ),
        estado_evento(
            "StateExit",
            "Ao sair do estado",
            "Roda uma vez quando o estado com este nome deixa de ser o ativo.",
        ),
        def(
            "GoToState",
            "Estados",
            "Ir para estado",
            "Troca o estado ativo. Sai o estado antigo, entra o novo.",
            vec![exec(), texto("state", "Estado", "")],
            vec![exec()],
        ),
    ]
}

/// Evento de estado: tem o nome do estado como valor embutido, e não tem entrada de fluxo.
fn estado_evento(kind: &'static str, label: &'static str, description: &'static str) -> NodeDef {
    NodeDef {
        kind,
        category: "Estados",
        label,
        description,
        event: true,
        inputs: vec![texto("state", "Estado", "")],
        outputs: vec![exec(), no("self", "Este nó")],
    }
}

/// Todos os tipos de nó, na ordem em que aparecem no menu.
pub fn library() -> &'static [NodeDef] {
    static LIB: OnceLock<Vec<NodeDef>> = OnceLock::new();
    LIB.get_or_init(build)
}

/// Definição do tipo de nó com o `kind` dado.
pub fn find(kind: &str) -> Option<&'static NodeDef> {
    library().iter().find(|d| d.kind == kind)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn biblioteca_serializa_para_o_editor() {
        assert_eq!(library().len(), 55);
        assert!(find("Branch").is_some_and(|d| !d.event));
        assert!(find("OnProcess").is_some_and(|d| d.event));
        assert!(find("OnPhysicsProcess").is_some_and(|d| d.event));
        assert!(find("Nada").is_none());
        // O editor lê esta tabela como JSON.
        let json = serde_json::to_value(library()).unwrap();
        assert_eq!(json[0]["kind"], "OnReady");
    }

    #[test]
    fn nomes_de_nos_sao_unicos() {
        let mut kinds: Vec<&str> = library().iter().map(|d| d.kind).collect();
        kinds.sort_unstable();
        let total = kinds.len();
        kinds.dedup();
        assert_eq!(kinds.len(), total, "há tipos de nó repetidos");
    }

    #[test]
    fn valores_padrao_batem_com_o_tipo_do_pino() {
        for def in library() {
            for pin in def.inputs.iter().chain(def.outputs.iter()) {
                if let (Some(ty), Some(value)) = (pin.ty, &pin.default) {
                    assert_eq!(value.ty(), ty, "{}.{}", def.kind, pin.name);
                }
            }
        }
    }
}
