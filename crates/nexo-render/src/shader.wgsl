// Shader 2D do Nexo: grade do editor e sprites procedurais.

struct Camera {
    center: vec2f,
    viewport: vec2f,
    screen: vec2f,
    zoom: f32,
    time: f32,
};

@group(0) @binding(0) var<uniform> cam: Camera;

fn world_to_clip(w: vec2f) -> vec4f {
    let p = (w - cam.center) * cam.zoom / (cam.viewport * 0.5);
    return vec4f(p.x, -p.y, 0.0, 1.0);
}

// ---------- Grade ----------

struct GridOut {
    @builtin(position) pos: vec4f,
    @location(0) world: vec2f,
};

@vertex
fn vs_grid(@builtin(vertex_index) i: u32) -> GridOut {
    let uv = vec2f(f32((i << 1u) & 2u), f32(i & 2u));
    let ndc = uv * 2.0 - 1.0;
    var o: GridOut;
    o.pos = vec4f(ndc, 0.0, 1.0);
    o.world = cam.center + vec2f(ndc.x, -ndc.y) * cam.viewport * 0.5 / cam.zoom;
    return o;
}

fn grid_line(p: vec2f, spacing: f32) -> f32 {
    let d = abs(fract(p / spacing - 0.5) - 0.5) * spacing;
    let w = fwidth(p);
    return 1.0 - min(min(d.x / w.x, d.y / w.y), 1.0);
}

@fragment
fn fs_grid(in: GridOut) -> @location(0) vec4f {
    let p = in.world;
    let minor_fade = clamp(32.0 * cam.zoom / 10.0 - 0.6, 0.0, 1.0);
    let minor = grid_line(p, 32.0) * 0.045 * minor_fade;
    let major = grid_line(p, 256.0) * 0.085;

    var col = vec3f(0.055, 0.059, 0.075);
    col = mix(col, vec3f(1.0), max(minor, major));

    // Eixos X (vermelho suave) e Y (verde suave).
    let fw = fwidth(p);
    let ax = 1.0 - min(abs(p.y) / (fw.y * 1.5), 1.0);
    let ay = 1.0 - min(abs(p.x) / (fw.x * 1.5), 1.0);
    col = mix(col, vec3f(1.0, 0.36, 0.48), ax * 0.35);
    col = mix(col, vec3f(0.24, 0.86, 0.59), ay * 0.35);

    // Moldura da tela do jogo, de (0,0) até screen.
    let half = cam.screen * 0.5;
    let q = abs(p - half) - half;
    let d = abs(length(max(q, vec2f(0.0))) + min(max(q.x, q.y), 0.0));
    let frame = 1.0 - min(d / (max(fw.x, fw.y) * 1.2), 1.0);
    col = mix(col, vec3f(0.486, 0.361, 1.0), frame * 0.7);

    return vec4f(col, 1.0);
}

// ---------- Sprites ----------

struct SpriteIn {
    @location(0) position: vec2f,
    @location(1) size: vec2f,
    @location(2) rotation: f32,
    @location(3) shape: f32,
    @location(4) flags: f32,
    @location(5) color: vec4f,
};

struct SpriteOut {
    @builtin(position) pos: vec4f,
    @location(0) local: vec2f,
    @location(1) half: vec2f,
    @location(2) color: vec4f,
    @location(3) @interpolate(flat) shape: u32,
    @location(4) @interpolate(flat) flags: u32,
};

@vertex
fn vs_sprite(@builtin(vertex_index) vi: u32, s: SpriteIn) -> SpriteOut {
    var corners = array<vec2f, 6>(
        vec2f(-1.0, -1.0), vec2f(1.0, -1.0), vec2f(1.0, 1.0),
        vec2f(-1.0, -1.0), vec2f(1.0, 1.0), vec2f(-1.0, 1.0),
    );
    let half = s.size * 0.5;
    // Margem para contorno de seleção e sombra.
    let pad = 14.0 / cam.zoom + 10.0;
    let local = corners[vi] * (half + vec2f(pad));
    let c = cos(s.rotation);
    let sn = sin(s.rotation);
    let world = s.position + vec2f(local.x * c - local.y * sn, local.x * sn + local.y * c);

    var o: SpriteOut;
    o.pos = world_to_clip(world);
    o.local = local;
    o.half = half;
    o.color = s.color;
    o.shape = u32(s.shape);
    o.flags = u32(s.flags);
    return o;
}

fn sd_shape(p: vec2f, b: vec2f, shape: u32) -> f32 {
    switch shape {
        case 1u: {
            return length(p) - min(b.x, b.y);
        }
        case 2u: {
            let q = abs(p) / b;
            return (q.x + q.y - 1.0) * min(b.x, b.y) * 0.7071;
        }
        default: {
            let r = min(b.x, b.y) * 0.22;
            let q = abs(p) - b + vec2f(r);
            return length(max(q, vec2f(0.0))) + min(max(q.x, q.y), 0.0) - r;
        }
    }
}

@fragment
fn fs_sprite(in: SpriteOut) -> @location(0) vec4f {
    let px = 1.0 / cam.zoom; // 1 pixel de tela em unidades de mundo
    let d = sd_shape(in.local, in.half, in.shape);
    let aa = max(fwidth(d), 0.0001);

    // Sombra suave deslocada para baixo.
    let ds = sd_shape(in.local - vec2f(0.0, 6.0), in.half, in.shape);
    let shadow = (1.0 - smoothstep(-2.0, 10.0, ds)) * 0.35;
    var out = vec4f(0.0, 0.0, 0.0, shadow);

    // Preenchimento com gradiente vertical e brilho na borda superior.
    let t = clamp(in.local.y / max(in.half.y, 1.0) * 0.5 + 0.5, 0.0, 1.0);
    var fill = in.color.rgb * mix(1.12, 0.82, t);
    let rim = 1.0 - smoothstep(0.0, 3.0 * px + 2.0, -d);
    fill = mix(fill, fill * 1.25 + 0.05, rim * (1.0 - t) * 0.6);
    let a_fill = (1.0 - smoothstep(-aa, aa, d)) * in.color.a;
    out = vec4f(fill * a_fill, a_fill) + out * (1.0 - a_fill);

    // Contorno de seleção e hover, em pixels de tela.
    let selected = (in.flags & 1u) != 0u;
    let hovered = (in.flags & 2u) != 0u;
    if (selected || hovered) {
        let gap = 3.0 * px;
        let w = select(1.0, 2.0, selected) * px;
        let ring = 1.0 - smoothstep(w - aa, w + aa, abs(d - gap - w));
        let rc = select(vec3f(1.0), vec3f(0.486, 0.361, 1.0), selected);
        let ra = ring * select(0.45, 1.0, selected);
        out = vec4f(rc * ra, ra) + out * (1.0 - ra);
    }
    return out;
}
