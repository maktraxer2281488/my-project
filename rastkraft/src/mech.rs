//! Механизмы (рычаг, кнопка, плита, провод, лампа, толкатель, двери на сигнале)
//! и текучие жидкости. Всё считает хозяин мира (или одиночная игра) и рассылает правки.

use crate::blocks::*;
use crate::shapes::{self, Form};
use crate::{App, Dim, edit_block};
use macroquad::prelude::*;
use std::collections::{HashMap, HashSet, VecDeque};

/// Состояние механизмов и жидкостей мира.
#[derive(Default)]
pub struct Mech {
    /// Нажатые кнопки: сколько ещё секунд держать.
    pub buttons: Vec<(IVec3, f32)>,
    pub plates: HashSet<IVec3>,
    pub fluid_q: VecDeque<IVec3>,
    queued: HashSet<IVec3>,
    fluid_t: f32,
    tick: u32,
}

const N6: [IVec3; 6] = [IVec3::X, IVec3::NEG_X, IVec3::Y, IVec3::NEG_Y, IVec3::Z, IVec3::NEG_Z];

pub fn is_source(b: u16) -> bool {
    matches!(b, LEVER_ON | BUTTON_ON | SIGNAL_BLOCK) || b == shapes::PLATE0 + 1
}

fn is_wire(b: u16) -> bool {
    matches!(shapes::form(b), Some(Form::Wire { .. }))
}

fn is_pusher(b: u16) -> bool {
    (PUSHER0..PUSHER0 + 10).contains(&b)
}

/// Блок участвует в сигнальной схеме.
pub fn is_mech(b: u16) -> bool {
    is_source(b)
        || is_wire(b)
        || is_pusher(b)
        || matches!(b, LEVER_OFF | BUTTON | LAMP_OFF | LAMP_ON)
        || matches!(shapes::form(b), Some(Form::Door { .. } | Form::Trapdoor { .. } | Form::Gate { .. } | Form::Plate { .. }))
}

fn get(app: &App, dim: Dim, p: IVec3) -> u16 {
    app.game.as_ref().map_or(AIR, |g| g.worlds[dim as usize].get(p.x, p.y, p.z))
}

/// Пересчитать сигналы вокруг точки (после правки механизма).
pub fn update_signals(app: &mut App, dim: Dim, c: IVec3) {
    const R: i32 = 16;
    let mut sources = Vec::new();
    let mut wires: HashSet<IVec3> = HashSet::new();
    let mut outputs = Vec::new();
    {
        let Some(g) = app.game.as_ref() else { return };
        let w = &g.worlds[dim as usize];
        for y in (c.y - R).max(0)..(c.y + R).min(127) {
            for z in c.z - R..=c.z + R {
                for x in c.x - R..=c.x + R {
                    let b = w.get(x, y, z);
                    if b == AIR || !is_mech(b) {
                        continue;
                    }
                    let p = ivec3(x, y, z);
                    if is_source(b) {
                        sources.push(p);
                    }
                    if is_wire(b) {
                        wires.insert(p);
                    } else if !is_source(b) {
                        outputs.push((p, b));
                    }
                }
            }
        }
    }
    // провода проводят сигнал от источника на 15 клеток
    let mut lit: HashMap<IVec3, u8> = HashMap::new();
    let mut q: VecDeque<(IVec3, u8)> = VecDeque::new();
    for s in &sources {
        for d in N6 {
            if wires.contains(&(*s + d)) && !lit.contains_key(&(*s + d)) {
                lit.insert(*s + d, 15);
                q.push_back((*s + d, 15));
            }
        }
    }
    while let Some((p, lvl)) = q.pop_front() {
        if lvl <= 1 {
            continue;
        }
        // по горизонтали и на ступеньку вверх/вниз
        for d in [IVec3::X, IVec3::NEG_X, IVec3::Z, IVec3::NEG_Z] {
            for dy in [0, 1, -1] {
                let n = p + d + IVec3::Y * dy;
                if wires.contains(&n) && !lit.contains_key(&n) {
                    lit.insert(n, lvl - 1);
                    q.push_back((n, lvl - 1));
                }
            }
        }
    }
    let srcs: HashSet<IVec3> = sources.iter().copied().collect();
    let powered = |p: IVec3| N6.iter().any(|d| srcs.contains(&(p + *d)) || lit.contains_key(&(p + *d)));
    let near_circuit = |p: IVec3| N6.iter().any(|d| srcs.contains(&(p + *d)) || wires.contains(&(p + *d)));
    let mut edits: Vec<(IVec3, u16)> = Vec::new();
    for p in &wires {
        let on = lit.contains_key(p);
        let want = shapes::id(Form::Wire { on });
        if get(app, dim, *p) != want {
            edits.push((*p, want));
        }
    }
    let mut pushes = Vec::new();
    for (p, b) in outputs {
        let on = powered(p);
        match b {
            LAMP_OFF | LAMP_ON => {
                let want = if on { LAMP_ON } else { LAMP_OFF };
                if want != b {
                    edits.push((p, want));
                }
            }
            _ if is_pusher(b) => {
                let (dir, active) = ((b - PUSHER0) / 2, (b - PUSHER0) % 2 == 1);
                if on != active {
                    edits.push((p, PUSHER0 + dir * 2 + on as u16));
                    if on {
                        pushes.push((p, dir));
                    }
                }
            }
            _ => {
                // двери, люки и калитки рядом со схемой открываются сигналом
                let f = shapes::form(b);
                let (circuit, on) = match f {
                    Some(Form::Door { upper, .. }) => {
                        let other = if upper { p - IVec3::Y } else { p + IVec3::Y };
                        (near_circuit(p) || near_circuit(other), on || powered(other))
                    }
                    _ => (near_circuit(p), on),
                };
                if !circuit {
                    continue;
                }
                let want = match f {
                    Some(Form::Door { m, dir, upper, .. }) => shapes::id(Form::Door { m, dir, open: on, upper }),
                    Some(Form::Trapdoor { m, dir, top, .. }) => shapes::id(Form::Trapdoor { m, dir, open: on, top }),
                    Some(Form::Gate { m, axis, .. }) => shapes::id(Form::Gate { m, axis, open: on }),
                    _ => b,
                };
                if want != b {
                    edits.push((p, want));
                }
            }
        }
    }
    if edits.iter().any(|(_, b)| *b == LAMP_ON) {
        if let Some(g) = app.game.as_mut() {
            g.unlock(crate::ach::ENGINEER);
        }
    }
    for (p, b) in edits {
        edit_block(app, dim, p, b, true);
    }
    for (p, dir) in pushes {
        push_line(app, dim, p, dir);
    }
}

/// Толкатель сдвигает ряд блоков (до 12) на одну клетку.
fn push_line(app: &mut App, dim: Dim, p: IVec3, dir: u16) {
    let d = [IVec3::X, IVec3::NEG_X, IVec3::Z, IVec3::NEG_Z, IVec3::Y][dir as usize];
    let mut line = Vec::new();
    let mut c = p + d;
    loop {
        let b = get(app, dim, c);
        let free = b == AIR || def(b).shape == Shape::Liquid || def(b).shape == Shape::Cross;
        if free {
            break;
        }
        if line.len() >= 12 || matches!(b, BEDROCK | OBSIDIAN | PORTAL | PORTAL_FRAME) || is_pusher(b) || c.y >= 127 {
            return;
        }
        line.push((c, b));
        c += d;
    }
    for (q, b) in line.iter().rev() {
        edit_block(app, dim, *q + d, *b, true);
    }
    if let Some((first, _)) = line.first() {
        edit_block(app, dim, *first, AIR, true);
    }
}

impl Mech {
    pub fn schedule(&mut self, p: IVec3) {
        for d in N6.iter().copied().chain([IVec3::ZERO]) {
            let q = p + d;
            if self.queued.insert(q) {
                self.fluid_q.push_back(q);
            }
        }
    }
}

fn replaceable(b: u16) -> bool {
    b == AIR || (def(b).shape == Shape::Cross && b != COBWEB && !matches!(b, LEVER_OFF | LEVER_ON | BUTTON | BUTTON_ON))
}

/// Растекание жидкостей: вода раз в 0.25 с, лава втрое медленнее.
pub fn fluid_tick(app: &mut App, dim: Dim, dt: f32) {
    let Some(g) = app.game.as_mut() else { return };
    g.mech.fluid_t -= dt;
    if g.mech.fluid_t > 0.0 || g.mech.fluid_q.is_empty() {
        return;
    }
    g.mech.fluid_t = 0.25;
    g.mech.tick = g.mech.tick.wrapping_add(1);
    let lava_turn = g.mech.tick % 3 == 0;
    let batch: Vec<IVec3> = (0..400).map_while(|_| g.mech.fluid_q.pop_front()).collect();
    for p in &batch {
        g.mech.queued.remove(p);
    }
    let mut edits: Vec<(IVec3, u16)> = Vec::new();
    let mut later: Vec<IVec3> = Vec::new();
    {
        let w = &g.worlds[dim as usize];
        let at = |q: IVec3| -> u16 { w.get(q.x, q.y, q.z) };
        for &p in &batch {
            if p.y <= 0 || p.y >= 127 || !w.is_loaded(p.x, p.z) {
                continue;
            }
            let b = at(p);
            let Some((water, lvl)) = fluid(b) else { continue };
            if !water && !lava_turn {
                later.push(p);
                continue;
            }
            let same = |q: IVec3| fluid(at(q)).filter(|(w2, _)| *w2 == water);
            // лава с водой: источник — в обсидиан, поток — в булыжник
            let touches_other = N6.iter().any(|d| fluid(at(p + *d)).is_some_and(|(w2, _)| w2 != water));
            if !water && touches_other {
                edits.push((p, if lvl == 0 { OBSIDIAN } else { COBBLE }));
                continue;
            }
            if lvl > 0 {
                let fed = same(p + IVec3::Y).is_some()
                    || [IVec3::X, IVec3::NEG_X, IVec3::Z, IVec3::NEG_Z].iter().any(|d| same(p + *d).is_some_and(|(_, l2)| l2 < lvl));
                if !fed {
                    edits.push((p, AIR));
                    continue;
                }
                // бесконечный источник: вода между двумя источниками на твёрдом дне
                if water {
                    let sources = [IVec3::X, IVec3::NEG_X, IVec3::Z, IVec3::NEG_Z].iter().filter(|d| at(p + **d) == WATER).count();
                    if sources >= 2 && is_solid(at(p - IVec3::Y)) {
                        edits.push((p, WATER));
                        continue;
                    }
                }
            }
            let below = p - IVec3::Y;
            let bb = at(below);
            if replaceable(bb) || fluid(bb).is_some_and(|(w2, l2)| w2 == water && l2 > 1) {
                edits.push((below, fluid_block(water, 1)));
                continue;
            }
            if fluid(bb).is_some_and(|(w2, _)| w2 == water) {
                continue; // течёт вниз по своей жидкости
            }
            let max = if water { 7 } else { 3 };
            let nl = lvl + 1;
            if nl > max {
                continue;
            }
            for d in [IVec3::X, IVec3::NEG_X, IVec3::Z, IVec3::NEG_Z] {
                let q = p + d;
                let nb = at(q);
                if replaceable(nb) || fluid(nb).is_some_and(|(w2, l2)| w2 == water && l2 > nl) {
                    edits.push((q, fluid_block(water, nl)));
                }
            }
        }
    }
    for p in later {
        g.mech.schedule(p);
    }
    for (p, b) in edits {
        if let Some(g) = app.game.as_mut() {
            g.mech.schedule(p);
        }
        edit_block(app, dim, p, b, true);
    }
}

/// Нажимные плиты под игроком и существами; отпущенные кнопки.
pub fn plates_and_buttons(app: &mut App, dim: Dim, dt: f32) {
    let Some(g) = app.game.as_mut() else { return };
    let w = &g.worlds[dim as usize];
    let mut on: HashSet<IVec3> = HashSet::new();
    let mut feet = vec![g.player.body.pos];
    feet.extend(g.mobs.list.iter().filter(|m| m.alive() && (m.body.pos - g.player.body.pos).length() < 40.0).map(|m| m.body.pos));
    feet.extend(g.remotes.values().map(|r| Vec3::from(r.st.pos)));
    for f in feet {
        let c = (f + vec3(0.0, 0.05, 0.0)).floor().as_ivec3();
        if matches!(shapes::form(w.get(c.x, c.y, c.z)), Some(Form::Plate { .. })) {
            on.insert(c);
        }
    }
    let mut changes: Vec<(IVec3, u16)> = Vec::new();
    for p in &on {
        if !g.mech.plates.contains(p) {
            changes.push((*p, shapes::PLATE0 + 1));
        }
    }
    for p in g.mech.plates.iter() {
        if !on.contains(p) && w.get(p.x, p.y, p.z) == shapes::PLATE0 + 1 {
            changes.push((*p, shapes::PLATE0));
        }
    }
    g.mech.plates = on;
    for b in g.mech.buttons.iter_mut() {
        b.1 -= dt;
    }
    let done: Vec<IVec3> = g.mech.buttons.iter().filter(|b| b.1 <= 0.0).map(|b| b.0).collect();
    g.mech.buttons.retain(|b| b.1 > 0.0);
    for p in done {
        if w.get(p.x, p.y, p.z) == BUTTON_ON {
            changes.push((p, BUTTON));
        }
    }
    for (p, b) in changes {
        edit_block(app, dim, p, b, true);
        update_signals(app, dim, p);
    }
}
