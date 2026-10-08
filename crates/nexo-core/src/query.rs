//! Consultas sobre a cena para o editor e o viewport: transformação mundial, pose de reprodução,
//! lista de desenho, seleção por clique, limites e o contorno usado pela hierarquia.

use crate::registry;
use crate::scene::{Node, Scene, Uid, as_vec2, value_of};
use crate::{SPRITE_FLOATS, SpriteInstance, flags};
use serde::Serialize;
use serde_json::{Map, Value};
use std::collections::HashMap;

/// Transformação de um nó no mundo (ou relativa ao pai).
#[derive(Clone, Copy, Debug, PartialEq, Serialize)]
pub struct Xf {
    pub x: f64,
    pub y: f64,
    /// Graus, sentido horário na tela (Y para baixo).
    pub rot: f64,
    pub sx: f64,
    pub sy: f64,
}

impl Xf {
    pub const IDENTITY: Xf = Xf { x: 0.0, y: 0.0, rot: 0.0, sx: 1.0, sy: 1.0 };

    /// Aplica `local` (expresso no espaço de `self`, o pai) e devolve a transformação no mundo.
    pub fn then(self, local: Xf) -> Xf {
        let (s, c) = self.rot.to_radians().sin_cos();
        let lx = local.x * self.sx;
        let ly = local.y * self.sy;
        Xf {
            x: self.x + c * lx - s * ly,
            y: self.y + s * lx + c * ly,
            rot: self.rot + local.rot,
            sx: self.sx * local.sx,
            sy: self.sy * local.sy,
        }
    }
}

fn num(node: &Node, key: &str) -> f64 {
    value_of(node, key).as_f64().unwrap_or(0.0)
}

fn text(node: &Node, key: &str) -> String {
    value_of(node, key).as_str().unwrap_or("").to_string()
}

fn parse_uid(s: &str) -> Option<Uid> {
    if s.is_empty() { None } else { s.parse().ok() }
}

/// Cor "#rrggbb" ou "#rrggbbaa" em componentes 0 a 1.
pub fn rgba(hex: &str) -> [f32; 4] {
    let h = hex.trim_start_matches('#');
    let byte = |i: usize| {
        h.get(i..i + 2)
            .and_then(|s| u8::from_str_radix(s, 16).ok())
            .unwrap_or(255) as f32
            / 255.0
    };
    let a = if h.len() >= 8 { byte(6) } else { 1.0 };
    [byte(0), byte(2), byte(4), a]
}

/// Pose local de um nó. Na reprodução (`time` presente), os comportamentos filhos modificam a pose.
/// A pose é calculada a partir do valor base em cada quadro, então não acumula erro.
fn local_xf(scene: &Scene, node: &Node, time: Option<f64>) -> Xf {
    // Só nós 2D têm transformação; os demais (como Behavior) ficam na identidade.
    if !registry::is_2d(&node.kind) {
        return Xf::IDENTITY;
    }
    let [x, y] = as_vec2(&value_of(node, "position"));
    let [sx, sy] = as_vec2(&value_of(node, "scale"));
    let mut xf = Xf { x, y, rot: num(node, "rotation"), sx, sy };
    let Some(t) = time else {
        return xf;
    };
    for c in &node.children {
        let Some(b) = scene.node(*c) else { continue };
        if b.kind != "Behavior" {
            continue;
        }
        let speed = num(b, "speed");
        let phase = t * speed * std::f64::consts::TAU;
        match text(b, "kind").as_str() {
            "spin" => xf.rot += speed * t,
            "float" => xf.y += phase.sin() * 14.0,
            "pulse" => {
                let s = 1.0 + phase.sin() * 0.08;
                xf.sx *= s;
                xf.sy *= s;
            }
            _ => {}
        }
    }
    xf
}

/// Transformação mundial e visibilidade efetiva de todos os nós, em ordem de árvore.
pub fn world_map(scene: &Scene, time: Option<f64>) -> Vec<(Uid, Xf, bool)> {
    let mut out = Vec::new();
    let mut stack: Vec<(Uid, Xf, bool)> =
        scene.roots().iter().rev().map(|u| (*u, Xf::IDENTITY, true)).collect();
    while let Some((uid, parent_xf, parent_vis)) = stack.pop() {
        let Some(n) = scene.node(uid) else { continue };
        let xf = parent_xf.then(local_xf(scene, n, time));
        let vis = parent_vis && value_of(n, "visible").as_bool().unwrap_or(true);
        out.push((uid, xf, vis));
        stack.extend(n.children.iter().rev().map(|c| (*c, xf, vis)));
    }
    out
}

/// Um sprite pronto para desenhar.
#[derive(Clone, Debug)]
pub struct Draw {
    pub uid: Uid,
    pub instance: SpriteInstance,
    /// Textura do asset, se houver. Sem textura, o sprite é a forma de teste.
    pub texture: Option<Uid>,
    pub z: i64,
}

/// Sprites visíveis em ordem de desenho: pela ordem Z e, em empate, pela ordem da árvore.
pub fn draws(scene: &Scene, time: Option<f64>, selected: &[Uid], hovered: Option<Uid>) -> Vec<Draw> {
    let mut out = Vec::new();
    for (uid, xf, vis) in world_map(scene, time) {
        if !vis {
            continue;
        }
        let Some(n) = scene.node(uid) else { continue };
        if n.kind != "Sprite2D" {
            continue;
        }
        let size = as_vec2(&value_of(n, "size"));
        let centered = value_of(n, "centered").as_bool().unwrap_or(true);
        let [ox, oy] = as_vec2(&value_of(n, "offset"));
        let w = (size[0] * xf.sx).abs();
        let h = (size[1] * xf.sy).abs();
        let (lx, ly) = if centered {
            (ox * xf.sx, oy * xf.sy)
        } else {
            (ox * xf.sx + w / 2.0, oy * xf.sy + h / 2.0)
        };
        let (s, c) = xf.rot.to_radians().sin_cos();
        let cx = xf.x + c * lx - s * ly;
        let cy = xf.y + s * lx + c * ly;

        let mut fl: f32 = 0.0;
        if selected.contains(&uid) {
            fl += flags::SELECTED;
        }
        if hovered == Some(uid) {
            fl += flags::HOVERED;
        }
        if value_of(n, "flip_h").as_bool().unwrap_or(false) {
            fl += flags::FLIP_X;
        }
        if value_of(n, "flip_v").as_bool().unwrap_or(false) {
            fl += flags::FLIP_Y;
        }
        let shape = match text(n, "shape").as_str() {
            "circle" => 1.0,
            "diamond" => 2.0,
            _ => 0.0,
        };
        out.push(Draw {
            uid,
            instance: SpriteInstance {
                position: [cx as f32, cy as f32],
                size: [w as f32, h as f32],
                rotation: xf.rot.to_radians() as f32,
                shape,
                flags: fl,
                _pad: 0.0,
                color: rgba(&text(n, "modulate")),
            },
            texture: parse_uid(&text(n, "texture")),
            z: value_of(n, "z_index").as_i64().unwrap_or(0),
        });
    }
    out.sort_by_key(|d| d.z);
    out
}

/// Achata a lista de desenho no formato que o viewport recebe: `SPRITE_FLOATS` floats por sprite.
pub fn floats(draws: &[Draw]) -> Vec<f32> {
    let mut buf = Vec::with_capacity(draws.len() * SPRITE_FLOATS);
    for d in draws {
        let i = &d.instance;
        buf.extend_from_slice(&[i.position[0], i.position[1], i.size[0], i.size[1], i.rotation, i.shape, i.flags, i._pad]);
        buf.extend_from_slice(&i.color);
    }
    buf
}

/// Node under the world point: a sprite first (the top one wins), then a collision shape without a
/// sprite, found by its outline (PHY-012). `None` if nothing is under the point.
pub fn pick(scene: &Scene, x: f64, y: f64, time: Option<f64>) -> Option<Uid> {
    let sprite = draws(scene, time, &[], None)
        .iter()
        .rev()
        .find(|d| inside(&d.instance, d.texture.is_some(), x, y))
        .map(|d| d.uid);
    sprite.or_else(|| pick_shape(scene, x, y, time))
}

/// Visible collision shape under the point, tested against its outline. The last one in tree order
/// (drawn on top) wins.
fn pick_shape(scene: &Scene, x: f64, y: f64, time: Option<f64>) -> Option<Uid> {
    world_map(scene, time)
        .into_iter()
        .rev()
        .filter(|(_, _, visible)| *visible)
        .filter_map(|(uid, xf, _)| shape_outline(scene, uid, xf))
        .find(|o| inside_outline(&o.points, x, y))
        .map(|o| o.uid)
}

/// Distance, in world units, within which a click picks a segment shape (it has no area).
const SEGMENT_PICK: f64 = 6.0;

/// Polygons (concave too) use the even-odd rule. A segment (two points) is picked within `SEGMENT_PICK`.
fn inside_outline(pts: &[[f64; 2]], x: f64, y: f64) -> bool {
    match pts {
        [] | [_] => false,
        [a, b] => dist_to_segment([x, y], *a, *b) <= SEGMENT_PICK,
        _ => {
            let mut inside = false;
            for (&[xi, yi], &[xj, yj]) in pts.iter().zip(pts.iter().cycle().skip(pts.len() - 1)) {
                // Edges crossing the horizontal line through `y`; here `yi != yj`, so the division is safe.
                if (yi > y) != (yj > y) && x < (xj - xi) * (y - yi) / (yj - yi) + xi {
                    inside = !inside;
                }
            }
            inside
        }
    }
}

/// Distance from `p` to the segment `a`-`b`.
fn dist_to_segment(p: [f64; 2], a: [f64; 2], b: [f64; 2]) -> f64 {
    let (dx, dy) = (b[0] - a[0], b[1] - a[1]);
    let len2 = dx * dx + dy * dy;
    let t = if len2 > 0.0 {
        (((p[0] - a[0]) * dx + (p[1] - a[1]) * dy) / len2).clamp(0.0, 1.0)
    } else {
        0.0
    };
    let (cx, cy) = (a[0] + t * dx, a[1] + t * dy);
    (p[0] - cx).hypot(p[1] - cy)
}

fn inside(inst: &SpriteInstance, textured: bool, x: f64, y: f64) -> bool {
    let (cx, cy) = (inst.position[0] as f64, inst.position[1] as f64);
    let hw = (inst.size[0] as f64 / 2.0).max(1e-6);
    let hh = (inst.size[1] as f64 / 2.0).max(1e-6);
    let (s, c) = (inst.rotation as f64).sin_cos();
    let (dx, dy) = (x - cx, y - cy);
    let lx = dx * c + dy * s;
    let ly = -dx * s + dy * c;
    let shape = if textured { 0 } else { inst.shape as u32 };
    match shape {
        1 => (lx * lx + ly * ly).sqrt() <= hw.min(hh),
        2 => lx.abs() / hw + ly.abs() / hh <= 1.0,
        _ => lx.abs() <= hw && ly.abs() <= hh,
    }
}

/// Retângulo alinhado aos eixos que envolve o nó: [minx, miny, maxx, maxy].
/// Nós sem sprite viram o próprio ponto de origem.
pub fn bounds(scene: &Scene, uid: Uid, time: Option<f64>) -> Option<[f64; 4]> {
    if let Some(d) = draws(scene, time, &[], None).into_iter().find(|d| d.uid == uid) {
        let i = d.instance;
        let (hw, hh) = (i.size[0] as f64 / 2.0, i.size[1] as f64 / 2.0);
        let (s, c) = (i.rotation as f64).sin_cos();
        let ex = (hw * c).abs() + (hh * s).abs();
        let ey = (hw * s).abs() + (hh * c).abs();
        let (x, y) = (i.position[0] as f64, i.position[1] as f64);
        return Some([x - ex, y - ey, x + ex, y + ey]);
    }
    let (_, xf, _) = world_map(scene, time).into_iter().find(|(u, _, _)| *u == uid)?;
    Some([xf.x, xf.y, xf.x, xf.y])
}

/// Contorno de uma forma de colisão no mundo, para o viewport (VPT-005): pontos em volta da forma.
#[derive(Clone, Debug, Serialize)]
pub struct ShapeOutline {
    /// O nó `CollisionShape2D`.
    pub uid: Uid,
    /// O corpo ou a área que contém a forma.
    pub owner: Uid,
    /// Retângulo: os quatro cantos. Círculo: pontos ao redor da circunferência.
    pub points: Vec<[f64; 2]>,
}

/// Ponto `(px, py)` expresso no espaço local de `xf`, em coordenadas do mundo.
fn apply(xf: Xf, px: f64, py: f64) -> [f64; 2] {
    let (s, c) = xf.rot.to_radians().sin_cos();
    let (lx, ly) = (px * xf.sx, py * xf.sy);
    [xf.x + c * lx - s * ly, xf.y + s * lx + c * ly]
}

/// Pontos do contorno de uma forma no espaço local do nó, antes da transformação do mundo.
fn local_outline(n: &Node) -> Vec<[f64; 2]> {
    if n.kind == "CollisionPolygon2D" {
        return value_of(n, "points")
            .as_array()
            .map(|a| a.iter().map(as_vec2).collect())
            .unwrap_or_default();
    }
    let [w, h] = as_vec2(&value_of(n, "size"));
    let (hw, hh) = (w / 2.0, h / 2.0);
    match text(n, "shape").as_str() {
        "circle" => (0..24)
            .map(|i| {
                let a = i as f64 / 24.0 * std::f64::consts::TAU;
                [hw * a.cos(), hw * a.sin()]
            })
            .collect(),
        // Cápsula vertical: duas semicircunferências de raio w/2, ligadas pelos lados retos.
        "capsule_y" => {
            use std::f64::consts::PI;
            let (r, half) = (hw, ((h - w) / 2.0).max(0.0));
            let mut pts = Vec::new();
            for i in 0..=12 {
                let a = i as f64 / 12.0 * PI;
                pts.push([r * a.cos(), half + r * a.sin()]);
            }
            for i in 0..=12 {
                let a = PI + i as f64 / 12.0 * PI;
                pts.push([r * a.cos(), -half + r * a.sin()]);
            }
            pts
        }
        "segment" => vec![[-hw, 0.0], [hw, 0.0]],
        _ => vec![[-hw, -hh], [hw, -hh], [hw, hh], [-hw, hh]],
    }
}

/// Contornos das formas de colisão ativas (`CollisionShape2D` e `CollisionPolygon2D` com `disabled`
/// falso) filhas de um corpo ou área. Formas fora de um corpo não aparecem: o Rapier também as ignora.
pub fn shape_outlines(scene: &Scene, time: Option<f64>) -> Vec<ShapeOutline> {
    world_map(scene, time).into_iter().filter_map(|(uid, xf, _)| shape_outline(scene, uid, xf)).collect()
}

/// Outline of one node in world space, if it is an active collision shape inside a body or area.
fn shape_outline(scene: &Scene, uid: Uid, xf: Xf) -> Option<ShapeOutline> {
    const BODIES: &[&str] = &["StaticBody2D", "RigidBody2D", "CharacterBody2D", "Area2D"];
    let n = scene.node(uid)?;
    let is_shape = n.kind == "CollisionShape2D" || n.kind == "CollisionPolygon2D";
    if !is_shape || value_of(n, "disabled").as_bool().unwrap_or(false) {
        return None;
    }
    let owner = n.parent.filter(|p| scene.node(*p).is_some_and(|b| BODIES.contains(&b.kind.as_str())))?;
    let points = local_outline(n).into_iter().map(|[px, py]| apply(xf, px, py)).collect();
    Some(ShapeOutline { uid, owner, points })
}

/// Câmera atual: posição mundial e zoom. `None` se a cena não tem uma `Camera2D` atual.
pub fn current_camera(scene: &Scene, time: Option<f64>) -> Option<(f64, f64, f64)> {
    for (uid, xf, _) in world_map(scene, time) {
        let Some(n) = scene.node(uid) else { continue };
        if n.kind == "Camera2D" && value_of(n, "current").as_bool().unwrap_or(true) {
            return Some((xf.x, xf.y, num(n, "zoom")));
        }
    }
    None
}

/// Todas as propriedades de um nó, com os padrões preenchidos (para o Inspector).
fn full_props(n: &Node) -> Map<String, Value> {
    let mut m = Map::new();
    for t in registry::chain(&n.kind) {
        for p in &t.props {
            m.insert(p.name.to_string(), value_of(n, p.name));
        }
    }
    m
}

#[derive(Serialize)]
#[serde(rename_all = "camelCase")]
pub struct OutlineNode {
    pub uid: Uid,
    pub name: String,
    pub kind: String,
    pub parent: Option<Uid>,
    pub children: Vec<Uid>,
    pub depth: usize,
    pub visible: bool,
    /// Transformação no mundo (na reprodução, já com os comportamentos aplicados).
    pub world: Xf,
    pub props: Map<String, Value>,
}

/// Estado da cena para a interface: árvore em ordem, histórico e modo de reprodução.
#[derive(Serialize)]
#[serde(rename_all = "camelCase")]
pub struct Outline {
    pub name: String,
    pub screen: [u32; 2],
    pub playing: bool,
    pub can_undo: bool,
    pub can_redo: bool,
    pub undo_label: Option<String>,
    pub redo_label: Option<String>,
    pub nodes: Vec<OutlineNode>,
}

pub fn outline(scene: &Scene, time: Option<f64>) -> Outline {
    let world: HashMap<Uid, (Xf, bool)> = world_map(scene, time).into_iter().map(|(u, x, v)| (u, (x, v))).collect();
    let mut depths: HashMap<Uid, usize> = HashMap::new();
    let mut nodes = Vec::with_capacity(scene.len());
    for uid in scene.pre_order() {
        let Some(n) = scene.node(uid) else { continue };
        let depth = n.parent.and_then(|p| depths.get(&p).copied()).map_or(0, |d| d + 1);
        depths.insert(uid, depth);
        let (world_xf, visible) = world.get(&uid).copied().unwrap_or((Xf::IDENTITY, true));
        nodes.push(OutlineNode {
            uid,
            name: n.name.clone(),
            kind: n.kind.clone(),
            parent: n.parent,
            children: n.children.clone(),
            depth,
            visible,
            world: world_xf,
            props: full_props(n),
        });
    }
    Outline {
        name: scene.name.clone(),
        screen: scene.screen,
        playing: time.is_some(),
        can_undo: scene.can_undo(),
        can_redo: scene.can_redo(),
        undo_label: scene.undo_label().map(str::to_string),
        redo_label: scene.redo_label().map(str::to_string),
        nodes,
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use serde_json::json;

    fn cena_com_sprite() -> (Scene, Uid, Uid) {
        let mut s = Scene::new_default("Principal", 7);
        let root = s.roots()[0];
        let a = s.add_node(Some(root), "Sprite2D", None).unwrap();
        s.set_prop(a, "position", json!([100.0, 50.0]), None).unwrap();
        s.set_prop(a, "size", json!([40.0, 20.0]), None).unwrap();
        (s, root, a)
    }

    #[test]
    fn transformacao_do_pai_se_propaga() {
        let (mut s, root, a) = cena_com_sprite();
        s.set_prop(root, "position", json!([1000.0, 0.0]), None).unwrap();
        let xf = world_map(&s, None).into_iter().find(|(u, _, _)| *u == a).unwrap().1;
        assert!((xf.x - 1100.0).abs() < 1e-9);
        assert!((xf.y - 50.0).abs() < 1e-9);
    }

    #[test]
    fn rotacao_do_pai_gira_os_filhos() {
        let (mut s, root, a) = cena_com_sprite();
        s.set_prop(root, "rotation", json!(90.0), None).unwrap();
        let xf = world_map(&s, None).into_iter().find(|(u, _, _)| *u == a).unwrap().1;
        // (100, 50) girado 90° no sentido horário da tela vira (-50, 100).
        assert!((xf.x + 50.0).abs() < 1e-9, "x = {}", xf.x);
        assert!((xf.y - 100.0).abs() < 1e-9, "y = {}", xf.y);
    }

    #[test]
    fn sprite_aparece_na_lista_de_desenho() {
        let (s, _, a) = cena_com_sprite();
        let d = draws(&s, None, &[a], None);
        assert_eq!(d.len(), 1);
        assert_eq!(d[0].instance.flags, flags::SELECTED);
        assert_eq!(floats(&d).len(), SPRITE_FLOATS);
    }

    #[test]
    fn pai_invisivel_esconde_os_filhos() {
        let (mut s, root, _) = cena_com_sprite();
        s.set_prop(root, "visible", json!(false), None).unwrap();
        assert!(draws(&s, None, &[], None).is_empty());
    }

    #[test]
    fn clique_acerta_o_sprite_certo() {
        let (s, _, a) = cena_com_sprite();
        assert_eq!(pick(&s, 100.0, 50.0, None), Some(a));
        assert_eq!(pick(&s, 500.0, 500.0, None), None);
    }

    #[test]
    fn comportamento_so_atua_na_reproducao() {
        let (mut s, _, a) = cena_com_sprite();
        let b = s.add_node(Some(a), "Behavior", None).unwrap();
        s.set_prop(b, "kind", json!("spin"), None).unwrap();
        s.set_prop(b, "speed", json!(90.0), None).unwrap();
        let edicao = world_map(&s, None).into_iter().find(|(u, _, _)| *u == a).unwrap().1;
        assert_eq!(edicao.rot, 0.0);
        let play = world_map(&s, Some(1.0)).into_iter().find(|(u, _, _)| *u == a).unwrap().1;
        assert!((play.rot - 90.0).abs() < 1e-9);
    }

    #[test]
    fn camera_atual_e_encontrada() {
        let s = Scene::new_default("Principal", 1);
        let (x, y, zoom) = current_camera(&s, None).unwrap();
        assert_eq!((x, y, zoom), (640.0, 360.0, 1.0));
    }

    #[test]
    fn limites_do_sprite_girado() {
        let (mut s, _, a) = cena_com_sprite();
        s.set_prop(a, "rotation", json!(90.0), None).unwrap();
        let [minx, miny, maxx, maxy] = bounds(&s, a, None).unwrap();
        // Os limites saem de f32 (bits da GPU), por isso a tolerância é maior.
        assert!((maxx - minx - 20.0).abs() < 1e-4);
        assert!((maxy - miny - 40.0).abs() < 1e-4);
    }

    #[test]
    fn contorno_traz_profundidade_e_props_completas() {
        let (s, _, a) = cena_com_sprite();
        let o = outline(&s, None);
        let no = o.nodes.iter().find(|n| n.uid == a).unwrap();
        assert_eq!(no.depth, 1);
        assert!(no.props.contains_key("flip_h"));
        assert!(no.props.contains_key("locked"));
        assert_eq!(no.props["size"], json!([40.0, 20.0]));
    }

    #[test]
    fn nos_nao_2d_ficam_na_identidade() {
        // O Behavior não acrescenta transformação: seu mundo é o do pai.
        let (mut s, _, a) = cena_com_sprite();
        let b = s.add_node(Some(a), "Behavior", None).unwrap();
        let mapa = world_map(&s, None);
        let do_pai = mapa.iter().find(|(u, _, _)| *u == a).unwrap().1;
        let do_behavior = mapa.iter().find(|(u, _, _)| *u == b).unwrap().1;
        assert_eq!(do_behavior, do_pai);
    }

    #[test]
    fn contorno_de_colisao_segue_o_corpo_e_a_forma() {
        let mut s = Scene::new("t", 3);
        let root = s.add_node(None, "Node2D", None).unwrap();
        let corpo = s.add_node(Some(root), "StaticBody2D", None).unwrap();
        s.set_prop(corpo, "position", json!([100.0, 50.0]), None).unwrap();
        let forma = s.add_node(Some(corpo), "CollisionShape2D", None).unwrap();
        s.set_prop(forma, "size", json!([32.0, 16.0]), None).unwrap();
        let fora = s.add_node(Some(root), "CollisionShape2D", None).unwrap();
        s.set_prop(fora, "size", json!([8.0, 8.0]), None).unwrap();

        let outlines = shape_outlines(&s, None);
        assert_eq!(outlines.len(), 1, "forma solta não tem contorno");
        assert_eq!(outlines[0].uid, forma);
        assert_eq!(outlines[0].owner, corpo);
        let p = &outlines[0].points;
        assert_eq!(p.len(), 4);
        assert!((p[0][0] - 84.0).abs() < 1e-9 && (p[0][1] - 42.0).abs() < 1e-9, "{:?}", p[0]);
        assert!((p[2][0] - 116.0).abs() < 1e-9 && (p[2][1] - 58.0).abs() < 1e-9, "{:?}", p[2]);
    }

    #[test]
    fn circulo_de_colisao_tem_pontos_no_raio() {
        let mut s = Scene::new("t", 4);
        let corpo = s.add_node(None, "RigidBody2D", None).unwrap();
        let forma = s.add_node(Some(corpo), "CollisionShape2D", None).unwrap();
        s.set_prop(forma, "shape", json!("circle"), None).unwrap();
        s.set_prop(forma, "size", json!([20.0, 20.0]), None).unwrap();
        let p = &shape_outlines(&s, None)[0].points;
        for q in p {
            assert!((q[0].hypot(q[1]) - 10.0).abs() < 1e-9);
        }
    }

    #[test]
    fn contorno_de_poligono_e_de_segmento() {
        let mut s = Scene::new("t", 5);
        let corpo = s.add_node(None, "StaticBody2D", None).unwrap();
        let poligono = s.add_node(Some(corpo), "CollisionPolygon2D", None).unwrap();
        s.set_prop(poligono, "points", json!([[0.0, 0.0], [10.0, 0.0], [5.0, 8.0]]), None).unwrap();
        let segmento = s.add_node(Some(corpo), "CollisionShape2D", None).unwrap();
        s.set_prop(segmento, "shape", json!("segment"), None).unwrap();
        s.set_prop(segmento, "size", json!([40.0, 0.0]), None).unwrap();

        let outlines = shape_outlines(&s, None);
        let p = outlines.iter().find(|o| o.uid == poligono).unwrap();
        assert_eq!(p.points, vec![[0.0, 0.0], [10.0, 0.0], [5.0, 8.0]]);
        let g = outlines.iter().find(|o| o.uid == segmento).unwrap();
        assert_eq!(g.points, vec![[-20.0, 0.0], [20.0, 0.0]]);
    }

    /// A body at `(100, 50)` with one shape child, for the picking tests.
    fn body_with_shape(shape: &str, size: Value) -> (Scene, Uid, Uid) {
        let mut s = Scene::new("t", 6);
        let corpo = s.add_node(None, "StaticBody2D", None).unwrap();
        s.set_prop(corpo, "position", json!([100.0, 50.0]), None).unwrap();
        let forma = s.add_node(Some(corpo), "CollisionShape2D", None).unwrap();
        s.set_prop(forma, "shape", json!(shape), None).unwrap();
        s.set_prop(forma, "size", size, None).unwrap();
        (s, corpo, forma)
    }

    #[test]
    fn pick_finds_rectangle_shape_without_sprite() {
        // Rectangle 32 x 16 centred on (100, 50): x in [84, 116], y in [42, 58].
        let (s, _, forma) = body_with_shape("rect", json!([32.0, 16.0]));
        assert_eq!(pick(&s, 100.0, 50.0, None), Some(forma));
        assert_eq!(pick(&s, 115.0, 57.0, None), Some(forma));
        assert_eq!(pick(&s, 117.0, 50.0, None), None);
    }

    #[test]
    fn pick_finds_circle_shape_without_sprite() {
        // Circle of radius 10 on (100, 50): the outline is a 24-gon, so 9 is inside and 11 is outside.
        let (s, _, forma) = body_with_shape("circle", json!([20.0, 20.0]));
        assert_eq!(pick(&s, 109.0, 50.0, None), Some(forma));
        assert_eq!(pick(&s, 111.0, 50.0, None), None);
    }

    #[test]
    fn pick_finds_concave_polygon_by_its_area() {
        // Concave "V": the notch at the top is outside the shape.
        let mut s = Scene::new("t", 7);
        let corpo = s.add_node(None, "StaticBody2D", None).unwrap();
        let poligono = s.add_node(Some(corpo), "CollisionPolygon2D", None).unwrap();
        let pontos = json!([[0.0, 0.0], [20.0, 0.0], [20.0, 20.0], [10.0, 5.0], [0.0, 20.0]]);
        s.set_prop(poligono, "points", pontos, None).unwrap();
        assert_eq!(pick(&s, 2.0, 15.0, None), Some(poligono));
        assert_eq!(pick(&s, 10.0, 15.0, None), None, "notch");
    }

    #[test]
    fn pick_finds_segment_within_tolerance() {
        // Segment from (-20, 0) to (20, 0), around the body origin.
        let (s, _, forma) = body_with_shape("segment", json!([40.0, 0.0]));
        assert_eq!(pick(&s, 100.0, 53.0, None), Some(forma));
        assert_eq!(pick(&s, 100.0, 60.0, None), None);
        assert_eq!(pick(&s, 130.0, 50.0, None), None, "past the end");
    }

    #[test]
    fn pick_ignores_shapes_of_hidden_bodies() {
        let (mut s, corpo, _) = body_with_shape("circle", json!([20.0, 20.0]));
        s.set_prop(corpo, "visible", json!(false), None).unwrap();
        assert_eq!(pick(&s, 100.0, 50.0, None), None);
    }

    #[test]
    fn sprite_wins_over_shape_at_same_point() {
        let (mut s, corpo, _) = body_with_shape("rect", json!([32.0, 16.0]));
        let sprite = s.add_node(Some(corpo), "Sprite2D", None).unwrap();
        s.set_prop(sprite, "size", json!([32.0, 16.0]), None).unwrap();
        assert_eq!(pick(&s, 100.0, 50.0, None), Some(sprite));
    }

    #[test]
    fn cor_hex_para_rgba() {
        assert_eq!(rgba("#ffffff"), [1.0, 1.0, 1.0, 1.0]);
        let c = rgba("#ff000080");
        assert!((c[0] - 1.0).abs() < 1e-6 && c[1] == 0.0 && (c[3] - 128.0 / 255.0).abs() < 1e-6);
    }
}
