//! Desenho de depuração da física (DBG-004): formas, contatos e velocidades do último passo, em pixels
//! do mundo. Este módulo só entrega os dados; o viewport decide como desenhar e a alternância fica no editor.

use super::*;
use rapier2d::parry::shape::TypedShape;
use std::f32::consts::{FRAC_PI_2, PI, TAU};

/// Pontos por circunferência inteira nos desenhos aproximados (círculos).
const ARC_STEPS: usize = 24;
/// Pontos por semicírculo nas pontas das cápsulas.
const CAP_STEPS: usize = 8;

/// Papel do corpo dono de uma forma. O viewport escolhe a cor por ele.
#[derive(Clone, Copy, Debug, PartialEq, Eq, Serialize)]
#[serde(rename_all = "snake_case")]
pub enum BodyRole {
    /// Corpo imóvel: StaticBody2D ou RigidBody2D congelado estático.
    Static,
    /// Corpo dinâmico: RigidBody2D sem congelamento.
    Dynamic,
    /// Corpo cinemático: RigidBody2D congelado cinemático.
    Kinematic,
    /// Personagem (CharacterBody2D).
    Character,
    /// Área: sensor que detecta sem colidir.
    Area,
}

/// Forma de colisão no mundo. Círculos e cápsulas viram polígonos aproximados; segmentos ficam abertos.
#[derive(Clone, Debug, Serialize)]
pub struct DebugShape {
    /// Nó do corpo ou da área que contém a forma.
    pub node: Uid,
    pub role: BodyRole,
    pub points: Vec<[f64; 2]>,
    /// True para polígono fechado; false para segmento.
    pub closed: bool,
}

/// Ponto de contato sólido ativo no último passo. A normal aponta do primeiro colisor para o segundo,
/// como o Rapier a dá.
#[derive(Clone, Copy, Debug, Serialize)]
pub struct DebugContact {
    pub point: [f64; 2],
    pub normal: [f64; 2],
}

/// Velocidade de um corpo dinâmico ou de um personagem, em px/s. A origem é o centro do corpo.
#[derive(Clone, Copy, Debug, Serialize)]
pub struct DebugVelocity {
    pub node: Uid,
    pub origin: [f64; 2],
    pub velocity: [f64; 2],
}

/// Tudo o que o desenho de depuração mostra num instante.
#[derive(Clone, Debug, Default, Serialize)]
pub struct DebugDraw {
    pub shapes: Vec<DebugShape>,
    pub contacts: Vec<DebugContact>,
    pub velocities: Vec<DebugVelocity>,
}

impl Physics {
    /// Formas, contatos e velocidades agora (DBG-004), a partir do último passo da simulação.
    pub fn debug_draw(&self) -> DebugDraw {
        let mut out = DebugDraw::default();

        for (handle, collider) in self.world.colliders.iter() {
            let Some(&node) = self.owner.get(&handle) else { continue };
            let role = self.role_of(node, collider);
            let mut parts = Vec::new();
            contornos(collider.shape(), *collider.position(), &mut parts);
            for (points, closed) in parts {
                out.shapes.push(DebugShape { node, role, points, closed });
            }
        }

        for pair in self.world.narrow_phase.contact_pairs() {
            let (Some(c1), Some(c2)) = (self.world.colliders.get(pair.collider1), self.world.colliders.get(pair.collider2))
            else {
                continue;
            };
            let (Some(a), Some(b)) = (self.owner.get(&pair.collider1), self.owner.get(&pair.collider2)) else {
                continue;
            };
            if a == b {
                continue;
            }
            let (p1, p2) = (*c1.position(), *c2.position());
            for m in pair.manifolds() {
                for c in m.points.iter().filter(|c| c.dist <= 0.0) {
                    let ponto = (p1.transform_point(c.local_p1) + p2.transform_point(c.local_p2)) * 0.5;
                    out.contacts.push(DebugContact { point: para_pt(ponto), normal: para_pt(m.data.normal) });
                }
            }
        }

        for d in &self.driven {
            let body = &self.world.bodies[d.handle];
            let velocity = match &d.character {
                // O personagem mostra a própria velocidade, sem a carona da superfície.
                Some(ch) => ch.velocity,
                None => para_pt(body.linvel()),
            };
            out.velocities.push(DebugVelocity { node: d.uid, origin: para_pt(body.translation()), velocity });
        }

        out
    }

    /// Papel do corpo que contém o colisor. Área é sensor; o resto segue o tipo do corpo no Rapier.
    fn role_of(&self, node: Uid, collider: &Collider) -> BodyRole {
        if collider.is_sensor() {
            return BodyRole::Area;
        }
        if self.index.get(&node).is_some_and(|&i| self.driven[i].character.is_some()) {
            return BodyRole::Character;
        }
        match self.bodies.get(&node).map(|&h| self.world.bodies[h].body_type()) {
            Some(RigidBodyType::Dynamic) => BodyRole::Dynamic,
            Some(RigidBodyType::Fixed) => BodyRole::Static,
            _ => BodyRole::Kinematic,
        }
    }
}

fn para_pt(v: Vector) -> [f64; 2] {
    [v.x as f64, v.y as f64]
}

/// Contornos de uma forma no mundo, com a pose dela. Composta (polígono côncavo) vira uma parte por vez.
fn contornos(shape: &dyn Shape, pose: Pose, out: &mut Vec<(Vec<[f64; 2]>, bool)>) {
    match shape.as_typed_shape() {
        TypedShape::Ball(b) => out.push((circulo(pose, b.radius), true)),
        TypedShape::Cuboid(c) => {
            let h = c.half_extents;
            let cantos = [(-h.x, -h.y), (h.x, -h.y), (h.x, h.y), (-h.x, h.y)];
            out.push((cantos.iter().map(|&(x, y)| para_pt(pose.transform_point(Vector::new(x, y)))).collect(), true));
        }
        TypedShape::Capsule(c) => out.push((capsula(pose, c.segment.a, c.segment.b, c.radius), true)),
        TypedShape::Segment(s) => {
            out.push((vec![para_pt(pose.transform_point(s.a)), para_pt(pose.transform_point(s.b))], false))
        }
        TypedShape::ConvexPolygon(p) => {
            out.push((p.points().iter().map(|&q| para_pt(pose.transform_point(q))).collect(), true))
        }
        TypedShape::Compound(c) => {
            for (sub_pose, sub) in c.shapes() {
                contornos(&**sub, pose * *sub_pose, out);
            }
        }
        // Os outros formatos não são criados pela física da cena.
        _ => {}
    }
}

/// Circunferência de raio `r` centrada na origem da pose.
fn circulo(pose: Pose, r: Real) -> Vec<[f64; 2]> {
    (0..ARC_STEPS)
        .map(|i| {
            let t = TAU * i as f32 / ARC_STEPS as f32;
            para_pt(pose.transform_point(Vector::new(t.cos(), t.sin()) * r))
        })
        .collect()
}

/// Cápsula de eixo `a`-`b` e raio `r`: dois semicírculos ligados pelos lados retos.
fn capsula(pose: Pose, a: Vector, b: Vector, r: Real) -> Vec<[f64; 2]> {
    let d = b - a;
    let base = d.y.atan2(d.x);
    let mut pontos = Vec::with_capacity(2 * (CAP_STEPS + 1));
    // Semicírculo em `b`, do lado direito ao esquerdo do eixo, e depois o de `a`.
    for i in 0..=CAP_STEPS {
        let t = base - FRAC_PI_2 + PI * i as f32 / CAP_STEPS as f32;
        pontos.push(para_pt(pose.transform_point(b + Vector::new(t.cos(), t.sin()) * r)));
    }
    for i in 0..=CAP_STEPS {
        let t = base + FRAC_PI_2 + PI * i as f32 / CAP_STEPS as f32;
        pontos.push(para_pt(pose.transform_point(a + Vector::new(t.cos(), t.sin()) * r)));
    }
    pontos
}
