//! Geração de C# a partir da IR (VIS-024, ADR 0003). Função pura: recebe o `Compiled` e devolve o texto. Não
//! depende do editor nem do runtime, e a saída é determinística (mesma IR, mesmo texto), por isso pode ir para
//! o painel de C# e para o controle de versão.
//!
//! Cada programa vira um método. Os registradores viram variáveis locais declaradas no começo, e cada op vira
//! uma instrução. Os saltos da IR viram `goto` com rótulos `L<n>`: a reconstrução de `if` e `while` estruturados
//! não é feita aqui. As chamadas `Nexo.*` são a API do motor em C#; as pausas latentes viram `await`.

use crate::ir::{AudioOp, Compiled, Op, Program};
use crate::types::{LogLevel, PinType, Value, Variable};
use std::collections::BTreeSet;
use std::fmt::Write;

/// Palavras reservadas de C#. Nomes de variável que coincidem com elas saem com `@` na frente.
const RESERVADAS: &[&str] = &[
    "abstract",
    "as",
    "base",
    "bool",
    "break",
    "byte",
    "case",
    "catch",
    "char",
    "checked",
    "class",
    "const",
    "continue",
    "decimal",
    "default",
    "delegate",
    "do",
    "double",
    "else",
    "enum",
    "event",
    "explicit",
    "extern",
    "false",
    "finally",
    "fixed",
    "float",
    "for",
    "foreach",
    "goto",
    "if",
    "implicit",
    "in",
    "int",
    "interface",
    "internal",
    "is",
    "lock",
    "long",
    "namespace",
    "new",
    "null",
    "object",
    "operator",
    "out",
    "override",
    "params",
    "private",
    "protected",
    "public",
    "readonly",
    "ref",
    "return",
    "sbyte",
    "sealed",
    "short",
    "sizeof",
    "stackalloc",
    "static",
    "string",
    "struct",
    "switch",
    "this",
    "throw",
    "true",
    "try",
    "typeof",
    "uint",
    "ulong",
    "unchecked",
    "unsafe",
    "ushort",
    "using",
    "virtual",
    "void",
    "volatile",
    "while",
];

/// Gera a classe C# do script. `class_name` precisa ser um identificador C# válido (quem chama garante).
pub fn generate_csharp(compiled: &Compiled, class_name: &str) -> String {
    let mut out = String::new();
    out.push_str("// Gerado pelo Nexo a partir da IR do script. Não edite à mão: a próxima geração sobrescreve.\n");
    out.push_str("using System.Threading.Tasks;\n");
    out.push_str("using Nexo;\n\n");
    let _ = writeln!(out, "public partial class {class_name} : NexoScript");
    out.push_str("{\n");
    for var in &compiled.variables {
        campo(&mut out, var);
    }
    for programa in &compiled.programs {
        out.push('\n');
        metodo(&mut out, compiled, programa);
    }
    out.push_str("}\n");
    out
}

/// Variável do script como campo. As exportadas aparecem no Inspector (`[Export]`), as outras são privadas.
fn campo(out: &mut String, var: &Variable) {
    let acesso = if var.exported {
        "    [Export] public "
    } else {
        "    private "
    };
    let _ = writeln!(
        out,
        "{acesso}{} {} = {};",
        tipo(var.ty),
        ident(&var.name),
        literal(&var.default)
    );
}

/// Um programa como método. Métodos com pausa latente são `async Task`.
fn metodo(out: &mut String, compiled: &Compiled, programa: &Program) {
    let assincrono = programa.ops.iter().any(latente);
    let retorno = if assincrono { "async Task" } else { "void" };
    let parametros = if matches!(programa.event.as_str(), "OnProcess" | "OnPhysicsProcess") {
        "NodeRef self, double delta"
    } else {
        "NodeRef self"
    };
    let _ = writeln!(
        out,
        "    public {retorno} {}_{}({parametros})",
        programa.event, programa.node
    );
    out.push_str("    {\n");
    for (r, ty) in tipos_dos_registradores(compiled, programa)
        .iter()
        .enumerate()
    {
        let _ = writeln!(out, "        {} r{r} = default;", tipo(*ty));
    }
    let alvos: BTreeSet<usize> = programa.ops.iter().filter_map(alvo).collect();
    for (i, op) in programa.ops.iter().enumerate() {
        if alvos.contains(&i) {
            let _ = writeln!(out, "        L{i}: {}", instrucao(compiled, op));
        } else {
            let _ = writeln!(out, "        {}", instrucao(compiled, op));
        }
    }
    if alvos.contains(&programa.ops.len()) {
        let _ = writeln!(out, "        L{}: ;", programa.ops.len());
    }
    out.push_str("    }\n");
}

/// Tipo de cada registrador do programa, pela op que o grava. Registrador sem escrita fica como decimal.
fn tipos_dos_registradores(compiled: &Compiled, programa: &Program) -> Vec<PinType> {
    let mut tipos = vec![PinType::Float; programa.regs];
    for op in &programa.ops {
        let (dst, ty) = match op {
            Op::Const { dst, value } => (*dst, value.ty()),
            Op::Owner { dst } | Op::FindNode { dst, .. } | Op::InstanceScene { dst, .. } => {
                (*dst, PinType::Node)
            }
            Op::Delta { dst }
            | Op::Add { dst, .. }
            | Op::Sub { dst, .. }
            | Op::Mul { dst, .. }
            | Op::Div { dst, .. }
            | Op::VecLength { dst, .. }
            | Op::GetAxis { dst, .. } => (*dst, PinType::Float),
            // Propriedade lida: o registrador tem o tipo da propriedade (VIS-021), não decimal fixo.
            Op::GetProp { dst, ty, .. } => (*dst, *ty),
            Op::GetVar { dst, var } => (
                *dst,
                compiled
                    .variables
                    .get(*var)
                    .map_or(PinType::Float, |v| v.ty),
            ),
            Op::Greater { dst, .. }
            | Op::Less { dst, .. }
            | Op::Equal { dst, .. }
            | Op::IntLess { dst, .. }
            | Op::ActionPressed { dst, .. }
            | Op::ActionJustPressed { dst, .. }
            | Op::ActionReleased { dst, .. }
            | Op::MoveAndSlide { dst, .. } => (*dst, PinType::Bool),
            Op::IntAdd { dst, .. } => (*dst, PinType::Int),
            Op::MakeVec { dst, .. }
            | Op::AddVec { dst, .. }
            | Op::ScaleVec { dst, .. }
            | Op::GetVector { dst, .. }
            | Op::MouseWorld { dst }
            | Op::MouseScreen { dst } => (*dst, PinType::Vec2),
            Op::Concat { dst, .. } => (*dst, PinType::String),
            _ => continue,
        };
        if let Some(slot) = tipos.get_mut(dst) {
            *slot = ty;
        }
    }
    tipos
}

/// Instrução C# de uma op. Os registradores viram `r<n>`.
fn instrucao(c: &Compiled, op: &Op) -> String {
    match op {
        Op::Const { dst, value } => format!("r{dst} = {};", literal(value)),
        Op::Owner { dst } => format!("r{dst} = self;"),
        Op::Delta { dst } => format!("r{dst} = delta;"),
        Op::GetVar { dst, var } => format!("r{dst} = {};", nome_variavel(c, *var)),
        Op::SetVar { var, src } => format!("{} = r{src};", nome_variavel(c, *var)),
        Op::Add { dst, a, b }
        | Op::IntAdd { dst, a, b }
        | Op::AddVec { dst, a, b }
        | Op::Concat { dst, a, b } => {
            format!("r{dst} = r{a} + r{b};")
        }
        Op::Sub { dst, a, b } => format!("r{dst} = r{a} - r{b};"),
        Op::Mul { dst, a, b } => format!("r{dst} = r{a} * r{b};"),
        Op::Div { dst, a, b } => format!("r{dst} = Nexo.Div(r{a}, r{b});"),
        Op::Greater { dst, a, b } => format!("r{dst} = r{a} > r{b};"),
        Op::Less { dst, a, b } | Op::IntLess { dst, a, b } => format!("r{dst} = r{a} < r{b};"),
        Op::Equal { dst, a, b } => format!("r{dst} = r{a} == r{b};"),
        Op::MakeVec { dst, x, y } => format!("r{dst} = new Vector2(r{x}, r{y});"),
        Op::ScaleVec { dst, vector, scale } => format!("r{dst} = r{vector} * r{scale};"),
        Op::VecLength { dst, vector } => format!("r{dst} = r{vector}.Length();"),
        Op::Log { level, msg } => format!("Nexo.Log(LogLevel.{}, r{msg});", nivel(*level)),
        Op::ActionPressed { dst, action } => format!("r{dst} = Nexo.ActionPressed(r{action});"),
        Op::ActionJustPressed { dst, action } => {
            format!("r{dst} = Nexo.ActionJustPressed(r{action});")
        }
        Op::ActionReleased { dst, action } => format!("r{dst} = Nexo.ActionReleased(r{action});"),
        Op::GetAxis {
            dst,
            negative,
            positive,
        } => format!("r{dst} = Nexo.GetAxis(r{negative}, r{positive});"),
        Op::GetVector {
            dst,
            left,
            right,
            up,
            down,
        } => format!("r{dst} = Nexo.GetVector(r{left}, r{right}, r{up}, r{down});"),
        Op::MouseWorld { dst } => format!("r{dst} = Nexo.MouseWorld();"),
        Op::MouseScreen { dst } => format!("r{dst} = Nexo.MouseScreen();"),
        Op::GetProp {
            dst,
            node,
            name,
            ty,
        } => format!(
            "r{dst} = Nexo.GetProperty<{}>(r{node}, r{name});",
            tipo(*ty)
        ),
        Op::SetProp {
            node, name, value, ..
        } => {
            format!("Nexo.SetProperty(r{node}, r{name}, r{value});")
        }
        Op::FindNode { dst, name } => format!("r{dst} = Nexo.FindNode(r{name});"),
        Op::InstanceScene {
            dst,
            scene,
            position,
        } => {
            format!("r{dst} = Nexo.InstanceScene(r{scene}, r{position});")
        }
        Op::Destroy { node } => format!("Nexo.Destroy(r{node});"),
        Op::MoveAndSlide {
            dst,
            node,
            velocity,
        } => {
            format!("r{dst} = Nexo.MoveAndSlide(r{node}, r{velocity});")
        }
        Op::Audio(comando) => match comando {
            AudioOp::Play {
                node,
                stream,
                volume_db,
                pitch,
                bus,
                looping,
                from_start,
            } => {
                format!(
                    "Nexo.Audio.Play(r{node}, r{stream}, r{bus}, r{volume_db}, r{pitch}, r{looping}, r{from_start});"
                )
            }
            AudioOp::Stop { node } => format!("Nexo.Audio.Stop(r{node});"),
            AudioOp::Fade {
                node,
                to_db,
                seconds,
            } => {
                format!("Nexo.Audio.Fade(r{node}, r{to_db}, r{seconds});")
            }
            AudioOp::SetBus { bus, volume_db } => {
                format!("Nexo.Audio.SetBusVolume(r{bus}, r{volume_db});")
            }
        },
        Op::Jump { target } => format!("goto L{target};"),
        Op::JumpIfFalse { cond, target } => format!("if (!r{cond}) goto L{target};"),
        Op::Wait { seconds } => format!("await Nexo.Wait(r{seconds});"),
        Op::Tween {
            node,
            name,
            to,
            seconds,
        } => format!("await Nexo.Tween(r{node}, r{name}, r{to}, r{seconds});"),
        Op::MoveTo {
            node,
            target,
            seconds,
        } => format!("await Nexo.MoveTo(r{node}, r{target}, r{seconds});"),
        Op::AwaitSignal { name } => format!("await Nexo.AwaitSignal(r{name});"),
        Op::EmitSignal { name } => format!("Nexo.EmitSignal(r{name});"),
        // O caminho do temporizador começa em L<entry> depois de `seconds`. Em C# gerado, isso ainda não vira
        // uma corrotina separada: fica como chamada e comentário, para quem lê ver a intenção.
        Op::Spawn { seconds, entry } => {
            format!(
                "Nexo.StartTimer(r{seconds}); // o caminho que começa em L{entry} roda ao esgotar o tempo"
            )
        }
        Op::GoToState { name } => format!("Nexo.GoToState(r{name});"),
    }
}

/// Rótulo de salto de uma op, se ela salta.
fn alvo(op: &Op) -> Option<usize> {
    match op {
        Op::Jump { target } | Op::JumpIfFalse { target, .. } => Some(*target),
        Op::Spawn { entry, .. } => Some(*entry),
        _ => None,
    }
}

/// Op que pausa a tarefa até outro evento ou tempo.
fn latente(op: &Op) -> bool {
    matches!(
        op,
        Op::Wait { .. } | Op::Tween { .. } | Op::MoveTo { .. } | Op::AwaitSignal { .. }
    )
}

fn nivel(level: LogLevel) -> &'static str {
    match level {
        LogLevel::Info => "Info",
        LogLevel::Warning => "Warning",
        LogLevel::Error => "Error",
    }
}

fn nome_variavel(c: &Compiled, var: usize) -> String {
    c.variables
        .get(var)
        .map_or_else(|| format!("v{var}"), |v| ident(&v.name))
}

fn tipo(ty: PinType) -> &'static str {
    match ty {
        PinType::Flow | PinType::Bool => "bool",
        PinType::Int => "long",
        PinType::Float => "double",
        PinType::String => "string",
        PinType::Vec2 => "Vector2",
        PinType::Node => "NodeRef",
    }
}

fn literal(v: &Value) -> String {
    match v {
        Value::Bool(b) => b.to_string(),
        Value::Int(n) => format!("{n}L"),
        Value::Float(f) => float_literal(*f),
        Value::String(s) => format!("\"{}\"", escapa(s)),
        Value::Vec2([x, y]) => format!("new Vector2({}, {})", float_literal(*x), float_literal(*y)),
        Value::Node(id) => format!("NodeRef.From({id}UL)"),
    }
}

fn float_literal(f: f64) -> String {
    if f.is_nan() {
        "double.NaN".into()
    } else if f.is_infinite() {
        if f > 0.0 {
            "double.PositiveInfinity".into()
        } else {
            "double.NegativeInfinity".into()
        }
    } else {
        // O Debug do Rust dá "3.0" para 3, que é literal válido em C#.
        format!("{f:?}")
    }
}

fn escapa(s: &str) -> String {
    let mut out = String::with_capacity(s.len());
    for ch in s.chars() {
        match ch {
            '\\' => out.push_str("\\\\"),
            '"' => out.push_str("\\\""),
            '\n' => out.push_str("\\n"),
            '\r' => out.push_str("\\r"),
            '\t' => out.push_str("\\t"),
            c if c.is_control() => {
                let _ = write!(out, "\\u{:04x}", c as u32);
            }
            c => out.push(c),
        }
    }
    out
}

/// Identificador C# a partir do nome do Nexo: troca o que não é letra, dígito ou `_`, e protege palavras
/// reservadas com `@`.
fn ident(nome: &str) -> String {
    let mut s: String = nome
        .chars()
        .map(|ch| {
            if ch.is_alphanumeric() || ch == '_' {
                ch
            } else {
                '_'
            }
        })
        .collect();
    if s.is_empty() || s.starts_with(|c: char| c.is_ascii_digit()) {
        s.insert(0, '_');
    }
    if RESERVADAS.contains(&s.as_str()) {
        s.insert(0, '@');
    }
    s
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::graph::Graph;
    use crate::ir::compile;
    use crate::types::PinType;

    fn adiciona(g: &mut Graph, kind: &str) -> u64 {
        g.add_node(kind, [0.0, 0.0]).unwrap()
    }

    /// Script com uma variável exportada, um OnReady que espera e um OnProcess que move o nó.
    fn script() -> Compiled {
        let mut g = Graph::default();
        g.add_variable("velocidade", PinType::Float, Value::Float(3.0), true)
            .unwrap();
        g.add_variable("class", PinType::Int, Value::Int(0), false)
            .unwrap();
        let ev = adiciona(&mut g, "OnReady");
        let w = adiciona(&mut g, "Wait");
        g.set_value(w, "seconds", Value::Float(0.5)).unwrap();
        g.connect((ev, "exec"), (w, "exec")).unwrap();
        let p = adiciona(&mut g, "Print");
        g.set_value(p, "msg", Value::String("oi \"mundo\"".into()))
            .unwrap();
        g.connect((w, "exec"), (p, "exec")).unwrap();
        let ep = adiciona(&mut g, "OnProcess");
        let gv = adiciona(&mut g, "GetVariable");
        g.set_value(gv, "name", Value::String("velocidade".into()))
            .unwrap();
        let sp = adiciona(&mut g, "SetProperty");
        g.set_value(sp, "name", Value::String("position_x".into()))
            .unwrap();
        g.connect((ep, "exec"), (sp, "exec")).unwrap();
        g.connect((ep, "self"), (sp, "node")).unwrap();
        g.connect((gv, "value"), (sp, "value")).unwrap();
        compile(&g).unwrap()
    }

    #[test]
    fn csharp_contem_classe_e_variaveis_exportadas() {
        let texto = generate_csharp(&script(), "Jogador");
        assert!(
            texto.contains("public partial class Jogador : NexoScript"),
            "{texto}"
        );
        assert!(
            texto.contains("[Export] public double velocidade = 3.0;"),
            "{texto}"
        );
        // "class" é palavra reservada em C#: sai protegida.
        assert!(texto.contains("private long @class = 0L;"), "{texto}");
    }

    #[test]
    fn csharp_e_deterministico() {
        let c = script();
        assert_eq!(
            generate_csharp(&c, "Jogador"),
            generate_csharp(&c, "Jogador")
        );
    }

    #[test]
    fn metodo_latente_e_assincrono_e_texto_e_escapado() {
        let texto = generate_csharp(&script(), "Jogador");
        assert!(
            texto.contains("public async Task OnReady_1(NodeRef self)"),
            "{texto}"
        );
        assert!(texto.contains("await Nexo.Wait(r"), "{texto}");
        assert!(texto.contains("\"oi \\\"mundo\\\"\""), "{texto}");
        assert!(texto.contains("public void OnProcess_"), "{texto}");
        assert!(texto.contains("NodeRef self, double delta"), "{texto}");
    }

    #[test]
    fn saltos_viram_goto_com_rotulo_no_destino() {
        let mut g = Graph::default();
        let ev = adiciona(&mut g, "OnReady");
        let br = adiciona(&mut g, "Branch");
        g.set_value(br, "condition", Value::Bool(true)).unwrap();
        let p = adiciona(&mut g, "Print");
        g.set_value(p, "msg", Value::String("sim".into())).unwrap();
        g.connect((ev, "exec"), (br, "exec")).unwrap();
        g.connect((br, "true"), (p, "exec")).unwrap();
        let texto = generate_csharp(&compile(&g).unwrap(), "Teste");
        assert!(texto.contains("if (!r0) goto L"), "{texto}");
        assert!(texto.contains("goto L"), "{texto}");
    }

    #[test]
    fn nome_de_variavel_invalido_vira_identificador() {
        assert_eq!(ident("vida total"), "vida_total");
        assert_eq!(ident("2x"), "_2x");
        assert_eq!(ident("int"), "@int");
    }

    #[test]
    fn typed_property_has_register_of_own_type() {
        let mut g = Graph::default();
        let ev = adiciona(&mut g, "OnReady");
        let gp = adiciona(&mut g, "GetProperty");
        g.set_value(gp, "type", Value::String("Sprite2D".into())).unwrap();
        g.set_value(gp, "name", Value::String("modulate".into())).unwrap();
        let pr = adiciona(&mut g, "Print");
        g.connect((ev, "exec"), (pr, "exec")).unwrap();
        g.connect((ev, "self"), (gp, "node")).unwrap();
        g.connect((gp, "value"), (pr, "msg")).unwrap();
        let texto = generate_csharp(&compile(&g).unwrap(), "Teste");
        // O registrador da leitura é texto, e a chamada pede texto: não é mais decimal fixo.
        assert!(
            texto.lines().any(|l| l.trim_start().starts_with("string r")),
            "{texto}"
        );
        assert!(texto.contains("Nexo.GetProperty<string>(r"), "{texto}");
        assert!(!texto.contains("double r2 = default;"), "{texto}");
    }
}
