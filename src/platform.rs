pub use imp::{screen_size, set_desktop, set_lock, system_is_dark};

#[cfg(windows)]
mod imp {
    use std::ffi::c_void;
    use std::os::windows::ffi::OsStrExt;
    use std::path::Path;
    use std::process::Command;
    use windows::{
        core::HSTRING,
        Storage::StorageFile,
        System::UserProfile::{LockScreen, UserProfilePersonalizationSettings},
    };

    #[link(name = "user32")]
    extern "system" {
        fn SystemParametersInfoW(ui_action: u32, ui_param: u32, pv_param: *mut c_void, f_win_ini: u32) -> i32;
        fn GetSystemMetrics(n_index: i32) -> i32;
        fn SetProcessDPIAware() -> i32;
    }

    pub fn screen_size() -> (u32, u32) {
        unsafe {
            SetProcessDPIAware();
            let w = GetSystemMetrics(0);
            let h = GetSystemMetrics(1);
            if w > 0 && h > 0 {
                (w as u32, h as u32)
            } else {
                (1920, 1080)
            }
        }
    }

    pub fn system_is_dark() -> bool {
        let out = Command::new("reg")
            .args([
                "query",
                r"HKCU\Software\Microsoft\Windows\CurrentVersion\Themes\Personalize",
                "/v",
                "AppsUseLightTheme",
            ])
            .output();
        match out {
            Ok(o) => {
                let s = String::from_utf8_lossy(&o.stdout).to_string();
                s.lines()
                    .any(|l| l.contains("AppsUseLightTheme") && l.trim_end().ends_with("0x0"))
            }
            Err(_) => true,
        }
    }

    pub fn set_desktop(path: &Path) -> Result<String, String> {
        let abs = std::path::absolute(path).map_err(|e| e.to_string())?;

        let _ = Command::new("reg")
            .args(["add", r"HKCU\Control Panel\Desktop", "/v", "WallpaperStyle", "/t", "REG_SZ", "/d", "10", "/f"])
            .output();
        let _ = Command::new("reg")
            .args(["add", r"HKCU\Control Panel\Desktop", "/v", "TileWallpaper", "/t", "REG_SZ", "/d", "0", "/f"])
            .output();
        let mut wide: Vec<u16> = abs.as_os_str().encode_wide().collect();
        wide.push(0);

        let ok = unsafe { SystemParametersInfoW(0x0014, 0, wide.as_mut_ptr() as *mut c_void, 0x01 | 0x02) };
        if ok != 0 {
            Ok(String::new())
        } else {
            Err("SystemParametersInfoW вернул ошибку".to_string())
        }
    }

    fn ensure_lock_screen_picture_mode() -> Result<(), String> {
        use std::process::Command;

        let out = Command::new("reg")
            .args([
                "query",
                r"HKCU\Software\Microsoft\Windows\CurrentVersion\Lock Screen",
                "/v",
                "CreativeMode",
            ])
            .output();

        let current_mode = match out {
            Ok(o) => {
                let s = String::from_utf8_lossy(&o.stdout).to_string();
                s.lines()
                    .find(|l| l.contains("CreativeMode"))
                    .and_then(|l| l.split_whitespace().last())
                    .and_then(|v| v.parse::<u32>().ok())
                    .unwrap_or(0)
            }
            Err(_) => 0,
        };

        if current_mode != 0 {
            let _ = Command::new("reg")
                .args([
                    "add",
                    r"HKCU\Software\Microsoft\Windows\CurrentVersion\Lock Screen",
                    "/v",
                    "CreativeMode",
                    "/t",
                    "REG_DWORD",
                    "/d",
                    "0",
                    "/f",
                ])
                .output();
        }
        Ok(())
    }

    fn copy_to_pictures(path: &Path) -> Result<std::path::PathBuf, String> {
        let pictures = std::env::var_os("USERPROFILE")
            .map(std::path::PathBuf::from)
            .unwrap_or_else(|| std::path::PathBuf::from("."))
            .join("Pictures");
        let fname = path.file_name().ok_or("нет имени файла")?;
        let dest = pictures.join(fname);
        std::fs::copy(path, &dest).map_err(|e| format!("не удалось скопировать в Pictures: {e}"))?;
        Ok(dest)
    }

    pub fn set_lock(path: &Path) -> Result<String, String> {
        if let Err(e) = ensure_lock_screen_picture_mode() {
            eprintln!("Предупреждение: не удалось проверить/установить режим экрана блокировки: {e}");
        }

        let abs = std::path::absolute(path).map_err(|e| e.to_string())?;
        let lock_path = copy_to_pictures(&abs)?;
        let path_str: String = lock_path.to_string_lossy().to_string();
        let hs = HSTRING::from(path_str.as_str());
        let file = StorageFile::GetFileFromPathAsync(&hs)
            .map_err(|e| format!("StorageFile.GetFileFromPathAsync: {e}"))?
            .join()
            .map_err(|e| format!("не удалось открыть файл через WinRT: {e}"))?;

        let mut log: Vec<String> = Vec::new();
        let mut applied = false;

        match LockScreen::SetImageFileAsync(&file) {
            Ok(op) => match op.join() {
                Ok(()) => {
                    applied = true;
                    log.push("LockScreen.SetImageFileAsync: принято".to_string());
                }
                Err(e) => log.push(format!("LockScreen.SetImageFileAsync: ошибка: {e}")),
            },
            Err(e) => log.push(format!("LockScreen.SetImageFileAsync: не удалось запустить: {e}")),
        }

        if !applied {
            match UserProfilePersonalizationSettings::Current() {
                Ok(settings) => match settings.TrySetLockScreenImageAsync(&file) {
                    Ok(op) => match op.join() {
                        Ok(true) => {
                            applied = true;
                            log.push("TrySetLockScreenImageAsync: принято".to_string());
                        }
                        Ok(false) => log.push("TrySetLockScreenImageAsync: Windows отклонил картинку".to_string()),
                        Err(e) => log.push(format!("TrySetLockScreenImageAsync: ошибка: {e}")),
                    },
                    Err(e) => log.push(format!("TrySetLockScreenImageAsync: не удалось запустить: {e}")),
                },
                Err(e) => log.push(format!("UserProfilePersonalizationSettings: {e}")),
            }
        }

        if !applied {
            log.push("Проверьте: Параметры → Персонализация → Экран блокировки → «Персонализировать» = «Картинка»".to_string());
            return Err(log.join("; "));
        }

        match LockScreen::GetImageStream() {
            Ok(stream) => match stream.Size() {
                Ok(sz) => {
                    let fsz = std::fs::metadata(&lock_path).map(|m| m.len()).unwrap_or(0);
                    if sz == fsz {
                        log.push("проверка чтением: размер совпал, картинка применена".to_string());
                    } else {
                        log.push(format!(
                            "проверка чтением: размер {sz} байт не равен размеру файла {fsz} \
                             (Windows мог перекодировать картинку — проверьте экран блокировки через Win+L)"
                        ));
                    }
                }
                Err(e) => log.push(format!("проверка чтением: не удалось получить размер: {e}")),
            },
            Err(e) => log.push(format!("проверка чтением: GetImageStream: {e}")),
        }
        Ok(log.join("\n"))
    }
}

#[cfg(not(windows))]
mod imp {
    use std::path::Path;
    use std::process::Command;

    fn run(cmd: &str, args: &[&str]) -> bool {
        Command::new(cmd)
            .args(args)
            .output()
            .map(|o| o.status.success())
            .unwrap_or(false)
    }

    fn run_out(cmd: &str, args: &[&str]) -> Option<String> {
        let o = Command::new(cmd).args(args).output().ok()?;
        if !o.status.success() {
            return None;
        }
        Some(String::from_utf8_lossy(&o.stdout).to_string())
    }

    fn have(cmd: &str) -> bool {
        Command::new(cmd).arg("--help").output().is_ok()
    }

    fn desktop_env() -> String {
        std::env::var("XDG_CURRENT_DESKTOP")
            .or_else(|_| std::env::var("DESKTOP_SESSION"))
            .unwrap_or_default()
            .to_lowercase()
    }

    fn percent(s: &str) -> String {
        let mut o = String::new();
        for b in s.bytes() {
            if b.is_ascii_alphanumeric() || b"/-_.~".contains(&b) {
                o.push(b as char);
            } else {
                o.push_str(&format!("%{:02X}", b));
            }
        }
        o
    }

    pub fn screen_size() -> (u32, u32) {
        if let Some(s) = drm_size() {
            return s;
        }
        if let Some(out) = run_out("xrandr", &["--current"]) {
            for line in out.lines() {
                if line.contains(" connected") {
                    for tok in line.split_whitespace() {
                        if let Some((a, rest)) = tok.split_once('x') {
                            if let Ok(w) = a.parse::<u32>() {
                                let digits: String = rest.chars().take_while(|c| c.is_ascii_digit()).collect();
                                if let Ok(h) = digits.parse::<u32>() {
                                    if w > 0 && h > 0 {
                                        return (w, h);
                                    }
                                }
                            }
                        }
                    }
                }
            }
        }
        (1920, 1080)
    }

    fn drm_size() -> Option<(u32, u32)> {
        let dir = std::fs::read_dir("/sys/class/drm").ok()?;
        for entry in dir.flatten() {
            let p = entry.path();
            let status = std::fs::read_to_string(p.join("status")).unwrap_or_default();
            if status.trim() != "connected" {
                continue;
            }
            let modes = std::fs::read_to_string(p.join("modes")).unwrap_or_default();
            if let Some(line) = modes.lines().next() {
                if let Some((a, rest)) = line.trim().split_once('x') {
                    let w = a.trim().parse::<u32>().ok();
                    let digits: String = rest.chars().take_while(|c| c.is_ascii_digit()).collect();
                    let h = digits.parse::<u32>().ok();
                    if let (Some(w), Some(h)) = (w, h) {
                        if w > 0 && h > 0 {
                            return Some((w, h));
                        }
                    }
                }
            }
        }
        None
    }

    pub fn system_is_dark() -> bool {
        if let Some(s) = run_out("gsettings", &["get", "org.gnome.desktop.interface", "color-scheme"]) {
            if s.contains("dark") {
                return true;
            }
            if s.contains("light") || s.contains("default") {
                return false;
            }
        }
        for tool in ["kreadconfig6", "kreadconfig5"] {
            if let Some(s) = run_out(tool, &["--file", "kdeglobals", "--group", "General", "--key", "ColorScheme"]) {
                return s.to_lowercase().contains("dark");
            }
        }
        false
    }

    fn set_xfce(p: &str) -> bool {
        let list = match run_out("xfconf-query", &["-c", "xfce4-desktop", "-l"]) {
            Some(l) => l,
            None => return false,
        };
        let mut any = false;
        for prop in list.lines().filter(|l| l.ends_with("/last-image")) {
            if run("xfconf-query", &["-c", "xfce4-desktop", "-p", prop, "-s", p]) {
                any = true;
            }
        }
        any
    }

    pub fn set_desktop(path: &Path) -> Result<String, String> {
        let abs = std::path::absolute(path).map_err(|e| e.to_string())?;
        let p = abs.to_string_lossy().to_string();
        let uri = format!("file://{}", percent(&p));
        let de = desktop_env();
        let mut ok = false;

        if de.contains("kde") || de.contains("plasma") {
            ok = run("plasma-apply-wallpaperimage", &[p.as_str()]);
        } else if de.contains("cinnamon") {
            ok = run("gsettings", &["set", "org.cinnamon.desktop.background", "picture-uri", uri.as_str()]);
            run("gsettings", &["set", "org.cinnamon.desktop.background", "picture-options", "zoom"]);
        } else if de.contains("mate") {
            ok = run("gsettings", &["set", "org.mate.background", "picture-filename", p.as_str()]);
            run("gsettings", &["set", "org.mate.background", "picture-options", "zoom"]);
        } else if de.contains("xfce") {
            ok = set_xfce(&p);
        } else if de.contains("gnome") || de.contains("budgie") || de.contains("unity") {
            ok = run("gsettings", &["set", "org.gnome.desktop.background", "picture-uri", uri.as_str()]);
            run("gsettings", &["set", "org.gnome.desktop.background", "picture-uri-dark", uri.as_str()]);
            run("gsettings", &["set", "org.gnome.desktop.background", "picture-options", "zoom"]);
        }

        if !ok && have("feh") {
            ok = run("feh", &["--bg-fill", p.as_str()]);
        }
        if ok {
            Ok(String::new())
        } else {
            Err(format!(
                "не удалось поставить обои: окружение «{}» не поддерживается (поддерживаются GNOME, KDE, Cinnamon, MATE, XFCE или feh)",
                if de.is_empty() { "неизвестно" } else { de.as_str() }
            ))
        }
    }

    pub fn set_lock(path: &Path) -> Result<String, String> {
        let abs = std::path::absolute(path).map_err(|e| e.to_string())?;
        let p = abs.to_string_lossy().to_string();
        let uri = format!("file://{}", percent(&p));
        let de = desktop_env();

        if de.contains("kde") || de.contains("plasma") {
            let tool = if have("kwriteconfig6") { "kwriteconfig6" } else { "kwriteconfig5" };
            let a = run(
                tool,
                &[
                    "--file", "kscreenlockerrc", "--group", "Greeter", "--group", "Wallpaper", "--group",
                    "org.kde.image", "--group", "General", "--key", "Image", uri.as_str(),
                ],
            );
            let b = run(tool, &["--file", "kscreenlockerrc", "--group", "Greeter", "--key", "WallpaperPlugin", "org.kde.image"]);
            if a && b {
                return Ok("KDE: новые обои появятся при следующей блокировке экрана.".to_string());
            }
            return Err("не удалось записать kscreenlockerrc (нужны kwriteconfig5/6)".to_string());
        }

        if de.contains("gnome") || de.contains("budgie") || de.contains("unity") {
            let ok = run("gsettings", &["set", "org.gnome.desktop.screensaver", "picture-uri", uri.as_str()]);
            let note = "GNOME 40+ не поддерживает отдельную картинку экрана блокировки — он показывает размытую копию обоев \
                        рабочего стола. Используйте --apply both. Ключ screensaver применён только для старых версий GNOME.";
            return if ok { Ok(note.to_string()) } else { Err(note.to_string()) };
        }

        Err(format!(
            "экран блокировки для окружения «{}» не поддерживается: поддерживаются KDE Plasma и GNOME",
            if de.is_empty() { "неизвестно" } else { de.as_str() }
        ))
    }
}
