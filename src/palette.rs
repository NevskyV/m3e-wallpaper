use crate::canvas::{color_dist, Rgb};
use crate::rng::Rng;
use material_color_utilities_rust::{DynamicScheme, Hct};
use std::collections::HashMap;

/// При --scheme random: expressive встречается чаще остальных.
pub const RANDOM_SCHEMES: [&str; 6] = ["expressive", "expressive", "vibrant", "fruit-salad", "rainbow", "tonal-spot"];

fn rgb(argb: u32) -> Rgb {
    (((argb >> 16) & 255) as u8, ((argb >> 8) & 255) as u8, (argb & 255) as u8)
}

fn hsv_to_rgb(h: f64, s: f64, v: f64) -> (u8, u8, u8) {
    let c = v * s;
    let hp = (h / 60.0) % 6.0;
    let x = c * (1.0 - ((hp % 2.0) - 1.0).abs());
    let (r1, g1, b1) = if hp < 1.0 {
        (c, x, 0.0)
    } else if hp < 2.0 {
        (x, c, 0.0)
    } else if hp < 3.0 {
        (0.0, c, x)
    } else if hp < 4.0 {
        (0.0, x, c)
    } else if hp < 5.0 {
        (x, 0.0, c)
    } else {
        (c, 0.0, x)
    };
    let m = v - c;
    (
        ((r1 + m) * 255.0).round() as u8,
        ((g1 + m) * 255.0).round() as u8,
        ((b1 + m) * 255.0).round() as u8,
    )
}

/// Исходный цвет: заданный пользователем (#RRGGBB) или случайный яркий.
pub fn seed_hct(rng: &mut Rng, color: Option<&str>) -> Result<Hct, String> {
    if let Some(c) = color {
        let hex = c.trim().trim_start_matches('#');
        let v = u32::from_str_radix(hex, 16).map_err(|_| format!("Неверный цвет: {c} (ожидается #RRGGBB)"))?;
        return Ok(Hct::from_int(0xFF00_0000 | (v & 0x00FF_FFFF)));
    }
    let h = rng.uniform(0.0, 360.0);
    let s = rng.uniform(0.55, 1.0);
    let v = rng.uniform(0.55, 0.95);
    let (r, g, b) = hsv_to_rgb(h, s, v);
    let argb: u32 = 0xFF00_0000 | ((r as u32) << 16) | ((g as u32) << 8) | (b as u32);
    Ok(Hct::from_int(argb))
}

pub fn make_scheme(name: &str, seed: Hct, dark: bool, contrast: f64) -> DynamicScheme {
    match name {
        "vibrant" => DynamicScheme::vibrant(seed, dark, contrast),
        "fruit-salad" => DynamicScheme::fruit_salad(seed, dark, contrast),
        "tonal-spot" => DynamicScheme::tonal_spot(seed, dark, contrast),
        "rainbow" => DynamicScheme::rainbow(seed, dark, contrast),
        "content" => DynamicScheme::content(seed, dark, contrast),
        "fidelity" => DynamicScheme::fidelity(seed, dark, contrast),
        "neutral" => DynamicScheme::neutral(seed, dark, contrast),
        "monochrome" => DynamicScheme::monochrome(seed, dark, contrast),
        _ => DynamicScheme::expressive(seed, dark, contrast),
    }
}

/// Нужные роли схемы -> RGB.
pub fn extract(s: &DynamicScheme) -> HashMap<&'static str, Rgb> {
    let mut m: HashMap<&'static str, Rgb> = HashMap::new();
    m.insert("primary", rgb(s.primary()));
    m.insert("secondary", rgb(s.secondary()));
    m.insert("tertiary", rgb(s.tertiary()));
    m.insert("primarycontainer", rgb(s.primary_container()));
    m.insert("secondarycontainer", rgb(s.secondary_container()));
    m.insert("tertiarycontainer", rgb(s.tertiary_container()));
    m.insert("primaryfixed", rgb(s.primary_fixed()));
    m.insert("primaryfixeddim", rgb(s.primary_fixed_dim()));
    m.insert("secondaryfixed", rgb(s.secondary_fixed()));
    m.insert("secondaryfixeddim", rgb(s.secondary_fixed_dim()));
    m.insert("tertiaryfixed", rgb(s.tertiary_fixed()));
    m.insert("tertiaryfixeddim", rgb(s.tertiary_fixed_dim()));
    m.insert("inverseprimary", rgb(s.inverse_primary()));
    m.insert("surface", rgb(s.surface()));
    m.insert("surfacecontainerlow", rgb(s.surface_container_low()));
    m.insert("surfacecontainerhigh", rgb(s.surface_container_high()));
    m.insert("surfacecontainerhighest", rgb(s.surface_container_highest()));
    m.insert("onprimarycontainer", rgb(s.on_primary_container()));
    m
}

/// Выбираем фон и набор акцентных цветов, контрастных к нему.
pub fn build_palette(colors: &HashMap<&'static str, Rgb>, rng: &mut Rng) -> Result<(Rgb, Vec<Rgb>), String> {
    let bg_names = [
        "surface",
        "surfacecontainerlow",
        "surfacecontainerhigh",
        "primarycontainer",
        "secondarycontainer",
        "tertiarycontainer",
        "primary",
        "primaryfixed",
    ];
    let fg_names = [
        "primary",
        "secondary",
        "tertiary",
        "primarycontainer",
        "secondarycontainer",
        "tertiarycontainer",
        "primaryfixed",
        "primaryfixeddim",
        "secondaryfixed",
        "secondaryfixeddim",
        "tertiaryfixed",
        "tertiaryfixeddim",
        "inverseprimary",
        "surfacecontainerhighest",
        "onprimarycontainer",
    ];
    let bgs: Vec<Rgb> = bg_names.iter().filter_map(|n| colors.get(n).copied()).collect();
    let fgs: Vec<Rgb> = fg_names.iter().filter_map(|n| colors.get(n).copied()).collect();
    if bgs.is_empty() || fgs.is_empty() {
        return Err("Не удалось получить цвета из material-color-utilities-rust".to_string());
    }
    let bg = bgs[rng.index(bgs.len())];

    let mut pool: Vec<Rgb> = Vec::new();
    for &c in fgs.iter() {
        if color_dist(c, bg) > 38.0 && !pool.contains(&c) {
            pool.push(c);
        }
    }
    if pool.len() < 3 {
        // запасной вариант: всё, что не совпадает с фоном
        let alt: Vec<Rgb> = fgs.iter().cloned().filter(|c| *c != bg).collect();
        pool = if alt.is_empty() { fgs.clone() } else { alt };
    }
    Ok((bg, pool))
}
