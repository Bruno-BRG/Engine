//! Juntas de física (PHY-031): PinJoint2D, GrooveJoint2D e DampedSpringJoint2D.
//!
//! Cada junta é um nó filho do corpo A (o corpo pai). A outra ponta é o corpo B, escolhido por UID na
//! propriedade `node_b`. Com `node_b` vazio, a ponta B vira um ponto fixo do mundo, na posição da
//! âncora na montagem (pino e mola). As âncoras são medidas no espaço local de cada corpo, como no
//! Rapier. Os dois corpos não colidem entre si (`contacts_enabled` falso).
//!
//! A junta é montada sobre os corpos que `Physics::build` já criou. Uma junta sem corpo pai, ou cujo
//! `node_b` não é um corpo, é ignorada.

use super::{Physics, num, text};
use nexo_core::scene::{as_vec2, value_of};
use nexo_core::{Node, Scene, Uid};
use rapier2d::prelude::*;

/// Monta as juntas da cena sobre os corpos de `p`. Chamada no fim de `Physics::build`.
pub(crate) fn add(p: &mut Physics, scene: &Scene) {
    for uid in scene.pre_order() {
        let Some(node) = scene.node(uid) else { continue };
        let Some(a) = node.parent.and_then(|pai| p.bodies.get(&pai).copied()) else {
            continue;
        };
        match node.kind.as_str() {
            "PinJoint2D" => pin(p, node, a),
            "GrooveJoint2D" => groove(p, node, a),
            "DampedSpringJoint2D" => spring(p, node, a),
            _ => {}
        }
    }
}

/// PinJoint2D: pino no ponto `anchor` do corpo pai. O corpo B fica preso no mesmo ponto do mundo.
fn pin(p: &mut Physics, node: &Node, a: RigidBodyHandle) {
    let ancora = vetor(as_vec2(&value_of(node, "anchor")));
    let ponto = para_mundo(&pose_de(p, a), ancora);
    let Some(b) = corpo_b(p, node, a, ponto) else { return };
    let junta = RevoluteJointBuilder::new()
        .local_anchor1(ancora)
        .local_anchor2(para_local(&pose_de(p, b), ponto))
        .contacts_enabled(false)
        .build();
    p.world.insert_impulse_joint(a, b, junta);
}

/// GrooveJoint2D: o corpo B desliza num trilho do corpo pai, de `groove_start` a `groove_end`. A âncora
/// `anchor_b` (no corpo B) fica sobre o trilho e não sai dele. Exige `node_b`: sem B, não há o que deslizar.
fn groove(p: &mut Physics, node: &Node, a: RigidBodyHandle) {
    if text(node, "node_b").is_empty() {
        return;
    }
    let inicio = vetor(as_vec2(&value_of(node, "groove_start")));
    let fim = vetor(as_vec2(&value_of(node, "groove_end")));
    let comprimento = (fim - inicio).length();
    if comprimento <= 1e-6 {
        return;
    }
    let Some(b) = corpo_b(p, node, a, Vector::ZERO) else {
        return;
    };
    // O eixo do trilho é medido em B na montagem: a mesma direção do mundo, no referencial de B.
    let eixo = (fim - inicio) / comprimento;
    let eixo_b = girar(eixo, pose_de(p, a).rotation.angle() - pose_de(p, b).rotation.angle());
    let junta = PinSlotJointBuilder::new(eixo)
        .local_anchor1(inicio)
        .local_anchor2(vetor(as_vec2(&value_of(node, "anchor_b"))))
        .local_axis2(eixo_b)
        .limits([0.0, comprimento])
        .contacts_enabled(false)
        .build();
    p.world.insert_impulse_joint(a, b, junta);
}

/// DampedSpringJoint2D: mola entre `anchor` (corpo pai) e `anchor_b` (corpo B). Puxa as âncoras até o
/// comprimento de repouso e amortece a oscilação. Usa o modelo de aceleração: rigidez e amortecimento
/// têm o mesmo efeito em corpos leves e pesados.
fn spring(p: &mut Physics, node: &Node, a: RigidBodyHandle) {
    let ancora = vetor(as_vec2(&value_of(node, "anchor")));
    let ponto = para_mundo(&pose_de(p, a), ancora);
    let Some(b) = corpo_b(p, node, a, ponto) else { return };
    let junta = SpringJointBuilder::new(
        num(node, "rest_length") as f32,
        num(node, "stiffness") as f32,
        num(node, "damping") as f32,
    )
    .spring_model(MotorModel::AccelerationBased)
    .local_anchor1(ancora)
    .local_anchor2(vetor(as_vec2(&value_of(node, "anchor_b"))))
    .contacts_enabled(false)
    .build();
    p.world.insert_impulse_joint(a, b, junta);
}

/// Corpo B da junta: o corpo de `node_b`, ou, com `node_b` vazio, um ponto fixo do mundo em `ponto`.
/// `None` se `node_b` não aponta para um corpo, ou se aponta para o próprio corpo A.
fn corpo_b(p: &mut Physics, node: &Node, a: RigidBodyHandle, ponto: Vector) -> Option<RigidBodyHandle> {
    let b = match text(node, "node_b").as_str() {
        "" => p
            .world
            .bodies
            .insert(RigidBodyBuilder::fixed().translation(ponto).build()),
        texto => *p.bodies.get(&texto.parse::<Uid>().ok()?)?,
    };
    (b != a).then_some(b)
}

fn vetor(v: [f64; 2]) -> Vector {
    Vector::new(v[0] as f32, v[1] as f32)
}

/// Pose atual do corpo no Rapier.
fn pose_de(p: &Physics, corpo: RigidBodyHandle) -> Pose {
    *p.world.bodies[corpo].position()
}

/// Gira `v` por `radianos`, com a mesma matriz de rotação do Rapier.
fn girar(v: Vector, radianos: f32) -> Vector {
    let (s, c) = radianos.sin_cos();
    Vector::new(c * v.x - s * v.y, s * v.x + c * v.y)
}

/// Ponto no espaço local do corpo para o mundo.
fn para_mundo(pose: &Pose, ponto: Vector) -> Vector {
    pose.translation + girar(ponto, pose.rotation.angle())
}

/// Ponto do mundo para o espaço local do corpo.
fn para_local(pose: &Pose, ponto: Vector) -> Vector {
    girar(ponto - pose.translation, -pose.rotation.angle())
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::STEP;
    use nexo_core::ProjectSettings;
    use nexo_core::Scene;
    use serde_json::json;

    fn node(s: &mut Scene, parent: Option<Uid>, kind: &str, props: serde_json::Value) -> Uid {
        let uid = s.add_node(parent, kind, None).unwrap();
        for (k, v) in props.as_object().unwrap() {
            s.set_prop(uid, k, v.clone(), None).unwrap();
        }
        uid
    }

    fn passa(p: &mut Physics, s: &mut Scene, segundos: f64) {
        for _ in 0..(segundos / STEP).round() as usize {
            p.advance(s, STEP);
        }
    }

    fn pos(s: &Scene, uid: Uid) -> [f64; 2] {
        as_vec2(&value_of(s.node(uid).unwrap(), "position"))
    }

    #[test]
    fn pino_mantem_a_distancia_ao_ponto_de_apoio() {
        let mut s = Scene::new("teste", 21);
        let root = node(&mut s, None, "Node2D", json!({}));
        let teto = node(&mut s, Some(root), "StaticBody2D", json!({}));
        let bola = node(&mut s, Some(root), "RigidBody2D", json!({"position": [100.0, 0.0]}));
        node(&mut s, Some(bola), "CollisionShape2D", json!({"size": [16.0, 16.0]}));
        node(&mut s, Some(teto), "PinJoint2D", json!({"node_b": bola.to_string()}));
        let mut p = Physics::build(&s, &ProjectSettings::default());

        // A bola começa na horizontal e cai balançando, como um pêndulo.
        passa(&mut p, &mut s, 0.3);
        let [_, y] = pos(&s, bola);
        assert!(y > 30.0, "a bola deveria ter descido: y = {y}");

        passa(&mut p, &mut s, 2.7);
        let [x, y] = pos(&s, bola);
        let d = (x * x + y * y).sqrt();
        assert!((d - 100.0).abs() < 1.5, "distância ao pino = {d}");
    }

    #[test]
    fn pino_sem_corpo_b_prende_no_ponto_do_mundo() {
        let mut s = Scene::new("teste", 22);
        let root = node(&mut s, None, "Node2D", json!({}));
        // A bola começa à direita de (0, -50). O pino está na própria bola, na âncora (-100, 0), e sem
        // corpo B ele prende a bola ao ponto do mundo (0, -50).
        let bola = node(&mut s, Some(root), "RigidBody2D", json!({"position": [100.0, -50.0]}));
        node(&mut s, Some(bola), "CollisionShape2D", json!({"size": [16.0, 16.0]}));
        node(&mut s, Some(bola), "PinJoint2D", json!({"anchor": [-100.0, 0.0]}));
        let mut p = Physics::build(&s, &ProjectSettings::default());
        passa(&mut p, &mut s, 3.0);

        let [x, y] = pos(&s, bola);
        let d = (x * x + (y + 50.0) * (y + 50.0)).sqrt();
        assert!((d - 100.0).abs() < 1.5, "distância ao ponto fixo = {d}");
    }

    #[test]
    fn mola_oscila_e_amortece() {
        let mut s = Scene::new("teste", 23);
        let root = node(&mut s, None, "Node2D", json!({}));
        let teto = node(&mut s, Some(root), "StaticBody2D", json!({}));
        let bola = node(&mut s, Some(root), "RigidBody2D", json!({"position": [0.0, 200.0]}));
        node(&mut s, Some(bola), "CollisionShape2D", json!({"size": [16.0, 16.0]}));
        // Valores padrão: repouso 100, rigidez 20, amortecimento 1.
        node(
            &mut s,
            Some(teto),
            "DampedSpringJoint2D",
            json!({"node_b": bola.to_string()}),
        );
        let mut p = Physics::build(&s, &ProjectSettings::default());

        // Equilíbrio com a gravidade: repouso + g / rigidez = 100 + 980 / 20 = 149 px.
        let equilibrio = 149.0;
        let mut distancias = Vec::new();
        for _ in 0..(6.0 / STEP) as usize {
            p.advance(&mut s, STEP);
            let [x, y] = pos(&s, bola);
            distancias.push((x * x + y * y).sqrt());
        }

        let cruzamentos = distancias
            .windows(2)
            .filter(|w| (w[0] - equilibrio).signum() != (w[1] - equilibrio).signum())
            .count();
        assert!(cruzamentos >= 4, "cruzamentos = {cruzamentos}");

        let amplitude = |trecho: &[f64]| trecho.iter().map(|d| (d - equilibrio).abs()).fold(0.0, f64::max);
        let inicio = amplitude(&distancias[..60]);
        let fim = amplitude(&distancias[distancias.len() - 60..]);
        assert!(fim < inicio * 0.5, "amplitude no início = {inicio}, no fim = {fim}");
    }

    #[test]
    fn trilho_restringe_o_corpo_a_uma_linha() {
        let mut s = Scene::new("teste", 24);
        let root = node(&mut s, None, "Node2D", json!({}));
        let pai = node(&mut s, Some(root), "StaticBody2D", json!({}));
        let bola = node(&mut s, Some(root), "RigidBody2D", json!({"position": [10.0, 10.0]}));
        node(&mut s, Some(bola), "CollisionShape2D", json!({"size": [16.0, 16.0]}));
        // Trilho diagonal de (0, 0) a (100, 100). A gravidade empurra a bola para o fim do trilho.
        node(
            &mut s,
            Some(pai),
            "GrooveJoint2D",
            json!({"node_b": bola.to_string(), "groove_start": [0.0, 0.0], "groove_end": [100.0, 100.0]}),
        );
        let mut p = Physics::build(&s, &ProjectSettings::default());
        passa(&mut p, &mut s, 3.0);

        // Sem o trilho, a bola cairia mais de 1000 px em 2 s. Preso à reta y = x, ela para no fim.
        let [x, y] = pos(&s, bola);
        assert!((y - x).abs() < 1.0, "fora do trilho: x = {x}, y = {y}");
        assert!((x - 100.0).abs() < 2.0 && (y - 100.0).abs() < 2.0, "x = {x}, y = {y}");
    }

    #[test]
    fn junta_sem_corpo_pai_ou_com_alvo_que_nao_e_corpo_e_ignorada() {
        let mut s = Scene::new("teste", 25);
        let root = node(&mut s, None, "Node2D", json!({}));
        let nao_corpo = node(&mut s, Some(root), "Node2D", json!({"position": [5.0, 5.0]}));
        // Pino na raiz: não tem corpo pai.
        node(
            &mut s,
            Some(root),
            "PinJoint2D",
            json!({"node_b": nao_corpo.to_string()}),
        );
        let teto = node(&mut s, Some(root), "StaticBody2D", json!({}));
        // Pino cujo node_b é um Node2D comum, não um corpo.
        node(
            &mut s,
            Some(teto),
            "PinJoint2D",
            json!({"node_b": nao_corpo.to_string()}),
        );

        let p = Physics::build(&s, &ProjectSettings::default());
        assert_eq!(p.world.impulse_joints().count(), 0);
    }
}
