# Audio

Audio exposed as nodes and buses. Playback runs on WebAudio in the editor (see the status notes below).

Legend: **P0** next phase · **P1** before 1.0 · **P2** after 1.0. References: **U** Unity, **G** Godot, **GM** GameMaker.

- [x] **AUD-001** (P0) `AudioStreamPlayer` (volume, pitch, autoplay, loop, bus) and `AudioStreamPlayer2D` (attenuation, max distance). *G.* *Status (Fase 2): feito. `AudioStreamPlayer` é um `Node` sem posição. `AudioStreamPlayer2D` é `Node2D` (tem posição) e repete as mesmas propriedades de áudio, porque o registro tem herança simples e o 2D precisa da posição. O núcleo calcula a atenuação a cada quadro pela distância até a câmera atual (`max_distance` em pixels, `attenuation` como curva: 0 não atenua até o alcance, 1 cai linearmente) e envia `set_attenuation` só quando muda. Autoplay e autostart começam ao iniciar a reprodução. Ver `crates/nexo-core/src/audio.rs`.*
- [x] **AUD-002** (P0) Import WAV/OGG/MP3 with preview in the Project browser and Inspector. *Status (Fase 2): feito: a importação aceita MP3; o campo Áudio do Inspector lista os assets de áudio e o painel Projeto tem o botão de ouvir. Não testado com som real, porque os assets só existem no desktop.*
- [x] **AUD-003** (P1) Audio Mixer panel: buses, volume, mute/solo, effects (reverb, delay, low-pass, compressor). *U: Audio Mixer; G: Audio bus layout.* *Status (Fase 2): feito. Aba Mixer no painel inferior, um canal por barramento de `projectSettings.buses`, com volume (fader e campo em dB), mudo, solo e os quatro efeitos. Volume, mudo e efeitos são gravados no projeto (`markSettingsDirty`) e valem na hora, mesmo sem reprodução. O solo é só da sessão e não entra no arquivo. Os efeitos ficam em `Bus.effects` (`settings.ron`, só quando diferentes dos padrões). Criar e remover barramentos fica nas configurações do projeto.*
- [x] **AUD-004** (P1) Visual nodes: Play, Stop, Fade, Set bus volume. *Status (Fase 2): feito: os nós Tocar, Parar, Fade e Volume do barramento usam o AudioCommand do core e são executados pelo Host na reprodução. Não há tipo duplicado.*
- [ ] **AUD-005** (P2) Music system with crossfades and stems.
