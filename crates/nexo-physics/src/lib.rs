//! Física 2D com Rapier (PHY-001 a PHY-005, PHY-010, PHY-020). Monta o mundo a partir dos nós de
//! física da cena, simula em passos fixos de 1/60 s e grava as poses na cópia de reprodução.
//! As poses são interpoladas entre dois passos, para o desenho acompanhar a taxa do monitor.
//!
//! Convenções: Y cresce para baixo, como na tela. Ângulos da cena são graus no sentido horário;
//! o Rapier usa radianos com a mesma matriz de rotação, então a conversão é só a unidade.

use nexo_core::query::{self, Xf};
use nexo_core::registry::LAYERS;
use nexo_core::scene::{as_vec2, value_of};
use nexo_core::{Node, Scene, Uid};
use rapier2d::control::KinematicCharacterController;
use rapier2d::prelude::*;
use serde::Serialize;
use serde_json::json;
use std::collections::HashMap;
use std::sync::Mutex;

/// Passo fixo da simulação, em segundos.
pub const STEP: f64 = 1.0 / 60.0;
/// Passos no máximo por quadro. Evita a espiral da morte quando o quadro demora demais.
const MAX_STEPS: u32 = 8;
/// Gravidade em pixels por segundo², para baixo.
const GRAVITY: f32 = 980.0;
/// Tamanho de referência do Rapier, em pixels. As tolerâncias de contato escalam com ele.
const LENGTH_UNIT: f32 = 100.0;

const BODY_KINDS: &[&str] = &["StaticBody2D", "RigidBody2D", "CharacterBody2D", "Area2D"];

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

/// Evento de física: `kind` aconteceu com `node` e `other`.
#[derive(Clone, Copy, Debug, PartialEq, Eq, Serialize)]
pub struct PhysicsEvent {
    pub kind: EventKind,
    pub node: Uid,
    pub other: Uid,
}

/// Coleta os eventos do Rapier durante um passo. O Rapier chama o handler com `&self`,
/// então a fila fica atrás de um Mutex.
#[derive(Default)]
struct Collector(Mutex<Vec<CollisionEvent>>);

impl Collector {
    fn take(&self) -> Vec<CollisionEvent> {
        self.0.lock().map(|mut v| std::mem::take(&mut *v)).unwrap_or_default()
    }
}

impl EventHandler for Collector {
    fn handle_collision_event(
        &self,
        _bodies: &RigidBodySet,
        _colliders: &ColliderSet,
        event: CollisionEvent,
        _pair: Option<&ContactPair>,
    ) {
        if let Ok(mut v) = self.0.lock() {
            v.push(event);
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
        Pose2d {
            x: a.x + (b.x - a.x) * t,
            y: a.y + (b.y - a.y) * t,
            angle: a.angle + (b.angle - a.angle) * t,
        }
    }
}

/// Estado de um personagem entre os passos.
struct Character {
    /// Velocidade em pixels por segundo.
    velocity: [f64; 2],
    grounded: bool,
    controller: KinematicCharacterController,
    /// Primeiro colisor do personagem: a forma que o controlador move.
    collider: ColliderHandle,
}

/// Corpo cuja pose vem da simulação e é gravada na cena: dinâmico ou personagem.
struct Driven {
    uid: Uid,
    handle: RigidBodyHandle,
    /// Transformação mundial do pai, medida na montagem. As poses são gravadas no espaço do pai.
    parent: Xf,
    prev: Pose2d,
    curr: Pose2d,
    character: Option<Character>,
}

/// Mundo de física de uma reprodução.
pub struct Physics {
    world: PhysicsWorld,
    driven: Vec<Driven>,
    index: HashMap<Uid, usize>,
    /// Nó dono de cada colisor (o corpo ou a área).
    owner: HashMap<ColliderHandle, Uid>,
    /// Áreas e se estão monitorando.
    areas: HashMap<Uid, bool>,
    events: Vec<PhysicsEvent>,
    acc: f64,
}

impl Physics {
    /// Monta o mundo com os nós de física da cena, na pose de `scene` no momento da chamada.
    pub fn build(scene: &Scene) -> Physics {
        let xfs: HashMap<Uid, Xf> = query::world_map(scene, None)
            .into_iter()
            .map(|(uid, xf, _)| (uid, xf))
            .collect();
        let mut world = PhysicsWorld::default();
        world.gravity = Vector::new(0.0, GRAVITY);
        world.integration_parameters.dt = STEP as f32;
        world.integration_parameters.length_unit = LENGTH_UNIT;
        let mut p = Physics {
            world,
            driven: Vec::new(),
            index: HashMap::new(),
            owner: HashMap::new(),
            areas: HashMap::new(),
            events: Vec::new(),
            acc: 0.0,
        };
        for uid in scene.pre_order() {
            let Some(node) = scene.node(uid) else { continue };
            if BODY_KINDS.contains(&node.kind.as_str()) {
                p.add_body(scene, uid, node, &xfs);
            }
        }
        p
    }

    fn add_body(&mut self, scene: &Scene, uid: Uid, node: &Node, xfs: &HashMap<Uid, Xf>) {
        let Some(&body_xf) = xfs.get(&uid) else { return };
        let parent_xf = node.parent.and_then(|p| xfs.get(&p).copied()).unwrap_or(Xf::IDENTITY);
        let is_area = node.kind == "Area2D";
        let mut builder = match node.kind.as_str() {
            "RigidBody2D" => {
                let mut b = RigidBodyBuilder::dynamic()
                    .gravity_scale(num(node, "gravity_scale") as f32)
                    .linear_damping(num(node, "linear_damping") as f32)
                    .ccd_enabled(flag(node, "continuous"));
                if flag(node, "lock_rotation") {
                    b = b.lock_rotations();
                }
                b
            }
            "CharacterBody2D" => RigidBodyBuilder::kinematic_position_based(),
            // Corpo imóvel e área: a área é fixa e só detecta, sem resposta física.
            _ => RigidBodyBuilder::fixed(),
        };
        builder = builder.translation(Vector::new(body_xf.x as f32, body_xf.y as f32)).rotation(to_rad(body_xf.rot));
        let handle = self.world.bodies.insert(builder.build());

        let groups = mask_groups(node);
        let mut first_collider = None;
        for child in &node.children {
            let Some(shape_node) = scene.node(*child) else { continue };
            if shape_node.kind != "CollisionShape2D" || flag(shape_node, "disabled") {
                continue;
            }
            let Some(&shape_xf) = xfs.get(child) else { continue };
            let [w, h] = as_vec2(&value_of(shape_node, "size"));
            let (w, h) = ((w * shape_xf.sx.abs()).max(0.5), (h * shape_xf.sy.abs()).max(0.5));
            let shape = match text(shape_node, "shape").as_str() {
                "circle" => ColliderBuilder::ball((w / 2.0) as f32),
                _ => ColliderBuilder::cuboid((w / 2.0) as f32, (h / 2.0) as f32),
            };
            let (rx, ry, ra) = relative(body_xf, shape_xf);
            let mut collider = shape
                .position(Pose::new(Vector::new(rx, ry), ra))
                .collision_groups(groups)
                .active_events(ActiveEvents::COLLISION_EVENTS)
                .sensor(is_area);
            if is_area {
                // Áreas são fixas. Sem isto, o Rapier não testa área contra área nem contra corpos fixos.
                collider = collider.active_collision_types(ActiveCollisionTypes::all());
            }
            let ch = self.world.colliders.insert_with_parent(collider.build(), handle, &mut self.world.bodies);
            self.owner.insert(ch, uid);
            if first_collider.is_none() {
                first_collider = Some(ch);
            }
        }

        match node.kind.as_str() {
            "Area2D" => {
                self.areas.insert(uid, flag_or(node, "monitoring", true));
            }
            "RigidBody2D" | "CharacterBody2D" => {
                let pose = Pose2d::of(&self.world.bodies[handle]);
                let character = match (node.kind.as_str(), first_collider) {
                    ("CharacterBody2D", Some(collider)) => Some(Character {
                        velocity: as_vec2(&value_of(node, "velocity")),
                        grounded: false,
                        controller: controller(),
                        collider,
                    }),
                    _ => None,
                };
                self.index.insert(uid, self.driven.len());
                self.driven.push(Driven { uid, handle, parent: parent_xf, prev: pose, curr: pose, character });
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
        self.move_characters();
        for d in &mut self.driven {
            d.prev = d.curr;
        }
        let collector = Collector::default();
        self.world.step_with_events(&(), &collector);
        for d in &mut self.driven {
            d.curr = Pose2d::of(&self.world.bodies[d.handle]);
        }
        self.process(collector.take());
    }

    /// Move cada personagem com o controlador de forma, que para em paredes e no chão.
    fn move_characters(&mut self) {
        let dt = STEP as f32;
        let mut results = Vec::new();
        for (i, d) in self.driven.iter().enumerate() {
            let Some(ch) = &d.character else { continue };
            let filter = QueryFilter {
                exclude_rigid_body: Some(d.handle),
                flags: QueryFilterFlags::EXCLUDE_SENSORS,
                ..Default::default()
            };
            let queries = self.world.query_pipeline_with_filter(filter);
            let collider = &self.world.colliders[ch.collider];
            let desired = Vector::new((ch.velocity[0] * STEP) as f32, (ch.velocity[1] * STEP) as f32);
            let mv = ch.controller.move_shape(dt, &queries, collider.shape(), collider.position(), desired, |_| {});
            results.push((i, mv.translation, desired, mv.grounded));
        }
        for (i, moved, desired, grounded) in results {
            let d = &mut self.driven[i];
            let body = &mut self.world.bodies[d.handle];
            let target = body.translation() + moved;
            body.set_next_kinematic_translation(target);
            let Some(ch) = d.character.as_mut() else { continue };
            ch.grounded = grounded;
            // Como o move_and_slide: o eixo bloqueado perde a velocidade.
            if (moved.x - desired.x).abs() > 1e-3 {
                ch.velocity[0] = 0.0;
            }
            if (moved.y - desired.y).abs() > 1e-3 {
                ch.velocity[1] = 0.0;
            }
            if grounded && ch.velocity[1] > 0.0 {
                ch.velocity[1] = 0.0;
            }
        }
    }

    /// Converte os eventos do Rapier em eventos de nó: sensores geram entrada e saída; sólidos, contato.
    fn process(&mut self, raw: Vec<CollisionEvent>) {
        for ev in raw {
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
                    if !self.world.colliders[sensor].is_sensor() || self.areas.get(&node) != Some(&true) {
                        continue;
                    }
                    let other_is_area = self.areas.contains_key(&other);
                    let kind = match (other_is_area, started) {
                        (false, true) => EventKind::BodyEntered,
                        (false, false) => EventKind::BodyExited,
                        (true, true) => EventKind::AreaEntered,
                        (true, false) => EventKind::AreaExited,
                    };
                    self.events.push(PhysicsEvent { kind, node, other });
                }
            } else {
                let kind = if started { EventKind::CollisionStarted } else { EventKind::CollisionEnded };
                self.events.push(PhysicsEvent { kind, node: ua, other: ub });
                self.events.push(PhysicsEvent { kind, node: ub, other: ua });
            }
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
            let _ = scene.set_runtime_prop(d.uid, "position", json!([local[0], local[1]]));
            let _ = scene.set_runtime_prop(d.uid, "rotation", json!(rot));
        }
    }

    /// Eventos desde a última chamada. A fila é esvaziada.
    pub fn take_events(&mut self) -> Vec<PhysicsEvent> {
        std::mem::take(&mut self.events)
    }

    /// Velocidade de um personagem, em pixels por segundo.
    pub fn velocity(&self, uid: Uid) -> Option<[f64; 2]> {
        self.character(uid).map(|c| c.velocity)
    }

    pub fn set_velocity(&mut self, uid: Uid, velocity: [f64; 2]) -> bool {
        let Some(ch) = self.index.get(&uid).and_then(|i| self.driven[*i].character.as_mut()) else {
            return false;
        };
        ch.velocity = velocity;
        true
    }

    /// True se o personagem está encostado no chão no último passo.
    pub fn grounded(&self, uid: Uid) -> bool {
        self.character(uid).is_some_and(|c| c.grounded)
    }

    fn character(&self, uid: Uid) -> Option<&Character> {
        self.index.get(&uid).and_then(|i| self.driven[*i].character.as_ref())
    }
}

fn controller() -> KinematicCharacterController {
    KinematicCharacterController {
        // Y para baixo: "para cima" é o sentido negativo.
        up: Vector::new(0.0, -1.0),
        ..Default::default()
    }
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

/// Camada e máscara do nó como grupos de colisão. A camada é um bit, pela posição em `LAYERS`.
/// Máscara -1 detecta todas as camadas.
fn mask_groups(n: &Node) -> InteractionGroups {
    let layer = LAYERS.iter().position(|l| *l == text(n, "layer")).unwrap_or(0);
    let mask = num(n, "collision_mask") as i64 as u32;
    InteractionGroups::new(
        Group::from_bits_retain(1 << layer),
        Group::from_bits_retain(mask),
        InteractionTestMode::And,
    )
}

#[cfg(test)]
mod tests {
    use super::*;

    fn node(s: &mut Scene, parent: Option<Uid>, kind: &str, props: serde_json::Value) -> Uid {
        let uid = s.add_node(parent, kind, None).unwrap();
        for (k, v) in props.as_object().unwrap() {
            s.set_prop(uid, k, v.clone(), None).unwrap();
        }
        uid
    }

    fn pos(s: &Scene, uid: Uid) -> [f64; 2] {
        as_vec2(&value_of(s.node(uid).unwrap(), "position"))
    }

    fn run(p: &mut Physics, s: &mut Scene, seconds: f64) {
        let frames = (seconds / STEP).round() as usize;
        for _ in 0..frames {
            p.advance(s, STEP);
        }
    }

    #[test]
    fn caixa_cai_e_para_no_chao() {
        let mut s = Scene::new("teste", 1);
        let root = node(&mut s, None, "Node2D", json!({}));
        let chao = node(&mut s, Some(root), "StaticBody2D", json!({"position": [0.0, 300.0]}));
        node(&mut s, Some(chao), "CollisionShape2D", json!({"size": [400.0, 32.0]}));
        let caixa = node(&mut s, Some(root), "RigidBody2D", json!({"position": [0.0, 0.0]}));
        node(&mut s, Some(caixa), "CollisionShape2D", json!({"size": [32.0, 32.0]}));

        let mut p = Physics::build(&s);
        run(&mut p, &mut s, 3.0);

        // O topo do chão está em y = 284; a caixa tem 32 px, então o centro repousa em y = 268.
        let y = pos(&s, caixa)[1];
        assert!((y - 268.0).abs() < 1.0, "y = {y}");
        let events = p.take_events();
        assert!(events.iter().any(|e| e.kind == EventKind::CollisionStarted && e.node == caixa && e.other == chao));
    }

    #[test]
    fn pose_e_gravada_no_espaco_do_pai() {
        let mut s = Scene::new("teste", 2);
        let root = node(&mut s, None, "Node2D", json!({"position": [100.0, 50.0]}));
        let caixa = node(&mut s, Some(root), "RigidBody2D", json!({}));
        node(&mut s, Some(caixa), "CollisionShape2D", json!({"size": [16.0, 16.0]}));

        let mut p = Physics::build(&s);
        run(&mut p, &mut s, 0.5);

        // Sem movimento lateral, a posição local em x continua zero; a queda aparece em y.
        let [x, y] = pos(&s, caixa);
        assert!(x.abs() < 1e-3, "x = {x}");
        assert!(y > 10.0, "y = {y}");
    }

    #[test]
    fn personagem_para_na_parede() {
        let mut s = Scene::new("teste", 3);
        let root = node(&mut s, None, "Node2D", json!({}));
        let parede = node(&mut s, Some(root), "StaticBody2D", json!({"position": [100.0, 0.0]}));
        node(&mut s, Some(parede), "CollisionShape2D", json!({"size": [20.0, 200.0]}));
        let heroi = node(
            &mut s,
            Some(root),
            "CharacterBody2D",
            json!({"velocity": [300.0, 0.0]}),
        );
        node(&mut s, Some(heroi), "CollisionShape2D", json!({"size": [32.0, 32.0]}));

        let mut p = Physics::build(&s);
        run(&mut p, &mut s, 2.0);

        // A face esquerda da parede está em x = 90; o herói tem 32 px, então para perto de x = 74.
        let x = pos(&s, heroi)[0];
        assert!(x > 60.0 && x < 75.0, "x = {x}");
        assert_eq!(p.velocity(heroi).unwrap()[0], 0.0);
    }

    #[test]
    fn personagem_anda_sem_bater() {
        let mut s = Scene::new("teste", 4);
        let root = node(&mut s, None, "Node2D", json!({}));
        let heroi = node(&mut s, Some(root), "CharacterBody2D", json!({"velocity": [100.0, 0.0]}));
        node(&mut s, Some(heroi), "CollisionShape2D", json!({"size": [32.0, 32.0]}));

        let mut p = Physics::build(&s);
        run(&mut p, &mut s, 1.0);

        let x = pos(&s, heroi)[0];
        assert!((x - 100.0).abs() < 2.0, "x = {x}");
    }

    #[test]
    fn area_detecta_corpo_que_entra() {
        let mut s = Scene::new("teste", 5);
        let root = node(&mut s, None, "Node2D", json!({}));
        let area = node(&mut s, Some(root), "Area2D", json!({"position": [0.0, 0.0]}));
        node(&mut s, Some(area), "CollisionShape2D", json!({"size": [64.0, 64.0]}));
        let corpo = node(&mut s, Some(root), "RigidBody2D", json!({"position": [0.0, -200.0]}));
        node(&mut s, Some(corpo), "CollisionShape2D", json!({"size": [16.0, 16.0]}));

        let mut p = Physics::build(&s);
        let mut entrou = Vec::new();
        for _ in 0..90 {
            p.advance(&mut s, STEP);
            entrou.extend(p.take_events());
        }

        assert!(entrou.iter().any(|e| e.kind == EventKind::BodyEntered && e.node == area && e.other == corpo));
    }

    #[test]
    fn mascara_filtra_quem_a_area_detecta() {
        let mut s = Scene::new("teste", 6);
        let root = node(&mut s, None, "Node2D", json!({}));
        // Máscara 8 = só a camada "enemy" (bit 3).
        let area = node(&mut s, Some(root), "Area2D", json!({"collision_mask": 8}));
        node(&mut s, Some(area), "CollisionShape2D", json!({"size": [64.0, 64.0]}));
        let padrao = node(&mut s, Some(root), "RigidBody2D", json!({"position": [-20.0, -200.0]}));
        node(&mut s, Some(padrao), "CollisionShape2D", json!({"size": [16.0, 16.0]}));
        let inimigo = node(
            &mut s,
            Some(root),
            "RigidBody2D",
            json!({"position": [20.0, -200.0], "layer": "enemy"}),
        );
        node(&mut s, Some(inimigo), "CollisionShape2D", json!({"size": [16.0, 16.0]}));

        let mut p = Physics::build(&s);
        let mut eventos = Vec::new();
        for _ in 0..90 {
            p.advance(&mut s, STEP);
            eventos.extend(p.take_events());
        }

        assert!(!eventos.iter().any(|e| e.other == padrao));
        assert!(eventos.iter().any(|e| e.kind == EventKind::BodyEntered && e.other == inimigo));
    }

    #[test]
    fn areas_sobrepostas_na_montagem_se_detectam() {
        let mut s = Scene::new("teste", 7);
        let root = node(&mut s, None, "Node2D", json!({}));
        let a = node(&mut s, Some(root), "Area2D", json!({}));
        node(&mut s, Some(a), "CollisionShape2D", json!({"size": [64.0, 64.0]}));
        let b = node(&mut s, Some(root), "Area2D", json!({"position": [20.0, 0.0]}));
        node(&mut s, Some(b), "CollisionShape2D", json!({"size": [64.0, 64.0]}));

        // As sobreposições da montagem viram eventos no primeiro passo da simulação.
        let mut p = Physics::build(&s);
        p.advance(&mut s, STEP);
        let eventos = p.take_events();
        assert!(eventos.iter().any(|e| e.kind == EventKind::AreaEntered && e.node == a && e.other == b));
        assert!(eventos.iter().any(|e| e.kind == EventKind::AreaEntered && e.node == b && e.other == a));
    }

    #[test]
    fn forma_desativada_nao_colide() {
        let mut s = Scene::new("teste", 8);
        let root = node(&mut s, None, "Node2D", json!({}));
        let chao = node(&mut s, Some(root), "StaticBody2D", json!({"position": [0.0, 300.0]}));
        node(&mut s, Some(chao), "CollisionShape2D", json!({"size": [400.0, 32.0], "disabled": true}));
        let caixa = node(&mut s, Some(root), "RigidBody2D", json!({}));
        node(&mut s, Some(caixa), "CollisionShape2D", json!({"size": [32.0, 32.0]}));

        let mut p = Physics::build(&s);
        run(&mut p, &mut s, 2.0);

        // Sem chão, a caixa passa do ponto onde ele estaria.
        assert!(pos(&s, caixa)[1] > 300.0);
    }
}

