//! Процедурные иконки предметов 16×16: форма задаётся функцией, контур
//! и блики достраиваются автоматически.

type Px = [u8; 4];

#[derive(Clone, Copy, PartialEq, Eq, Debug)]
pub enum Icon {
    Stick,
    Lump,
    Ingot,
    Gem,
    Dust,
    Coil,
    Hide,
    Bone,
    Feather,
    Blob,
    Stalks,
    Sack,
    Brick,
    Bottle,
    Bowl,
    Paper,
    Book,
    Comb,
    Drop,
    Shard,
    Egg,
    Berries,
    Apple,
    Bread,
    Carrot,
    Potato,
    Jar,
    Pie,
    Cookie,
    Cake,
    Cheese,
    Soup,
    Meat,
    Drumstick,
    Fish,
    Sushi,
    Coconut,
    Melon,
    Roll,
    Skewer,
    Pancake,
    Star,
    IceCream,
    Mug,
    Horn,
    Fang,
    Claw,
    Scale,
    Eye,
    Orb,
    Shell,
    Tail,
    Wing,
    Heart,
    Mane,
    Spore,
}

fn hsh(x: i32, y: i32, s: u32) -> f32 {
    crate::blocks::hash(x, y, s)
}

fn ell(x: f32, y: f32, cx: f32, cy: f32, rx: f32, ry: f32) -> bool {
    ((x - cx) / rx).powi(2) + ((y - cy) / ry).powi(2) <= 1.0
}

fn seg(x: f32, y: f32, ax: f32, ay: f32, bx: f32, by: f32) -> (f32, f32) {
    let (dx, dy) = (bx - ax, by - ay);
    let t = (((x - ax) * dx + (y - ay) * dy) / (dx * dx + dy * dy)).clamp(0.0, 1.0);
    let (px, py) = (ax + dx * t, ay + dy * t);
    (((x - px).powi(2) + (y - py).powi(2)).sqrt(), t)
}

fn rect(x: f32, y: f32, x0: f32, y0: f32, x1: f32, y1: f32) -> bool {
    x >= x0 && x <= x1 && y >= y0 && y <= y1
}

/// 0 — пусто, 1 — основной цвет, 2 — второй цвет, 3 — блик.
fn region(icon: Icon, xi: i32, yi: i32) -> u8 {
    let (x, y) = (xi as f32 + 0.5, yi as f32 + 0.5);
    use Icon::*;
    match icon {
        Stick => (seg(x, y, 3.5, 13.0, 12.5, 3.0).0 < 1.1) as u8,
        Lump => {
            if ell(x, y, 8.0, 8.5, 5.5 + hsh(xi / 2, 0, 1) * 1.2, 4.8) {
                if hsh(xi, yi, 2) > 0.85 { 3 } else { 1 }
            } else {
                0
            }
        }
        Ingot => {
            if yi >= 5 && yi <= 11 {
                let inset = (11 - yi) as f32 * 0.5;
                if x >= 2.0 + inset && x <= 14.0 - inset {
                    return if yi <= 6 { 3 } else { 1 };
                }
            }
            0
        }
        Gem => {
            let d = (x - 8.0).abs() / 6.0 + (y - 8.0).abs() / 7.0;
            if d <= 1.0 {
                if x < 8.0 && y < 8.0 { 3 } else if x > 8.0 && y > 8.0 { 2 } else { 1 }
            } else {
                0
            }
        }
        Dust => {
            if ell(x, y, 8.0, 11.5, 6.5, 3.5) || ell(x, y, 8.0, 9.0, 4.0, 3.0) {
                if hsh(xi, yi, 3) > 0.8 { 3 } else { 1 }
            } else {
                0
            }
        }
        Coil => {
            let inner = ell(x, y, 8.0, 8.0, 2.5, 2.5);
            if ell(x, y, 8.0, 8.0, 6.0, 6.0) && !inner {
                if (xi + yi) % 3 == 0 { 2 } else { 1 }
            } else {
                0
            }
        }
        Hide => {
            let notch = (xi == 2 || xi == 13) && (yi == 7 || yi == 8);
            if rect(x, y, 2.0, 3.0, 14.0, 13.0) && !notch && !((xi < 4 || xi > 11) && (yi < 4 || yi > 11)) {
                if hsh(xi / 2, yi / 2, 4) > 0.7 { 2 } else { 1 }
            } else {
                0
            }
        }
        Bone => {
            let (d, _) = seg(x, y, 4.5, 11.5, 11.5, 4.5);
            let knob = ell(x, y, 3.5, 11.0, 1.6, 1.6) || ell(x, y, 5.0, 12.8, 1.6, 1.6) || ell(x, y, 11.0, 3.2, 1.6, 1.6) || ell(x, y, 12.8, 5.0, 1.6, 1.6);
            (d < 1.2 || knob) as u8
        }
        Feather => {
            let (d, t) = seg(x, y, 3.0, 13.5, 12.5, 2.5);
            if d < 0.6 {
                2
            } else if d < 3.0 * (1.0 - (t - 0.55).abs() * 1.6).max(0.0) && t > 0.15 {
                if (xi + yi) % 3 == 0 { 3 } else { 1 }
            } else {
                0
            }
        }
        Blob => {
            if ell(x, y, 8.0, 9.0, 5.5, 4.8) {
                if ell(x, y, 6.0, 7.0, 1.5, 1.2) { 3 } else { 1 }
            } else {
                0
            }
        }
        Stalks => {
            for k in 0..3 {
                let bx = 5.0 + k as f32 * 3.0;
                if (x - bx).abs() < 0.7 && y > 6.0 {
                    return 2;
                }
                if ell(x, y, bx, 4.5, 1.4, 3.0) {
                    return 1;
                }
            }
            0
        }
        Sack => {
            if ell(x, y, 8.0, 10.0, 5.5, 4.5) || rect(x, y, 6.0, 3.0, 10.0, 6.0) {
                if yi == 5 { 2 } else if ell(x, y, 6.0, 9.0, 1.2, 1.5) { 3 } else { 1 }
            } else {
                0
            }
        }
        Brick => {
            if rect(x, y, 2.0, 5.0, 14.0, 11.0) {
                if yi == 5 { 3 } else { 1 }
            } else {
                0
            }
        }
        Bottle => {
            if rect(x, y, 6.5, 1.5, 9.5, 3.0) {
                2
            } else if rect(x, y, 7.0, 3.0, 9.0, 6.0) || ell(x, y, 8.0, 10.0, 4.6, 4.6) {
                if y > 9.0 { 1 } else if ell(x, y, 6.5, 8.5, 1.0, 1.2) { 3 } else { 4 }
            } else {
                0
            }
        }
        Bowl => {
            if ell(x, y, 8.0, 8.0, 6.5, 2.0) {
                1
            } else if ell(x, y, 8.0, 8.0, 6.5, 5.5) && y > 8.0 {
                2
            } else {
                0
            }
        }
        Paper => {
            if rect(x, y, 3.0, 2.0, 13.0, 14.0) {
                if yi % 3 == 0 && xi > 4 && xi < 12 { 2 } else { 1 }
            } else {
                0
            }
        }
        Book => {
            if rect(x, y, 3.0, 2.0, 13.0, 14.0) {
                if xi <= 4 { 2 } else if yi == 6 && xi > 6 && xi < 12 { 3 } else { 1 }
            } else {
                0
            }
        }
        Comb => {
            if ell(x, y, 8.0, 8.0, 6.5, 6.0) {
                if (xi + (yi / 2) % 2 * 2) % 4 == 0 || yi % 2 == 0 && xi % 4 == 2 { 2 } else { 1 }
            } else {
                0
            }
        }
        Drop => {
            let top = y < 8.0 && (x - 8.0).abs() < (y - 2.0).max(0.0) * 0.7;
            if ell(x, y, 8.0, 10.0, 4.5, 4.5) || top {
                if ell(x, y, 6.5, 9.5, 1.0, 1.4) { 3 } else { 1 }
            } else {
                0
            }
        }
        Shard => {
            let inside = y > 2.0 && y < 14.0 && (x - 8.0 + (y - 8.0) * 0.3).abs() < (14.0 - y) * 0.42;
            if inside { if x < 8.0 { 3 } else { 1 } } else { 0 }
        }
        Egg => {
            if ell(x, y, 8.0, 8.8, 4.3, 5.6) {
                if ell(x, y, 6.5, 6.5, 1.0, 1.5) { 3 } else if hsh(xi, yi, 5) > 0.9 { 2 } else { 1 }
            } else {
                0
            }
        }
        Berries => {
            if ell(x, y, 6.0, 10.0, 2.6, 2.6) || ell(x, y, 10.0, 10.0, 2.6, 2.6) || ell(x, y, 8.0, 6.5, 2.6, 2.6) {
                if hsh(xi, yi, 6) > 0.85 { 3 } else { 1 }
            } else if seg(x, y, 8.0, 4.0, 10.0, 1.5).0 < 0.8 {
                2
            } else {
                0
            }
        }
        Apple => {
            if ell(x, y, 8.0, 9.5, 5.5, 5.0) {
                if ell(x, y, 6.0, 7.5, 1.2, 1.5) { 3 } else { 1 }
            } else if seg(x, y, 8.0, 4.5, 9.0, 1.5).0 < 0.7 || ell(x, y, 11.0, 3.5, 2.0, 1.0) {
                2
            } else {
                0
            }
        }
        Bread => {
            if ell(x, y, 8.0, 9.0, 6.5, 4.0) {
                if (xi == 5 || xi == 8 || xi == 11) && yi < 9 { 2 } else if yi < 7 { 3 } else { 1 }
            } else {
                0
            }
        }
        Carrot => {
            let (d, t) = seg(x, y, 4.0, 13.0, 10.5, 5.0);
            if d < 0.8 + t * 1.8 {
                if (xi + yi) % 4 == 0 { 3 } else { 1 }
            } else if seg(x, y, 11.0, 4.5, 13.5, 1.5).0 < 0.8 || seg(x, y, 11.0, 4.5, 14.0, 4.0).0 < 0.8 {
                2
            } else {
                0
            }
        }
        Potato => {
            if ell(x, y, 8.0, 8.5, 5.5 + hsh(0, yi / 3, 7), 4.2) {
                if hsh(xi, yi, 8) > 0.88 { 2 } else { 1 }
            } else {
                0
            }
        }
        Jar => {
            if rect(x, y, 4.0, 2.5, 12.0, 4.5) {
                2
            } else if rect(x, y, 3.5, 4.5, 12.5, 14.0) {
                if xi == 5 && yi < 11 { 3 } else { 1 }
            } else {
                0
            }
        }
        Pie => {
            if ell(x, y, 8.0, 8.0, 6.5, 6.5) {
                if !ell(x, y, 8.0, 8.0, 5.0, 5.0) { 2 } else if xi % 3 == 0 || yi % 3 == 0 { 2 } else { 1 }
            } else {
                0
            }
        }
        Cookie => {
            if ell(x, y, 8.0, 8.0, 5.5, 5.5) {
                if hsh(xi / 2, yi / 2, 9) > 0.75 { 2 } else { 1 }
            } else {
                0
            }
        }
        Cake => {
            let tri = y > 4.0 && y < 13.0 && x > 2.0 && x < 2.0 + (y - 4.0) * 1.4;
            if tri {
                if yi <= 5 || yi == 9 { 3 } else if yi == 7 || yi == 11 { 2 } else { 1 }
            } else {
                0
            }
        }
        Cheese => {
            let tri = y > 5.0 && y < 13.0 && x > 2.0 && x < 14.0 && y > 13.0 - (x - 2.0) * 0.7;
            if tri {
                if ell(x, y, 9.0, 10.0, 1.2, 1.2) || ell(x, y, 12.0, 11.0, 0.9, 0.9) { 2 } else { 1 }
            } else {
                0
            }
        }
        Soup => {
            if ell(x, y, 8.0, 7.5, 6.5, 2.0) {
                1
            } else if ell(x, y, 8.0, 7.5, 6.5, 6.0) && y > 7.5 {
                2
            } else {
                0
            }
        }
        Meat => {
            if ell(x, y, 7.0, 8.5, 5.5, 4.5) {
                if ell(x, y, 7.0, 8.5, 2.0, 1.5) { 3 } else { 1 }
            } else if seg(x, y, 11.0, 8.0, 14.0, 6.0).0 < 1.0 {
                2
            } else {
                0
            }
        }
        Drumstick => {
            if ell(x, y, 6.5, 6.5, 4.5, 4.0) {
                if ell(x, y, 5.0, 5.0, 1.2, 1.0) { 3 } else { 1 }
            } else if seg(x, y, 9.0, 9.0, 13.0, 13.0).0 < 0.9 || ell(x, y, 13.0, 13.5, 1.3, 1.3) {
                2
            } else {
                0
            }
        }
        Fish => {
            let tail = x > 11.0 && x < 15.0 && (y - 8.0).abs() < (x - 10.5) * 0.9;
            if ell(x, y, 7.0, 8.0, 5.5, 3.5) {
                if ell(x, y, 4.0, 7.0, 0.8, 0.8) { 2 } else if y < 7.0 { 3 } else { 1 }
            } else if tail {
                1
            } else {
                0
            }
        }
        Sushi => {
            if ell(x, y, 8.0, 8.0, 6.0, 6.0) {
                if !ell(x, y, 8.0, 8.0, 5.0, 5.0) { 2 } else if ell(x, y, 8.0, 8.0, 2.2, 2.2) { 1 } else { 5 }
            } else {
                0
            }
        }
        Coconut => {
            if ell(x, y, 8.0, 8.0, 5.8, 5.8) {
                if ell(x, y, 6.5, 6.5, 0.8, 0.8) || ell(x, y, 9.5, 6.5, 0.8, 0.8) || ell(x, y, 8.0, 9.0, 0.8, 0.8) { 2 } else { 1 }
            } else {
                0
            }
        }
        Melon => {
            if ell(x, y, 8.0, 5.0, 7.0, 8.5) && y > 5.0 {
                if !ell(x, y, 8.0, 5.0, 5.5, 7.0) { 2 } else if (xi + yi) % 5 == 0 && y > 7.0 { 4 } else { 1 }
            } else {
                0
            }
        }
        Roll => {
            if ell(x, y, 8.0, 8.0, 6.0, 6.0) {
                let r = ((x - 8.0).powi(2) + (y - 8.0).powi(2)).sqrt();
                let a = (y - 8.0).atan2(x - 8.0);
                if ((r - a * 0.9).rem_euclid(2.8)) < 1.0 { 2 } else { 1 }
            } else {
                0
            }
        }
        Skewer => {
            if ell(x, y, 5.5, 10.5, 2.0, 2.0) || ell(x, y, 8.0, 8.0, 2.0, 2.0) || ell(x, y, 10.5, 5.5, 2.0, 2.0) {
                if xi % 2 == 0 { 1 } else { 3 }
            } else if seg(x, y, 2.0, 14.0, 13.5, 2.5).0 < 0.6 {
                2
            } else {
                0
            }
        }
        Pancake => {
            for k in 0..3 {
                if ell(x, y, 8.0, 11.0 - k as f32 * 2.5, 6.0, 2.0) {
                    return if k == 2 && ell(x, y, 8.0, 5.5, 2.0, 0.8) { 2 } else { 1 };
                }
            }
            0
        }
        Star => {
            let (dx, dy) = (x - 8.0, y - 8.5);
            let a = dy.atan2(dx) + std::f32::consts::FRAC_PI_2;
            let r = (dx * dx + dy * dy).sqrt();
            let k = (a * 5.0 / std::f32::consts::TAU).fract();
            let lim = 3.0 + 3.6 * (1.0 - (k - 0.5).abs() * 2.0).powf(2.0);
            if r < lim {
                if dx < 0.0 && dy < 0.0 { 3 } else { 1 }
            } else {
                0
            }
        }
        IceCream => {
            if ell(x, y, 8.0, 5.5, 4.5, 4.0) {
                if ell(x, y, 6.5, 4.0, 1.0, 1.0) { 3 } else { 1 }
            } else if y > 7.5 && y < 15.0 && (x - 8.0).abs() < (15.0 - y) * 0.55 {
                2
            } else {
                0
            }
        }
        Mug => {
            if rect(x, y, 3.0, 4.0, 11.0, 14.0) {
                if yi <= 5 { 3 } else { 1 }
            } else if ell(x, y, 12.0, 9.0, 2.5, 3.0) && !ell(x, y, 12.0, 9.0, 1.2, 1.6) {
                2
            } else {
                0
            }
        }
        Horn => {
            let a = (yi as f32 / 15.0) * 2.2;
            let cx = 4.0 + a.sin() * 7.0;
            let w = 3.2 * (1.0 - y / 16.0) + 0.6;
            if (x - cx).abs() < w && y > 1.0 {
                if yi % 3 == 0 { 2 } else { 1 }
            } else {
                0
            }
        }
        Fang => {
            let inside = y > 2.0 && y < 14.0 && (x - 8.0).abs() < (14.0 - y) * 0.35 + 0.4;
            if inside { if x < 7.5 { 3 } else { 1 } } else { 0 }
        }
        Claw => {
            for k in 0..3 {
                let ox = 3.5 + k as f32 * 3.5;
                let t = ((y - 3.0) / 10.0).clamp(0.0, 1.0);
                let cx = ox + (1.0 - t).powi(2) * 2.5;
                if y > 3.0 && y < 13.0 && (x - cx).abs() < 0.6 + t * 0.9 {
                    return 1;
                }
            }
            if rect(x, y, 3.0, 12.5, 13.0, 14.5) { 2 } else { 0 }
        }
        Scale => {
            if ell(x, y, 8.0, 7.0, 6.0, 7.0) && y > 2.0 {
                if (y - 4.0).rem_euclid(3.0) < 0.9 { 2 } else if x < 6.0 && y < 7.0 { 3 } else { 1 }
            } else {
                0
            }
        }
        Eye => {
            if ell(x, y, 8.0, 8.0, 6.5, 4.5) {
                if ell(x, y, 8.0, 8.0, 1.4, 1.6) { 2 } else if ell(x, y, 8.0, 8.0, 3.2, 3.2) { 1 } else { 5 }
            } else {
                0
            }
        }
        Orb => {
            if ell(x, y, 8.0, 8.0, 5.8, 5.8) {
                if ell(x, y, 6.2, 6.2, 1.6, 1.6) { 3 } else if !ell(x, y, 8.0, 8.0, 4.4, 4.4) { 2 } else { 1 }
            } else {
                0
            }
        }
        Shell => {
            if ell(x, y, 8.0, 8.0, 6.0, 5.5) {
                let r = ((x - 8.0).powi(2) + (y - 8.0).powi(2)).sqrt();
                let a = (y - 8.0).atan2(x - 8.0);
                if ((r + a * 1.1).rem_euclid(3.0)) < 0.9 { 2 } else { 1 }
            } else {
                0
            }
        }
        Tail => {
            let cx = 8.0 + (y * 0.6).sin() * 3.5;
            let w = 1.0 + (y / 16.0) * 2.5;
            if (x - cx).abs() < w && y > 1.0 && y < 15.0 {
                if y > 11.0 { 2 } else { 1 }
            } else {
                0
            }
        }
        Wing => {
            let (dx, dy) = (x - 2.0, 14.0 - y);
            let r = (dx * dx + dy * dy).sqrt();
            let a = dy.atan2(dx);
            if r < 12.5 && a > 0.1 && a < 1.45 && r < 12.5 - ((a * 6.0).sin().abs() * 2.5) {
                if (r as i32) % 4 == 0 { 2 } else { 1 }
            } else {
                0
            }
        }
        Heart => {
            let (u, v) = ((x - 8.0) / 6.5, (9.0 - y) / 6.5);
            let f = (u * u + v * v - 0.3).powi(3) - u * u * v * v * v;
            if f <= 0.0 {
                if u < -0.3 && v > 0.2 { 3 } else { 1 }
            } else {
                0
            }
        }
        Mane => {
            for k in 0..5 {
                let bx = 3.0 + k as f32 * 2.5;
                let (d, _) = seg(x, y, bx, 14.0, bx + 1.5, 2.5 + (k % 2) as f32 * 2.0);
                if d < 1.0 {
                    return if k % 2 == 0 { 1 } else { 3 };
                }
            }
            0
        }
        Spore => {
            for k in 0..6 {
                let (cx, cy) = (3.0 + hsh(k, 1, 11) * 10.0, 3.0 + hsh(k, 2, 11) * 10.0);
                if ell(x, y, cx, cy, 1.6, 1.6) {
                    return if k % 2 == 0 { 1 } else { 2 };
                }
            }
            0
        }
    }
}

fn shade(c: Px, k: f32) -> Px {
    let f = |v: u8| (v as f32 * k).clamp(0.0, 255.0) as u8;
    [f(c[0]), f(c[1]), f(c[2]), c[3]]
}

/// Готовая функция плитки для атласа.
pub fn draw(icon: Icon, c1: Px, c2: Px) -> impl Fn(i32, i32) -> Px {
    move |x, y| {
        let r = region(icon, x, y);
        if r == 0 {
            // контур вокруг формы
            let near = [(1, 0), (-1, 0), (0, 1), (0, -1)].iter().any(|(dx, dy)| {
                let (nx, ny) = (x + dx, y + dy);
                (0..16).contains(&nx) && (0..16).contains(&ny) && region(icon, nx, ny) != 0
            });
            return if near { shade(c1, 0.35) } else { [0, 0, 0, 0] };
        }
        match r {
            1 => shade(c1, 0.92 + hsh(x, y, 13) * 0.12),
            2 => c2,
            3 => shade(c1, 1.35),
            4 => [235, 240, 248, 255],
            _ => [250, 250, 245, 255],
        }
    }
}
