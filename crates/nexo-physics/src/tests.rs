//! Testes headless da física: cenas pequenas, passos fixos, sem janela.

use super::*;
use nexo_core::settings::PhysicsMaterial;
use serde_json::json;

fn cfg() -> ProjectSettings {
    ProjectSettings::default()
}

fn node(s: &mut Scene, parent: Option<Uid>, kind: &str, props: Value) -> Uid {
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

/// Eventos de física durante `seconds`.
fn eventos(p: &mut Physics, s: &mut Scene, seconds: f64) -> Vec<PhysicsEvent> {
    let frames = (seconds / STEP).round() as usize;
    let mut out = Vec::new();
    for _ in 0..frames {
        p.advance(s, STEP);
        out.extend(p.take_events());
    }
    out
}

fn raiz(s: &mut Scene) -> Uid {
    node(s, None, "Node2D", json!({}))
}

/// Chão estático de `largura` px com o topo em `topo` (y), centrado em `x`.
fn piso(s: &mut Scene, root: Uid, x: f64, topo: f64, largura: f64) -> Uid {
    let c = node(s, Some(root), "StaticBody2D", json!({"position": [x, topo + 16.0]}));
    node(s, Some(c), "CollisionShape2D", json!({"size": [largura, 32.0]}));
    c
}

/// Caixa dinâmica de 32 px em (x, y), com propriedades extras.
fn caixa_em(s: &mut Scene, parent: Uid, x: f64, y: f64, extra: Value) -> Uid {
    let mut props = json!({"position": [x, y]});
    props.as_object_mut().unwrap().extend(extra.as_object().unwrap().clone());
    let b = node(s, Some(parent), "RigidBody2D", props);
    node(s, Some(b), "CollisionShape2D", json!({"size": [32.0, 32.0]}));
    b
}

fn handle_de(p: &Physics, uid: Uid) -> RigidBodyHandle {
    p.driven[p.index[&uid]].handle
}

#[test]
fn caixa_cai_e_para_no_chao() {
    let mut s = Scene::new("teste", 1);
    let root = node(&mut s, None, "Node2D", json!({}));
    let chao = node(&mut s, Some(root), "StaticBody2D", json!({"position": [0.0, 300.0]}));
    node(&mut s, Some(chao), "CollisionShape2D", json!({"size": [400.0, 32.0]}));
    let caixa = node(&mut s, Some(root), "RigidBody2D", json!({"position": [0.0, 0.0]}));
    node(&mut s, Some(caixa), "CollisionShape2D", json!({"size": [32.0, 32.0]}));

    let mut p = Physics::build(&s, &cfg());
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

    let mut p = Physics::build(&s, &cfg());
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
    let heroi = node(&mut s, Some(root), "CharacterBody2D", json!({"velocity": [300.0, 0.0]}));
    node(&mut s, Some(heroi), "CollisionShape2D", json!({"size": [32.0, 32.0]}));

    let mut p = Physics::build(&s, &cfg());
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

    let mut p = Physics::build(&s, &cfg());
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

    let mut p = Physics::build(&s, &cfg());
    let entrou = eventos(&mut p, &mut s, 1.5);

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
    let inimigo = node(&mut s, Some(root), "RigidBody2D", json!({"position": [20.0, -200.0], "layer": "enemy"}));
    node(&mut s, Some(inimigo), "CollisionShape2D", json!({"size": [16.0, 16.0]}));

    let mut p = Physics::build(&s, &cfg());
    let eventos = eventos(&mut p, &mut s, 1.5);

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
    let mut p = Physics::build(&s, &cfg());
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

    let mut p = Physics::build(&s, &cfg());
    run(&mut p, &mut s, 2.0);

    // Sem chão, a caixa passa do ponto onde ele estaria.
    assert!(pos(&s, caixa)[1] > 300.0);
}

// ---------- PHY-002: esteiras ----------

#[test]
fn esteira_arrasta_caixa_apoiada() {
    let mut s = Scene::new("teste", 10);
    let root = raiz(&mut s);
    let esteira = node(
        &mut s,
        Some(root),
        "StaticBody2D",
        json!({"position": [0.0, 300.0], "constant_linear_velocity": [100.0, 0.0]}),
    );
    node(&mut s, Some(esteira), "CollisionShape2D", json!({"size": [400.0, 32.0]}));
    let caixa = caixa_em(&mut s, root, 0.0, 0.0, json!({}));

    let mut p = Physics::build(&s, &cfg());
    run(&mut p, &mut s, 2.0);

    let x = pos(&s, caixa)[0];
    assert!(x > 50.0 && x < 200.0, "x = {x}");
    // A esteira não se move: ela só empurra o que está em cima.
    assert_eq!(pos(&s, esteira), [0.0, 300.0]);
}

#[test]
fn esteira_arrasta_caixa_com_esteira_criada_depois() {
    // A ordem dos nós muda a ordem dos colisores no Rapier: o sentido tem que dar o mesmo resultado.
    let mut s = Scene::new("teste", 46);
    let root = raiz(&mut s);
    let caixa = caixa_em(&mut s, root, 0.0, 0.0, json!({}));
    let esteira = node(
        &mut s,
        Some(root),
        "StaticBody2D",
        json!({"position": [0.0, 300.0], "constant_linear_velocity": [100.0, 0.0]}),
    );
    node(&mut s, Some(esteira), "CollisionShape2D", json!({"size": [400.0, 32.0]}));

    let mut p = Physics::build(&s, &cfg());
    run(&mut p, &mut s, 2.0);

    let x = pos(&s, caixa)[0];
    assert!(x > 50.0 && x < 200.0, "x = {x}");
}

#[test]
fn sem_esteira_caixa_fica_parada() {
    let mut s = Scene::new("teste", 11);
    let root = raiz(&mut s);
    piso(&mut s, root, 0.0, 300.0, 400.0);
    let caixa = caixa_em(&mut s, root, 0.0, 0.0, json!({}));

    let mut p = Physics::build(&s, &cfg());
    run(&mut p, &mut s, 2.0);

    assert!(pos(&s, caixa)[0].abs() < 1.0, "x = {}", pos(&s, caixa)[0]);
}

// ---------- PHY-003: massa, amortecimento, sono e congelamento ----------

#[test]
fn massa_define_massa_total_do_corpo() {
    let mut s = Scene::new("teste", 12);
    let root = raiz(&mut s);
    let caixa = caixa_em(&mut s, root, 0.0, 0.0, json!({"mass": 5.0}));

    let p = Physics::build(&s, &cfg());
    let massa = p.world.bodies[handle_de(&p, caixa)].mass();
    assert!((massa - 5.0).abs() < 1e-3, "massa = {massa}");
}

#[test]
fn massa_nao_muda_a_queda() {
    let mut s = Scene::new("teste", 13);
    let root = raiz(&mut s);
    let leve = caixa_em(&mut s, root, 0.0, 0.0, json!({"mass": 1.0}));
    let pesada = caixa_em(&mut s, root, 200.0, 0.0, json!({"mass": 10.0}));

    let mut p = Physics::build(&s, &cfg());
    run(&mut p, &mut s, 0.5);

    // A gravidade acelera todos os corpos igual, independentemente da massa.
    assert!((pos(&s, leve)[1] - pos(&s, pesada)[1]).abs() < 1e-3);
}

#[test]
fn amortecimento_angular_chega_ao_corpo() {
    let mut s = Scene::new("teste", 14);
    let root = raiz(&mut s);
    let caixa = caixa_em(&mut s, root, 0.0, 0.0, json!({"angular_damping": 2.5}));

    let p = Physics::build(&s, &cfg());
    let amort = p.world.bodies[handle_de(&p, caixa)].angular_damping();
    assert!((amort - 2.5).abs() < 1e-6, "amortecimento = {amort}");
}

#[test]
fn pode_dormir_quando_parado_e_nao_dorme_se_proibido() {
    let mut s = Scene::new("teste", 15);
    let root = raiz(&mut s);
    piso(&mut s, root, 0.0, 300.0, 600.0);
    let dorme = caixa_em(&mut s, root, -150.0, 0.0, json!({}));
    let acordado = caixa_em(&mut s, root, 150.0, 0.0, json!({"can_sleep": false}));

    let mut p = Physics::build(&s, &cfg());
    run(&mut p, &mut s, 4.0);

    assert!(p.world.bodies[handle_de(&p, dorme)].is_sleeping());
    assert!(!p.world.bodies[handle_de(&p, acordado)].is_sleeping());
}

#[test]
fn congelado_estatico_nao_se_move() {
    let mut s = Scene::new("teste", 16);
    let root = raiz(&mut s);
    let caixa = caixa_em(&mut s, root, 0.0, 0.0, json!({"freeze": "static"}));

    let mut p = Physics::build(&s, &cfg());
    run(&mut p, &mut s, 2.0);

    assert_eq!(pos(&s, caixa), [0.0, 0.0]);
}

#[test]
fn congelado_cinematico_nao_se_move() {
    let mut s = Scene::new("teste", 17);
    let root = raiz(&mut s);
    let caixa = caixa_em(&mut s, root, 0.0, 0.0, json!({"freeze": "kinematic"}));

    let mut p = Physics::build(&s, &cfg());
    run(&mut p, &mut s, 2.0);

    assert_eq!(pos(&s, caixa), [0.0, 0.0]);
}

// ---------- PHY-004: personagens ----------

#[test]
fn personagem_no_chao_tem_angulo_zero() {
    let mut s = Scene::new("teste", 18);
    let root = raiz(&mut s);
    piso(&mut s, root, 0.0, 300.0, 400.0);
    let heroi = node(&mut s, Some(root), "CharacterBody2D", json!({"position": [0.0, 200.0]}));
    node(&mut s, Some(heroi), "CollisionShape2D", json!({"size": [32.0, 32.0]}));

    let mut p = Physics::build(&s, &cfg());
    // Sem gravidade própria, o personagem só desce quando recebe velocidade.
    p.set_velocity(heroi, [0.0, 300.0]);
    run(&mut p, &mut s, 1.0);

    let st = p.character_state(heroi).unwrap();
    assert!(st.on_floor, "{st:?}");
    assert!(st.floor_angle.abs() < 1.0, "angulo = {}", st.floor_angle);
    assert!(p.character_state(root).is_none(), "nó sem personagem não tem estado");
}

#[test]
fn personagem_em_rampa_mede_o_angulo() {
    let mut s = Scene::new("teste", 19);
    let root = raiz(&mut s);
    let rampa = node(&mut s, Some(root), "StaticBody2D", json!({"position": [0.0, 300.0], "rotation": 20.0}));
    node(&mut s, Some(rampa), "CollisionShape2D", json!({"size": [400.0, 32.0]}));
    let heroi = node(&mut s, Some(root), "CharacterBody2D", json!({"position": [0.0, 200.0]}));
    node(&mut s, Some(heroi), "CollisionShape2D", json!({"size": [32.0, 32.0]}));

    let mut p = Physics::build(&s, &cfg());
    p.set_velocity(heroi, [0.0, 300.0]);
    run(&mut p, &mut s, 1.0);

    let st = p.character_state(heroi).unwrap();
    assert!(st.on_floor, "{st:?}");
    assert!((st.floor_angle - 20.0).abs() < 2.0, "angulo = {}", st.floor_angle);
}

#[test]
fn personagem_marca_teto() {
    let mut s = Scene::new("teste", 20);
    let root = raiz(&mut s);
    let teto = node(&mut s, Some(root), "StaticBody2D", json!({"position": [0.0, 0.0]}));
    node(&mut s, Some(teto), "CollisionShape2D", json!({"size": [400.0, 32.0]}));
    let heroi =
        node(&mut s, Some(root), "CharacterBody2D", json!({"position": [0.0, 100.0], "velocity": [0.0, -300.0]}));
    node(&mut s, Some(heroi), "CollisionShape2D", json!({"size": [32.0, 32.0]}));

    let mut p = Physics::build(&s, &cfg());
    let mut achou = false;
    for _ in 0..120 {
        p.advance(&mut s, STEP);
        if p.character_state(heroi).unwrap().on_ceiling {
            achou = true;
            break;
        }
    }
    assert!(achou, "o personagem devia bater no teto");
}

#[test]
fn personagem_marca_parede() {
    let mut s = Scene::new("teste", 21);
    let root = raiz(&mut s);
    let parede = node(&mut s, Some(root), "StaticBody2D", json!({"position": [100.0, 0.0]}));
    node(&mut s, Some(parede), "CollisionShape2D", json!({"size": [20.0, 200.0]}));
    let heroi = node(&mut s, Some(root), "CharacterBody2D", json!({"velocity": [300.0, 0.0]}));
    node(&mut s, Some(heroi), "CollisionShape2D", json!({"size": [32.0, 32.0]}));

    let mut p = Physics::build(&s, &cfg());
    let mut achou = false;
    for _ in 0..120 {
        p.advance(&mut s, STEP);
        if p.character_state(heroi).unwrap().on_wall {
            achou = true;
            break;
        }
    }
    assert!(achou, "o personagem devia bater na parede");
}

#[test]
fn personagem_carregado_pela_esteira() {
    let mut s = Scene::new("teste", 22);
    let root = raiz(&mut s);
    let esteira = node(
        &mut s,
        Some(root),
        "StaticBody2D",
        json!({"position": [0.0, 300.0], "constant_linear_velocity": [60.0, 0.0]}),
    );
    node(&mut s, Some(esteira), "CollisionShape2D", json!({"size": [400.0, 32.0]}));
    let heroi = node(&mut s, Some(root), "CharacterBody2D", json!({"position": [0.0, 240.0]}));
    node(&mut s, Some(heroi), "CollisionShape2D", json!({"size": [32.0, 32.0]}));

    let mut p = Physics::build(&s, &cfg());
    // Cai sobre a esteira (com velocidade para baixo) e se assenta.
    p.set_velocity(heroi, [0.0, 300.0]);
    run(&mut p, &mut s, 0.5);
    p.set_velocity(heroi, [0.0, 0.0]);
    run(&mut p, &mut s, 0.2);
    let x1 = pos(&s, heroi)[0];
    run(&mut p, &mut s, 1.0);
    let x2 = pos(&s, heroi)[0];

    // Sem velocidade própria, o personagem anda com a esteira: 60 px/s.
    assert!(((x2 - x1) - 60.0).abs() < 6.0, "deslocamento = {}", x2 - x1);
    assert!(p.character_state(heroi).unwrap().platform_velocity[0] > 50.0);
}

// ---------- PHY-005: áreas ----------

#[test]
fn area_combina_gravidade() {
    // Sem área: queda normal.
    let mut base = Scene::new("teste", 23);
    let root = raiz(&mut base);
    let sozinha = caixa_em(&mut base, root, 0.0, 0.0, json!({}));
    let mut p0 = Physics::build(&base, &cfg());
    run(&mut p0, &mut base, 0.5);
    let y_base = pos(&base, sozinha)[1];

    // Com área que soma 980 px/s² à gravidade, cobrindo a caixa desde o início.
    let mut s = Scene::new("teste", 24);
    let root = raiz(&mut s);
    let area = node(&mut s, Some(root), "Area2D", json!({"gravity_override": "combine", "gravity": [0.0, 980.0]}));
    node(&mut s, Some(area), "CollisionShape2D", json!({"size": [200.0, 200.0]}));
    let caixa = caixa_em(&mut s, root, 0.0, 0.0, json!({}));
    let mut p = Physics::build(&s, &cfg());
    run(&mut p, &mut s, 0.5);
    let y = pos(&s, caixa)[1];

    // Dobrar a aceleração dobra o caminho percorrido (o primeiro passo ainda não tem a área).
    let razao = y / y_base;
    assert!(razao > 1.85 && razao < 2.05, "razão = {razao}");
}

#[test]
fn area_substitui_gravidade() {
    let mut s = Scene::new("teste", 25);
    let root = raiz(&mut s);
    let area = node(&mut s, Some(root), "Area2D", json!({"gravity_override": "replace", "gravity": [0.0, 0.0]}));
    node(&mut s, Some(area), "CollisionShape2D", json!({"size": [200.0, 200.0]}));
    let caixa = caixa_em(&mut s, root, 0.0, 0.0, json!({}));

    let mut p = Physics::build(&s, &cfg());
    run(&mut p, &mut s, 1.0);

    // Sem gravidade, a caixa quase não sai do lugar (só o primeiro passo, antes da área valer).
    assert!(pos(&s, caixa)[1] < 40.0, "y = {}", pos(&s, caixa)[1]);
}

#[test]
fn area_amortece_queda() {
    let mut s = Scene::new("teste", 26);
    let root = raiz(&mut s);
    let area = node(&mut s, Some(root), "Area2D", json!({"damp_override": "combine", "linear_damp": 10.0}));
    node(&mut s, Some(area), "CollisionShape2D", json!({"size": [200.0, 200.0]}));
    let caixa = caixa_em(&mut s, root, 0.0, 0.0, json!({}));

    let mut p = Physics::build(&s, &cfg());
    run(&mut p, &mut s, 1.0);

    // Sem amortecimento a caixa cairia uns 490 px em 1 s. Com 10 de amortecimento (o máximo do
    // registro), a velocidade limite é 980 / 10 = 98 px/s, então ela cai bem menos.
    assert!(pos(&s, caixa)[1] < 150.0, "y = {}", pos(&s, caixa)[1]);
}

#[test]
fn saida_da_area_dispara_body_exited() {
    let mut s = Scene::new("teste", 27);
    let root = raiz(&mut s);
    let area = node(&mut s, Some(root), "Area2D", json!({}));
    node(&mut s, Some(area), "CollisionShape2D", json!({"size": [64.0, 64.0]}));
    let caixa = caixa_em(&mut s, root, 0.0, 0.0, json!({}));

    let mut p = Physics::build(&s, &cfg());
    let ev = eventos(&mut p, &mut s, 1.0);

    assert!(ev.iter().any(|e| e.kind == EventKind::BodyEntered && e.node == area && e.other == caixa));
    assert!(ev.iter().any(|e| e.kind == EventKind::BodyExited && e.node == area && e.other == caixa));
}

// ---------- PHY-010: formas e colisão de uma face ----------

#[test]
fn capsula_repousa_no_chao() {
    let mut s = Scene::new("teste", 28);
    let root = raiz(&mut s);
    piso(&mut s, root, 0.0, 300.0, 400.0);
    let corpo = node(&mut s, Some(root), "RigidBody2D", json!({"position": [0.0, 0.0]}));
    // Largura 16 (raio 8) e altura total 48: a cápsula repousa com o centro a 24 px do topo do chão.
    node(&mut s, Some(corpo), "CollisionShape2D", json!({"shape": "capsule_y", "size": [16.0, 48.0]}));

    let mut p = Physics::build(&s, &cfg());
    run(&mut p, &mut s, 2.0);

    let y = pos(&s, corpo)[1];
    assert!((y - 276.0).abs() < 1.5, "y = {y}");
}

#[test]
fn segmento_sustenta_caixa() {
    let mut s = Scene::new("teste", 29);
    let root = raiz(&mut s);
    let chao = node(&mut s, Some(root), "StaticBody2D", json!({"position": [0.0, 300.0]}));
    node(&mut s, Some(chao), "CollisionShape2D", json!({"shape": "segment", "size": [400.0, 0.0]}));
    let caixa = caixa_em(&mut s, root, 0.0, 0.0, json!({}));

    let mut p = Physics::build(&s, &cfg());
    run(&mut p, &mut s, 2.0);

    let y = pos(&s, caixa)[1];
    assert!((y - 284.0).abs() < 1.5, "y = {y}");
}

#[test]
fn plataforma_de_uma_face_segura_de_cima() {
    let mut s = Scene::new("teste", 30);
    let root = raiz(&mut s);
    let plataforma = node(&mut s, Some(root), "StaticBody2D", json!({"position": [0.0, 200.0]}));
    node(&mut s, Some(plataforma), "CollisionShape2D", json!({"size": [200.0, 16.0], "one_way": true}));
    let caixa = caixa_em(&mut s, root, 0.0, 0.0, json!({}));

    let mut p = Physics::build(&s, &cfg());
    run(&mut p, &mut s, 2.0);

    // O topo da plataforma está em y = 192; a caixa repousa com o centro em 176.
    let y = pos(&s, caixa)[1];
    assert!((y - 176.0).abs() < 2.0, "y = {y}");
}

#[test]
fn plataforma_de_uma_face_deixa_caixa_subir_de_baixo() {
    // Gravidade para cima: a caixa sobe e atravessa a plataforma de baixo para cima.
    let mut c = cfg();
    c.gravity = [0.0, -980.0];
    let mut s = Scene::new("teste", 31);
    let root = raiz(&mut s);
    let plataforma = node(&mut s, Some(root), "StaticBody2D", json!({"position": [0.0, 200.0]}));
    node(&mut s, Some(plataforma), "CollisionShape2D", json!({"size": [200.0, 16.0], "one_way": true}));
    let caixa = caixa_em(&mut s, root, 0.0, 260.0, json!({}));

    let mut p = Physics::build(&s, &c);
    run(&mut p, &mut s, 1.0);

    assert!(pos(&s, caixa)[1] < 150.0, "y = {}", pos(&s, caixa)[1]);
}

#[test]
fn plataforma_comum_bloqueia_caixa_de_baixo() {
    // Mesma cena, sem colisão de uma face: a caixa para embaixo da plataforma.
    let mut c = cfg();
    c.gravity = [0.0, -980.0];
    let mut s = Scene::new("teste", 32);
    let root = raiz(&mut s);
    let plataforma = node(&mut s, Some(root), "StaticBody2D", json!({"position": [0.0, 200.0]}));
    node(&mut s, Some(plataforma), "CollisionShape2D", json!({"size": [200.0, 16.0]}));
    let caixa = caixa_em(&mut s, root, 0.0, 260.0, json!({}));

    let mut p = Physics::build(&s, &c);
    run(&mut p, &mut s, 1.0);

    let y = pos(&s, caixa)[1];
    assert!(y > 200.0 && y < 240.0, "y = {y}");
}

#[test]
fn personagem_sobe_por_plataforma_de_uma_face() {
    let mut s = Scene::new("teste", 33);
    let root = raiz(&mut s);
    let plataforma = node(&mut s, Some(root), "StaticBody2D", json!({"position": [0.0, 200.0]}));
    node(&mut s, Some(plataforma), "CollisionShape2D", json!({"size": [200.0, 16.0], "one_way": true}));
    let heroi =
        node(&mut s, Some(root), "CharacterBody2D", json!({"position": [0.0, 260.0], "velocity": [0.0, -300.0]}));
    node(&mut s, Some(heroi), "CollisionShape2D", json!({"size": [32.0, 32.0]}));

    let mut p = Physics::build(&s, &cfg());
    run(&mut p, &mut s, 1.0);

    assert!(pos(&s, heroi)[1] < 150.0, "y = {}", pos(&s, heroi)[1]);
}

#[test]
fn personagem_pousa_em_plataforma_de_uma_face() {
    let mut s = Scene::new("teste", 34);
    let root = raiz(&mut s);
    let plataforma = node(&mut s, Some(root), "StaticBody2D", json!({"position": [0.0, 200.0]}));
    node(&mut s, Some(plataforma), "CollisionShape2D", json!({"size": [200.0, 16.0], "one_way": true}));
    let heroi =
        node(&mut s, Some(root), "CharacterBody2D", json!({"position": [0.0, 100.0], "velocity": [0.0, 300.0]}));
    node(&mut s, Some(heroi), "CollisionShape2D", json!({"size": [32.0, 32.0]}));

    let mut p = Physics::build(&s, &cfg());
    run(&mut p, &mut s, 1.0);

    let y = pos(&s, heroi)[1];
    assert!((y - 176.0).abs() < 2.0, "y = {y}");
}

// ---------- PHY-011: polígonos ----------

#[test]
fn poligono_convexo_vira_colisor() {
    let mut s = Scene::new("teste", 35);
    let root = raiz(&mut s);
    let chao = node(&mut s, Some(root), "StaticBody2D", json!({"position": [0.0, 300.0]}));
    node(
        &mut s,
        Some(chao),
        "CollisionPolygon2D",
        json!({"points": [[-150.0, -16.0], [150.0, -16.0], [150.0, 16.0], [-150.0, 16.0]]}),
    );
    let caixa = caixa_em(&mut s, root, 0.0, 0.0, json!({}));

    let mut p = Physics::build(&s, &cfg());
    run(&mut p, &mut s, 2.0);

    let y = pos(&s, caixa)[1];
    assert!((y - 268.0).abs() < 1.5, "y = {y}");
}

#[test]
fn poligono_concavo_vira_convexos() {
    // Forma em L: o braço de cima tem topo em y = 0 (y = 300 no mundo) e a caixa cai sobre ele.
    let mut s = Scene::new("teste", 36);
    let root = raiz(&mut s);
    let chao = node(&mut s, Some(root), "StaticBody2D", json!({"position": [0.0, 300.0]}));
    node(
        &mut s,
        Some(chao),
        "CollisionPolygon2D",
        json!({"points": [[0.0, 0.0], [100.0, 0.0], [100.0, 20.0], [20.0, 20.0], [20.0, 100.0], [0.0, 100.0]]}),
    );
    let caixa = caixa_em(&mut s, root, 60.0, 0.0, json!({}));

    let mut p = Physics::build(&s, &cfg());
    run(&mut p, &mut s, 2.0);

    let y = pos(&s, caixa)[1];
    assert!((y - 284.0).abs() < 1.5, "y = {y}");
}

#[test]
fn poligono_com_poucos_pontos_e_ignorado() {
    let mut s = Scene::new("teste", 37);
    let root = raiz(&mut s);
    let corpo = node(&mut s, Some(root), "StaticBody2D", json!({"position": [0.0, 300.0]}));
    node(&mut s, Some(corpo), "CollisionPolygon2D", json!({"points": [[0.0, 0.0], [10.0, 0.0]]}));
    let caixa = caixa_em(&mut s, root, 0.0, 0.0, json!({}));

    let mut p = Physics::build(&s, &cfg());
    run(&mut p, &mut s, 2.0);

    // Sem forma válida, o corpo não tem colisor: a caixa cai além do ponto do chão.
    assert!(pos(&s, caixa)[1] > 300.0);
}

// ---------- PHY-020, PHY-021, PHY-022: camadas, gravidade e matriz ----------

#[test]
fn camada_com_nome_usa_o_bit_da_lista() {
    let mut c = cfg();
    c.layers[4] = "moeda".into();
    let mut s = Scene::new("teste", 38);
    let root = raiz(&mut s);
    let moeda = node(&mut s, Some(root), "RigidBody2D", json!({"layer": "moeda"}));
    let chao = node(&mut s, Some(root), "RigidBody2D", json!({"layer": "ground"}));

    let g_moeda = layer_groups(s.node(moeda).unwrap(), &c);
    let g_chao = layer_groups(s.node(chao).unwrap(), &c);
    assert_eq!(g_moeda.memberships.bits(), 1 << 4);
    // As camadas antigas continuam nos mesmos bits.
    assert_eq!(g_chao.memberships.bits(), 1 << 1);
}

#[test]
fn camada_desconhecida_nao_colide() {
    let mut s = Scene::new("teste", 39);
    let root = raiz(&mut s);
    piso(&mut s, root, 0.0, 300.0, 400.0);
    let caixa = caixa_em(&mut s, root, 0.0, 0.0, json!({"layer": "fantasma"}));

    let mut p = Physics::build(&s, &cfg());
    run(&mut p, &mut s, 2.0);

    assert!(pos(&s, caixa)[1] > 300.0, "y = {}", pos(&s, caixa)[1]);
}

/// Queda de uma caixa em 1 s, com a gravidade dada. Devolve o deslocamento em y.
fn queda_com_gravidade(gravidade: [f64; 2]) -> f64 {
    let mut c = cfg();
    c.gravity = gravidade;
    let mut s = Scene::new("teste", 40);
    let root = raiz(&mut s);
    let caixa = caixa_em(&mut s, root, 0.0, 0.0, json!({}));
    let mut p = Physics::build(&s, &c);
    run(&mut p, &mut s, 1.0);
    pos(&s, caixa)[1]
}

#[test]
fn gravidade_vem_das_configuracoes() {
    // A queda é proporcional à gravidade do projeto: metade da gravidade, metade do deslocamento.
    let padrao = queda_com_gravidade([0.0, 980.0]);
    let metade = queda_com_gravidade([0.0, 490.0]);
    let razao = metade / padrao;
    assert!((razao - 0.5).abs() < 0.01, "razão = {razao}");
}

#[test]
fn matriz_bloqueia_colisao_entre_camadas() {
    let mut c = cfg();
    c.layers[4] = "moeda".into();
    // A camada "moeda" não colide com nenhuma outra (só com ela mesma).
    c.layer_matrix[4] = 1 << 4;
    let mut s = Scene::new("teste", 41);
    let root = raiz(&mut s);
    piso(&mut s, root, 0.0, 300.0, 400.0);
    let caixa = caixa_em(&mut s, root, 0.0, 0.0, json!({"layer": "moeda"}));

    let mut p = Physics::build(&s, &c);
    run(&mut p, &mut s, 2.0);

    assert!(pos(&s, caixa)[1] > 300.0, "y = {}", pos(&s, caixa)[1]);
}

#[test]
fn incluir_camadas_vence_a_matriz() {
    let mut c = cfg();
    c.layers[4] = "moeda".into();
    c.layer_matrix[4] = 0;
    let mut s = Scene::new("teste", 42);
    let root = raiz(&mut s);
    piso(&mut s, root, 0.0, 300.0, 400.0);
    // A matriz diz que "moeda" não colide, mas a inclusão da camada padrão (bit 0) faz colidir.
    let caixa = caixa_em(&mut s, root, 0.0, 0.0, json!({"layer": "moeda", "include_layers": 1}));

    let mut p = Physics::build(&s, &c);
    run(&mut p, &mut s, 2.0);

    // O topo do chão está em 300 (piso), então a caixa repousa com o centro em 284.
    let y = pos(&s, caixa)[1];
    assert!((y - 284.0).abs() < 2.0, "y = {y}");
}

#[test]
fn excluir_camadas_vence_incluir() {
    let mut c = cfg();
    c.layers[4] = "moeda".into();
    c.layer_matrix[4] = 0;
    let mut s = Scene::new("teste", 43);
    let root = raiz(&mut s);
    piso(&mut s, root, 0.0, 300.0, 400.0);
    let caixa = caixa_em(&mut s, root, 0.0, 0.0, json!({"layer": "moeda", "include_layers": 1, "exclude_layers": 1}));

    let mut p = Physics::build(&s, &c);
    run(&mut p, &mut s, 2.0);

    assert!(pos(&s, caixa)[1] > 300.0, "y = {}", pos(&s, caixa)[1]);
}

// ---------- PHY-023: materiais de física ----------

#[test]
fn material_gelo_faz_caixa_deslizar_na_rampa() {
    let gelo = PhysicsMaterial { name: "Gelo".into(), friction: 0.0, bounce: 0.0, rough: false, absorbent: false };
    let mut c = cfg();
    c.physics_materials.push(gelo);

    // Rampa de 20°: com atrito médio 0.5 a caixa para (tan 20° ≈ 0.36); no gelo (atrito médio 0.25) desliza.
    // A caixa começa encostada na face de cima da rampa, para o teste não depender da queda.
    let angulo: f64 = 20.0;
    let (sin, cos) = angulo.to_radians().sin_cos();
    // Centro da rampa em (0, 300), meia altura 16. Normal da face de cima: (sin, -cos).
    let centro_da_caixa = [0.0 + 2.0 * 16.0 * sin, 300.0 - 2.0 * 16.0 * cos];
    let deslizou = |c: &ProjectSettings, material: &str| {
        let mut s = Scene::new("teste", 44);
        let root = raiz(&mut s);
        let rampa = node(
            &mut s,
            Some(root),
            "StaticBody2D",
            json!({"position": [0.0, 300.0], "rotation": angulo, "physics_material": material}),
        );
        node(&mut s, Some(rampa), "CollisionShape2D", json!({"size": [400.0, 32.0]}));
        let caixa = caixa_em(&mut s, root, centro_da_caixa[0], centro_da_caixa[1], json!({}));
        let mut p = Physics::build(&s, c);
        // Primeiro assenta (a caixa se acomoda no encaixe da face); depois mede o deslizamento.
        run(&mut p, &mut s, 0.5);
        let [x0, y0] = pos(&s, caixa);
        run(&mut p, &mut s, 1.0);
        let [x1, y1] = pos(&s, caixa);
        (x1 - x0).hypot(y1 - y0)
    };

    assert!(deslizou(&c, "Gelo") > 10.0, "gelo: {}", deslizou(&c, "Gelo"));
    assert!(deslizou(&c, "") < 3.0, "padrão: {}", deslizou(&c, ""));
}

#[test]
fn material_resolve_regras_de_combinacao() {
    let mut c = cfg();
    c.physics_materials.push(PhysicsMaterial {
        name: "Borracha".into(),
        friction: 0.9,
        bounce: 0.0,
        rough: true,
        absorbent: false,
    });
    c.physics_materials.push(PhysicsMaterial {
        name: "Esponja".into(),
        friction: 0.5,
        bounce: 0.8,
        rough: false,
        absorbent: true,
    });
    let mut s = Scene::new("teste", 45);
    let root = raiz(&mut s);
    let borracha = node(&mut s, Some(root), "StaticBody2D", json!({"physics_material": "Borracha"}));
    node(&mut s, Some(borracha), "CollisionShape2D", json!({"size": [32.0, 32.0]}));
    let esponja = node(&mut s, Some(root), "StaticBody2D", json!({"physics_material": "Esponja"}));
    node(&mut s, Some(esponja), "CollisionShape2D", json!({"size": [32.0, 32.0]}));

    let p = Physics::build(&s, &c);
    // Áspera: atrito pelo máximo. Absorvente: quique pelo mínimo. Sem material: média.
    assert_eq!(colisor_de(&p, borracha).friction_combine_rule(), CoefficientCombineRule::Max);
    assert_eq!(colisor_de(&p, esponja).restitution_combine_rule(), CoefficientCombineRule::Min);
    assert_eq!(colisor_de(&p, esponja).friction_combine_rule(), CoefficientCombineRule::Average);
    assert!((colisor_de(&p, borracha).friction() - 0.9).abs() < 1e-6);
}

/// Colisor do corpo `uid` (o primeiro, para corpos com uma forma).
fn colisor_de(p: &Physics, uid: Uid) -> &Collider {
    let h = p.owner.iter().find(|(_, u)| **u == uid).map(|(h, _)| *h).unwrap();
    &p.world.colliders[h]
}

// ---------- PHY-032: eventos de contato ----------

#[test]
fn evento_de_contato_traz_ponto_e_normal() {
    let mut s = Scene::new("teste", 46);
    let root = raiz(&mut s);
    let chao = piso(&mut s, root, 0.0, 300.0, 400.0);
    let caixa = caixa_em(&mut s, root, 0.0, 0.0, json!({}));

    let mut p = Physics::build(&s, &cfg());
    let ev = eventos(&mut p, &mut s, 1.0);

    let da_caixa = ev
        .iter()
        .find(|e| e.kind == EventKind::CollisionStarted && e.node == caixa && e.other == chao)
        .expect("contato da caixa com o chão");
    let [px, py] = da_caixa.point.expect("ponto de contato");
    // O topo do chão (piso com topo em 300) é a superfície onde a caixa encosta.
    assert!((py - 300.0).abs() < 1.5 && px.abs() < 20.0, "ponto = ({px}, {py})");
    // A normal aponta do chão para a caixa: para cima, em Y para baixo.
    let [nx, ny] = da_caixa.normal.expect("normal");
    assert!(nx.abs() < 1e-3 && ny < -0.9, "normal = ({nx}, {ny})");

    let do_chao = ev
        .iter()
        .find(|e| e.kind == EventKind::CollisionStarted && e.node == chao && e.other == caixa)
        .expect("contato do chão com a caixa");
    assert!(do_chao.normal.unwrap()[1] > 0.9, "normal do chão = {:?}", do_chao.normal);
}

#[test]
fn evento_serializa_ponto_e_normal() {
    let uid = Uid(1);
    let contato = PhysicsEvent {
        kind: EventKind::CollisionStarted,
        node: uid,
        other: Uid(2),
        point: Some([1.0, 2.0]),
        normal: Some([0.0, -1.0]),
    };
    let v = serde_json::to_value(contato).unwrap();
    assert_eq!(v["kind"], "collision_started");
    assert_eq!(v["point"], json!([1.0, 2.0]));
    assert_eq!(v["normal"], json!([0.0, -1.0]));

    let area = PhysicsEvent { kind: EventKind::BodyEntered, node: uid, other: Uid(2), point: None, normal: None };
    let v = serde_json::to_value(area).unwrap();
    assert!(v["point"].is_null() && v["normal"].is_null());
}

// ---------- Validação de referências (usada pelo editor) ----------

#[test]
fn valida_camada_e_material_contra_configuracoes() {
    let mut c = cfg();
    c.physics_materials.push(PhysicsMaterial {
        name: "Gelo".into(),
        friction: 0.0,
        bounce: 0.0,
        rough: false,
        absorbent: false,
    });
    assert!(check_reference(&c, "layer", &json!("ground")).is_ok());
    assert!(check_reference(&c, "layer", &json!("fantasma")).is_err());
    assert!(check_reference(&c, "layer", &json!("")).is_err());
    assert!(check_reference(&c, "physics_material", &json!("")).is_ok());
    assert!(check_reference(&c, "physics_material", &json!("Gelo")).is_ok());
    assert!(check_reference(&c, "physics_material", &json!("Madeira")).is_err());
    // Outras propriedades não são checadas aqui.
    assert!(check_reference(&c, "mass", &json!(2.0)).is_ok());
}

#[test]
fn gravidade_zero_usa_a_direcao_da_tela_para_cima() {
    assert_eq!(up_of([0.0, 0.0]), Vector::new(0.0, -1.0));
    assert_eq!(up_of([0.0, 980.0]), Vector::new(0.0, -1.0));
    assert_eq!(up_of([0.0, -980.0]), Vector::new(0.0, 1.0));
}
