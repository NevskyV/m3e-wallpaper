use crate::canvas::{apply_grain, Canvas};
use crate::layouts;
use crate::palette;
use crate::rng::Rng;

pub const STYLES: [(&str, f64); 10] = [
    ("solid", 4.0),  
    ("outline", 2.0),
    ("contour", 1.0),
    ("duotone", 2.0),
    ("glass", 2.0),  
    ("ghost", 1.0),  
    ("striped", 1.0),
    ("dotted", 1.0), 
    ("daub", 2.0),   
    ("mixed", 2.0),  
];

pub const TEXTURES: [(&str, f64); 5] = [("none", 3.0), ("grain", 2.0), ("dots", 1.0), ("lines", 1.0), ("grid", 1.0)];

pub struct GenOpts {
    pub width: usize,
    pub height: usize,
    pub seed: u64,
    pub scheme: String,
    pub theme: String, // "light" | "dark" | "random"
    pub color: Option<String>,
    pub contrast: f64,
    pub layout: String,
    pub ss: usize,
    pub style: String,
    pub texture: String,
    pub opacity: Option<f64>,
}

fn pick_weighted(rng: &mut Rng, table: &[(&str, f64)]) -> String {
    let w: Vec<f64> = table.iter().map(|t| t.1).collect();
    table[rng.weighted(&w)].0.to_string()
}

/// Возвращает RGB-буфер (width*height*3) и строку с описанием параметров.
pub fn generate(o: &GenOpts) -> Result<(Vec<u8>, String), String> {
    let mut rng = Rng::new(o.seed);

    let scheme_name = if o.scheme == "random" {
        palette::RANDOM_SCHEMES[rng.index(palette::RANDOM_SCHEMES.len())].to_string()
    } else {
        o.scheme.clone()
    };
    let dark = match o.theme.as_str() {
        "dark" => true,
        "light" => false,
        _ => rng.chance(0.5),
    };

    let seed_hct = palette::seed_hct(&mut rng, o.color.as_deref())?;
    let scheme = palette::make_scheme(&scheme_name, seed_hct, dark, o.contrast);
    let colors = palette::extract(&scheme);
    let (bg, pool) = palette::build_palette(&colors, &mut rng)?;

    let layout = if o.layout == "random" {
        pick_weighted(&mut rng, &layouts::LAYOUTS)
    } else {
        o.layout.clone()
    };
    let style = if o.style == "random" {
        pick_weighted(&mut rng, &STYLES)
    } else {
        o.style.clone()
    };
    let texture = if o.texture == "random" {
        pick_weighted(&mut rng, &TEXTURES)
    } else {
        o.texture.clone()
    };

    let mut cv = Canvas::new(o.width, o.height, bg, o.ss.max(1), rng, pool, style.clone(), o.opacity);
    if texture == "dots" || texture == "lines" || texture == "grid" {
        cv.bg_texture(&texture);
    }
    layouts::run(&layout, &mut cv);

    let mut img = cv.finish();
    if texture == "grain" {
        apply_grain(&mut img, &mut cv.rng);
    }

    let info = format!(
        "схема={}, тема={}, композиция={}, стиль={}, текстура={}, seed={}",
        scheme_name,
        if dark { "dark" } else { "light" },
        layout,
        style,
        texture,
        o.seed
    );
    Ok((img, info))
}
