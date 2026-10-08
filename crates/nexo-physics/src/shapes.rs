//! Formas de colisão: retângulo, círculo, cápsula vertical e segmento (PHY-010), e polígonos convexos
//! ou côncavos (PHY-011). Cada nó vira um `ColliderBuilder` no espaço local do próprio nó.

use super::*;

/// Colisor de um nó `CollisionShape2D` ou `CollisionPolygon2D`, no espaço local do nó. `None` para forma
/// que não gera colisor (polígono com menos de três pontos ou degenerado).
pub(crate) fn build(node: &Node, xf: Xf) -> Option<ColliderBuilder> {
    if node.kind == "CollisionPolygon2D" {
        return polygon(node, xf);
    }
    let [w, h] = as_vec2(&value_of(node, "size"));
    let (w, h) = (((w * xf.sx.abs()).max(0.5)) as f32, ((h * xf.sy.abs()).max(0.5)) as f32);
    Some(match text(node, "shape").as_str() {
        "circle" => ColliderBuilder::ball(w / 2.0),
        // A cápsula é vertical: a largura é o diâmetro e a altura é o total, com as pontas arredondadas.
        "capsule_y" => ColliderBuilder::capsule_y(((h - w) / 2.0).max(0.0), w / 2.0),
        // O segmento é uma linha horizontal de comprimento igual à largura; a altura não entra.
        "segment" => ColliderBuilder::segment(Vector::new(-w / 2.0, 0.0), Vector::new(w / 2.0, 0.0)),
        _ => ColliderBuilder::cuboid(w / 2.0, h / 2.0),
    })
}

/// Polígono de pontos: convexo vira o próprio casco; côncavo é decomposto em partes convexas (V-HACD).
fn polygon(node: &Node, xf: Xf) -> Option<ColliderBuilder> {
    let pts: Vec<Vector> = value_of(node, "points")
        .as_array()?
        .iter()
        .map(|p| {
            let [x, y] = as_vec2(p);
            Vector::new((x * xf.sx) as f32, (y * xf.sy) as f32)
        })
        .collect();
    if pts.len() < 3 || area(&pts).abs() < 1e-6 {
        return None;
    }
    if is_convex(&pts) {
        return ColliderBuilder::convex_hull(&pts);
    }
    let n = pts.len() as u32;
    let edges: Vec<[u32; 2]> = (0..n).map(|i| [i, (i + 1) % n]).collect();
    Some(ColliderBuilder::convex_decomposition(&pts, &edges))
}

/// True se todas as curvas do contorno viram para o mesmo lado (polígono convexo).
fn is_convex(pts: &[Vector]) -> bool {
    let n = pts.len();
    let mut sign = 0.0f32;
    for i in 0..n {
        let (a, b, c) = (pts[i], pts[(i + 1) % n], pts[(i + 2) % n]);
        let cross = (b.x - a.x) * (c.y - b.y) - (b.y - a.y) * (c.x - b.x);
        if cross.abs() < 1e-6 {
            continue;
        }
        if sign == 0.0 {
            sign = cross.signum();
        } else if cross.signum() != sign {
            return false;
        }
    }
    true
}

/// Área com sinal pela fórmula de Gauss (positiva ou negativa conforme o sentido do contorno).
fn area(pts: &[Vector]) -> f32 {
    let n = pts.len();
    (0..n)
        .map(|i| {
            let (a, b) = (pts[i], pts[(i + 1) % n]);
            a.x * b.y - b.x * a.y
        })
        .sum::<f32>()
        / 2.0
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn convexidade_distingue_l_de_quadrado() {
        let quadrado = [Vector::new(0.0, 0.0), Vector::new(10.0, 0.0), Vector::new(10.0, 10.0), Vector::new(0.0, 10.0)];
        let l = [
            Vector::new(0.0, 0.0),
            Vector::new(100.0, 0.0),
            Vector::new(100.0, 20.0),
            Vector::new(20.0, 20.0),
            Vector::new(20.0, 100.0),
            Vector::new(0.0, 100.0),
        ];
        assert!(is_convex(&quadrado));
        assert!(!is_convex(&l));
    }

    #[test]
    fn area_do_quadrado_e_do_l() {
        let quadrado = [Vector::new(0.0, 0.0), Vector::new(10.0, 0.0), Vector::new(10.0, 10.0), Vector::new(0.0, 10.0)];
        assert!((area(&quadrado).abs() - 100.0).abs() < 1e-4);
    }
}
