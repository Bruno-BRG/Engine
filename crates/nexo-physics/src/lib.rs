//! Física 2D com Rapier (PHY-001 a PHY-011, PHY-020 a PHY-023, PHY-032). Monta o mundo a partir dos nós
//! de física da cena, simula em passos fixos de 1/60 s e grava as poses na cópia de reprodução.
//! As poses são interpoladas entre dois passos, para o desenho acompanhar a taxa do monitor.
//!
//! Convenções: Y cresce para baixo, como na tela. Ângulos da cena são graus no sentido horário;
//! o Rapier usa radianos com a mesma matriz de rotação, então a conversão é só a unidade.
//! Gravidade, camadas, matriz de colisão e materiais vêm de `ProjectSettings` (PHY-021).

mod character;
mod hooks;
mod shapes;
#[cfg(test)]
mod tests;

pub use character::CharacterState;

use character::Character;
use hooks::{ONE_WAY, Rules};
use nexo_core::query::{self, Xf};
use nexo_core::scene::{as_vec2, value_of};
use nexo_core::settings::{LAYER_COUNT, PhysicsMaterial};
use nexo_core::{Node, ProjectSettings, Scene, Uid};
use rapier2d::prelude::*;
use serde::Serialize;
use serde_json::Value;
use std::collections::{BTreeSet, HashMap};
use std::sync::Mutex;

mod joints;
mod queries;

/// Passo fixo da simulação, em segundos.
pub const STEP: f64 = 1.0 / 60.0;
/// Passos no máximo por quadro. Evita a espiral da morte quando o quadro demora demais.
const MAX_STEPS: u32 = 8;
/// Tamanho de referência do Rapier, em pixels. As tolerâncias de contato escalam com ele.
const LENGTH_UNIT: f32 = 100.0;
/// Massa mínima de um corpo dinâmico: o Rapier não aceita massa zero.
const MIN_MASS: f64 = 0.01;

const BODY_KINDS: &[&str] = &["StaticBody2D", "RigidBody2D", "CharacterBody2D", "Area2D"];
const SHAPE_KINDS: &[&str] = &["CollisionShape2D", "CollisionPolygon2D"];

/// Tipo de evento que a física emite para um nó.
#[derive(Clone, Copy, Debug, PartialEq, Eq, Serialize)]
#[serde(rename_all = "snake_case")]
pub enum EventKind {
    /// Um corpo entrou na área `node`; `other` é o corpo.
    BodyEntered,
    BodyExited,
    /// Uma área entrou em outra área; `node` é a que detecta e `other` a que entrou.
    AreaEntered,
    AreaExited,
    /// Dois corpos sólidos começaram a se tocar; os dois lados recebem o evento.
    CollisionStarted,
    CollisionEnded,
}

/// Evento de física: `kind` aconteceu com `node` e `other`. Colisões sólidas trazem o ponto de contato
/// (`point`, no mundo) e a normal, apontando de `other` para `node` (PHY-032), quando o Rapier informa.
/// Eventos de área não trazem ponto nem normal.
#[derive(Clone, Copy, Debug, PartialEq, Serialize)]
pub struct PhysicsEvent {
    pub kind: EventKind,
    pub node: Uid,
    pub other: Uid,
    pub point: Option<[f64; 2]>,
    pub normal: Option<[f64; 2]>,
}

/// Contato sólido: ponto médio no mundo e normal de `collider1` para `collider2`, como o Rapier a dá.
#[derive(Clone, Copy)]
struct Contact {
    point: Vector,
    normal: Vector,
}

/// Evento bruto do Rapier, com o contato quando há.
type RawEvent = (CollisionEvent, Option<Contact>);

/// Coleta os eventos do Rapier durante um passo. O Rapier chama o handler com `&self`,
/// então a fila fica atrás de um Mutex.
#[derive(Default)]
struct Collector(Mutex<Vec<RawEvent>>);

impl Collector {
    fn take(&self) -> Vec<RawEvent> {
        self.0.lock().map(|mut v| std::mem::take(&mut *v)).unwrap_or_default()
    }
}

impl EventHandler for Collector {
    fn handle_collision_event(
        &self,
        _bodies: &RigidBodySet,
        colliders: &ColliderSet,
        event: CollisionEvent,
        pair: Option<&ContactPair>,
    ) {
        let contact = pair.and_then(|p| contact_of(colliders, p));
        if let Ok(mut v) = self.0.lock() {
            v.push((event, contact));
        }
    }

    fn handle_contact_force_event(
        &self,
        _dt: Real,
        _bodies: &RigidBodySet,
        _colliders: &ColliderSet,
        _pair: &ContactPair,
        _total_force: Real,
    ) {
    }

    fn handle_soft_body_tear_event(&self, _soft: &SoftBodySet, _event: &SoftBodyTearEvent) {}
}

/// Ponto médio dos contatos do par, no mundo, e a normal do contato mais fundo.
fn contact_of(colliders: &ColliderSet, pair: &ContactPair) -> Option<Contact> {
    let (deepest, _) = pair.find_deepest_contact()?;
    let p1 = colliders.get(pair.collider1)?.position();
    let p2 = colliders.get(pair.collider2)?.position();
    let mut sum = Vector::ZERO;
    let mut n = 0.0f32;
    for m in pair.manifolds() {
        for c in &m.points {
            sum += (p1.transform_point(c.local_p1) + p2.transform_point(c.local_p2)) * 0.5;
            n += 1.0;
        }
    }
    (n > 0.0).then(|| Contact { point: sum / n, normal: deepest.data.normal })
}

/// Pose no mundo do Rapier: posição em pixels e ângulo em radianos.
#[derive(Clone, Copy, Debug)]
struct Pose2d {
    x: f32,
    y: f32,
    angle: f32,
}

impl Pose2d {
    fn of(body: &RigidBody) -> Pose2d {
        let p = body.position();
        Pose2d { x: p.translation.x, y: p.translation.y, angle: p.rotation.angle() }
    }

    fn lerp(a: Pose2d, b: Pose2d, t: f32) -> Pose2d {
        Pose2d { x: a.x + (b.x - a.x) * t, y: a.y + (b.y - a.y) * t, angle: a.angle + (b.angle - a.angle) * t }
    }
}

/// Como uma área muda a gravidade ou o amortecimento de um corpo (PHY-005).
#[derive(Clone, Copy)]
enum Override {
    Disabled,
    /// Soma ao valor do corpo.
    Combine,
    /// Troca o valor do corpo pelo da área.
    Replace,
}

impl Override {
    fn of(n: &Node, key: &str) -> Override {
        match text(n, key).as_str() {
            "combine" => Override::Combine,
            "replace" => Override::Replace,
            _ => Override::Disabled,
        }
    }
}

/// Uma área: detecta corpos e áreas e, opcionalmente, muda a gravidade e o amortecimento dos corpos dentro.
struct AreaInfo {
    monitoring: bool,
    gravity_override: Override,
    gravity: Vector,
    damp_override: Override,
    linear_damp: f32,
}

/// Corpo cuja pose vem da simulação e é gravada na cena: corpo dinâmico ou personagem.
struct Driven {
    uid: Uid,
    handle: RigidBodyHandle,
    /// Transformação mundial do pai, medida na montagem. As poses são gravadas no espaço do pai.
    parent: Xf,
    prev: Pose2d,
    curr: Pose2d,
    /// Corpo dinâmico (RigidBody2D sem congelamento): recebe as sobrescritas das áreas.
    dynamic: bool,
    /// Escala de gravidade e amortecimento linear próprios do corpo, sem as áreas.
    gravity_scale: f32,
    linear_damping: f32,
    character: Option<Character>,
}

/// Mundo de física de uma reprodução.
pub struct Physics {
    world: PhysicsWorld,
    /// Regras de contato: esteiras e formas de uma face só.
    rules: Rules,
    /// Direção "para cima" dos personagens: oposta à gravidade (ou (0, -1) sem gravidade).
    up: Vector,
    driven: Vec<Driven>,
    index: HashMap<Uid, usize>,
    /// Corpo rígido de cada nó com corpo (estático, dinâmico, personagem ou área), para juntas e consultas.
    bodies: HashMap<Uid, RigidBodyHandle>,
    /// Nó dono de cada colisor (o corpo ou a área).
    owner: HashMap<ColliderHandle, Uid>,
    /// Áreas e o que elas fazem.
    areas: HashMap<Uid, AreaInfo>,
    /// Quantos pares de colisores estão sobrepostos entre uma área e outro nó.
    overlaps: HashMap<(Uid, Uid), u32>,
    /// Áreas em que cada corpo está agora, em ordem (para as sobrescritas).
    inside: HashMap<Uid, BTreeSet<Uid>>,
    events: Vec<PhysicsEvent>,
    acc: f64,
}

impl Physics {
    /// Monta o mundo com os nós de física da cena, na pose de `scene` no momento da chamada.
    /// Gravidade, camadas, matriz e materiais vêm de `settings` (PHY-021).
    pub fn build(scene: &Scene, settings: &ProjectSettings) -> Physics {
        let xfs: HashMap<Uid, Xf> = query::world_map(scene, None).into_iter().map(|(uid, xf, _)| (uid, xf)).collect();
        let world = PhysicsWorld {
            gravity: Vector::new(settings.gravity[0] as f32, settings.gravity[1] as f32),
            integration_parameters: IntegrationParameters {
                dt: STEP as f32,
                length_unit: LENGTH_UNIT,
                ..Default::default()
            },
            ..Default::default()
        };
        let mut p = Physics {
            world,
            rules: Rules::default(),
            up: up_of(settings.gravity),
            driven: Vec::new(),
            index: HashMap::new(),
            bodies: HashMap::new(),
            owner: HashMap::new(),
            areas: HashMap::new(),
            overlaps: HashMap::new(),
            inside: HashMap::new(),
            events: Vec::new(),
            acc: 0.0,
        };
        for uid in scene.pre_order() {
            let Some(node) = scene.node(uid) else { continue };
            if BODY_KINDS.contains(&node.kind.as_str()) {
                p.add_body(scene, uid, node, &xfs, settings);
            }
        }
        joints::add(&mut p, scene);
        p
    }

    fn add_body(&mut self, scene: &Scene, uid: Uid, node: &Node, xfs: &HashMap<Uid, Xf>, settings: &ProjectSettings) {
        let Some(&body_xf) = xfs.get(&uid) else { return };
        let parent_xf = node.parent.and_then(|p| xfs.get(&p).copied()).unwrap_or(Xf::IDENTITY);
        let kind = node.kind.as_str();
        let is_area = kind == "Area2D";
        // Só o RigidBody2D sem congelamento é simulado. Congelado vira imóvel ou cinemático e não se move.
        let freeze = if kind == "RigidBody2D" { text(node, "freeze") } else { "none".to_string() };
        let dynamic = kind == "RigidBody2D" && freeze == "none";

        let mut builder = if dynamic {
            RigidBodyBuilder::dynamic()
                .gravity_scale(num(node, "gravity_scale") as f32)
                .linear_damping(num(node, "linear_damping") as f32)
                .angular_damping(num(node, "angular_damping") as f32)
                .can_sleep(flag_or(node, "can_sleep", true))
                .ccd_enabled(flag(node, "continuous"))
        } else if kind == "CharacterBody2D" || freeze == "kinematic" {
            RigidBodyBuilder::kinematic_position_based()
        } else {
            RigidBodyBuilder::fixed()
        };
        if dynamic && flag(node, "lock_rotation") {
            builder = builder.lock_rotations();
        }
        builder = builder.translation(Vector::new(body_xf.x as f32, body_xf.y as f32)).rotation(to_rad(body_xf.rot));

        let groups = layer_groups(node, settings);
        let material = if is_area { None } else { material_of(settings, &text(node, "physics_material")) };
        let belt = if kind == "StaticBody2D" {
            let v = as_vec2(&value_of(node, "constant_linear_velocity"));
            Vector::new(v[0] as f32, v[1] as f32)
        } else {
            Vector::ZERO
        };

        let mut colliders = Vec::new();
        for child in &node.children {
            let Some(shape_node) = scene.node(*child) else { continue };
            if !SHAPE_KINDS.contains(&shape_node.kind.as_str()) || flag(shape_node, "disabled") {
                continue;
            }
            let Some(&shape_xf) = xfs.get(child) else { continue };
            let Some(shape) = shapes::build(shape_node, shape_xf) else { continue };
            let (rx, ry, ra) = relative(body_xf, shape_xf);
            let mut b = shape
                .position(Pose::new(Vector::new(rx, ry), ra))
                .collision_groups(groups)
                .active_events(ActiveEvents::COLLISION_EVENTS)
                .sensor(is_area);
            if is_area {
                // Áreas são fixas. Sem isto, o Rapier não testa área contra área nem contra corpos fixos.
                b = b.active_collision_types(ActiveCollisionTypes::all());
            } else {
                if let Some(m) = material {
                    b = b
                        .friction(m.friction as f32)
                        .restitution(m.bounce as f32)
                        .friction_combine_rule(if m.rough {
                            CoefficientCombineRule::Max
                        } else {
                            CoefficientCombineRule::Average
                        })
                        .restitution_combine_rule(if m.absorbent {
                            CoefficientCombineRule::Min
                        } else {
                            CoefficientCombineRule::Average
                        });
                }
                // Formas de uma face e esteiras entram no gancho de contato (ver hooks.rs).
                let mut active = ActiveHooks::empty();
                if flag(shape_node, "one_way") {
                    b = b.user_data(ONE_WAY);
                    active |= ActiveHooks::MODIFY_SOLVER_CONTACTS;
                }
                if belt != Vector::ZERO {
                    active |= ActiveHooks::MODIFY_SOLVER_CONTACTS;
                }
                b = b.active_hooks(active);
            }
            colliders.push(b.build());
        }

        // A massa é dividida entre as formas pela área de cada uma. Sem área, vai como massa extra.
        if dynamic {
            let mass = num(node, "mass").max(MIN_MASS) as f32;
            let total: f32 = colliders.iter().map(|c| c.mass()).sum();
            if total > 0.0 {
                for c in &mut colliders {
                    let share = c.mass() / total;
                    c.set_mass(mass * share);
                }
            } else {
                builder = builder.additional_mass(mass);
            }
        }

        let handle = self.world.bodies.insert(builder.build());
        self.bodies.insert(uid, handle);
        let mut first_collider = None;
        for c in colliders {
            let ch = self.world.colliders.insert_with_parent(c, handle, &mut self.world.bodies);
            self.owner.insert(ch, uid);
            if belt != Vector::ZERO && !is_area {
                self.rules.belts.insert(ch, belt);
            }
            if first_collider.is_none() {
                first_collider = Some(ch);
            }
        }

        match kind {
            "Area2D" => {
                let g = as_vec2(&value_of(node, "gravity"));
                self.areas.insert(
                    uid,
                    AreaInfo {
                        monitoring: flag_or(node, "monitoring", true),
                        gravity_override: Override::of(node, "gravity_override"),
                        gravity: Vector::new(g[0] as f32, g[1] as f32),
                        damp_override: Override::of(node, "damp_override"),
                        linear_damp: num(node, "linear_damp") as f32,
                    },
                );
            }
            "CharacterBody2D" => {
                if let Some(collider) = first_collider {
                    let velocity = as_vec2(&value_of(node, "velocity"));
                    let pose = Pose2d::of(&self.world.bodies[handle]);
                    self.index.insert(uid, self.driven.len());
                    self.driven.push(Driven {
                        uid,
                        handle,
                        parent: parent_xf,
                        prev: pose,
                        curr: pose,
                        dynamic: false,
                        gravity_scale: 0.0,
                        linear_damping: 0.0,
                        character: Some(Character::new(velocity, collider, self.up)),
                    });
                }
            }
            _ if dynamic => {
                let pose = Pose2d::of(&self.world.bodies[handle]);
                self.index.insert(uid, self.driven.len());
                self.driven.push(Driven {
                    uid,
                    handle,
                    parent: parent_xf,
                    prev: pose,
                    curr: pose,
                    dynamic: true,
                    gravity_scale: num(node, "gravity_scale") as f32,
                    linear_damping: num(node, "linear_damping") as f32,
                    character: None,
                });
            }
            _ => {}
        }
    }

    /// Avança a simulação por `dt` segundos e grava as poses interpoladas na cena de reprodução.
    pub fn advance(&mut self, scene: &mut Scene, dt: f64) {
        self.acc = (self.acc + dt.max(0.0)).min(STEP * MAX_STEPS as f64);
        while self.acc >= STEP {
            self.step();
            self.acc -= STEP;
        }
        self.write_poses(scene, (self.acc / STEP) as f32);
    }

    fn step(&mut self) {
        self.apply_areas();
        self.move_characters();
        for d in &mut self.driven {
            d.prev = d.curr;
        }
        let collector = Collector::default();
        self.world.step_with_events(&self.rules, &collector);
        for d in &mut self.driven {
            d.curr = Pose2d::of(&self.world.bodies[d.handle]);
        }
        self.process(collector.take());
    }

    /// Aplica as áreas em que cada corpo dinâmico está (PHY-005): gravidade e amortecimento linear.
    /// As forças são zeradas a cada passo; a área age a partir do passo seguinte à entrada.
    fn apply_areas(&mut self) {
        for i in 0..self.driven.len() {
            let d = &self.driven[i];
            if !d.dynamic {
                continue;
            }
            let mut combined = Vector::ZERO;
            let mut replaced: Option<Vector> = None;
            let mut damp_extra = 0.0f32;
            let mut damp_replaced: Option<f32> = None;
            for area in self.inside.get(&d.uid).into_iter().flatten() {
                let Some(a) = self.areas.get(area) else { continue };
                match a.gravity_override {
                    Override::Combine => combined += a.gravity,
                    Override::Replace => {
                        replaced.get_or_insert(a.gravity);
                    }
                    Override::Disabled => {}
                }
                match a.damp_override {
                    Override::Combine => damp_extra += a.linear_damp,
                    Override::Replace => {
                        damp_replaced.get_or_insert(a.linear_damp);
                    }
                    Override::Disabled => {}
                }
            }
            let (handle, base_scale, base_damp) = (d.handle, d.gravity_scale, d.linear_damping);
            // Gravidade trocada: zera a escala do corpo e aplica a da área como força (força = massa × aceleração).
            let scale = if replaced.is_some() { 0.0 } else { base_scale };
            let accel = combined + replaced.unwrap_or(Vector::ZERO);
            let damping = damp_replaced.unwrap_or(base_damp) + damp_extra;
            let body = &mut self.world.bodies[handle];
            body.set_gravity_scale(scale, false);
            body.set_linear_damping(damping);
            body.reset_forces(false);
            let force = accel * body.mass();
            if force != Vector::ZERO {
                body.add_force(force, true);
            }
        }
    }

    /// Velocidade da superfície de um colisor (px/s): a da esteira, ou a do corpo que o contém.
    fn surface_velocity(&self, c: ColliderHandle) -> [f64; 2] {
        if let Some(v) = self.rules.belts.get(&c) {
            return [v.x as f64, v.y as f64];
        }
        let Some(&i) = self.owner.get(&c).and_then(|uid| self.index.get(uid)) else {
            return [0.0, 0.0];
        };
        let d = &self.driven[i];
        match &d.character {
            Some(ch) => ch.velocity,
            None => {
                let v = self.world.bodies[d.handle].linvel();
                [v.x as f64, v.y as f64]
            }
        }
    }

    /// Converte os eventos do Rapier em eventos de nó: sensores geram entrada e saída; sólidos, contato.
    fn process(&mut self, raw: Vec<RawEvent>) {
        for (ev, contact) in raw {
            let (a, b, flags, started) = match ev {
                CollisionEvent::Started(a, b, f) => (a, b, f, true),
                CollisionEvent::Stopped(a, b, f) => (a, b, f, false),
            };
            let (Some(&ua), Some(&ub)) = (self.owner.get(&a), self.owner.get(&b)) else { continue };
            if ua == ub {
                continue;
            }
            if flags.contains(CollisionEventFlags::SENSOR) {
                for (sensor, node, other) in [(a, ua, ub), (b, ub, ua)] {
                    if !self.world.colliders[sensor].is_sensor() || !self.areas.contains_key(&node) {
                        continue;
                    }
                    // Um evento só quando o par inteiro entra ou sai, não por forma do corpo.
                    if !self.count_overlap(node, other, started) {
                        continue;
                    }
                    let other_is_area = self.areas.contains_key(&other);
                    if !other_is_area {
                        self.track_inside(other, node, started);
                    }
                    if !self.areas.get(&node).is_some_and(|a| a.monitoring) {
                        continue;
                    }
                    let kind = match (other_is_area, started) {
                        (false, true) => EventKind::BodyEntered,
                        (false, false) => EventKind::BodyExited,
                        (true, true) => EventKind::AreaEntered,
                        (true, false) => EventKind::AreaExited,
                    };
                    self.events.push(PhysicsEvent { kind, node, other, point: None, normal: None });
                }
            } else {
                let kind = if started { EventKind::CollisionStarted } else { EventKind::CollisionEnded };
                let point = contact.map(|c| [c.point.x as f64, c.point.y as f64]);
                // A normal de `collider1` para `collider2`: para `ua` ela se inverte, para `ub` fica.
                let n = contact.map(|c| c.normal);
                let to_a = n.map(|n| [-n.x as f64, -n.y as f64]);
                let to_b = n.map(|n| [n.x as f64, n.y as f64]);
                self.events.push(PhysicsEvent { kind, node: ua, other: ub, point, normal: to_a });
                self.events.push(PhysicsEvent { kind, node: ub, other: ua, point, normal: to_b });
            }
        }
    }

    /// Conta um par de colisores sobrepostos entre área e outro nó. True quando o par entra (1ª vez)
    /// ou sai (última vez).
    fn count_overlap(&mut self, area: Uid, other: Uid, started: bool) -> bool {
        let key = (area, other);
        if started {
            let c = self.overlaps.entry(key).or_insert(0);
            *c += 1;
            *c == 1
        } else {
            match self.overlaps.get_mut(&key) {
                Some(c) if *c > 1 => {
                    *c -= 1;
                    false
                }
                Some(_) => {
                    self.overlaps.remove(&key);
                    true
                }
                None => false,
            }
        }
    }

    /// Guarda em quais áreas cada corpo está agora.
    fn track_inside(&mut self, body: Uid, area: Uid, entered: bool) {
        if entered {
            self.inside.entry(body).or_default().insert(area);
            return;
        }
        let empty = match self.inside.get_mut(&body) {
            Some(set) => {
                set.remove(&area);
                set.is_empty()
            }
            None => false,
        };
        if empty {
            self.inside.remove(&body);
        }
    }

    /// Grava a pose de cada corpo dinâmico e personagem na cena, no espaço do pai.
    fn write_poses(&self, scene: &mut Scene, alpha: f32) {
        for d in &self.driven {
            let p = Pose2d::lerp(d.prev, d.curr, alpha);
            let (dx, dy) = (p.x - d.parent.x as f32, p.y - d.parent.y as f32);
            let (s, c) = to_rad(d.parent.rot).sin_cos();
            // Inverso da rotação do pai: a mesma matriz que Xf::then usa, com o ângulo negado.
            let local = [c * dx + s * dy, -s * dx + c * dy];
            let rot = p.angle.to_degrees() as f64 - d.parent.rot;
            // Os UIDs vêm da própria cena, então os dois valores sempre existem.
            let _ = scene.set_runtime_prop(d.uid, "position", serde_json::json!([local[0], local[1]]));
            let _ = scene.set_runtime_prop(d.uid, "rotation", serde_json::json!(rot));
        }
    }

    /// Eventos desde a última chamada. A fila é esvaziada.
    pub fn take_events(&mut self) -> Vec<PhysicsEvent> {
        std::mem::take(&mut self.events)
    }
}

/// Confere uma referência por nome contra as configurações do projeto: a camada (`layer`) precisa existir
/// e o material (`physics_material`) também, sendo vazio = nenhum. As demais propriedades passam.
pub fn check_reference(settings: &ProjectSettings, key: &str, value: &Value) -> Result<(), String> {
    let Some(name) = value.as_str() else { return Ok(()) };
    match key {
        "layer" if layer_index(settings, name).is_none() => Err(format!("Camada desconhecida: {name}")),
        "physics_material" if !name.is_empty() && material_of(settings, name).is_none() => {
            Err(format!("Material de física desconhecido: {name}"))
        }
        _ => Ok(()),
    }
}

/// Direção "para cima" a partir da gravidade: a oposta. Sem gravidade, a da tela (Y para baixo).
fn up_of(gravity: [f64; 2]) -> Vector {
    let len = gravity[0].hypot(gravity[1]);
    if len > 0.0 { Vector::new((-gravity[0] / len) as f32, (-gravity[1] / len) as f32) } else { Vector::new(0.0, -1.0) }
}

/// Índice (bit) de uma camada pelo nome, ou `None` se a camada não existe ou está vazia.
fn layer_index(settings: &ProjectSettings, name: &str) -> Option<usize> {
    settings.layers.iter().take(LAYER_COUNT).position(|l| !l.is_empty() && l == name)
}

/// Material de física pelo nome. Vazio ou desconhecido = sem material.
fn material_of<'a>(settings: &'a ProjectSettings, name: &str) -> Option<&'a PhysicsMaterial> {
    if name.is_empty() {
        return None;
    }
    settings.physics_materials.iter().find(|m| m.name == name)
}

/// Camada e máscaras do nó como grupos de colisão (PHY-020, PHY-022).
///
/// A camada é um bit, pela posição em `settings.layers`. O filtro junta a linha da matriz com as camadas
/// incluídas, tira as excluídas (exclusão vence inclusão) e cruza com a máscara do nó (-1 = todas). Camada
/// desconhecida não colide com nada. Dois corpos colidem quando cada um aceita a camada do outro.
fn layer_groups(n: &Node, settings: &ProjectSettings) -> InteractionGroups {
    let (membership, matrix) = match layer_index(settings, &text(n, "layer")) {
        Some(i) => (1u32 << i, settings.matrix_of(i)),
        None => (0, 0),
    };
    let include = num(n, "include_layers") as i64 as u32;
    let exclude = num(n, "exclude_layers") as i64 as u32;
    let mask = num(n, "collision_mask") as i64 as u32;
    let filter = (matrix | include) & !exclude & mask;
    InteractionGroups::new(
        Group::from_bits_retain(membership),
        Group::from_bits_retain(filter),
        InteractionTestMode::And,
    )
}

fn num(n: &Node, key: &str) -> f64 {
    value_of(n, key).as_f64().unwrap_or(0.0)
}

fn flag(n: &Node, key: &str) -> bool {
    flag_or(n, key, false)
}

fn flag_or(n: &Node, key: &str, default: bool) -> bool {
    value_of(n, key).as_bool().unwrap_or(default)
}

fn text(n: &Node, key: &str) -> String {
    value_of(n, key).as_str().unwrap_or("").to_string()
}

fn to_rad(degrees: f64) -> f32 {
    degrees.to_radians() as f32
}

/// Posição e ângulo de uma forma relativos ao corpo: a forma em coordenadas do corpo.
fn relative(body: Xf, shape: Xf) -> (f32, f32, f32) {
    let (s, c) = to_rad(body.rot).sin_cos();
    let (dx, dy) = ((shape.x - body.x) as f32, (shape.y - body.y) as f32);
    (c * dx + s * dy, -s * dx + c * dy, to_rad(shape.rot - body.rot))
}
