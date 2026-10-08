//! Personagens (CharacterBody2D): movimento com o controlador de forma do Rapier, estado de contato
//! (PHY-004) e carregamento por superfícies com velocidade (plataformas e esteiras).

use super::*;
use rapier2d::control::{CharacterCollision, KinematicCharacterController};
use rapier2d::parry::query::ShapeCastOptions;

/// Distância, em pixels, que a sonda procura o chão logo abaixo do personagem.
const FLOOR_PROBE: f32 = 2.0;
/// Folga, em pixels, para uma forma de uma face contar como "abaixo" do personagem.
const ONE_WAY_SLACK: f32 = 1.0;

/// Estado de um personagem no último passo da física.
#[derive(Clone, Debug, PartialEq, Serialize)]
pub struct CharacterState {
    /// Encostado no chão (com a folga de encaixe do controlador).
    pub on_floor: bool,
    /// Encostou numa parede no último movimento.
    pub on_wall: bool,
    /// Encostou num teto no último movimento.
    pub on_ceiling: bool,
    /// Ângulo do chão em graus, entre a normal do chão e "para cima". 0 sem chão.
    pub floor_angle: f64,
    /// Velocidade própria, em px/s.
    pub velocity: [f64; 2],
    /// Velocidade da superfície onde o personagem está apoiado, em px/s (carregamento).
    pub platform_velocity: [f64; 2],
}

/// Estado de um personagem entre os passos.
pub(crate) struct Character {
    /// Velocidade própria em pixels por segundo.
    pub(crate) velocity: [f64; 2],
    pub(crate) grounded: bool,
    pub(crate) on_wall: bool,
    pub(crate) on_ceiling: bool,
    pub(crate) floor_angle: f64,
    /// Velocidade da superfície de apoio, medida no passo anterior.
    pub(crate) platform: [f64; 2],
    pub(crate) controller: KinematicCharacterController,
    /// Primeiro colisor do personagem: a forma que o controlador move.
    pub(crate) collider: ColliderHandle,
}

impl Character {
    pub(crate) fn new(velocity: [f64; 2], collider: ColliderHandle, up: Vector) -> Character {
        Character {
            velocity,
            grounded: false,
            on_wall: false,
            on_ceiling: false,
            floor_angle: 0.0,
            platform: [0.0, 0.0],
            controller: KinematicCharacterController {
                // "Para cima" é o oposto da gravidade; sem gravidade, o padrão da tela (Y para baixo).
                up,
                ..Default::default()
            },
            collider,
        }
    }
}

/// Tipo de contato de um personagem, pela normal da superfície (mesma regra do controlador).
#[derive(Clone, Copy, PartialEq, Eq)]
enum Surface {
    Floor,
    Wall,
    Ceiling,
}

/// Cosseno abaixo do qual a normal aponta para baixo: teto. A folga cobre normais de parede
/// quase horizontais (cerca de 95°), que o arredondamento do Rapier pode inclinar um pouco.
const CEILING_COS: f32 = -0.1;

fn classify(normal: Vector, up: Vector, max_climb: f32) -> Surface {
    let c = up.dot(normal);
    if c < CEILING_COS {
        Surface::Ceiling
    } else if c.clamp(-1.0, 1.0).acos() < max_climb {
        Surface::Floor
    } else {
        Surface::Wall
    }
}

fn angle_deg(up: Vector, normal: Vector) -> f64 {
    up.dot(normal).clamp(-1.0, 1.0).acos().to_degrees() as f64
}

/// Maior coordenada de um retângulo de eixos alinhados ao longo de `dir`.
fn max_along(aabb: &Aabb, dir: Vector) -> f32 {
    let x = if dir.x >= 0.0 { aabb.maxs.x } else { aabb.mins.x };
    let y = if dir.y >= 0.0 { aabb.maxs.y } else { aabb.mins.y };
    dir.x * x + dir.y * y
}

/// Uma forma de uma face só deixa o personagem passar enquanto a base dele está abaixo do topo.
/// Formas comuns sempre são sólidas. A comparação usa os retângulos de eixos alinhados.
fn solid_for(c: &Collider, character: &Aabb) -> bool {
    if !hooks::is_one_way(c) {
        return true;
    }
    let up = c.position().rotation.transform_vector(Vector::new(0.0, -1.0));
    let top = max_along(&c.compute_aabb(), up);
    // A parte mais baixa do personagem, medida no sentido "para cima" da plataforma.
    let low = -max_along(character, -up);
    low >= top - ONE_WAY_SLACK
}

impl Physics {
    /// Move cada personagem antes do passo (PHY-004). A velocidade usada é a própria mais a da superfície
    /// onde ele estava apoiado no passo anterior, e o estado de contato vem dos choques do movimento.
    pub(crate) fn move_characters(&mut self) {
        let dt = STEP as f32;
        let up = self.up;
        let mut moves = Vec::new();
        for (i, d) in self.driven.iter().enumerate() {
            let Some(ch) = &d.character else { continue };
            let collider = &self.world.colliders[ch.collider];
            let pos = *collider.position();
            let body_aabb = collider.compute_aabb();
            let solid = |_: ColliderHandle, c: &Collider| solid_for(c, &body_aabb);
            let filter = QueryFilter {
                exclude_rigid_body: Some(d.handle),
                flags: QueryFilterFlags::EXCLUDE_SENSORS,
                groups: Some(collider.collision_groups()),
                predicate: Some(&solid),
                ..Default::default()
            };
            let queries = self.world.query_pipeline_with_filter(filter);
            let desired = Vector::new(
                ((ch.velocity[0] + ch.platform[0]) * STEP) as f32,
                ((ch.velocity[1] + ch.platform[1]) * STEP) as f32,
            );
            let mut hits: Vec<CharacterCollision> = Vec::new();
            let mv = ch.controller.move_shape(dt, &queries, collider.shape(), &pos, desired, |h| hits.push(h));

            let (mut on_wall, mut on_ceiling) = (false, false);
            for h in &hits {
                match classify(h.hit.normal1, up, ch.controller.max_slope_climb_angle) {
                    Surface::Wall => on_wall = true,
                    Surface::Ceiling => on_ceiling = true,
                    Surface::Floor => {}
                }
            }
            // Com o chão encontrado, a sonda logo abaixo mede o ângulo e a superfície de apoio.
            let (mut floor_angle, mut platform) = (0.0, [0.0, 0.0]);
            if mv.grounded {
                let mut at = pos;
                at.translation += mv.translation;
                if let Some((c, hit)) = queries.cast_shape(
                    &at,
                    -up,
                    collider.shape(),
                    ShapeCastOptions::with_max_time_of_impact(FLOOR_PROBE),
                ) {
                    floor_angle = angle_deg(up, hit.normal1);
                    platform = self.surface_velocity(c);
                }
            }
            moves.push((i, mv.translation, desired, mv.grounded, on_wall, on_ceiling, floor_angle, platform));
        }

        for (i, moved, desired, grounded, on_wall, on_ceiling, floor_angle, platform) in moves {
            let d = &mut self.driven[i];
            let body = &mut self.world.bodies[d.handle];
            let target = body.translation() + moved;
            body.set_next_kinematic_translation(target);
            let Some(ch) = d.character.as_mut() else { continue };
            ch.grounded = grounded;
            ch.on_wall = on_wall;
            ch.on_ceiling = on_ceiling;
            ch.floor_angle = floor_angle;
            ch.platform = platform;
            // Como o move_and_slide: o eixo bloqueado perde a velocidade própria.
            if (moved.x - desired.x).abs() > 1e-3 {
                ch.velocity[0] = 0.0;
            }
            if (moved.y - desired.y).abs() > 1e-3 {
                ch.velocity[1] = 0.0;
            }
            // No chão, a velocidade que aponta para baixo some.
            let along = ch.velocity[0] * up.x as f64 + ch.velocity[1] * up.y as f64;
            if grounded && along < 0.0 {
                ch.velocity[0] -= along * up.x as f64;
                ch.velocity[1] -= along * up.y as f64;
            }
        }
    }

    fn character(&self, uid: Uid) -> Option<&Character> {
        self.index.get(&uid).and_then(|i| self.driven[*i].character.as_ref())
    }

    /// Velocidade própria de um personagem, em pixels por segundo.
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

    /// Estado de contato de um personagem (PHY-004), ou `None` se o nó não é um personagem simulado.
    pub fn character_state(&self, uid: Uid) -> Option<CharacterState> {
        self.character(uid).map(|c| CharacterState {
            on_floor: c.grounded,
            on_wall: c.on_wall,
            on_ceiling: c.on_ceiling,
            floor_angle: c.floor_angle,
            velocity: c.velocity,
            platform_velocity: c.platform,
        })
    }
}
