use std::collections::{BTreeMap, BTreeSet};
use std::fs::{self, OpenOptions};
use std::io::{self, Write};
use std::path::Path;

/// Our fixes, measured here, given to every start (and kept in the F1 menu's file).
pub const FIXED: &[(&str, &str)] = &[
    // Title buffer copies in card memory (copying back 22 -> 13 ms)
    ("VICTORY_CARD_BUFFERS_MB", "2048"),
    // A cold cache fills faster
    ("VICTORY_PIPELINE_WORKERS", "14"),
    // Their memory stack off: on, new memory was waited for 51,000 times, 18 s, 12 fps, the card 41% busy
    ("VICTORY_SLOT_EVICT", "0"),
    ("VICTORY_ALLOCATOR_TRIM", "0"),
    ("VICTORY_BUDGET_PACK", "0"),
    // The water's two tiled volumetrics (wave64 compute): on NVIDIA they left dark squares on the sea
    // where boats pass; left out, the water is clean (our patch makes the switch work in release)
    (
        "VICTORY_SKIP_PIPELINES",
        "5dc3b25986622949,6f225b641c5b0708",
    ),
];

/// A setting of the game's F1 menu, chosen in Sunline: its variable, its name (English, Arabic) and
/// its values (value, English, Arabic). The first value is ours. Left out on purpose: the ones that
/// caused problems here (ray tracing, the constants heap in place, thin lists and the like).
pub struct GameItem {
    pub var: &'static str,
    pub en: &'static str,
    pub ar: &'static str,
    pub values: &'static [(&'static str, &'static str, &'static str)],
}

#[rustfmt::skip]
pub const GAME_ITEMS: &[GameItem] = &[
    GameItem { var: "VICTORY_MENU_CONSOLE", en: "Console", ar: "وضع الجهاز", values: &[("series-x", "Series X (more detail)", "Series X (تفاصيل أكثر)"), ("series-s", "Series S (lighter)", "Series S (أخف)")] },
    GameItem { var: "VICTORY_MENU_RES", en: "Series X picture", ar: "دقة Series X", values: &[("1440", "2560x1440", "2560x1440"), ("1080to1440", "1080p to 1440p (lighter)", "من 1080p إلى 1440p (أخف)"), ("4k", "3840x2160 (heaviest)", "3840x2160 (الأثقل)")] },
    GameItem { var: "VICTORY_MENU_DLSS", en: "DLSS", ar: "DLSS", values: &[("quality", "Quality", "الجودة"), ("balanced", "Balanced", "متوازن"), ("performance", "Performance", "الأداء"), ("dlaa", "DLAA (native, heaviest)", "DLAA (بدون تصغير، الأثقل)"), ("off", "Off", "إيقاف")] },
    GameItem { var: "VICTORY_MENU_FRAMEGEN", en: "Frame generation", ar: "توليد الإطارات", values: &[("2", "2x", "2x"), ("0", "Off", "إيقاف"), ("3", "3x", "3x"), ("4", "4x", "4x")] },
    GameItem { var: "VICTORY_MENU_FG_TYPE", en: "Frame generation type", ar: "نوع توليد الإطارات", values: &[("auto", "Auto", "تلقائي"), ("dlss", "DLSS (RTX 40/50)", "DLSS (RTX 40/50)"), ("fsr", "FSR 3 (older cards)", "FSR 3 (للكروت الأقدم)")] },
    GameItem { var: "VICTORY_MENU_FPS", en: "Frame rate cap", ar: "حد معدل الإطارات", values: &[("120", "120", "120"), ("1000", "None", "بدون حد"), ("60", "60", "60"), ("30", "30 (the game's)", "30 (حد اللعبة)")] },
    GameItem { var: "VICTORY_DLSS_PRESET", en: "DLSS model", ar: "نموذج DLSS", values: &[("", "Default", "الافتراضي"), ("k", "K", "K"), ("j", "J", "J"), ("m", "M", "M"), ("l", "L", "L")] },
    GameItem { var: "VICTORY_FULLSCREEN", en: "Fullscreen (no border)", ar: "ملء الشاشة (بدون إطار)", values: &[("1", "On", "تشغيل"), ("", "Off", "إيقاف")] },
    GameItem { var: "VICTORY_FPS_LINE", en: "FPS at the top", ar: "عرض معدل الإطارات", values: &[("1", "On", "تشغيل"), ("0", "Off", "إيقاف")] },
    GameItem { var: "VICTORY_MOUSE_CAPTURE", en: "Mouse capture", ar: "حصر مؤشر الفأرة", values: &[("", "Off", "إيقاف"), ("1", "On", "تشغيل")] },
    GameItem { var: "VICTORY_IME_ENGLISH", en: "English keyboard", ar: "لوحة مفاتيح إنجليزية", values: &[("", "Off", "إيقاف"), ("1", "On", "تشغيل")] },
    GameItem { var: "VICTORY_AMBIENT", en: "Light estimate", ar: "تقدير الإضاءة", values: &[("0.8", "0.8", "0.8"), ("1", "1", "1"), ("0.5", "0.5", "0.5"), ("", "Off", "إيقاف")] },
    GameItem { var: "VICTORY_AMBIENT_SPECULAR", en: "Reflection estimate", ar: "تقدير الانعكاسات", values: &[("1", "On", "تشغيل"), ("", "Off", "إيقاف")] },
    GameItem { var: "VICTORY_AMBIENT_AIR", en: "Fog cap", ar: "حد الضباب", values: &[("1.5", "1.5", "1.5"), ("1", "1", "1"), ("2", "2", "2"), ("", "None", "بدون")] },
    GameItem { var: "VICTORY_WATER_LIGHT", en: "Water light", ar: "إضاءة الماء", values: &[("", "On", "تشغيل"), ("0", "Off", "إيقاف")] },
];

pub fn value<'a>(game: &'a BTreeMap<String, String>, var: &str) -> &'a str {
    game.get(var)
        .map(String::as_str)
        .or_else(|| {
            GAME_ITEMS
                .iter()
                .find(|item| item.var == var)
                .map(|item| item.values[0].0)
        })
        .unwrap_or_default()
}

pub fn option_visible(game: &BTreeMap<String, String>, key: &str) -> bool {
    match key {
        "VICTORY_MENU_RES" => value(game, "VICTORY_MENU_CONSOLE") == "series-x",
        "VICTORY_MENU_FG_TYPE" => value(game, "VICTORY_MENU_FRAMEGEN") != "0",
        _ => true,
    }
}

pub fn adopt(game: &mut BTreeMap<String, String>, pending: &BTreeSet<String>, text: &str) {
    for line in text.lines() {
        let Some((key, val)) = line.split_once('=') else {
            continue;
        };
        let (key, val) = (key.trim(), val.trim());
        if !pending.contains(key)
            && GAME_ITEMS.iter().any(|item| {
                item.var == key
                    && item
                        .values
                        .iter()
                        .any(|(candidate, _, _)| *candidate == val)
            })
        {
            game.insert(key.into(), val.into());
        }
    }
}

pub fn render_menu(game: &BTreeMap<String, String>, text: &str) -> String {
    let ours = GAME_ITEMS
        .iter()
        .map(|item| (item.var, value(game, item.var)))
        .chain(FIXED.iter().copied())
        .collect::<Vec<_>>();
    let mut output = Vec::new();
    for line in text.lines() {
        let key = line
            .split_once('=')
            .map(|(key, _)| key.trim())
            .unwrap_or("");
        if ours.iter().any(|(ours, _)| *ours == key) {
            continue;
        }
        output.push(line.into());
    }
    output.extend(
        ours.into_iter()
            .map(|(key, value)| format!("{key}={value}")),
    );
    output.join("\r\n") + "\r\n"
}

pub fn write_settings_file(path: &Path, contents: &str) -> io::Result<()> {
    let temporary = path.with_extension(format!("sunline-{}.tmp", std::process::id()));
    let mut file = OpenOptions::new()
        .write(true)
        .create_new(true)
        .open(&temporary)?;
    let written = file
        .write_all(contents.as_bytes())
        .and_then(|()| file.sync_all());
    drop(file);
    let result = written.and_then(|()| fs::rename(&temporary, path));
    if result.is_err() {
        let _ = fs::remove_file(&temporary);
    }
    result
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn dependent_options_follow_console_and_frame_generation_changes() {
        let mut game = BTreeMap::new();
        assert!(option_visible(&game, "VICTORY_MENU_RES"));
        assert!(option_visible(&game, "VICTORY_MENU_FG_TYPE"));
        game.insert("VICTORY_MENU_CONSOLE".into(), "series-s".into());
        game.insert("VICTORY_MENU_FRAMEGEN".into(), "0".into());
        assert!(!option_visible(&game, "VICTORY_MENU_RES"));
        assert!(!option_visible(&game, "VICTORY_MENU_FG_TYPE"));
        assert!(option_visible(&game, "VICTORY_MENU_DLSS"));
    }

    #[test]
    fn launcher_changes_survive_an_old_f1_file_and_other_f1_values_load() {
        let mut game = BTreeMap::new();
        let pending = BTreeSet::from(["VICTORY_MENU_FPS".to_string()]);
        game.insert("VICTORY_MENU_FPS".into(), "60".into());
        let text =
            "VICTORY_MENU_FPS=120\nVICTORY_MENU_DLSS=balanced\nVICTORY_MENU_CONSOLE=unknown\n";
        adopt(&mut game, &pending, text);
        assert_eq!(value(&game, "VICTORY_MENU_FPS"), "60");
        assert_eq!(value(&game, "VICTORY_MENU_DLSS"), "balanced");
        assert_eq!(value(&game, "VICTORY_MENU_CONSOLE"), "series-x");
        adopt(&mut game, &BTreeSet::new(), text);
        assert_eq!(value(&game, "VICTORY_MENU_FPS"), "120");
    }

    #[test]
    fn f1_file_keeps_user_keys_and_contains_every_menu_option_and_fix_once() {
        let game = BTreeMap::from([("VICTORY_MENU_FPS".into(), "60".into())]);
        let text =
            "# user note\nVICTORY_TEST_CUSTOM=yes\nVICTORY_MENU_FPS=120\nVICTORY_SLOT_EVICT=1\n";
        let output = render_menu(&game, text);
        assert!(output.contains("# user note"));
        assert!(output.contains("VICTORY_TEST_CUSTOM=yes"));
        assert!(output.contains("VICTORY_MENU_FPS=60"));
        for key in GAME_ITEMS
            .iter()
            .map(|item| item.var)
            .chain(FIXED.iter().map(|&(key, _)| key))
        {
            assert_eq!(
                output
                    .lines()
                    .filter(|line| line.starts_with(&format!("{key}=")))
                    .count(),
                1
            );
        }
        for &(key, expected) in FIXED {
            assert!(output
                .lines()
                .any(|line| line == format!("{key}={expected}")));
        }
        assert_eq!(render_menu(&game, &output), output);
    }

    #[test]
    fn writes_and_replaces_menu_file() -> io::Result<()> {
        let directory = std::env::temp_dir().join(format!("sunline-menu-{}", std::process::id()));
        fs::create_dir(&directory)?;
        let path = directory.join("victory-settings.txt");
        write_settings_file(&path, "VICTORY_MENU_FPS=120\r\n")?;
        write_settings_file(&path, "VICTORY_MENU_FPS=60\r\n")?;
        assert_eq!(fs::read_to_string(&path)?, "VICTORY_MENU_FPS=60\r\n");
        assert_eq!(fs::read_dir(&directory)?.count(), 1);
        fs::remove_file(path)?;
        fs::remove_dir(directory)
    }
}
