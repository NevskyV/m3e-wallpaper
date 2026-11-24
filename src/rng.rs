pub struct Rng {
    s: u64,
}

impl Rng {
    pub fn new(seed: u64) -> Rng {
        let mut r = Rng {
            s: seed.wrapping_mul(0x2545F4914F6CDD1D).wrapping_add(0x9E3779B97F4A7C15),
        };
        r.next_u64();
        r
    }

    pub fn next_u64(&mut self) -> u64 {
        self.s = self.s.wrapping_add(0x9E3779B97F4A7C15);
        let mut z = self.s;
        z = (z ^ (z >> 30)).wrapping_mul(0xBF58476D1CE4E5B9);
        z = (z ^ (z >> 27)).wrapping_mul(0x94D049BB133111EB);
        z ^ (z >> 31)
    }

    /// Число из [0, 1).
    pub fn f(&mut self) -> f64 {
        (self.next_u64() >> 11) as f64 / (1u64 << 53) as f64
    }

    pub fn uniform(&mut self, a: f64, b: f64) -> f64 {
        a + (b - a) * self.f()
    }

    /// Целое из [a, b] включительно.
    pub fn int(&mut self, a: i64, b: i64) -> i64 {
        a + (self.next_u64() % ((b - a + 1) as u64)) as i64
    }

    /// Индекс из [0, n).
    pub fn index(&mut self, n: usize) -> usize {
        (self.next_u64() % (n as u64)) as usize
    }

    pub fn chance(&mut self, p: f64) -> bool {
        self.f() < p
    }

    /// Выбор индекса по весам.
    pub fn weighted(&mut self, w: &[f64]) -> usize {
        let total: f64 = w.iter().sum();
        let mut t = self.f() * total;
        for (i, &x) in w.iter().enumerate() {
            if t < x {
                return i;
            }
            t -= x;
        }
        w.len() - 1
    }
    
    pub fn sample<T: Clone>(&mut self, items: &[T], k: usize) -> Vec<T> {
        let mut v: Vec<T> = items.to_vec();
        let n = v.len();
        let k = k.min(n);
        for i in 0..k {
            let j = i + self.index(n - i);
            v.swap(i, j);
        }
        v.truncate(k);
        v
    }
}
