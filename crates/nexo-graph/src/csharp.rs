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

/// Generates the C# class of a script. `class_name` must be a valid C# identifier: `csharp_class_name` builds it
/// from the script name.
pub fn generate_csharp(compiled: &Compiled, class_name: &str) -> String {
    let mut out = String::new();
    out.push_str("// Gerado pelo Nexo a partir da IR do script. Não edite à mão: a próxima geração sobrescreve.\n");
    out.push_str("using System.Threading.Tasks;\n");
    out.push_str("using Nexo;\n\n");
    let _ = writeln!(out, "public partial class {class_name} : NexoScript");
    out.push_str("{\n");
    let names = field_names(compiled, class_name);
    for (var, name) in compiled.variables.iter().zip(&names) {
        campo(&mut out, var, name);
    }
    for programa in &compiled.programs {
        out.push('\n');
        metodo(&mut out, compiled, programa, &names);
    }
    out.push_str("}\n");
    out
}

/// C# class name of a script (VIS-024). It follows the identifier rule: "Script 1" becomes `Script_1`, and the
/// generated file is that name plus `.cs`.
pub fn csharp_class_name(script: &str) -> String {
    ident(script)
}

/// C# name of each variable, in the order of `compiled.variables`. Names that become the same identifier ("vida
/// total" and "vida_total") get a numeric suffix. The class name and the generated methods are reserved first: a
/// field cannot have the name of its enclosing class (CS0542) or of a method (CS0102).
fn field_names(compiled: &Compiled, class_name: &str) -> Vec<String> {
    let mut used = BTreeSet::from([class_name.to_string()]);
    used.extend(compiled.programs.iter().map(method_name));
    let mut names = Vec::with_capacity(compiled.variables.len());
    for var in &compiled.variables {
        let base = ident(&var.name);
        let mut name = base.clone();
        let mut number = 2;
        while !used.insert(name.clone()) {
            name = format!("{base}_{number}");
            number += 1;
        }
        names.push(name);
    }
    names
}

/// Variável do script como campo. As exportadas aparecem no Inspector (`[Export]`), as outras são privadas.
fn campo(out: &mut String, var: &Variable, name: &str) {
    let acesso = if var.exported {
        "    [Export] public "
    } else {
        "    private "
    };
    let _ = writeln!(
        out,
        "{acesso}{} {name} = {};",
        tipo(var.ty),
        literal(&var.default)
    );
}

/// Name of the C# method of a program: its event and the id of the event node, as in `OnReady_1`.
fn method_name(program: &Program) -> String {
    format!("{}_{}", program.event, program.node)
}

/// Um programa como método. Métodos com pausa latente são `async Task`.
fn metodo(out: &mut String, compiled: &Compiled, programa: &Program, names: &[String]) {
    let assincrono = programa.ops.iter().any(latente);
    let retorno = if assincrono { "async Task" } else { "void" };
    let parametros = if matches!(programa.event.as_str(), "OnProcess" | "OnPhysicsProcess") {
        "NodeRef self, double delta"
    } else {
        "NodeRef self"
    };
    let _ = writeln!(
        out,
        "    public {retorno} {}({parametros})",
        method_name(programa)
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
            let _ = writeln!(out, "        L{i}: {}", instrucao(names, op));
        } else {
            let _ = writeln!(out, "        {}", instrucao(names, op));
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
            | Op::GetAxis { dst, .. }
            | Op::GetProp { dst, .. } => (*dst, PinType::Float),
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
fn instrucao(names: &[String], op: &Op) -> String {
    match op {
        Op::Const { dst, value } => format!("r{dst} = {};", literal(value)),
        Op::Owner { dst } => format!("r{dst} = self;"),
        Op::Delta { dst } => format!("r{dst} = delta;"),
        Op::GetVar { dst, var } => format!("r{dst} = {};", nome_variavel(names, *var)),
        Op::SetVar { var, src } => format!("{} = r{src};", nome_variavel(names, *var)),
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
        Op::GetProp { dst, node, name } => format!("r{dst} = Nexo.GetProperty(r{node}, r{name});"),
        Op::SetProp { node, name, value } => {
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

fn nome_variavel(names: &[String], var: usize) -> String {
    names.get(var).cloned().unwrap_or_else(|| format!("v{var}"))
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

/// Names the generated code uses in expression position: the frame parameter (`delta`), the owner (`self`), the
/// engine (`Nexo`, `NodeRef`, `LogLevel`) and the types of the class (`NexoScript`, `Vector2`, `Task`, `Export`). A
/// field with one of these names would hide what the code means, so it gets a `_` at the end.
const GENERATED_NAMES: &[&str] = &[
    "self",
    "delta",
    "Nexo",
    "NexoScript",
    "NodeRef",
    "LogLevel",
    "Vector2",
    "Task",
    "Export",
];

/// C# identifier from a Nexo name: replaces what is not a letter, digit or `_`, and protects reserved words with
/// `@`. Names the generated code uses itself (`GENERATED_NAMES` and the registers `r<n>`) get a `_` at the end.
fn ident(nome: &str) -> String {
    let mut s: String = nome
        .chars()
        .map(|ch| {
            if ch.is_alphabetic() || ch.is_ascii_digit() || ch == '_' {
                ch
            } else {
                '_'
            }
        })
        .collect();
    if !s.starts_with(|c: char| c.is_alphabetic() || c == '_') {
        s.insert(0, '_');
    }
    if RESERVADAS.contains(&s.as_str()) {
        s.insert(0, '@');
    } else if GENERATED_NAMES.contains(&s.as_str()) || is_register(&s) {
        s.push('_');
    }
    s
}

/// `r<n>`: the name of the registers in the generated code.
fn is_register(s: &str) -> bool {
    s.strip_prefix('r')
        .is_some_and(|n| !n.is_empty() && n.bytes().all(|b| b.is_ascii_digit()))
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

    /// Script with two variables that become the same identifier ("vida total" and "vida_total"), and a variable
    /// named `delta`, the name of the frame parameter in the generated code.
    fn collisions() -> Compiled {
        let mut g = Graph::default();
        g.add_variable("vida total", PinType::Float, Value::Float(1.0), true)
            .unwrap();
        g.add_variable("vida_total", PinType::Float, Value::Float(2.0), true)
            .unwrap();
        g.add_variable("delta", PinType::Float, Value::Float(0.5), false)
            .unwrap();
        let ep = adiciona(&mut g, "OnProcess");
        let gv = adiciona(&mut g, "GetVariable");
        g.set_value(gv, "name", Value::String("delta".into()))
            .unwrap();
        let sp = adiciona(&mut g, "SetProperty");
        g.set_value(sp, "name", Value::String("position_x".into()))
            .unwrap();
        g.connect((ep, "exec"), (sp, "exec")).unwrap();
        g.connect((ep, "self"), (sp, "node")).unwrap();
        g.connect((gv, "value"), (sp, "value")).unwrap();
        compile(&g).unwrap()
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
    fn names_used_by_generated_code_get_suffix() {
        // `self`, `delta`, the engine and the registers `r<n>` are used in the generated code: a field with one of
        // these names would hide what the expression means.
        assert_eq!(ident("delta"), "delta_");
        assert_eq!(ident("self"), "self_");
        assert_eq!(ident("Nexo"), "Nexo_");
        assert_eq!(ident("r1"), "r1_");
        assert_eq!(ident("r"), "r");
        assert_eq!(ident("rato"), "rato");
    }

    #[test]
    fn class_name_becomes_valid_identifier() {
        assert_eq!(csharp_class_name("Script 1"), "Script_1");
        assert_eq!(csharp_class_name("Ação de salto"), "Ação_de_salto");
        assert_eq!(csharp_class_name("Task"), "Task_");
        assert_eq!(csharp_class_name("3 vidas"), "_3_vidas");
        assert_eq!(csharp_class_name(""), "_");
    }

    #[test]
    fn fields_with_same_identifier_get_number() {
        let code = generate_csharp(&collisions(), "Teste");
        assert!(
            code.contains("[Export] public double vida_total = 1.0;"),
            "{code}"
        );
        assert!(
            code.contains("[Export] public double vida_total_2 = 2.0;"),
            "{code}"
        );
        // No repeated field: each identifier appears once in the declarations.
        assert_eq!(code.matches("double vida_total ").count(), 1, "{code}");
    }

    #[test]
    fn variable_named_delta_does_not_become_frame_parameter() {
        let code = generate_csharp(&collisions(), "Teste");
        assert!(code.contains("private double delta_ = 0.5;"), "{code}");
        assert!(code.contains("= delta_;"), "{code}");
        assert!(!code.contains("= delta;"), "{code}");
    }

    #[test]
    fn field_named_like_the_class_gets_suffix() {
        // A member cannot have the name of the type that contains it (CS0542).
        let mut g = Graph::default();
        g.add_variable("Jogador", PinType::Float, Value::Float(1.0), true)
            .unwrap();
        let code = generate_csharp(&compile(&g).unwrap(), "Jogador");
        assert!(
            code.contains("public partial class Jogador : NexoScript"),
            "{code}"
        );
        assert!(
            code.contains("[Export] public double Jogador_2 = 1.0;"),
            "{code}"
        );
        assert!(!code.contains("double Jogador = 1.0;"), "{code}");
    }

    #[test]
    fn field_named_like_a_generated_method_gets_suffix() {
        // A field and a method cannot share a name in C# (CS0102). The method of an OnReady node is `OnReady_<id>`.
        let mut g = Graph::default();
        let ev = adiciona(&mut g, "OnReady");
        let method = format!("OnReady_{ev}");
        g.add_variable(&method, PinType::Float, Value::Float(0.5), false)
            .unwrap();
        let code = generate_csharp(&compile(&g).unwrap(), "Teste");
        assert!(
            code.contains(&format!("void {method}(NodeRef self)")),
            "{code}"
        );
        assert!(
            code.contains(&format!("private double {method}_2 = 0.5;")),
            "{code}"
        );
        assert!(
            !code.contains(&format!("private double {method} =")),
            "{code}"
        );
    }
}
