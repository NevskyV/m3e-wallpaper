use crate::mask::Mask;
use crate::rng::Rng;
use crate::shapes::{smooth, Pt};
use std::f64::consts::PI;

pub type Rgb = (u8, u8, u8);

/// Как рисовать стиль «contour» для полигонов:
/// Shift — копии со сдвигом вниз (волны), Scale — вложенные уменьшенные копии, Line — одна линия.
#[derive(Clone, Copy, PartialEq)]
pub enum Contour {
    Shift,
    Scale,
    Line,
}

pub fn color_dist(a: Rgb, b: Rgb) -> f64 {
    let dr = a.0 as f64 - b.0 as f64;
    let dg = a.1 as f64 - b.1 as f64;
    let db = a.2 as f64 - b.2 as f64;
    (dr * dr + dg * dg + db * db).sqrt()
}

pub fn mix(a: Rgb, b: Rgb, t: f64) -> Rgb {
    let f = |x: u8, y: u8| -> u8 { (x as f64 + (y as f64 - x as f64) * t).round().clamp(0.0, 255.0) as u8 };
    (f(a.0, b.0), f(a.1, b.1), f(a.2, b.2))
}

/// Неровный «рукотворный» край: смещаем точки контура вдоль нормали сглаженным шумом.
fn roughen(p: &[Pt], amp: f64, rng: &mut Rng) -> Vec<Pt> {
    let n = p.len();
    if n < 3 {
        return p.to_vec();
    }
    let mut noise = |w: usize| -> Vec<f64> {
        let raw: Vec<f64> = (0..n).map(|_| rng.uniform(-1.0, 1.0)).collect();
        let v = smooth(&raw, w);
        let mut m = 0.0f64;
        for &x in &v {
            m = m.max(x.abs());
        }
        if m <= 0.0 {
            m = 1.0;
        }
        v.into_iter().map(|x| x / m).collect()
    };
    let lo = noise((n / 10).max(2));
    let hi = noise((n / 45).max(2));
    let mut out = Vec::with_capacity(n);
    for i in 0..n {
        let a = p[(i + n - 1) % n];
        let b = p[(i + 1) % n];
        let tx = b.0 - a.0;
        let ty = b.1 - a.1;
        let ln = tx.hypot(ty).max(1e-9);
        let off = amp * (0.6 * lo[i] + 0.4 * hi[i]);
        out.push((p[i].0 + ty / ln * off, p[i].1 - tx / ln * off));
    }
    out
}

pub struct Canvas {
    pub w: usize,
    pub h: usize,
    pub rng: Rng,
    pub pool: Vec<Rgb>,
    ss: usize,
    pw: usize,
    ph: usize,
    img: Vec<u8>,
    bg: Rgb,
    style: String,
    opacity: Option<f64>,
    thick: f64,
    occlude: bool,
    pstep: f64,
    pangle: f64,
    pthick: f64,
    halftone: bool,
    daub_amp: f64,
    daub_angle: f64,
    strong_map: Vec<(Rgb, Rgb)>,
}

impl Canvas {
    pub fn new(
        w: usize,
        h: usize,
        bg: Rgb,
        ss: usize,
        mut rng: Rng,
        pool: Vec<Rgb>,
        style: String,
        opacity: Option<f64>,
    ) -> Canvas {
        let pw = w * ss;
        let ph = h * ss;
        let mut img = vec![0u8; pw * ph * 3];
        for px in img.chunks_exact_mut(3) {
            px[0] = bg.0;
            px[1] = bg.1;
            px[2] = bg.2;
        }
        let m = w.min(h) as f64;
        let ssf = ss as f64;
        // глобальные параметры «руки» для этих обоев
        let thick = if rng.chance(0.5) {
            m * rng.uniform(0.004, 0.010)
        } else {
            m * rng.uniform(0.012, 0.026)
        };
        let occlude = rng.chance(0.5);
        let pstep = m * ssf * rng.uniform(0.011, 0.024);
        let angs = [PI / 4.0, -PI / 4.0, 0.0, PI / 2.0];
        let k = rng.index(5);
        let pangle = if k < 4 { angs[k] } else { rng.uniform(0.0, PI) };
        let pthick = rng.uniform(0.28, 0.5);
        let halftone = rng.chance(0.5);
        let daub_amp = m * ssf * rng.uniform(0.004, 0.010);
        let daub_angle = rng.uniform(0.0, PI);
        Canvas {
            w,
            h,
            rng,
            pool,
            ss,
            pw,
            ph,
            img,
            bg,
            style,
            opacity,
            thick,
            occlude,
            pstep,
            pangle,
            pthick,
            halftone,
            daub_amp,
            daub_angle,
            strong_map: Vec::new(),
        }
    }

    // ───────── цвета ─────────

    /// Для линий нужен хороший контраст с фоном; подбираем и запоминаем замену.
    fn strong(&mut self, c: Rgb) -> Rgb {
        for &(k, v) in self.strong_map.iter() {
            if k == c {
                return v;
            }
        }
        let bg = self.bg;
        let r = if color_dist(c, bg) >= 100.0 {
            c
        } else {
            let far: Vec<Rgb> = self.pool.iter().cloned().filter(|p| color_dist(*p, bg) >= 100.0).collect();
            if far.is_empty() {
                let mut best = self.pool[0];
                let mut bd = -1.0f64;
                for &p in self.pool.iter() {
                    let d = color_dist(p, bg);
                    if d > bd {
                        bd = d;
                        best = p;
                    }
                }
                best
            } else {
                let i = self.rng.index(far.len());
                far[i]
            }
        };
        self.strong_map.push((c, r));
        r
    }

    fn alt(&mut self, c: Rgb) -> Rgb {
        let bg = self.bg;
        let cands: Vec<Rgb> = self
            .pool
            .iter()
            .cloned()
            .filter(|p| color_dist(*p, c) > 50.0 && color_dist(*p, bg) > 38.0)
            .collect();
        let list = if cands.is_empty() { self.pool.clone() } else { cands };
        let i = self.rng.index(list.len());
        list[i]
    }

    fn resolve(&mut self) -> String {
        if self.style == "mixed" {
            let opts = ["solid", "outline", "striped", "dotted", "glass", "duotone"];
            let i = self.rng.index(opts.len());
            opts[i].to_string()
        } else {
            self.style.clone()
        }
    }

    // ───────── низкоуровневые операции с масками ─────────

    fn paste(&mut self, color: Rgb, x0: usize, y0: usize, mask: &Mask, alpha: f64) {
        let a256 = (alpha.clamp(0.0, 1.0) * 256.0) as u32;
        if a256 == 0 {
            return;
        }
        let c = [color.0 as i32, color.1 as i32, color.2 as i32];
        for y in 0..mask.h {
            let dy = y0 + y;
            if dy >= self.ph {
                break;
            }
            for x in 0..mask.w {
                let dx = x0 + x;
                if dx >= self.pw {
                    break;
                }
                let m = mask.d[y * mask.w + x] as u32;
                if m == 0 {
                    continue;
                }
                let a = ((m * a256) >> 8) as i32;
                if a == 0 {
                    continue;
                }
                let o = (dy * self.pw + dx) * 3;
                for k in 0..3 {
                    let cur = self.img[o + k] as i32;
                    self.img[o + k] = (cur + (c[k] - cur) * a / 255) as u8;
                }
            }
        }
    }

    fn stroke_mask(&self, q: &[Pt], bw: usize, bh: usize) -> Mask {
        let mut m = Mask::new(bw, bh);
        m.stroke_closed(q, (self.thick * self.ss as f64).max(2.0), 255);
        m
    }

    /// Копия маски, сдвинутая внутрь на ~d px, с плавно скруглёнными углами.
    fn inset(&self, fill: &Mask, d: f64) -> Mask {
        let mut m = fill.blurred((d / 0.77).max(1.0));
        m.apply_lut(&Mask::ramp_table(190, 215));
        m
    }

    /// Скругляет концы и углы тонких линий (размытие + порог).
    fn round_mask(&self, m: &Mask, sigma: f64) -> Mask {
        if sigma < 0.7 {
            return m.clone();
        }
        let mut r = m.blurred(sigma);
        r.apply_lut(&Mask::ramp_table(100, 156));
        r
    }

    /// Штриховка в мировых координатах — узор непрерывен между фигурами.
    fn stripes(&self, x0: usize, y0: usize, bw: usize, bh: usize) -> Mask {
        let mut m = Mask::new(bw, bh);
        let step = self.pstep;
        let ang = self.pangle;
        let (dx, dy) = (ang.cos(), ang.sin());
        let (nx, ny) = (-dy, dx);
        let (fx0, fy0) = (x0 as f64, y0 as f64);
        let (fw, fh) = (bw as f64, bh as f64);
        let corners = [(fx0, fy0), (fx0 + fw, fy0), (fx0, fy0 + fh), (fx0 + fw, fy0 + fh)];
        let (mut tmin, mut tmax, mut umin, mut umax) = (f64::MAX, f64::MIN, f64::MAX, f64::MIN);
        for &(cx, cy) in corners.iter() {
            let t = cx * nx + cy * ny;
            let u = cx * dx + cy * dy;
            tmin = tmin.min(t);
            tmax = tmax.max(t);
            umin = umin.min(u);
            umax = umax.max(u);
        }
        let w = (step * self.pthick).max(1.0);
        let u0 = umin - step;
        let u1 = umax + step;
        let mut t = (tmin / step).floor() * step;
        while t <= tmax + step {
            let a = (t * nx + u0 * dx - fx0, t * ny + u0 * dy - fy0);
            let b = (t * nx + u1 * dx - fx0, t * ny + u1 * dy - fy0);
            m.line_round(a, b, w, 255);
            t += step;
        }
        m
    }

    /// Точки; у края фигуры они плавно уменьшаются до нуля, а не обрезаются.
    fn dots(&self, x0: usize, y0: usize, bw: usize, bh: usize, fill: &Mask) -> Mask {
        let step = self.pstep;
        let ang = self.pangle;
        let blurred = fill.blurred(step * 0.9);
        let mut m = Mask::new(bw, bh);
        let (dx, dy) = (ang.cos(), ang.sin());
        let (nx, ny) = (-dy, dx);
        let (fx0, fy0) = (x0 as f64, y0 as f64);
        let (fw, fh) = (bw as f64, bh as f64);
        let corners = [(fx0, fy0), (fx0 + fw, fy0), (fx0, fy0 + fh), (fx0 + fw, fy0 + fh)];
        let (mut tmin, mut tmax, mut umin, mut umax) = (f64::MAX, f64::MIN, f64::MAX, f64::MIN);
        for &(cx, cy) in corners.iter() {
            let t = cx * nx + cy * ny;
            let u = cx * dx + cy * dy;
            tmin = tmin.min(t);
            tmax = tmax.max(t);
            umin = umin.min(u);
            umax = umax.max(u);
        }
        let mut t = (tmin / step).floor() * step;
        let mut row = 0i32;
        while t <= tmax + step {
            let shift = if row % 2 == 1 { step / 2.0 } else { 0.0 };
            let mut u = (umin / step).floor() * step + shift;
            while u <= umax + step {
                let cx = t * nx + u * dx - fx0;
                let cy = t * ny + u * dy - fy0;
                if cx >= 0.0 && cy >= 0.0 && (cx as usize) < bw && (cy as usize) < bh {
                    let v = blurred.d[(cy as usize) * bw + (cx as usize)] as f64 / 255.0;
                    let inside = ((v - 0.5) * 2.0).max(0.0); // 0 на краю -> 1 внутри
                    let f = if self.halftone { (u - umin) / (umax - umin).max(1e-9) } else { 0.6 };
                    let r = step * (0.10 + 0.38 * f) * (inside * 1.6).min(1.0);
                    if r >= 0.8 {
                        m.fill_disc(cx, cy, r + 0.5, 255);
                    }
                }
                u += step;
            }
            t += step;
            row += 1;
        }
        m
    }

    // ───────── рисование полигона в нужном стиле ─────────

    fn paint(&mut self, poly: &[Pt], color: Rgb, style: &str) {
        let s = self.ss as f64;
        let p: Vec<Pt> = poly.iter().map(|&(x, y)| (x * s, y * s)).collect();
        if p.len() < 3 {
            return;
        }
        let mut pad = (self.thick * s) as i64 + 6;
        if style == "daub" {
            pad += (self.daub_amp * 4.0) as i64;
        }
        let (mut minx, mut miny, mut maxx, mut maxy) = (f64::MAX, f64::MAX, f64::MIN, f64::MIN);
        for &(x, y) in p.iter() {
            minx = minx.min(x);
            miny = miny.min(y);
            maxx = maxx.max(x);
            maxy = maxy.max(y);
        }
        let x0 = ((minx.floor() as i64) - pad).max(0);
        let y0 = ((miny.floor() as i64) - pad).max(0);
        let x1 = ((maxx.floor() as i64) + pad).min(self.pw as i64);
        let y1 = ((maxy.floor() as i64) + pad).min(self.ph as i64);
        if x1 <= x0 || y1 <= y0 {
            return;
        }
        let bw = (x1 - x0) as usize;
        let bh = (y1 - y0) as usize;
        let ux = x0 as usize;
        let uy = y0 as usize;
        let q: Vec<Pt> = p.iter().map(|&(x, y)| (x - x0 as f64, y - y0 as f64)).collect();

        if style == "daub" {
            self.daub(&q, color, ux, uy, bw, bh);
            return;
        }

        let mut fill = Mask::new(bw, bh);
        fill.fill_polygon(&q, 255);
        let op = self.opacity;

        match style {
            "solid" | "glass" => {
                let a = match op {
                    Some(o) => o,
                    None => {
                        if style == "solid" {
                            1.0
                        } else {
                            self.rng.uniform(0.5, 0.78)
                        }
                    }
                };
                self.paste(color, ux, uy, &fill, a);
            }
            "outline" | "line" => {
                if style == "outline" && self.occlude {
                    let bg = self.bg;
                    self.paste(bg, ux, uy, &fill, 1.0);
                }
                let col = self.strong(color);
                let sm = self.stroke_mask(&q, bw, bh);
                self.paste(col, ux, uy, &sm, 1.0);
            }
            "duotone" => {
                self.paste(color, ux, uy, &fill, op.unwrap_or(1.0));
                let col = self.alt(color);
                let sm = self.stroke_mask(&q, bw, bh);
                self.paste(col, ux, uy, &sm, 1.0);
            }
            "ghost" => {
                self.paste(color, ux, uy, &fill, op.unwrap_or(0.2));
                let col = self.strong(color);
                let sm = self.stroke_mask(&q, bw, bh);
                self.paste(col, ux, uy, &sm, 1.0);
            }
            "striped" | "dotted" => {
                if self.rng.chance(0.6) {
                    self.paste(color, ux, uy, &fill, 0.18);
                }
                let pat = if style == "striped" {
                    // отступ от края + скруглённые концы линий
                    let s0 = self.stripes(ux, uy, bw, bh);
                    let inner = self.inset(&fill, self.pstep * 0.55);
                    let prod = s0.multiply(&inner);
                    self.round_mask(&prod, self.pstep * self.pthick * 0.35)
                } else {
                    self.dots(ux, uy, bw, bh, &fill)
                };
                let col = self.strong(color);
                self.paste(col, ux, uy, &pat, op.unwrap_or(1.0));
            }
            _ => {}
        }
    }

    /// Один слой мазка: неровный полигон, смещённый на `shift`.
    #[allow(clippy::too_many_arguments)]
    fn daub_layer(
        &mut self,
        q: &[Pt],
        col: Rgb,
        alpha: f64,
        jitter: f64,
        shift: (f64, f64),
        x0: usize,
        y0: usize,
        bw: usize,
        bh: usize,
    ) -> Mask {
        let amp = self.daub_amp * jitter;
        let mut pts = roughen(q, amp, &mut self.rng);
        for p in pts.iter_mut() {
            p.0 += shift.0;
            p.1 += shift.1;
        }
        let mut m = Mask::new(bw, bh);
        m.fill_polygon(&pts, 255);
        self.paste(col, x0, y0, &m, alpha);
        m
    }

    /// Борозды от кисти: короткие штрихи одного направления.
    fn streaks(&mut self, bw: usize, bh: usize) -> Mask {
        let s = self.ss as f64;
        let mut m = Mask::new(bw, bh);
        let jitter = self.rng.uniform(-0.25, 0.25);
        let ang = self.daub_angle + jitter;
        let cnt = (((bw * bh) as f64 / (s * s * 700.0)) as i64).max(20).min(500);
        let longest = bw.max(bh) as f64;
        for _ in 0..cnt {
            let cx = self.rng.uniform(0.0, bw as f64);
            let cy = self.rng.uniform(0.0, bh as f64);
            let ln = self.rng.uniform(0.1, 0.45) * longest;
            let dx = ang.cos() * ln / 2.0;
            let dy = ang.sin() * ln / 2.0;
            let wd = (s * self.rng.uniform(1.0, 4.0)).max(1.0);
            let v = self.rng.int(110, 255) as u8;
            m.line_round((cx - dx, cy - dy), (cx + dx, cy + dy), wd, v);
        }
        m
    }

    /// Мазок краски: два смещённых неровных подслоя + борозды, плавно затухающие к краю.
    fn daub(&mut self, q: &[Pt], color: Rgb, x0: usize, y0: usize, bw: usize, bh: usize) {
        let amp = self.daub_amp;
        let op = self.opacity.unwrap_or(1.0);
        let off = amp * 1.6;
        let black = (0u8, 0u8, 0u8);
        let white = (255u8, 255u8, 255u8);

        for k in 0..2 {
            let col = if k == 0 { mix(color, black, 0.16) } else { mix(color, white, 0.20) };
            let sx = self.rng.uniform(-off, off);
            let sy = self.rng.uniform(-off, off);
            let _ = self.daub_layer(q, col, 0.55 * op, 1.4, (sx, sy), x0, y0, bw, bh);
        }
        let base = self.daub_layer(q, color, op, 1.0, (0.0, 0.0), x0, y0, bw, bh);

        let mut soft = base.blurred((amp * 3.0).max(2.0));
        soft.apply_lut(&Mask::ramp_table(128, 255));

        for k in 0..2 {
            let (col, alpha) = if k == 0 {
                (mix(color, white, 0.28), 0.32)
            } else {
                (mix(color, black, 0.22), 0.26)
            };
            let st = self.streaks(bw, bh);
            let m = st.multiply(&soft);
            self.paste(col, x0, y0, &m, alpha);
        }
    }

    // ───────── публичный API для композиций ─────────

    pub fn polygon(&mut self, pts: &[Pt], color: Rgb, style: Option<&str>, contour: Contour) {
        let st = match style {
            Some(s) => s.to_string(),
            None => self.resolve(),
        };
        if st != "contour" {
            self.paint(pts, color, &st);
            return;
        }
        let col = self.strong(color);
        let k = self.rng.int(3, 5);
        match contour {
            Contour::Shift => {
                let gap = self.thick * 3.5;
                for i in 0..k {
                    let moved: Vec<Pt> = pts.iter().map(|&(x, y)| (x, y + i as f64 * gap)).collect();
                    self.paint(&moved, col, "line");
                }
            }
            Contour::Scale => {
                let n = pts.len() as f64;
                let cx = pts.iter().map(|p| p.0).sum::<f64>() / n;
                let cy = pts.iter().map(|p| p.1).sum::<f64>() / n;
                let step = self.rng.uniform(0.10, 0.16);
                for i in 0..k {
                    let f = 1.0 - i as f64 * step;
                    if f < 0.2 {
                        break;
                    }
                    let scaled: Vec<Pt> = pts.iter().map(|&(x, y)| (cx + (x - cx) * f, cy + (y - cy) * f)).collect();
                    self.paint(&scaled, col, "line");
                }
            }
            Contour::Line => {
                self.paint(pts, col, "line");
            }
        }
    }

    pub fn shape(&mut self, pts: &[Pt], cx: f64, cy: f64, r: f64, rot: f64, color: Rgb, style: Option<&str>) {
        let st = match style {
            Some(s) => s.to_string(),
            None => self.resolve(),
        };
        let c = rot.cos();
        let s = rot.sin();
        let build = |rr: f64| -> Vec<Pt> {
            pts.iter()
                .map(|&(x, y)| (cx + rr * (x * c - y * s), cy + rr * (x * s + y * c)))
                .collect()
        };
        if st == "contour" {
            let col = self.strong(color);
            let step = self.rng.uniform(0.12, 0.2);
            let k = self.rng.int(3, 5);
            for i in 0..k {
                let rr = r * (1.0 - i as f64 * step);
                if rr <= self.thick * 3.0 {
                    break;
                }
                let poly = build(rr);
                self.paint(&poly, col, "line");
            }
        } else {
            let poly = build(r);
            self.paint(&poly, color, &st);
        }
    }

    /// Едва заметный узор на фоне: dots / lines / grid.
    pub fn bg_texture(&mut self, kind: &str) {
        let n = self.pool.len();
        let i = self.rng.index(n);
        let refc = self.pool[i];
        let t = self.rng.uniform(0.10, 0.20);
        let col = mix(self.bg, refc, t);
        let (pw, ph) = (self.pw as f64, self.ph as f64);
        let m_side = self.w.min(self.h) as f64 * self.ss as f64;
        let step = m_side * self.rng.uniform(0.025, 0.045);
        let lw = ((self.ss as f64) * 1.5).round().max(1.0);
        let mut m = Mask::new(self.pw, self.ph);
        match kind {
            "dots" => {
                let r = step * 0.11;
                let mut y = step / 2.0;
                let mut row = 0;
                while y < ph + step {
                    let mut x = if row % 2 == 1 { step / 2.0 } else { 0.0 };
                    while x < pw + step {
                        m.fill_disc(x, y, r, 255);
                        x += step;
                    }
                    y += step * 0.866;
                    row += 1;
                }
            }
            "lines" => {
                let mut t = -ph;
                while t < pw {
                    m.line_round((t, 0.0), (t + ph, ph), lw, 255);
                    t += step;
                }
            }
            "grid" => {
                let mut x = 0.0;
                while x < pw {
                    m.line_round((x, 0.0), (x, ph), lw, 255);
                    x += step;
                }
                let mut y = 0.0;
                while y < ph {
                    m.line_round((0.0, y), (pw, y), lw, 255);
                    y += step;
                }
            }
            _ => return,
        }
        self.paste(col, 0, 0, &m, 1.0);
    }

    /// Усреднение блоков ss×ss -> итоговое изображение RGB.
    pub fn finish(&self) -> Vec<u8> {
        if self.ss == 1 {
            return self.img.clone();
        }
        let ss = self.ss;
        let div = (ss * ss) as u32;
        let mut out = vec![0u8; self.w * self.h * 3];
        for y in 0..self.h {
            for x in 0..self.w {
                let mut acc = [0u32; 3];
                for dy in 0..ss {
                    let row = ((y * ss + dy) * self.pw + x * ss) * 3;
                    for dx in 0..ss {
                        let o = row + dx * 3;
                        acc[0] += self.img[o] as u32;
                        acc[1] += self.img[o + 1] as u32;
                        acc[2] += self.img[o + 2] as u32;
                    }
                }
                let o = (y * self.w + x) * 3;
                out[o] = ((acc[0] + div / 2) / div) as u8;
                out[o + 1] = ((acc[1] + div / 2) / div) as u8;
                out[o + 2] = ((acc[2] + div / 2) / div) as u8;
            }
        }
        out
    }
}

/// Зерно плёнки: одинаковый шум на все три канала пикселя.
pub fn apply_grain(data: &mut [u8], rng: &mut Rng) {
    let k = rng.uniform(0.08, 0.16);
    let n = data.len() / 3;
    let mut i = 0usize;
    while i < n {
        let r = rng.next_u64();
        for j in 0..8usize {
            if i + j >= n {
                break;
            }
            let b = ((r >> (j * 8)) & 0xFF) as f64;
            let delta = ((b - 128.0) * k).round() as i32;
            for c in 0..3 {
                let o = (i + j) * 3 + c;
                data[o] = (data[o] as i32 + delta).clamp(0, 255) as u8;
            }
        }
        i += 8;
    }
}
