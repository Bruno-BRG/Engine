//! Grafo visual de scripts (Fase 2): modelo (VIS-020), biblioteca de nós (VIS-021), variáveis (VIS-022),
//! compilação para IR (VIS-023) e a máquina virtual que executa a IR.
//!
//! O caminho é: `Graph` (o que o editor edita e grava em `scripts/*.graph.ron`) -> `compile` ->
//! `Compiled` (programas de IR) -> `Vm::run`, com um `Host` que a cena implementa. Este crate não depende
//! da cena nem da física: o mundo entra só pelo trait `Host`.

pub mod graph;
pub mod ir;
pub mod library;
pub mod types;
pub mod vm;

pub use graph::{Comment, Graph, GraphNode, PinRef, Wire};
pub use ir::{CompileError, Compiled, Op, Program, Reg, compile};
pub use library::{NodeDef, PinDef};
pub use types::{PinType, Value, Variable};
pub use vm::{Context, Host, Vm, VmError};
