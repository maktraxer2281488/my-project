//! Торговля с жителями: у каждой профессии свои предложения, валюта — монеты.

use crate::items::{self as it, Item, Mat, Slot, ToolKind};
use crate::mobdefs::MOBS;
use crate::{App, Screen};
use macroquad::prelude::*;

/// Что отдаёт игрок → что получает.
pub type Offer = ((Item, u16), (Item, u16));

fn coin(n: u16) -> (Item, u16) {
    (Item::Thing(it::COIN), n)
}

fn t(id: u16, n: u16) -> (Item, u16) {
    (Item::Thing(id), n)
}

/// Предложения жителя (None — с ним торговать нельзя).
pub fn offers(def: usize, uid: u32) -> Option<Vec<Offer>> {
    let name = MOBS[def].name;
    let mut v: Vec<Offer> = match name {
        "Селянин" => vec![(t(it::WHEAT, 20), coin(1)), (t(it::CARROT, 15), coin(1)), (coin(1), t(it::BREAD, 3)), (coin(1), t(it::APPLE, 4)), (coin(8), t(it::GOLDEN_APPLE, 1))],
        "Мельник" => vec![(t(it::WHEAT, 12), coin(1)), (coin(1), t(it::FLOUR, 4)), (coin(2), t(it::CAKE, 2)), (coin(3), t(it::PUMPKIN_PIE, 2)), (coin(1), t(it::WHEAT_SEEDS, 8))],
        "Кузнец" => vec![
            (t(it::COAL, 15), coin(1)),
            (t(it::IRON_INGOT, 4), coin(1)),
            (coin(6), (Item::Tool(ToolKind::Sword, Mat::Iron), 1)),
            (coin(7), (Item::Tool(ToolKind::Pickaxe, Mat::Iron), 1)),
            (coin(12), (Item::Armor(Slot::Chest, Mat::Iron), 1)),
            (coin(20), (Item::Tool(ToolKind::Katana, Mat::Ruby), 1)),
        ],
        "Караванщик" => vec![(t(it::LEATHER, 6), coin(1)), (coin(2), t(it::SALT, 8)), (coin(10), t(it::SAPPHIRE, 1)), (coin(12), t(it::RUBY, 1)), (coin(4), t(it::DATES, 6))],
        "Жрица солнца" => vec![(t(it::GOLD_INGOT, 3), coin(2)), (coin(5), t(it::POTION0, 1)), (coin(6), t(it::POTION0 + 1, 1)), (coin(8), t(it::POTION0 + 4, 1)), (coin(6), t(it::POTION0 + 6, 1))],
        "Отшельник" => vec![(t(it::BERRIES, 16), coin(1)), (coin(4), t(it::POTION0 + 5, 1)), (coin(3), t(it::TEA, 3))],
        "Лесник" => vec![(t(it::STICK, 32), coin(1)), (coin(1), t(it::APPLE, 3)), (coin(2), t(it::NUTS, 8)), (coin(5), (Item::Tool(ToolKind::Axe, Mat::Iron), 1))],
        "Ангел-проводник" => vec![(t(it::HEAVEN_DUST, 8), coin(1)), (coin(10), t(it::HEAVEN_INGOT, 1)), (coin(15), t(it::GOLDEN_APPLE, 1)), (coin(6), t(it::AMBROSIA, 1))],
        "Странствующий торговец" => vec![
            (t(it::PEARL, 2), coin(3)),
            (coin(8), t(it::CRYSTAL, 1)),
            (coin(16), t(it::SUMMON0 + (uid % it::SUMMON_COUNT as u32) as u16, 1)),
            (coin(4), t(it::CHOCOLATE, 4)),
            (coin(3), t(it::BUCKET, 1)),
        ],
        "Рыбак" => vec![(t(it::RAW_FISH, 6), coin(1)), (coin(2), t(it::COOKED_FISH, 4)), (coin(6), t(it::PEARL, 1)), (coin(3), t(it::SUSHI, 4))],
        _ => return None,
    };
    v.retain(|((_, n), _)| *n > 0);
    Some(v)
}

/// Экран торговли.
pub fn screen(app: &mut App) -> Option<Screen> {
    let Some((def, uid)) = app.trade else { return Some(Screen::Playing) };
    let Some(list) = offers(def, uid) else { return Some(Screen::Playing) };
    let (w, h) = (screen_width(), screen_height());
    let s = app.ui.s;
    draw_rectangle(0.0, 0.0, w, h, Color::new(0.0, 0.0, 0.0, 0.5));
    let pw = (760.0 * s).min(w - 20.0);
    let row = 54.0 * s;
    let ph = 110.0 * s + row * list.len() as f32;
    let (px, py) = ((w - pw) / 2.0, (h - ph) / 2.0);
    app.ui.panel(px, py, pw, ph);
    app.ui.text(&format!("Торговля: {}", MOBS[def].name), px + 18.0 * s, py + 34.0 * s, 20.0, WHITE);
    let Some(g) = app.game.as_mut() else { return Some(Screen::Playing) };
    let coins = g.player.inv.count(Item::Thing(it::COIN));
    let ct = format!("Монет: {coins}");
    app.ui.text(&ct, px + pw - app.ui.measure(&ct, 15.0) - 18.0 * s, py + 34.0 * s, 15.0, Color::new(1.0, 0.85, 0.3, 1.0));
    let mut y = py + 56.0 * s;
    let mut deal: Option<Offer> = None;
    for o in &list {
        let ((gi, gn), (ri, rn)) = *o;
        let have = g.player.inv.count(gi) >= gn as u32;
        draw_rectangle(px + 14.0 * s, y, pw - 28.0 * s, row - 6.0 * s, Color::new(0.12, 0.13, 0.16, 0.9));
        let ic = row - 14.0 * s;
        app.ui.item_icon(gi, px + 22.0 * s, y + 4.0 * s, ic);
        app.ui.text(&format!("{} ×{gn}", gi.name()), px + 26.0 * s + ic, y + row * 0.55, 13.0, if have { WHITE } else { Color::new(1.0, 0.5, 0.5, 1.0) });
        app.ui.text("→", px + pw * 0.48, y + row * 0.55, 18.0, GRAY);
        app.ui.item_icon(ri, px + pw * 0.53, y + 4.0 * s, ic);
        app.ui.text(&format!("{} ×{rn}", ri.name()), px + pw * 0.53 + ic + 6.0 * s, y + row * 0.55, 13.0, WHITE);
        let bw = 96.0 * s;
        if app.ui.button_small(px + pw - bw - 22.0 * s, y + 8.0 * s, bw, row - 22.0 * s, if have { "Обменять" } else { "Не хватает" }) && have {
            deal = Some(*o);
        }
        if app.ui.hover(px + 22.0 * s, y, ic, ic) {
            app.ui.set_tooltip(gi.name(), gi.description());
        } else if app.ui.hover(px + pw * 0.53, y, ic, ic) {
            app.ui.set_tooltip(ri.name(), ri.description());
        }
        y += row;
    }
    app.ui.text("Монеты — за продажу товаров жителям. Esc или E — закрыть", px + 18.0 * s, py + ph - 16.0 * s, 12.0, GRAY);
    if let Some(((gi, gn), (ri, rn))) = deal {
        g.player.inv.remove(gi, gn);
        let left = g.player.inv.add(ri, rn);
        if left > 0 {
            let p = g.player.body.pos + vec3(0.0, 1.0, 0.0);
            g.ents.spawn_drop(ri, left, p, 0.3);
        }
        g.toast = Some((format!("Куплено: {} ×{rn}", ri.name()), 1.5));
        g.unlock(crate::ach::TRADE);
        app.sfx.push((crate::music::Sfx::Pickup, 0.6));
    }
    if !app.just_opened && (is_key_pressed(KeyCode::Escape) || is_key_pressed(KeyCode::E)) {
        return Some(Screen::Playing);
    }
    None
}
