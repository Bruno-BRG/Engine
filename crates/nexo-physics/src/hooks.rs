//! Regras de contato pelo gancho `modify_solver_contacts` do Rapier: esteiras (PHY-002) e formas de
//! uma face só (PHY-010). Os colisores marcados entram no gancho pela flag `ActiveHooks`.

use rapier2d::prelude::*;
use std::collections::HashMap;

/// Bit de `Collider::user_data` que marca uma forma de uma face só: colide só de cima.
pub(crate) const ONE_WAY: u128 = 1;

/// Ângulo, em radianos, dentro do qual um contato conta como vindo de cima de uma forma de uma face.
const ONE_WAY_ANGLE: f32 = std::f32::consts::FRAC_PI_3;

/// Esteira: velocidade de superfície de um colisor. A linear vale em toda a superfície; a angular gira a
/// superfície em torno da origem do corpo, como um disco que gira (PHY-002).
#[derive(Clone, Copy, Debug, Default, PartialEq)]
pub(crate) struct Belt {
    /// Velocidade linear, em px/s no mundo.
    pub(crate) linear: Vector,
    /// Velocidade angular, em rad/s. Positiva gira no sentido da rotação da cena (horário na tela).
    pub(crate) angular: f32,
    /// Origem do corpo no mundo: o eixo da rotação.
    pub(crate) center: Vector,
}

impl Belt {
    /// Velocidade da superfície no ponto `p` do mundo: a linear mais ω × r, com r a partir da origem.
    pub(crate) fn at(&self, p: Vector) -> Vector {
        let r = p - self.center;
        self.linear + Vector::new(-self.angular * r.y, self.angular * r.x)
    }

    /// True se a esteira move alguma coisa.
    pub(crate) fn is_active(&self) -> bool {
        self.linear != Vector::ZERO || self.angular != 0.0
    }
}

#[derive(Default)]
pub(crate) struct Rules {
    /// Esteira de cada colisor que move a superfície.
    pub(crate) belts: HashMap<ColliderHandle, Belt>,
}

/// True se o colisor é uma forma de uma face só.
pub(crate) fn is_one_way(c: &Collider) -> bool {
    c.user_data & ONE_WAY != 0
}

impl PhysicsHooks for Rules {
    fn modify_solver_contacts(&self, ctx: &mut ContactModificationContext) {
        let (c1, c2) = (ctx.collider1, ctx.collider2);
        let one_way1 = is_one_way(&ctx.colliders[c1]);
        let one_way2 = is_one_way(&ctx.colliders[c2]);
        if one_way1 || one_way2 {
            // A direção permitida vem no espaço local do primeiro colisor, que é o que o Rapier usa.
            let allowed = if one_way1 {
                Vector::new(0.0, -1.0)
            } else {
                // A plataforma é o segundo colisor: o corpo de cima tem normal apontando para baixo.
                let up = ctx.colliders[c2].position().rotation.transform_vector(Vector::new(0.0, -1.0));
                ctx.colliders[c1].position().rotation.inverse_transform_vector(-up)
            };
            ctx.update_as_oneway_platform(allowed, ONE_WAY_ANGLE);
        }

        // O Rapier soma `tangent_velocity` à superfície do primeiro colisor e faz o contato igualar
        // as duas superfícies. Por isso a esteira no primeiro colisor entra com o sinal da velocidade
        // dela, e no segundo com o sinal oposto. Verificado nos testes de esteira (ordem dos colisores).
        // Com velocidade angular, a superfície muda de ponto para ponto: cada contato usa o próprio ponto
        // (as âncoras vêm no mundo neste gancho) e o ponto médio entre os dois lados.
        let (belt1, belt2) = (self.belts.get(&c1), self.belts.get(&c2));
        if belt1.is_none() && belt2.is_none() {
            return;
        }
        if let Some(m) = ctx.rigid_mut() {
            for s in m.solver_contacts.iter_mut() {
                let p = (s.anchor1 + s.anchor2) * 0.5;
                let mut tangent = Vector::ZERO;
                if let Some(b) = belt1 {
                    tangent += b.at(p);
                }
                if let Some(b) = belt2 {
                    tangent -= b.at(p);
                }
                s.tangent_velocity = tangent;
            }
        }
    }
}
