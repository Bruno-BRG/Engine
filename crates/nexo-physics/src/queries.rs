//! Consultas de física (PHY-030): RayCast2D, ShapeCast2D e consultas livres (raio, ponto e retângulo),
//! expostas ao WASM e, depois, à programação visual.
//!
//! Camadas: um colisor pertence às camadas do seu corpo, como em `mask_groups` (lib.rs). Cada camada
//! é um bit, pela posição em `LAYERS`. A máscara de uma consulta usa o mesmo formato: o bit `i`
//! escolhe a camada `LAYERS[i]`, e -1 escolhe todas. A consulta acerta um colisor se alguma camada
//! dele está na máscara. A máscara do próprio corpo (quais camadas ele detecta) não entra na consulta.
//! Áreas são sensores: só são atingidas com `com_areas`, como `collide_with_areas` do nó.
//!
//! As consultas usam a árvore de colisão do último passo da física. Antes do primeiro passo, não veem
//! corpo nenhum.

use super::{Physics, flag, flag_or, num, text, to_rad};
use nexo_core::query::{self, Xf};
use nexo_core::scene::{as_vec2, value_of};
use nexo_core::{Node, Scene, Uid};
use rapier2d::parry::query::{Ray, ShapeCastOptions};
use rapier2d::prelude::*;
use serde::Serialize;

/// Resultado de uma consulta. Sem acerto, `hit` é falso e os demais campos são nulos.
#[derive(Clone, Copy, Debug, Default, PartialEq, Serialize)]
pub struct Contact {
    pub hit: bool,
    /// Nó do corpo atingido.
    pub node: Option<Uid>,
    /// Ponto do acerto, em pixels do mundo.
    pub point: Option<[f64; 2]>,
    /// Normal da superfície atingida, apontando para fora do corpo.
    pub normal: Option<[f64; 2]>,
    /// Distância percorrida até o acerto, em pixels.
    pub distance: Option<f64>,
}

/// Predicado de camada: o colisor passa se alguma de suas camadas está em `mascara`.
fn na_mascara(mascara: u32) -> impl Fn(ColliderHandle, &Collider) -> bool {
    move |_: ColliderHandle, c: &Collider| c.collision_groups().memberships.bits() & mascara != 0
}

/// Máscara de um nó de consulta. -1 vira todas as camadas, como no corpo.
fn mascara_do_no(node: &Node) -> u32 {
    num(node, "collision_mask") as i64 as u32
}

fn vetor(v: [f64; 2]) -> Vector {
    Vector::new(v[0] as f32, v[1] as f32)
}

/// Transformação mundial do nó de consulta e o ponto do mundo que o alvo indica.
fn origem_e_alvo(scene: &Scene, uid: Uid, node: &Node) -> Option<(Xf, [f64; 2])> {
    let xf = query::world_map(scene, None).into_iter().find(|(u, _, _)| *u == uid)?.1;
    let [tx, ty] = as_vec2(&value_of(node, "target"));
    let alvo = xf.then(Xf {
        x: tx,
        y: ty,
        ..Xf::IDENTITY
    });
    Some((xf, [alvo.x, alvo.y]))
}

impl Physics {
    /// Resultado do RayCast2D `uid` agora (PHY-030). `None` se `uid` não é um RayCast2D.
    /// O raio sai da posição do nó até o alvo (relativo ao nó) e ignora o corpo pai.
    pub fn ray_hit(&self, scene: &Scene, uid: Uid) -> Option<Contact> {
        let node = scene.node(uid)?;
        if node.kind != "RayCast2D" {
            return None;
        }
        if !flag_or(node, "enabled", true) {
            return Some(Contact::default());
        }
        let (xf, alvo) = origem_e_alvo(scene, uid, node)?;
        let direcao = [alvo[0] - xf.x, alvo[1] - xf.y];
        let distancia = direcao[0].hypot(direcao[1]);
        Some(self.raio(
            [xf.x, xf.y],
            direcao,
            distancia,
            mascara_do_no(node),
            flag(node, "collide_with_areas"),
            self.corpo_pai(node),
        ))
    }

    /// Resultado do ShapeCast2D `uid` agora (PHY-030). A forma (`shape`, `size`) varre o caminho do nó
    /// até o alvo. `None` se `uid` não é um ShapeCast2D. `distance` é o caminho percorrido pelo centro.
    pub fn shape_hit(&self, scene: &Scene, uid: Uid) -> Option<Contact> {
        let node = scene.node(uid)?;
        if node.kind != "ShapeCast2D" {
            return None;
        }
        if !flag_or(node, "enabled", true) {
            return Some(Contact::default());
        }
        let (xf, alvo) = origem_e_alvo(scene, uid, node)?;
        // Mesma regra do CollisionShape2D: o tamanho acompanha a escala do nó.
        let [w, h] = as_vec2(&value_of(node, "size"));
        let (w, h) = ((w * xf.sx.abs()).max(0.5), (h * xf.sy.abs()).max(0.5));
        let circulo = Ball::new((w / 2.0) as f32);
        let retangulo = Cuboid::new(vetor([w / 2.0, h / 2.0]));
        let forma: &dyn Shape = if text(node, "shape") == "circle" {
            &circulo
        } else {
            &retangulo
        };
        let pose = Pose::new(vetor([xf.x, xf.y]), to_rad(xf.rot));
        let velocidade = vetor([alvo[0] - xf.x, alvo[1] - xf.y]);
        let comprimento = velocidade.length();
        if comprimento <= 1e-6 {
            return Some(Contact::default());
        }
        let camadas = na_mascara(mascara_do_no(node));
        let consulta = self.pipeline(&camadas, flag(node, "collide_with_areas"), self.corpo_pai(node));
        let resultado = consulta.cast_shape(&pose, velocidade, forma, ShapeCastOptions::with_max_time_of_impact(1.0));
        Some(match resultado {
            Some((colisor, hit)) => Contact {
                hit: true,
                node: self.owner.get(&colisor).copied(),
                point: Some([hit.witness1.x as f64, hit.witness1.y as f64]),
                normal: Some([hit.normal1.x as f64, hit.normal1.y as f64]),
                distance: Some(hit.time_of_impact as f64 * comprimento as f64),
            },
            None => Contact::default(),
        })
    }

    /// Consulta de raio sem nó: o primeiro colisor atingido por um raio que sai de `origem` na
    /// direção `direcao`, por até `distancia` pixels. `mascara` usa os bits das camadas.
    pub fn raycast(
        &self,
        origem: [f64; 2],
        direcao: [f64; 2],
        distancia: f64,
        mascara: u32,
        com_areas: bool,
    ) -> Contact {
        self.raio(origem, direcao, distancia, mascara, com_areas, None)
    }

    /// Nós com algum colisor que contém o `ponto`, sem repetição. Consulta livre, sem nó.
    pub fn overlap_point(&self, ponto: [f64; 2], mascara: u32, com_areas: bool) -> Vec<Uid> {
        let camadas = na_mascara(mascara);
        let consulta = self.pipeline(&camadas, com_areas, None);
        let mut nos = Vec::new();
        for (colisor, _) in consulta.intersect_point(vetor(ponto)) {
            self.push_no(&mut nos, colisor);
        }
        nos
    }

    /// Nós com algum colisor que toca o retângulo de centro `centro` e tamanho `tamanho`.
    /// Consulta livre, sem nó.
    pub fn overlap_rect(&self, centro: [f64; 2], tamanho: [f64; 2], mascara: u32, com_areas: bool) -> Vec<Uid> {
        let camadas = na_mascara(mascara);
        let consulta = self.pipeline(&camadas, com_areas, None);
        let retangulo = Cuboid::new(vetor([tamanho[0] / 2.0, tamanho[1] / 2.0]));
        let mut nos = Vec::new();
        for (colisor, _) in consulta.intersect_shape(Pose::new(vetor(centro), 0.0), &retangulo) {
            self.push_no(&mut nos, colisor);
        }
        nos
    }

    /// Raio de `origem` na direção `direcao` (normalizada aqui), por até `distancia`.
    fn raio(
        &self,
        origem: [f64; 2],
        direcao: [f64; 2],
        distancia: f64,
        mascara: u32,
        com_areas: bool,
        ignorar: Option<RigidBodyHandle>,
    ) -> Contact {
        let dir = vetor(direcao);
        let comprimento = dir.length();
        if comprimento <= 1e-6 || distancia <= 0.0 {
            return Contact::default();
        }
        let raio = Ray::new(vetor(origem), dir / comprimento);
        let camadas = na_mascara(mascara);
        let consulta = self.pipeline(&camadas, com_areas, ignorar);
        // `solid`: um raio que começa dentro de um corpo já o atinge no ponto de partida.
        match consulta.cast_ray_and_get_normal(&raio, distancia as f32, true) {
            Some((colisor, hit)) => self.contato(
                colisor,
                raio.point_at(hit.time_of_impact),
                hit.normal,
                hit.time_of_impact,
            ),
            None => Contact::default(),
        }
    }

    /// Pipeline de consulta com o filtro de camadas, o de áreas e o corpo que a consulta ignora.
    fn pipeline<'a>(
        &'a self,
        camadas: &'a dyn Fn(ColliderHandle, &Collider) -> bool,
        com_areas: bool,
        ignorar: Option<RigidBodyHandle>,
    ) -> QueryPipeline<'a> {
        let flags = if com_areas {
            QueryFilterFlags::empty()
        } else {
            QueryFilterFlags::EXCLUDE_SENSORS
        };
        let filtro = QueryFilter {
            flags,
            exclude_rigid_body: ignorar,
            predicate: Some(camadas),
            ..Default::default()
        };
        self.world.query_pipeline_with_filter(filtro)
    }

    /// Corpo pai de um nó de consulta, se o pai é um corpo da física.
    fn corpo_pai(&self, node: &Node) -> Option<RigidBodyHandle> {
        node.parent.and_then(|pai| self.bodies.get(&pai).copied())
    }

    fn contato(&self, colisor: ColliderHandle, ponto: Vector, normal: Vector, distancia: f32) -> Contact {
        Contact {
            hit: true,
            node: self.owner.get(&colisor).copied(),
            point: Some([ponto.x as f64, ponto.y as f64]),
            normal: Some([normal.x as f64, normal.y as f64]),
            distance: Some(distancia as f64),
        }
    }

    /// Acrescenta o nó dono do colisor, se ainda não está na lista.
    fn push_no(&self, nos: &mut Vec<Uid>, colisor: ColliderHandle) {
        if let Some(uid) = self.owner.get(&colisor) {
            if !nos.contains(uid) {
                nos.push(*uid);
            }
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::STEP;
    use nexo_core::ProjectSettings;
    use serde_json::json;

    fn node(s: &mut Scene, parent: Option<Uid>, kind: &str, props: serde_json::Value) -> Uid {
        let uid = s.add_node(parent, kind, None).unwrap();
        for (k, v) in props.as_object().unwrap() {
            s.set_prop(uid, k, v.clone(), None).unwrap();
        }
        uid
    }

    /// Avança a física `n` passos. A árvore de colisão só enxerga os corpos depois de um passo.
    fn passos(p: &mut Physics, s: &mut Scene, n: usize) {
        for _ in 0..n {
            p.advance(s, STEP);
        }
    }

    #[test]
    fn raio_acerta_o_primeiro_corpo() {
        let mut s = Scene::new("teste", 11);
        let root = node(&mut s, None, "Node2D", json!({}));
        let raio = node(&mut s, Some(root), "RayCast2D", json!({"target": [0.0, 500.0]}));
        let caixa = node(&mut s, Some(root), "StaticBody2D", json!({"position": [0.0, 200.0]}));
        node(&mut s, Some(caixa), "CollisionShape2D", json!({"size": [40.0, 40.0]}));
        let mut p = Physics::build(&s, &ProjectSettings::default());
        passos(&mut p, &mut s, 1);

        let c = p.ray_hit(&s, raio).unwrap();
        assert!(c.hit, "{c:?}");
        assert_eq!(c.node, Some(caixa));
        // A face de cima da caixa está em y = 180.
        let d = c.distance.unwrap();
        assert!((d - 180.0).abs() < 0.5, "d = {d}");
        let [px, py] = c.point.unwrap();
        assert!(px.abs() < 0.5 && (py - 180.0).abs() < 0.5, "{c:?}");
        let [nx, ny] = c.normal.unwrap();
        assert!(nx.abs() < 1e-3 && ny < -0.999, "{c:?}");
    }

    #[test]
    fn raio_erra_sem_corpo_no_caminho_e_desligado_nao_acerta() {
        let mut s = Scene::new("teste", 12);
        let root = node(&mut s, None, "Node2D", json!({}));
        let raio = node(&mut s, Some(root), "RayCast2D", json!({"target": [500.0, 0.0]}));
        let caixa = node(&mut s, Some(root), "StaticBody2D", json!({"position": [0.0, 200.0]}));
        node(&mut s, Some(caixa), "CollisionShape2D", json!({"size": [40.0, 40.0]}));
        let mut p = Physics::build(&s, &ProjectSettings::default());
        passos(&mut p, &mut s, 1);

        // O raio aponta para a direita; a caixa está embaixo.
        assert_eq!(p.ray_hit(&s, raio).unwrap(), Contact::default());
        s.set_prop(raio, "target", json!([0.0, 500.0]), None).unwrap();
        s.set_prop(raio, "enabled", json!(false), None).unwrap();
        assert_eq!(p.ray_hit(&s, raio).unwrap(), Contact::default());
    }

    #[test]
    fn raio_respeita_a_mascara_de_camadas() {
        let mut s = Scene::new("teste", 13);
        let root = node(&mut s, None, "Node2D", json!({}));
        // A camada "enemy" é o bit 3, valor 8.
        let inimigo = node(
            &mut s,
            Some(root),
            "StaticBody2D",
            json!({"position": [0.0, 200.0], "layer": "enemy"}),
        );
        node(&mut s, Some(inimigo), "CollisionShape2D", json!({"size": [40.0, 40.0]}));
        let raio = node(
            &mut s,
            Some(root),
            "RayCast2D",
            json!({"target": [0.0, 500.0], "collision_mask": 1}),
        );
        let mut p = Physics::build(&s, &ProjectSettings::default());
        passos(&mut p, &mut s, 1);

        assert!(!p.ray_hit(&s, raio).unwrap().hit, "a camada default não vê o inimigo");
        s.set_prop(raio, "collision_mask", json!(8), None).unwrap();
        assert_eq!(p.ray_hit(&s, raio).unwrap().node, Some(inimigo));
        s.set_prop(raio, "collision_mask", json!(-1), None).unwrap();
        assert_eq!(p.ray_hit(&s, raio).unwrap().node, Some(inimigo));
    }

    #[test]
    fn raio_ignora_areas_sem_collide_with_areas() {
        let mut s = Scene::new("teste", 14);
        let root = node(&mut s, None, "Node2D", json!({}));
        let area = node(&mut s, Some(root), "Area2D", json!({"position": [0.0, 100.0]}));
        node(&mut s, Some(area), "CollisionShape2D", json!({"size": [40.0, 40.0]}));
        let caixa = node(&mut s, Some(root), "StaticBody2D", json!({"position": [0.0, 200.0]}));
        node(&mut s, Some(caixa), "CollisionShape2D", json!({"size": [40.0, 40.0]}));
        let raio = node(&mut s, Some(root), "RayCast2D", json!({"target": [0.0, 500.0]}));
        let mut p = Physics::build(&s, &ProjectSettings::default());
        passos(&mut p, &mut s, 1);

        assert_eq!(p.ray_hit(&s, raio).unwrap().node, Some(caixa));
        s.set_prop(raio, "collide_with_areas", json!(true), None).unwrap();
        // A área tem topo em y = 80, antes da caixa.
        let c = p.ray_hit(&s, raio).unwrap();
        assert_eq!(c.node, Some(area));
        assert!((c.distance.unwrap() - 80.0).abs() < 0.5, "{c:?}");
    }

    #[test]
    fn raio_nao_acerta_o_proprio_corpo_pai() {
        let mut s = Scene::new("teste", 15);
        let root = node(&mut s, None, "Node2D", json!({}));
        let pai = node(&mut s, Some(root), "StaticBody2D", json!({}));
        node(&mut s, Some(pai), "CollisionShape2D", json!({"size": [40.0, 40.0]}));
        let raio = node(&mut s, Some(pai), "RayCast2D", json!({"target": [0.0, 500.0]}));
        let caixa = node(&mut s, Some(root), "StaticBody2D", json!({"position": [0.0, 200.0]}));
        node(&mut s, Some(caixa), "CollisionShape2D", json!({"size": [40.0, 40.0]}));
        let mut p = Physics::build(&s, &ProjectSettings::default());
        passos(&mut p, &mut s, 1);

        // A origem está dentro da forma do pai, que é ignorada.
        assert_eq!(p.ray_hit(&s, raio).unwrap().node, Some(caixa));
    }

    #[test]
    fn shape_cast_para_na_forma_antes_do_corpo() {
        let mut s = Scene::new("teste", 16);
        let root = node(&mut s, None, "Node2D", json!({}));
        let caixa = node(&mut s, Some(root), "StaticBody2D", json!({"position": [0.0, 200.0]}));
        node(&mut s, Some(caixa), "CollisionShape2D", json!({"size": [40.0, 40.0]}));
        // Círculo de raio 8 descendo da origem: a base toca o topo (y = 180) depois de 172 px.
        let varredura = node(
            &mut s,
            Some(root),
            "ShapeCast2D",
            json!({"shape": "circle", "size": [16.0, 16.0], "target": [0.0, 500.0]}),
        );
        let mut p = Physics::build(&s, &ProjectSettings::default());
        passos(&mut p, &mut s, 1);

        let c = p.shape_hit(&s, varredura).unwrap();
        assert!(c.hit, "{c:?}");
        assert_eq!(c.node, Some(caixa));
        let d = c.distance.unwrap();
        assert!((d - 172.0).abs() < 1.0, "d = {d}");
        let [nx, ny] = c.normal.unwrap();
        assert!(nx.abs() < 0.05 && ny < -0.99, "{c:?}");
    }

    #[test]
    fn consultas_livres_acertam_e_respeitam_a_mascara() {
        let mut s = Scene::new("teste", 17);
        let root = node(&mut s, None, "Node2D", json!({}));
        let inimigo = node(
            &mut s,
            Some(root),
            "StaticBody2D",
            json!({"position": [0.0, 200.0], "layer": "enemy"}),
        );
        node(&mut s, Some(inimigo), "CollisionShape2D", json!({"size": [40.0, 40.0]}));
        let mut p = Physics::build(&s, &ProjectSettings::default());
        passos(&mut p, &mut s, 1);

        let so_inimigo = 1 << 3;
        let c = p.raycast([0.0, 0.0], [0.0, 1.0], 500.0, so_inimigo, false);
        assert_eq!(c.node, Some(inimigo));
        assert!((c.distance.unwrap() - 180.0).abs() < 0.5, "{c:?}");
        assert!(!p.raycast([0.0, 0.0], [0.0, 1.0], 500.0, 1, false).hit);
        assert!(
            !p.raycast([0.0, 0.0], [0.0, 1.0], 100.0, u32::MAX, false).hit,
            "curto demais"
        );

        assert_eq!(p.overlap_point([0.0, 200.0], so_inimigo, false), vec![inimigo]);
        assert!(p.overlap_point([100.0, 200.0], u32::MAX, false).is_empty());
        assert!(
            p.overlap_point([0.0, 200.0], 1, false).is_empty(),
            "máscara sem a camada do inimigo"
        );

        assert_eq!(
            p.overlap_rect([0.0, 190.0], [10.0, 10.0], u32::MAX, false),
            vec![inimigo]
        );
        assert!(p.overlap_rect([100.0, 0.0], [10.0, 10.0], u32::MAX, false).is_empty());
    }
}
