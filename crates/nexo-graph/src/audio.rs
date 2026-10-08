//! Comandos de áudio que a VM entrega ao host (AUD-004). O contrato real ainda não existe em `nexo-core`
//! (não há `audio.rs` lá nesta branch), então a forma está aqui: quando o áudio chegar ao core, este tipo
//! pode virar reexportação sem mudar os nós. Por enquanto só o host executa estes comandos.

/// Um comando de áudio, já com os valores lidos dos pinos.
#[derive(Clone, Debug, PartialEq)]
pub enum AudioCommand {
    /// Toca o som (nome do asset) no barramento, com o volume dado (0 a 1).
    Play {
        sound: String,
        bus: String,
        volume: f64,
    },
    /// Para o som dado.
    Stop { sound: String },
    /// Leva o volume do som ao valor dado, em `seconds`.
    Fade {
        sound: String,
        volume: f64,
        seconds: f64,
    },
    /// Define o volume de um barramento.
    SetBusVolume { bus: String, volume: f64 },
}
