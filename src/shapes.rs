use crate::rng::Rng;
use std::f64::consts::PI;

pub type Pt = (f64, f64);

pub const TAU: f64 = std::f64::consts::TAU;
pub const N: usize = 360;

pub const DECOR: &[&str] = &["cookie", "flower", "burst"];
pub const SOFT4: &[&str] = &["cookie", "flower", "burst", "circle"];
pub const HERO: &[&str] = &["cookie", "flower", "burst", "squircle", "circle"];
pub const INNER: &[&str] = &["cookie", "flower", "burst", "squircle", "circle", "ngon"];
pub const CONC: &[&str] = &["cookie", "flower", "burst", "squircle", "circle"];

fn normalize(mut pts: Vec<Pt>, center: bool) -> Vec<Pt> {
    if center {
        let (mut minx, mut maxx, mut miny, mut maxy) = (f64::MAX, f64::MIN, f64::MAX, f64::MIN);
        for &(x, y) in &pts {
            minx = minx.min(x);
            maxx = maxx.max(x);
            miny = miny.min(y);
            maxy = maxy.max(y);
        }
        let mx = (minx + maxx) / 2.0;
        let my = (miny + maxy) / 2.0;
        pts = pts.into_iter().map(|(x, y)| (x - mx, y - my)).collect();
    }
    let mut m = 0.0f64;
    for &(x, y) in &pts {
        m = m.max(x.hypot(y));
    }
    if m <= 0.0 {
        m = 1.0;
    }
    pts.into_iter().map(|(x, y)| (x / m, y / m)).collect()
}

fn polar<F: Fn(f64) -> f64>(f: F) -> Vec<Pt> {
    let mut pts = Vec::with_capacity(N);
    for i in 0..N {
        let t = TAU * i as f64 / N as f64;
        let r = f(t);
        pts.push((r * t.cos(), r * t.sin()));
    }
    normalize(pts, false)
}

/// Круговое скользящее среднее с окном (2w+1).
pub fn smooth(vals: &[f64], w: usize) -> Vec<f64> {
    let n = vals.len() as i64;
    let wi = w as i64;
    let mut out = Vec::with_capacity(vals.len());
    for i in 0..n {
        let mut s = 0.0;
        for j in -wi..=wi {
            let idx = (((i + j) % n) + n) % n;
            s += vals[idx as usize];
        }
        out.push(s / (2 * w + 1) as f64);
    }
    out
}

/// Срезание углов (алгоритм Чайкина).
fn chaikin(mut pts: Vec<Pt>, iterations: usize) -> Vec<Pt> {
    for _ in 0..iterations {
        let n = pts.len();
        let mut new_pts = Vec::with_capacity(n * 2);
        for i in 0..n {
            let p = pts[i];
            let q = pts[(i + 1) % n];
            new_pts.push((0.75 * p.0 + 0.25 * q.0, 0.75 * p.1 + 0.25 * q.1));
            new_pts.push((0.25 * p.0 + 0.75 * q.0, 0.25 * p.1 + 0.75 * q.1));
        }
        pts = new_pts;
    }
    pts
}

pub fn circle() -> Vec<Pt> {
    polar(|_t| 1.0)
}

pub fn cookie(rng: &mut Rng) -> Vec<Pt> {
    let ns = [6.0f64, 8.0, 9.0, 10.0, 12.0];
    let n = ns[rng.index(ns.len())];
    let a = rng.uniform(0.04, 0.09);
    polar(move |t| 1.0 + a * (n * t).cos())
}

pub fn flower(rng: &mut Rng) -> Vec<Pt> {
    let ns = [4.0f64, 5.0, 6.0, 7.0, 8.0];
    let n = ns[rng.index(ns.len())];
    let a = rng.uniform(0.18, 0.32);
    polar(move |t| 1.0 + a * (n * t).cos())
}

pub fn burst(rng: &mut Rng) -> Vec<Pt> {
    let n = rng.int(8, 12) as f64;
    let a = rng.uniform(0.06, 0.11);
    let p = rng.uniform(0.6, 0.85);
    polar(move |t| 1.0 + a * (1.0 - 2.0 * (n * t / 2.0).sin().abs().powf(p)))
}

pub fn squircle(rng: &mut Rng) -> Vec<Pt> {
    let k = rng.uniform(3.0, 5.0);
    let e = 2.0 / k;
    let mut pts = Vec::with_capacity(N);
    for i in 0..N {
        let t = TAU * i as f64 / N as f64;
        let c = t.cos();
        let s = t.sin();
        pts.push((c.abs().powf(e).copysign(c), s.abs().powf(e).copysign(s)));
    }
    normalize(pts, false)
}

pub fn ngon(rng: &mut Rng) -> Vec<Pt> {
    let sides_list = [3.0f64, 5.0, 6.0];
    let sides = sides_list[rng.index(3)];
    let seg = TAU / sides;
    let mut raw = Vec::with_capacity(N);
    for i in 0..N {
        let t = TAU * i as f64 / N as f64;
        raw.push((PI / sides).cos() / ((t % seg) - PI / sides).cos());
    }
    let w = rng.int(22, 38) as usize;
    let r = smooth(&raw, w);
    let mut pts = Vec::with_capacity(N);
    for i in 0..N {
        let t = TAU * i as f64 / N as f64;
        pts.push((r[i] * t.cos(), r[i] * t.sin()));
    }
    normalize(pts, false)
}

pub fn pill(rng: &mut Rng) -> Vec<Pt> {
    let h = rng.uniform(0.8, 1.8);
    let mut pts = Vec::with_capacity(182);
    for i in 0..=90 {
        let a = -PI / 2.0 + PI * i as f64 / 90.0;
        pts.push((h + a.cos(), a.sin()));
    }
    for i in 0..=90 {
        let a = PI / 2.0 + PI * i as f64 / 90.0;
        pts.push((-h + a.cos(), a.sin()));
    }
    normalize(pts, false)
}

pub fn arch(rng: &mut Rng) -> Vec<Pt> {
    let mut pts: Vec<Pt> = (0..=30)
        .map(|i| {
            let a = PI * i as f64 / 30.0;
            (a.cos(), -a.sin())
        })
        .collect();
    let y = rng.uniform(0.9, 1.5);
    pts.push((-1.0, y));
    pts.push((1.0, y));
    normalize(chaikin(pts, 3), true)
}

/// Таблетка заданной длины и толщины с центром в нуле (в пикселях, не нормализована).
pub fn stadium(length: f64, thickness: f64) -> Vec<Pt> {
    let r = thickness / 2.0;
    let h = ((length - thickness) / 2.0).max(0.0);
    let seg = 18;
    let mut pts = Vec::with_capacity(2 * (seg + 1));
    for i in 0..=seg {
        let a = -PI / 2.0 + PI * i as f64 / seg as f64;
        pts.push((h + r * a.cos(), r * a.sin()));
    }
    for i in 0..=seg {
        let a = PI / 2.0 + PI * i as f64 / seg as f64;
        pts.push((-h + r * a.cos(), r * a.sin()));
    }
    pts
}

/// Скруглённый прямоугольник (в пикселях).
pub fn rounded_rect(x0: f64, y0: f64, x1: f64, y1: f64, r: f64) -> Vec<Pt> {
    let seg = 14;
    let mut pts = Vec::with_capacity(4 * (seg + 1));
    let corners = [
        (x1 - r, y0 + r, -90.0f64),
        (x1 - r, y1 - r, 0.0),
        (x0 + r, y1 - r, 90.0),
        (x0 + r, y0 + r, 180.0),
    ];
    for &(cx, cy, a0) in corners.iter() {
        for i in 0..=seg {
            let a = (a0 + 90.0 * i as f64 / seg as f64).to_radians();
            pts.push((cx + r * a.cos(), cy + r * a.sin()));
        }
    }
    pts
}

const KINDS: [(&str, f64); 8] = [
    ("cookie", 3.0),
    ("flower", 3.0),
    ("burst", 2.0),
    ("squircle", 2.0),
    ("ngon", 2.0),
    ("pill", 1.0),
    ("arch", 1.0),
    ("circle", 1.0),
];

/// Случайная форма; `kinds` ограничивает набор допустимых видов.
pub fn make_shape(rng: &mut Rng, kinds: Option<&[&str]>) -> (&'static str, Vec<Pt>) {
    let mut names: Vec<&'static str> = Vec::new();
    let mut weights: Vec<f64> = Vec::new();
    for &(k, w) in KINDS.iter() {
        let ok = match kinds {
            Some(list) => list.contains(&k),
            None => true,
        };
        if ok {
            names.push(k);
            weights.push(w);
        }
    }
    if names.is_empty() {
        return ("circle", circle());
    }
    let i = rng.weighted(&weights);
    let kind = names[i];
    let pts = match kind {
        "cookie" => cookie(rng),
        "flower" => flower(rng),
        "burst" => burst(rng),
        "squircle" => squircle(rng),
        "ngon" => ngon(rng),
        "pill" => pill(rng),
        "arch" => arch(rng),
        _ => circle(),
    };
    (kind, pts)
}
