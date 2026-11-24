mod canvas;
mod generator;
mod layouts;
mod mask;
mod palette;
mod platform;
mod rng;
mod shapes;

use clap::Parser;
use std::path::{Path, PathBuf};
use std::time::{Duration, SystemTime, UNIX_EPOCH};

#[derive(Parser, Debug)]
#[command(
    name = "m3e-wallpaper",
    version,
    about = "Генератор случайных обоев Material 3 Expressive для рабочего стола и экрана блокировки (Windows, Linux)"
)]
struct Cli {
    /// Ширина (по умолчанию — разрешение основного монитора)
    #[arg(long)]
    width: Option<u32>,

    /// Высота
    #[arg(long)]
    height: Option<u32>,

    /// Seed для воспроизводимости
    #[arg(long)]
    seed: Option<u64>,

    /// Схема цветов Material You
    #[arg(
        long,
        default_value = "random",
        value_parser = ["random", "expressive", "vibrant", "fruit-salad", "tonal-spot", "rainbow", "content", "fidelity", "neutral", "monochrome"]
    )]
    scheme: String,

    /// Тема: auto — как в настройках системы
    #[arg(long, default_value = "auto", value_parser = ["auto", "light", "dark", "random"])]
    theme: String,

    /// Исходный цвет, например #6750A4 (по умолчанию случайный)
    #[arg(long)]
    color: Option<String>,

    /// Уровень контраста от -1.0 до 1.0
    #[arg(long, default_value_t = 0.0, allow_negative_numbers = true)]
    contrast: f64,

    /// Композиция
    #[arg(
        long,
        default_value = "random",
        value_parser = ["random", "scatter", "hero", "grid", "waves", "concentric", "bento", "pills", "rays", "hills", "trail", "rosette"]
    )]
    layout: String,

    /// Стиль фигур
    #[arg(
        long,
        default_value = "random",
        value_parser = ["random", "solid", "outline", "contour", "duotone", "glass", "ghost", "striped", "dotted", "daub", "mixed"]
    )]
    style: String,

    /// Текстура: grain — поверх всего; dots, lines, grid — узор на фоне
    #[arg(long, default_value = "random", value_parser = ["random", "none", "grain", "dots", "lines", "grid"])]
    texture: String,

    /// Непрозрачность заливки 0.05–1.0 (по умолчанию зависит от стиля)
    #[arg(long)]
    opacity: Option<f64>,

    /// Куда поставить обои: рабочий стол, экран блокировки, оба или никуда
    #[arg(long, default_value = "both", value_parser = ["desktop", "lock", "both", "none"])]
    apply: String,

    /// Папка для сохранения (по умолчанию ~/Pictures/M3E_Wallpapers)
    #[arg(long)]
    out: Option<PathBuf>,

    /// Коэффициент сглаживания (1 — быстрее, 3 — гладче)
    #[arg(long, default_value_t = 2)]
    ss: usize,

    /// Менять обои каждые N минут
    #[arg(long = "loop", value_name = "МИНУТЫ")]
    loop_minutes: Option<f64>,
}

fn random_seed() -> u64 {
    SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .map(|d| d.as_nanos() as u64)
        .unwrap_or(12345)
        % 1_000_000_000
}

fn unix_secs() -> u64 {
    SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .map(|d| d.as_secs())
        .unwrap_or(0)
}

fn default_out_dir() -> PathBuf {
    let home = std::env::var_os("USERPROFILE")
        .or_else(|| std::env::var_os("HOME"))
        .map(PathBuf::from)
        .unwrap_or_else(|| PathBuf::from("."));
    home.join("Pictures").join("M3E_Wallpapers")
}

fn save_png(path: &Path, w: u32, h: u32, data: Vec<u8>) -> Result<(), String> {
    let img = image::RgbImage::from_raw(w, h, data).ok_or("внутренняя ошибка: неверный размер буфера")?;
    img.save(path).map_err(|e| format!("не удалось сохранить {}: {e}", path.display()))
}

fn apply(path: &Path, mode: &str) {
    if mode == "desktop" || mode == "both" {
        match platform::set_desktop(path) {
            Ok(msg) => {
                if msg.is_empty() {
                    println!("Обои рабочего стола установлены.");
                } else {
                    println!("{msg}");
                }
            }
            Err(e) => eprintln!("Рабочий стол: {e}"),
        }
    }
    if mode == "lock" || mode == "both" {
        match platform::set_lock(path) {
            Ok(msg) => {
                if msg.is_empty() {
                    println!("Обои экрана блокировки установлены.");
                } else {
                    println!("{msg}");
                }
            }
            Err(e) => eprintln!("Экран блокировки: {e}"),
        }
    }
}

fn run() -> Result<(), String> {
    let cli = Cli::parse();

    let (sw, sh) = platform::screen_size();
    let width = cli.width.unwrap_or(sw) as usize;
    let height = cli.height.unwrap_or(sh) as usize;
    if width < 64 || height < 64 || width > 16384 || height > 16384 {
        return Err("размер обоев должен быть от 64 до 16384 пикселей".to_string());
    }

    let out_dir = cli.out.clone().unwrap_or_else(default_out_dir);
    std::fs::create_dir_all(&out_dir).map_err(|e| format!("не удалось создать {}: {e}", out_dir.display()))?;

    let apply_mode = cli.apply.clone();
    let opacity = cli.opacity.map(|o| o.clamp(0.05, 1.0));
    let ss = cli.ss.clamp(1, 4);
    let mut prev_file: Option<PathBuf> = None;

    loop {
        let seed = cli.seed.unwrap_or_else(random_seed);
        let theme = if cli.theme == "auto" {
            if platform::system_is_dark() {
                "dark".to_string()
            } else {
                "light".to_string()
            }
        } else {
            cli.theme.clone()
        };

        let opts = generator::GenOpts {
            width,
            height,
            seed,
            scheme: cli.scheme.clone(),
            theme,
            color: cli.color.clone(),
            contrast: cli.contrast,
            layout: cli.layout.clone(),
            ss,
            style: cli.style.clone(),
            texture: cli.texture.clone(),
            opacity,
        };
        let (rgb, info) = generator::generate(&opts)?;

        // Имя файла каждый раз новое: Windows не принимает для экрана блокировки файл с прежним именем.
        let mut path = out_dir.join(format!("m3e_{seed}.png"));
        if path.exists() {
            path = out_dir.join(format!("m3e_{seed}_{}.png", unix_secs()));
        }
        save_png(&path, width as u32, height as u32, rgb)?;
        println!("{width}x{height}  {info}\n-> {}", path.display());

        if apply_mode != "none" {
            apply(&path, &apply_mode);
        }

        match cli.loop_minutes {
            None => break,
            Some(mins) => {
                if apply_mode != "none" {
                    if let Some(old) = prev_file.take() {
                        if old != path {
                            let _ = std::fs::remove_file(old);
                        }
                    }
                }
                prev_file = Some(path);
                std::thread::sleep(Duration::from_secs_f64((mins * 60.0).max(1.0)));
            }
        }
    }
    Ok(())
}

fn main() {
    if let Err(e) = run() {
        eprintln!("Ошибка: {e}");
        std::process::exit(1);
    }
}
