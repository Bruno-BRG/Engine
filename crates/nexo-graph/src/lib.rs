//! Grafo visual de scripts (Fase 2): modelo (VIS-020, VIS-014, VIS-022), biblioteca de nós (VIS-021), compilação
//! para IR (VIS-023, VIS-025), máquina virtual e agendador (VIS-025), recarga a quente (VIS-023), rastro e pontos
//! de parada (VIS-030, VIS-031, DBG-003).
//!
//! O caminho é: `Graph` (o que o editor edita e grava em `scripts/*.graph.ron`) -> `compile` -> `Compiled`
//! (programas de IR) -> `Scheduler::tick` (ou `Vm::run`), com um `Host` que a cena implementa. Este crate não
//! depende da cena nem da física: o mundo entra só pelo trait `Host`, cujo contrato está documentado em `vm.rs`.

pub mod audio;
pub mod csharp;
pub mod graph;
pub mod ir;
pub mod library;
pub mod scheduler;
pub mod types;
pub mod vm;

pub use audio::AudioCommand;
pub use csharp::generate_csharp;
pub use graph::{Colados, Comment, CompileError, Fragment, Graph, GraphNode, PinInfo, PinRef, Wire};
pub use ir::{Compiled, Op, Program, Reg, compile};
pub use library::{NodeDef, PinDef};
pub use scheduler::{Pausa, Scheduler, recarregar_contexto};
pub use types::{LogLevel, PinType, Value, Variable};
pub use vm::{Context, Host, TraceEvent, Vm, VmError};
