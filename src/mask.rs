#[derive(Clone)]
pub struct Mask {
    pub w: usize,
    pub h: usize,
    pub d: Vec<u8>,
}

impl Mask {
    pub fn new(w: usize, h: usize) -> Mask {
        Mask { w, h, d: vec![0u8; w * h] }
    }

    /// Закрашивает пиксели строки `y`, центры которых лежат в [xa, xb).
    fn span(&mut self, y: i64, xa: f64, xb: f64, v: u8) {
        if y < 0 || y >= self.h as i64 {
            return;
        }
        let mut i0 = (xa - 0.5).ceil() as i64;
        let mut i1 = (xb - 0.5).ceil() as i64 - 1;
        if i0 < 0 {
            i0 = 0;
        }
        if i1 >= self.w as i64 {
            i1 = self.w as i64 - 1;
        }
        if i1 < i0 {
            return;
        }
        let base = y as usize * self.w;
        let a = base + i0 as usize;
        let b = base + i1 as usize;
        self.d[a..=b].fill(v);
    }

    /// Заливка полигона (правило ненулевой обмотки), выборка в центрах пикселей.
    pub fn fill_polygon(&mut self, pts: &[(f64, f64)], v: u8) {
        let n = pts.len();
        if n < 3 {
            return;
        }
        let mut ymin = f64::MAX;
        let mut ymax = f64::MIN;
        for p in pts {
            if p.1 < ymin {
                ymin = p.1;
            }
            if p.1 > ymax {
                ymax = p.1;
            }
        }
        let y_start = ymin.floor().max(0.0) as i64;
        let y_end = ymax.ceil().min(self.h as f64 - 1.0) as i64;
        let mut xs: Vec<(f64, i32)> = Vec::new();
        let mut y = y_start;
        while y <= y_end {
            let yc = y as f64 + 0.5;
            xs.clear();
            for i in 0..n {
                let p = pts[i];
                let q = pts[(i + 1) % n];
                if p.1 <= yc && q.1 > yc {
                    xs.push((p.0 + (yc - p.1) * (q.0 - p.0) / (q.1 - p.1), 1));
                } else if q.1 <= yc && p.1 > yc {
                    xs.push((p.0 + (yc - p.1) * (q.0 - p.0) / (q.1 - p.1), -1));
                }
            }
            if xs.len() >= 2 {
                xs.sort_by(|a, b| a.0.partial_cmp(&b.0).unwrap_or(std::cmp::Ordering::Equal));
                let mut wind = 0i32;
                let mut start = 0.0f64;
                for k in 0..xs.len() {
                    let (x, dir) = xs[k];
                    let prev = wind;
                    wind += dir;
                    if prev == 0 && wind != 0 {
                        start = x;
                    } else if prev != 0 && wind == 0 {
                        self.span(y, start, x, v);
                    }
                }
            }
            y += 1;
        }
    }

    pub fn fill_disc(&mut self, cx: f64, cy: f64, r: f64, v: u8) {
        if r <= 0.0 {
            return;
        }
        let y0 = (cy - r).floor().max(0.0) as i64;
        let y1 = (cy + r).ceil().min(self.h as f64 - 1.0) as i64;
        let mut y = y0;
        while y <= y1 {
            let dy = y as f64 + 0.5 - cy;
            let d2 = r * r - dy * dy;
            if d2 >= 0.0 {
                let half = d2.sqrt();
                self.span(y, cx - half, cx + half, v);
            }
            y += 1;
        }
    }

    fn segment_quad(a: (f64, f64), b: (f64, f64), r: f64) -> Option<[(f64, f64); 4]> {
        let dx = b.0 - a.0;
        let dy = b.1 - a.1;
        let len = dx.hypot(dy);
        if len < 1e-9 {
            return None;
        }
        let nx = -dy / len * r;
        let ny = dx / len * r;
        Some([
            (a.0 + nx, a.1 + ny),
            (b.0 + nx, b.1 + ny),
            (b.0 - nx, b.1 - ny),
            (a.0 - nx, a.1 - ny),
        ])
    }

    /// Отрезок с круглыми концами.
    pub fn line_round(&mut self, a: (f64, f64), b: (f64, f64), width: f64, v: u8) {
        let r = width / 2.0;
        if let Some(q) = Mask::segment_quad(a, b, r) {
            self.fill_polygon(&q, v);
        }
        self.fill_disc(a.0, a.1, r, v);
        self.fill_disc(b.0, b.1, r, v);
    }

    /// Замкнутая ломаная заданной толщины с круглыми стыками.
    pub fn stroke_closed(&mut self, pts: &[(f64, f64)], width: f64, v: u8) {
        let n = pts.len();
        let r = width / 2.0;
        for i in 0..n {
            let a = pts[i];
            let b = pts[(i + 1) % n];
            if let Some(q) = Mask::segment_quad(a, b, r) {
                self.fill_polygon(&q, v);
            }
            self.fill_disc(a.0, a.1, r, v);
        }
    }

    pub fn multiply(&self, o: &Mask) -> Mask {
        let mut out = self.clone();
        for i in 0..out.d.len() {
            out.d[i] = ((self.d[i] as u32 * o.d[i] as u32 + 127) / 255) as u8;
        }
        out
    }

    pub fn apply_lut(&mut self, t: &[u8; 256]) {
        for p in self.d.iter_mut() {
            *p = t[*p as usize];
        }
    }

    /// Аналог функции point(...) из Pillow: ниже lo -> 0, выше hi -> 255, между — линейно.
    pub fn ramp_table(lo: i32, hi: i32) -> [u8; 256] {
        let mut t = [0u8; 256];
        for v in 0..256i32 {
            t[v as usize] = if v <= lo {
                0
            } else if v >= hi {
                255
            } else {
                ((v - lo) * 255 / (hi - lo)) as u8
            };
        }
        t
    }

    pub fn blurred(&self, sigma: f64) -> Mask {
        let mut buf: Vec<f32> = self.d.iter().map(|&v| v as f32).collect();
        blur_f32(&mut buf, self.w, self.h, sigma);
        let d: Vec<u8> = buf.iter().map(|&v| (v + 0.5).clamp(0.0, 255.0) as u8).collect();
        Mask { w: self.w, h: self.h, d }
    }
}

// ───────────── гауссово размытие через три прохода box-blur ─────────────

fn boxes_for_gauss(sigma: f64, n: usize) -> Vec<usize> {
    let nf = n as f64;
    let w_ideal = (12.0 * sigma * sigma / nf + 1.0).sqrt();
    let mut wl = w_ideal.floor() as i64;
    if wl % 2 == 0 {
        wl -= 1;
    }
    if wl < 1 {
        wl = 1;
    }
    let wu = wl + 2;
    let wlf = wl as f64;
    let m_ideal = (12.0 * sigma * sigma - nf * wlf * wlf - 4.0 * nf * wlf - 3.0 * nf) / (-4.0 * wlf - 4.0);
    let m = m_ideal.round() as i64;
    let mut out = Vec::with_capacity(n);
    for i in 0..n {
        let b = if (i as i64) < m { wl } else { wu };
        out.push(b as usize);
    }
    out
}

fn box_line(src: &[f32], dst: &mut [f32], r: usize) {
    let l = src.len() as i64;
    let ri = r as i64;
    let at = |i: i64| -> f32 {
        let j = if i < 0 {
            0
        } else if i >= l {
            l - 1
        } else {
            i
        };
        src[j as usize]
    };
    let mut sum = 0.0f32;
    for k in -ri..=ri {
        sum += at(k);
    }
    let inv = 1.0f32 / ((2 * r + 1) as f32);
    for i in 0..l {
        dst[i as usize] = sum * inv;
        sum += at(i + ri + 1) - at(i - ri);
    }
}

fn blur_f32(buf: &mut Vec<f32>, w: usize, h: usize, sigma: f64) {
    if w == 0 || h == 0 || sigma <= 0.0 {
        return;
    }
    let boxes = boxes_for_gauss(sigma, 3);
    let longest = w.max(h);
    let mut tmp = vec![0f32; longest];
    let mut line = vec![0f32; longest];
    for b in boxes {
        let r = (b - 1) / 2;
        if r == 0 {
            continue;
        }
        // по горизонтали
        for y in 0..h {
            let row = &mut buf[y * w..(y + 1) * w];
            box_line(row, &mut tmp[..w], r);
            row.copy_from_slice(&tmp[..w]);
        }
        // по вертикали
        for x in 0..w {
            for y in 0..h {
                line[y] = buf[y * w + x];
            }
            box_line(&line[..h], &mut tmp[..h], r);
            for y in 0..h {
                buf[y * w + x] = tmp[y];
            }
        }
    }
}
