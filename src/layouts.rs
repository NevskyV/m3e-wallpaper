use crate::canvas::{color_dist, Canvas, Contour, Rgb};
use crate::shapes::{circle, make_shape, rounded_rect, stadium, Pt, CONC, DECOR, HERO, INNER, N, SOFT4, TAU};
use std::cmp::Ordering;
use std::f64::consts::PI;

pub const LAYOUTS: [(&str, f64); 11] = [
    ("scatter", 3.0),
    ("hero", 3.0),
    ("grid", 2.0),
    ("waves", 2.0),
    ("concentric", 2.0),
    ("bento", 2.0),
    ("pills", 2.0),
    ("rays", 2.0),
    ("hills", 2.0),
    ("trail", 2.0),
    ("rosette", 2.0),
];

pub fn run(name: &str, cv: &mut Canvas) {
    match name {
        "hero" => scatter(cv, true),
        "grid" => grid(cv),
        "waves" => waves(cv),
        "concentric" => concentric(cv),
        "bento" => bento(cv),
        "pills" => pills(cv),
        "rays" => rays(cv),
        "hills" => hills(cv),
        "trail" => trail(cv),
        "rosette" => rosette(cv),
        _ => scatter(cv, false),
    }
}

/// Случайный цвет палитры, не входящий в `avoid` (если возможно).
fn pick_color(cv: &mut Canvas, avoid: &[Rgb]) -> Rgb {
    let mut list: Vec<Rgb> = cv.pool.iter().cloned().filter(|c| !avoid.contains(c)).collect();
    if list.is_empty() {
        list = cv.pool.clone();
    }
    let i = cv.rng.index(list.len());
    list[i]
}

/// 2–3 разных цвета палитры.
fn pick_cols(cv: &mut Canvas, lo: usize, hi: usize) -> Vec<Rgb> {
    let k = cv.rng.int(lo as i64, hi as i64) as usize;
    let pool = cv.pool.clone();
    cv.rng.sample(&pool, k.min(pool.len()))
}

/// «Солнце» позади волн/холмов.
fn sun(cv: &mut Canvas, lo: f64, hi: f64) {
    let w = cv.w as f64;
    let h = cv.h as f64;
    let m = w.min(h);
    let (_, pts) = make_shape(&mut cv.rng, Some(DECOR));
    let x = cv.rng.uniform(0.2, 0.8) * w;
    let y = cv.rng.uniform(0.15, 0.4) * h;
    let r = cv.rng.uniform(lo, hi) * m;
    let rot = cv.rng.uniform(0.0, TAU);
    let col = pick_color(cv, &[]);
    cv.shape(&pts, x, y, r, rot, col, None);
}

// ───────────────────────── scatter / hero ─────────────────────────

/// Крупные фигуры, частично вылезающие за края; в режиме hero — одна огромная.
pub fn scatter(cv: &mut Canvas, hero: bool) {
    let w = cv.w as f64;
    let h = cv.h as f64;
    let m = w.min(h);
    let t = if hero { cv.rng.int(5, 8) } else { cv.rng.int(8, 14) };
    let target = t as usize;
    let mut sizes: Vec<f64> = Vec::new();
    if hero {
        let s = cv.rng.uniform(0.55, 0.85) * m;
        sizes.push(s);
    }
    while sizes.len() < target {
        let s = cv.rng.uniform(0.09, 0.26) * m;
        sizes.push(s);
    }
    sizes.sort_by(|a, b| b.partial_cmp(a).unwrap_or(Ordering::Equal));

    let circle_pts = circle();
    let mut placed: Vec<(f64, f64, f64, Rgb)> = Vec::new();
    for (idx, &r) in sizes.iter().enumerate() {
        let mut best = (0.0f64, 0.0f64);
        let mut best_score = -1.0f64;
        for _ in 0..60 {
            let x = cv.rng.uniform(-0.05 * w, 1.05 * w);
            let y = cv.rng.uniform(-0.05 * h, 1.05 * h);
            let mut score = 9.0f64;
            for &(px, py, pr, _) in placed.iter() {
                let sc = (x - px).hypot(y - py) / (r + pr);
                if sc < score {
                    score = sc;
                }
            }
            if score > best_score {
                best = (x, y);
                best_score = score;
            }
        }
        let (x, y) = best;
        let near: Vec<Rgb> = placed
            .iter()
            .filter(|p| (x - p.0).hypot(y - p.1) < r + p.2)
            .map(|p| p.3)
            .collect();
        let col = pick_color(cv, &near);
        let big = hero && idx == 0;
        let (kind, pts) = if big {
            make_shape(&mut cv.rng, Some(HERO))
        } else {
            make_shape(&mut cv.rng, None)
        };
        let rot = cv.rng.uniform(0.0, TAU);
        cv.shape(&pts, x, y, r, rot, col, None);

        if DECOR.contains(&kind) && cv.rng.chance(0.4) {
            let dots: Vec<Rgb> = cv.pool.iter().cloned().filter(|c| color_dist(*c, col) > 40.0).collect();
            let dots = if dots.is_empty() { cv.pool.clone() } else { dots };
            let rr = r * cv.rng.uniform(0.14, 0.25);
            let di = cv.rng.index(dots.len());
            cv.shape(&circle_pts, x, y, rr, 0.0, dots[di], Some("solid"));
        }
        placed.push((x, y, r, col));
    }
}

// ───────────────────────── grid ─────────────────────────

/// Регулярный паттерн из фигур с чередованием цветов.
pub fn grid(cv: &mut Canvas) {
    let w = cv.w as f64;
    let h = cv.h as f64;
    let m = w.min(h);
    let cell = m / cv.rng.uniform(3.5, 7.0);
    let (_, shape_a) = make_shape(&mut cv.rng, None);
    let shape_b = if cv.rng.chance(0.5) {
        make_shape(&mut cv.rng, None).1
    } else {
        shape_a.clone()
    };
    let stagger = cv.rng.chance(0.6);
    let dy = if stagger { cell * 0.866 } else { cell };
    let cols = pick_cols(cv, 2, 3);
    let rot_mode = cv.rng.index(3);
    let base = cv.rng.uniform(0.0, TAU);
    let r = cell * cv.rng.uniform(0.40, 0.50);
    let ox = cv.rng.uniform(0.0, cell);
    let oy = cv.rng.uniform(0.0, dy);

    let rows = (h / dy) as usize + 3;
    let ncols = (w / cell) as usize + 3;
    for j in 0..rows {
        for i in 0..ncols {
            let shift = if stagger && j % 2 == 1 { cell / 2.0 } else { 0.0 };
            let x = (i as f64 - 1.0) * cell + ox + shift;
            let y = (j as f64 - 1.0) * dy + oy;
            let mut rot = base;
            if rot_mode == 1 {
                rot += j as f64 * 0.35;
            } else if rot_mode == 2 {
                rot += ((i + j) % 2) as f64 * PI / 4.0;
            }
            let pts = if (i + j) % 2 == 0 { &shape_a } else { &shape_b };
            cv.shape(pts, x, y, r, rot, cols[(i + j) % cols.len()], None);
        }
    }
}

// ───────────────────────── waves ─────────────────────────

/// Волнистые полосы с «солнцем» позади.
pub fn waves(cv: &mut Canvas) {
    let w = cv.w as f64;
    let h = cv.h as f64;
    if cv.rng.chance(0.8) {
        sun(cv, 0.22, 0.35);
    }
    let bands = cv.rng.int(4, 7) as usize;
    let slope = cv.rng.uniform(-0.2, 0.2);
    let mut prev: Vec<Rgb> = Vec::new();
    for i in 0..bands {
        let base = h * (0.30 + 0.75 * i as f64 / bands as f64);
        let amp = h * cv.rng.uniform(0.025, 0.07);
        let f1 = cv.rng.uniform(0.8, 2.2);
        let f2 = cv.rng.uniform(2.5, 5.0);
        let p1 = cv.rng.uniform(0.0, TAU);
        let p2 = cv.rng.uniform(0.0, TAU);
        let mut top: Vec<Pt> = Vec::with_capacity(364);
        for s in 0..=360 {
            let x = w * s as f64 / 360.0;
            let t = x / w;
            let y = base
                + slope * (x - w / 2.0)
                + amp * (TAU * f1 * t + p1).sin()
                + 0.35 * amp * (TAU * f2 * t + p2).sin();
            top.push((x, y));
        }
        top.push((w, 3.0 * h));
        top.push((0.0, 3.0 * h));
        let col = pick_color(cv, &prev);
        cv.polygon(&top, col, None, Contour::Shift);
        prev = vec![col];
    }
}

// ───────────────────────── concentric ─────────────────────────

/// Вложенные «закрученные» кольца одной формы.
pub fn concentric(cv: &mut Canvas) {
    let w = cv.w as f64;
    let h = cv.h as f64;
    let (_, pts) = make_shape(&mut cv.rng, Some(CONC));
    let cx = w * cv.rng.uniform(0.15, 0.85);
    let cy = h * cv.rng.uniform(0.15, 0.85);
    let mut rmax = 0.0f64;
    for &(x, y) in [(0.0, 0.0), (w, 0.0), (0.0, h), (w, h)].iter() {
        rmax = rmax.max((cx - x).hypot(cy - y));
    }
    rmax *= 1.15;
    let n = cv.rng.int(6, 11) as usize;
    let twist = cv.rng.uniform(-0.25, 0.25);
    let rot0 = cv.rng.uniform(0.0, TAU);
    let cols = pick_cols(cv, 2, 4);
    for i in 0..n {
        let r = rmax * (n - i) as f64 / n as f64;
        cv.shape(&pts, cx, cy, r, rot0 + i as f64 * twist, cols[i % cols.len()], None);
    }
}

// ───────────────────────── bento ─────────────────────────

type Rect = (f64, f64, f64, f64);

fn touch(a: &Rect, b: &Rect) -> bool {
    a.0 <= b.2 + 1.0 && b.0 <= a.2 + 1.0 && a.1 <= b.3 + 1.0 && b.1 <= a.3 + 1.0
}

/// Сетка скруглённых плиток; в части плиток лежат фигуры.
pub fn bento(cv: &mut Canvas) {
    let w = cv.w as f64;
    let h = cv.h as f64;
    let m = w.min(h);
    let gap = m * cv.rng.uniform(0.012, 0.03);
    let rad = m * cv.rng.uniform(0.025, 0.08);
    let mut cells: Vec<Rect> = vec![(0.0, 0.0, w, h)];
    let target = cv.rng.int(7, 14) as usize;
    while cells.len() < target {
        cells.sort_by(|a, b| {
            let aa = (a.2 - a.0) * (a.3 - a.1);
            let bb = (b.2 - b.0) * (b.3 - b.1);
            bb.partial_cmp(&aa).unwrap_or(Ordering::Equal)
        });
        let (x0, y0, x1, y1) = cells.remove(0);
        let cw = x1 - x0;
        let ch = y1 - y0;
        let t = cv.rng.uniform(0.35, 0.65);
        let ratio = cv.rng.uniform(0.8, 1.25);
        if cw > ch * ratio {
            cells.push((x0, y0, x0 + cw * t, y1));
            cells.push((x0 + cw * t, y0, x1, y1));
        } else {
            cells.push((x0, y0, x1, y0 + ch * t));
            cells.push((x0, y0 + ch * t, x1, y1));
        }
    }

    let mut drawn: Vec<(Rect, Rgb)> = Vec::new();
    for &c in cells.iter() {
        let (x0, y0, x1, y1) = c;
        // у краёв холста плитки «вылезают» наружу, чтобы скругления были только внутри
        let bx0 = if x0 <= 0.0 { -rad } else { x0 + gap / 2.0 };
        let by0 = if y0 <= 0.0 { -rad } else { y0 + gap / 2.0 };
        let bx1 = if x1 >= w { w + rad } else { x1 - gap / 2.0 };
        let by1 = if y1 >= h { h + rad } else { y1 - gap / 2.0 };
        let near: Vec<Rgb> = drawn.iter().filter(|d| touch(&c, &d.0)).map(|d| d.1).collect();
        let col = pick_color(cv, &near);
        let rr = rad.min((bx1 - bx0) / 2.0).min((by1 - by0) / 2.0);
        let poly = rounded_rect(bx0, by0, bx1, by1, rr);
        cv.polygon(&poly, col, None, Contour::Scale);
        drawn.push((c, col));

        if cv.rng.chance(0.55) {
            let (_, sp) = make_shape(&mut cv.rng, Some(INNER));
            let size = (x1 - x0).min(y1 - y0) * cv.rng.uniform(0.26, 0.42);
            let rot = cv.rng.uniform(0.0, TAU);
            let c2 = pick_color(cv, &[col]);
            cv.shape(&sp, (x0 + x1) / 2.0, (y0 + y1) / 2.0, size, rot, c2, None);
        }
    }
}

// ───────────────────────── pills ─────────────────────────

/// Наклонённые ряды таблеток разной длины.
pub fn pills(cv: &mut Canvas) {
    let w = cv.w as f64;
    let h = cv.h as f64;
    let d = w.hypot(h);
    let ang = if cv.rng.chance(0.5) { 0.0 } else { cv.rng.uniform(-0.6, 0.6) };
    let (ca, sa) = (ang.cos(), ang.sin());
    let rows = cv.rng.int(6, 11) as usize;
    let rh = d / rows as f64;
    let t = rh * cv.rng.uniform(0.62, 0.86);
    for j in 0..rows {
        let v = -d / 2.0 + rh * (j as f64 + 0.5);
        let start = cv.rng.uniform(0.0, rh * 2.0);
        let mut u = -d / 2.0 - start;
        let mut prev: Vec<Rgb> = Vec::new();
        while u < d / 2.0 {
            let l = rh * cv.rng.uniform(1.0, 4.2);
            let col = pick_color(cv, &prev);
            prev = vec![col];
            let uc = u + l / 2.0;
            let base = stadium(l.max(t), t);
            let pts: Vec<Pt> = base
                .iter()
                .map(|&(x, y)| {
                    (
                        w / 2.0 + (uc + x) * ca - (v + y) * sa,
                        h / 2.0 + (uc + x) * sa + (v + y) * ca,
                    )
                })
                .collect();
            cv.polygon(&pts, col, None, Contour::Scale);
            let gap = rh * cv.rng.uniform(0.12, 0.4);
            u += l + gap;
        }
    }
}

// ───────────────────────── rays ─────────────────────────

/// Лучи из одной точки (иногда закрученные и волнистые) + фигура в центре.
pub fn rays(cv: &mut Canvas) {
    let w = cv.w as f64;
    let h = cv.h as f64;
    let m = w.min(h);
    let cx = w * cv.rng.uniform(0.25, 0.75);
    let cy = h * cv.rng.uniform(0.25, 0.75);
    let rr = w.hypot(h) * 1.25;
    let ns: [usize; 6] = [8, 10, 12, 14, 16, 20];
    let n = ns[cv.rng.index(6)];
    let cols = pick_cols(cv, 2, 3);
    let rot0 = cv.rng.uniform(0.0, TAU);
    let twist = if cv.rng.chance(0.5) { 0.0 } else { cv.rng.uniform(-1.3, 1.3) };
    let wave = if cv.rng.chance(0.5) { 0.0 } else { cv.rng.uniform(0.02, 0.06) };
    let lam = cv.rng.uniform(0.15, 0.35) * rr;
    let ph = cv.rng.uniform(0.0, TAU);
    let skip = cv.rng.chance(0.5);
    let seg = TAU / n as f64;

    let edge = |a: f64, d: f64| -> f64 { a + twist * d / rr + wave * (TAU * d / lam + ph).sin() };
    let ds: Vec<f64> = (0..=40).map(|i| rr * i as f64 / 40.0).collect();

    for i in 0..n {
        if skip && i % 2 == 1 {
            continue;
        }
        let a0 = rot0 + i as f64 * seg;
        let a1 = rot0 + (i + 1) as f64 * seg;
        let mut pts: Vec<Pt> = vec![(cx, cy)];
        for &d in ds.iter().skip(1) {
            let a = edge(a0, d);
            pts.push((cx + d * a.cos(), cy + d * a.sin()));
        }
        for &d in ds.iter().skip(1).rev() {
            let a = edge(a1, d);
            pts.push((cx + d * a.cos(), cy + d * a.sin()));
        }
        cv.polygon(&pts, cols[i % cols.len()], None, Contour::Line);
    }

    let (_, sp) = make_shape(&mut cv.rng, Some(SOFT4));
    let col = pick_color(cv, &cols[..1]);
    let size = m * cv.rng.uniform(0.12, 0.22);
    let rot = cv.rng.uniform(0.0, TAU);
    cv.shape(&sp, cx, cy, size, rot, col, None);
}

// ───────────────────────── hills ─────────────────────────

/// Слои скруглённых холмов снизу, позади — «солнце».
pub fn hills(cv: &mut Canvas) {
    let w = cv.w as f64;
    let h = cv.h as f64;
    if cv.rng.chance(0.8) {
        sun(cv, 0.14, 0.26);
    }
    let n = cv.rng.int(3, 6) as usize;
    let mut prev: Vec<Rgb> = Vec::new();
    for i in 0..n {
        let top = h * (0.32 + 0.58 * i as f64 / n as f64);
        let rx = w * cv.rng.uniform(0.35, 0.8);
        let ry = h * cv.rng.uniform(0.22, 0.5);
        let cx = cv.rng.uniform(0.0, w);
        let cy = top + ry;
        let ks = [2.0f64, 2.0, 3.0, 4.0];
        let k = ks[cv.rng.index(4)];
        let e = 2.0 / k;
        let mut pts: Vec<Pt> = Vec::with_capacity(N);
        for j in 0..N {
            let t = TAU * j as f64 / N as f64;
            let c = t.cos();
            let s = t.sin();
            pts.push((cx + rx * c.abs().powf(e).copysign(c), cy + ry * s.abs().powf(e).copysign(s)));
        }
        let col = pick_color(cv, &prev);
        cv.polygon(&pts, col, None, Contour::Shift);
        prev = vec![col];
    }
}

// ───────────────────────── trail ─────────────────────────

/// Цепочка одинаковых фигур вдоль волнистой траектории с изменением размера.
pub fn trail(cv: &mut Canvas) {
    let w = cv.w as f64;
    let h = cv.h as f64;
    let m = w.min(h);
    let (_, pts) = make_shape(&mut cv.rng, Some(INNER));
    let n = cv.rng.int(9, 16) as usize;
    let mut a = (cv.rng.uniform(-0.05, 0.1) * w, cv.rng.uniform(0.1, 0.9) * h);
    let mut b = (cv.rng.uniform(0.9, 1.05) * w, cv.rng.uniform(0.1, 0.9) * h);
    if cv.rng.chance(0.5) {
        a = (cv.rng.uniform(0.1, 0.9) * w, -0.05 * h);
        b = (cv.rng.uniform(0.1, 0.9) * w, 1.05 * h);
    }
    let l = (b.0 - a.0).hypot(b.1 - a.1).max(1.0);
    let (nx, ny) = (-(b.1 - a.1) / l, (b.0 - a.0) / l);
    let amp = m * cv.rng.uniform(0.15, 0.3);
    let freq = cv.rng.uniform(0.8, 1.8);
    let ph = cv.rng.uniform(0.0, TAU);
    let mode = cv.rng.index(3);
    let cols = pick_cols(cv, 2, 3);
    let spin = cv.rng.uniform(0.1, 0.5);

    let mut items: Vec<(f64, f64, f64, usize)> = Vec::new();
    for i in 0..n {
        let t = i as f64 / (n - 1) as f64;
        let s = (TAU * freq * t + ph).sin();
        let x = a.0 + (b.0 - a.0) * t + nx * amp * s;
        let y = a.1 + (b.1 - a.1) * t + ny * amp * s;
        let f = match mode {
            0 => t,
            1 => 1.0 - t,
            _ => (PI * t).sin(),
        };
        items.push((m * (0.05 + 0.2 * f), x, y, i));
    }
    // крупные — снизу
    items.sort_by(|p, q| q.0.partial_cmp(&p.0).unwrap_or(Ordering::Equal));
    for &(r, x, y, i) in items.iter() {
        cv.shape(&pts, x, y, r, i as f64 * spin, cols[i % cols.len()], None);
    }
}

// ───────────────────────── rosette ─────────────────────────

/// Кольца из фигур вокруг крупной центральной.
pub fn rosette(cv: &mut Canvas) {
    let w = cv.w as f64;
    let h = cv.h as f64;
    let m = w.min(h);
    let cx = w * cv.rng.uniform(0.3, 0.7);
    let cy = h * cv.rng.uniform(0.3, 0.7);
    let rb = m * cv.rng.uniform(0.22, 0.36);
    let n = cv.rng.int(6, 11) as usize;
    let (_, pts) = make_shape(&mut cv.rng, Some(INNER));
    let cols = pick_cols(cv, 2, 3);
    let rings_opts: [usize; 4] = [1, 2, 2, 3];
    let rings = rings_opts[cv.rng.index(4)];
    let twist = cv.rng.uniform(0.0, 0.5);

    for k in (0..rings).rev() {
        let rk = rb * (1.0 + 0.95 * k as f64);
        let nk = n * (k + 1);
        let jitter = cv.rng.uniform(0.9, 1.15);
        let r = rk * (PI / nk as f64).sin() * jitter;
        for i in 0..nk {
            let a = TAU * i as f64 / nk as f64 + k as f64 * twist;
            cv.shape(&pts, cx + rk * a.cos(), cy + rk * a.sin(), r, a, cols[(i + k) % cols.len()], None);
        }
    }
    let (_, cp) = make_shape(&mut cv.rng, Some(SOFT4));
    let col = pick_color(cv, &cols[..1]);
    let size = rb * cv.rng.uniform(0.55, 0.8);
    let rot = cv.rng.uniform(0.0, TAU);
    cv.shape(&cp, cx, cy, size, rot, col, None);
}
