# m3e-wallpaper

Генератор случайных обоев в стиле Material 3 Expressive для **Windows** и **Linux**.
Ставит обои на рабочий стол и/или экран блокировки. Цвета — из
[material-color-utilities-rust](https://github.com/SaKongA/material-color-utilities-rust).

## Использование
`m3e-wallpaper.exe [OPTIONS]`

Опции:
```
      --width <WIDTH>        Ширина (по умолчанию — разрешение основного монитора)
      --height <HEIGHT>      Высота
      --seed <SEED>          Seed для воспроизводимости
      --scheme <SCHEME>      Схема цветов Material You [default: random] [possible values: random, expressive, vibrant, fruit-salad, tonal-spot, rainbow, content, fidelity, neutral, monochrome]
      --theme <THEME>        Тема: auto — как в настройках системы [default: auto] [possible values: auto, light, dark, random]
      --color <COLOR>        Исходный цвет, например #6750A4 (по умолчанию случайный)
      --contrast <CONTRAST>  Уровень контраста от -1.0 до 1.0 [default: 0]
      --layout <LAYOUT>      Композиция [default: random] [possible values: random, scatter, hero, grid, waves, concentric, bento, pills, rays, hills, trail, rosette]
      --style <STYLE>        Стиль фигур [default: random] [possible values: random, solid, outline, contour, duotone, glass, ghost, striped, dotted, daub, mixed]
      --texture <TEXTURE>    Текстура: grain — поверх всего; dots, lines, grid — узор на фоне [default: random] [possible values: random, none, grain, dots, lines, grid]
      --opacity <OPACITY>    Непрозрачность заливки 0.05–1.0 (по умолчанию зависит от стиля)
      --apply <APPLY>        Куда поставить обои: рабочий стол, экран блокировки, оба или никуда [default: both] [possible values: desktop, lock, both, none]
      --out <OUT>            Папка для сохранения (по умолчанию ~/Pictures/M3E_Wallpapers)
      --ss <SS>              Коэффициент сглаживания (1 — быстрее, 3 — гладче) [default: 2]
      --loop <МИНУТЫ>        Менять обои каждые N минут
  -h, --help                 Print help
  -V, --version              Print version
```

## Экран блокировки

**Windows** — через WinRT: основной путь `LockScreen.SetImageFileAsync`, запасной
`UserProfilePersonalizationSettings.TrySetLockScreenImageAsync`. После установки программа читает
картинку обратно и пишет в консоль, совпал ли размер файла (если не совпал — Windows мог перекодировать
картинку; проверьте через Win+L). Собирайте x64-версию. Если картинка не применилась, проверьте
Параметры → Персонализация → Экран блокировки → «Персонализировать» = «Картинка» (не «Windows: интересное»).
Файл для экрана блокировки каждый раз должен называться по-новому — программа это учитывает.

**Linux**
- *KDE Plasma* — пишет `kscreenlockerrc` (kwriteconfig5/6); обои появятся при следующей блокировке.
- *GNOME 40+* — отдельной картинки экрана блокировки нет: он показывает размытые обои рабочего
  стола, поэтому используйте `--apply both`.
- Рабочий стол: GNOME, KDE, Cinnamon, MATE, XFCE, либо `feh`.
- Прочие окружения для экрана блокировки не поддерживаются.

## Заметки
- Один и тот же `--seed` даёт одни и те же обои
