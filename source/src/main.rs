// Sunline: a launcher for VIctory builds (personal use).
// Reads the game folder only; it never writes into it. Settings live next to
// the launcher in launcher.json; logs go to the logs folder chosen below.
#![windows_subsystem = "windows"]

use eframe::egui::{self, Color32, RichText, Stroke};
use serde::{Deserialize, Serialize};
use std::collections::BTreeMap;
use std::io::Read;
use std::path::{Path, PathBuf};
use std::process::{Child, Command, Stdio};
use std::sync::{Arc, Mutex};

#[cfg(windows)]
use std::os::windows::process::CommandExt;
#[cfg(windows)]
const CREATE_NO_WINDOW: u32 = 0x0800_0000;
/// Microsoft's D3D12 Agility SDK 1.620.2 (the version the build exports in D3D12SDKVersion).
const AGILITY_URL: &str = "https://www.nuget.org/api/v2/package/Microsoft.Direct3D.D3D12/1.620.2";

#[derive(Clone, Copy, PartialEq, Eq, Serialize, Deserialize, Debug)]
enum Build {
    Main,
    New,
}

#[derive(Clone, Copy, PartialEq, Eq, Serialize, Deserialize, Debug)]
enum Gpu {
    Nvidia,
    Amd,
}

#[derive(Clone, Serialize, Deserialize, Debug)]
#[serde(default)]
struct Settings {
    game_exe: String,
    game_dir: String,
    gpu: Gpu,
    setup_done: bool,
    gpu_confirmed: bool,
    played_once: bool,
    main_exe: String,
    new_exe: String,
    logs_dir: String,
    background: String,
    build: Build,
    series_s: bool,
    output: String,
    render_scale: u32,
    port_constants: bool,
    port_descriptors: bool,
    port_cbv: bool,
    port_guard: bool,
    port_journal: bool,
    view_bytes: bool,
    own_upscaler: bool,
    port_copies: bool,
    texture_store: bool,
    texture_store_dir: String,
    lang: String,
    planes: bool,
    safe_paths: bool,
    ambient: f32,
    bvh_miss: bool,
    warm_before: bool,
    warm_after: bool,
    extra: String,
    main_cache: String,
    new_cache: String,
    dev_mode: bool,
    tests: Vec<TestProfile>,
    advanced: bool,
}

/// A developer test: the current settings plus switches of its own.
#[derive(Clone, Serialize, Deserialize, Debug, Default)]
#[serde(default)]
struct TestProfile {
    name: String,
    notes: String,
    extra: String,
}

impl Default for Settings {
    fn default() -> Self {
        let mut s = Settings {
            game_exe: String::new(),
            game_dir: String::new(),
            gpu: Gpu::Nvidia,
            setup_done: false,
            gpu_confirmed: false,
            played_once: false,
            main_exe: here(r"victory\main\target\diag\victory.exe"),
            new_exe: here(r"victory\victory.exe"),
            logs_dir: here("logs"),
            background: String::new(),
            build: Build::New,
            series_s: true,
            output: "1920x1080".into(),
            render_scale: 67,
            port_constants: false,
            port_descriptors: true,
            port_cbv: false,
            port_guard: true,
            port_journal: false,
            view_bytes: false,
            own_upscaler: false,
            port_copies: true,
            texture_store: false,
            texture_store_dir: here("texture-store"),
            lang: String::new(),
            planes: true,
            safe_paths: true,
            ambient: 0.8,
            bvh_miss: true,
            warm_before: true,
            warm_after: true,
            extra: String::new(),
            main_cache: Settings::home_cache("main"),
            new_cache: Settings::home_cache("new"),
            dev_mode: false,
            advanced: false,
            tests: vec![
                TestProfile { name: "Test 1".into(), notes: "Free slot for the next change".into(), extra: String::new() },
                TestProfile { name: "Test 2".into(), notes: "Free slot for the next change".into(), extra: String::new() },
            ],
        };
        s.recommended();
        s
    }
}

impl Settings {
    /// The set measured best so far on this PC (Ryzen 7 7700X, RTX 4070 Ti, 32 GB).
    /// Quality: the sharpest picture with the best frame rate measured.
    fn recommended(&mut self) {
        // New build (latest VIctory-7900XTX with all Belobug ports): best picture, and with
        // VICTORY_VIEW_BYTES=0 9.8 fps against 5 without it.
        self.build = Build::New;
        self.series_s = true;
        self.output = "1920x1080".into();
        self.render_scale = 67;
        self.port_constants = false; // off: 13.4-14.5 fps against 9.8 with it, picture as good
        self.port_descriptors = true;
        self.port_cbv = false; // needs patch 1
        self.port_guard = true;
        self.port_journal = true;
        self.view_bytes = false; // their own copies: 13x the new-memory waits, 5 fps
        self.own_upscaler = false; // FSR4: sharper, 13.4-14.5 fps
        self.port_copies = true; // fewer missed texture updates
        self.texture_store = false; // off: made textures flicker (two textures sharing small levels swap)
        self.planes = true; // faces with skin on Series S
        self.safe_paths = true;
        self.ambient = 0.8;
        self.bvh_miss = true;
        self.warm_before = true;
        self.warm_after = true;
    }

    /// Performance: the same as Quality, with the game's own upscaler instead of FSR4:
    /// about 16 fps instead of 13.4, the picture softer (a little blurry).
    fn recommended_fast(&mut self) {
        self.recommended();
        self.own_upscaler = true;
    }

    fn victory_exe(&self) -> &str {
        match self.build {
            Build::Main => &self.main_exe,
            Build::New => &self.new_exe,
        }
    }

    /// The VIctory cache (translated shaders, the pipeline book) of the build;
    /// empty: VIctory's own default.
    fn cache_dir(&self) -> String {
        match self.build {
            Build::Main => self.main_cache.clone(),
            Build::New => self.new_cache.clone(),
        }
    }

    fn file() -> PathBuf {
        std::env::current_exe()
            .ok()
            .and_then(|p| p.parent().map(|d| d.join("launcher.json")))
            .unwrap_or_else(|| PathBuf::from("launcher.json"))
    }

    /// Default shader cache: next to Sunline.exe, in cache\main or cache\new.
    fn home_cache(which: &str) -> String {
        std::env::current_exe()
            .ok()
            .and_then(|p| p.parent().map(|d| d.join("cache").join(which)))
            .map(|p| p.to_string_lossy().into_owned())
            .unwrap_or_default()
    }

    /// Where VIctory kept the cache before Sunline had its own.
    fn old_cache(build: Build) -> PathBuf {
        let l = std::env::var("LOCALAPPDATA").unwrap_or_default();
        match build {
            Build::Main => PathBuf::from(&l).join("victory").join("cache"),
            Build::New => PathBuf::from(&l).join("VIctory").join("cache-new"),
        }
    }

    fn load() -> Settings {
        let mut s: Settings = std::fs::read_to_string(Self::file())
            .ok()
            .and_then(|t| serde_json::from_str(&t).ok())
            .unwrap_or_default();
        // Old defaults (empty, or VIctory's own folders) move to the Sunline folder.
        let old_new = Self::old_cache(Build::New).to_string_lossy().into_owned();
        if s.main_cache.trim().is_empty() {
            s.main_cache = Self::home_cache("main");
        }
        if s.new_cache.trim().is_empty() || s.new_cache.eq_ignore_ascii_case(&old_new) {
            s.new_cache = Self::home_cache("new");
        }
        s
    }

    fn save(&self) {
        if let Ok(t) = serde_json::to_string_pretty(self) {
            let _ = std::fs::write(Self::file(), t);
        }
    }

    /// What each switch means to VIctory, for the chosen build.
    fn env(&self, test: Option<&TestProfile>) -> BTreeMap<String, String> {
        let mut e: BTreeMap<String, String> = BTreeMap::new();
        let mut set = |k: &str, v: &str| {
            e.insert(k.to_string(), v.to_string());
        };
        // The base play set (scripts\play.ps1).
        for (k, v) in [
            ("VICTORY_TEXTURE_NO_WAIT", "1"),
            ("VICTORY_RELAY", "2"),
            ("VICTORY_WIDE_BACK", "1"),
            ("VICTORY_CARD_REHASH_RUNS", "1"),
            ("VICTORY_JOIN_COPIES", "1"),
            ("VICTORY_INDIRECT_WINDOW", "1"),
            ("VICTORY_RELAY_COPY", "1"),
            ("VICTORY_CARD_STEPS", "2"),
            ("VICTORY_INDIRECT_BATCH", "2"),
            ("VICTORY_MAP_THREADS", "1"),
            ("VICTORY_TITLE_PRIORITIES", "0"),
            ("VICTORY_CARD_AWAKE", "1"),
            ("VICTORY_SHOW_AFTER", "1"),
            ("VICTORY_D3D12_DRED", "light"),
            ("VICTORY_SKIP_PIPELINES", "none"),
            ("VICTORY_CARD_BUFFERS_MIN_MB", "1"),
            ("VICTORY_CARD_BUFFERS_MB", "1536"),
            ("VICTORY_PIPELINE_WAIT_MS", "50"),
            ("VICTORY_PIPELINE_BOOK", "1"),
        ] {
            set(k, v);
        }
        if self.gpu == Gpu::Nvidia {
            // NVIDIA's view heap holds 1,000,000 views: more dynamic slots, fewer refused tables.
            set("VICTORY_DYNAMIC_SLOTS", "524288");
        }
        if self.safe_paths {
            set("VICTORY_UNDER_WIDE", "0");
            set("VICTORY_LAND_ECHO", "0");
        } else {
            set("VICTORY_THIN", "1");
            set("VICTORY_JOIN_PARTS", "8");
            set("VICTORY_UPLOAD_AHEAD", "1");
        }
        set("VICTORY_AMBIENT", &format!("{:.2}", self.ambient));
        set("VICTORY_BVH_MISS", if self.bvh_miss { "1" } else { "0" });
        let output = if self.series_s { "1920x1080".to_string() } else { self.output.clone() };
        set("VICTORY_OUTPUT", &output);
        if !self.series_s && self.render_scale != 67 {
            set("VICTORY_RENDER_SCALE", &format!("{:.2}", self.render_scale as f32 / 100.0));
        }
        set("VICTORY_PLANES", if self.planes { "1" } else { "0" });
        let b = |v: bool| if v { "1" } else { "0" };
        set("VICTORY_PORT_CONSTANTS", b(self.port_constants));
        set("VICTORY_PORT_DESCRIPTORS", b(self.port_descriptors));
        set("VICTORY_SYNC_IN_PLACE", b(self.port_descriptors));
        set("VICTORY_PORT_GUARD", b(self.port_guard));
        if !self.view_bytes {
            set("VICTORY_VIEW_BYTES", "0");
        }
        if self.own_upscaler {
            set("VICTORY_UPSCALER", "6");
        }
        if self.texture_store && self.build == Build::New && !self.texture_store_dir.trim().is_empty() {
            set("VICTORY_TEXTURE_STORE", self.texture_store_dir.trim());
        }
        if self.build == Build::New {
            set("VICTORY_PORT_CBV", b(self.port_cbv && self.port_constants));
            set("VICTORY_PORT_JOURNAL", b(self.port_journal));
            set("VICTORY_PORT_COPIES", b(self.port_copies && self.port_descriptors));
        }
        let cache = self.cache_dir();
        if !cache.trim().is_empty() {
            set("VICTORY_CACHE_DIR", cache.trim());
        }
        if !self.game_dir.trim().is_empty() {
            set("VICTORY_GAME_DIR", self.game_dir.trim());
        } else if let Some(dir) = Path::new(&self.game_exe).parent() {
            set("VICTORY_GAME_DIR", &dir.to_string_lossy());
        }
        let test_extra = test.map(|t| t.extra.clone()).unwrap_or_default();
        for pair in self.extra.split_whitespace().chain(test_extra.split_whitespace()) {
            if let Some((k, v)) = pair.split_once('=') {
                if k.starts_with("VICTORY_") {
                    set(k, v);
                }
            }
        }
        e
    }
}

#[derive(Default)]
struct Progress {
    phase: String,
    done: usize,
    total: usize,
    line: String,
    log: Vec<String>,
    running: bool,
    finished_msg: String,
}

struct App {
    s: Settings,
    progress: Arc<Mutex<Progress>>,
    child: Arc<Mutex<Option<Child>>>,
    bg: Option<egui::TextureHandle>,
    bg_loaded_for: String,
    tab: usize,
    show_startup: bool,
    detected: String,
    saved: String,
}

impl App {
    fn new(cc: &eframe::CreationContext) -> App {
        style(&cc.egui_ctx);
        fonts(&cc.egui_ctx);
        App {
            s: Settings::load(),
            progress: Arc::new(Mutex::new(Progress::default())),
            child: Arc::new(Mutex::new(None)),
            bg: None,
            bg_loaded_for: String::new(),
            tab: 0,
            show_startup: false,
            detected: String::new(),
            saved: String::new(),
        }
    }

    fn checks(&self) -> Vec<String> {
        let l = &self.s.lang;
        let mut bad = Vec::new();
        match exe_state(&self.s.game_exe, &self.s.game_dir) {
            ExeState::Ok => {}
            ExeState::Missing => bad.push(tr(l, "Game EXE not found: choose where it is", "لم يتم العثور على ملف اللعبة: اختر مكانه")),
            ExeState::Outside => bad.push(tr(l, "The game EXE is not inside the game folder", "ملف اللعبة ليس داخل مجلد اللعبة")),
        }
        if !self.s.game_dir.trim().is_empty() && !Path::new(self.s.game_dir.trim()).is_dir() {
            bad.push(tr(l, "Game folder not found", "لم يتم العثور على مجلد اللعبة"));
        }
        if !Path::new(self.s.victory_exe()).is_file() {
            bad.push(tr(l, "VIctory is not built yet", "VIctory غير جاهز بعد"));
        }
        bad
    }

    /// Runs the steps one after another on a thread: warm --used, the game,
    /// warm --used again. Only VIctory's own files and the logs are written.
    fn start(&mut self, mode: &'static str, test: Option<usize>, ctx: egui::Context) {
        self.s.save();
        let s = self.s.clone();
        let progress = self.progress.clone();
        let slot = self.child.clone();
        {
            let mut p = progress.lock().unwrap();
            *p = Progress { running: true, ..Default::default() };
        }
        std::thread::spawn(move || {
            let l = s.lang.clone();
            let exe = s.victory_exe().to_string();
            let profile = test.and_then(|i| s.tests.get(i).cloned());
            let env = s.env(profile.as_ref());
            // A source build (repo\target\diag\victory.exe) runs from its repo; the ready
            // package (Sunline\victory\victory.exe) runs from its own folder.
            let exe_dir = Path::new(&exe).parent().map(Path::to_path_buf).unwrap_or_else(|| PathBuf::from("."));
            let work = if exe.to_lowercase().contains("\\target\\") {
                exe_dir.parent().and_then(|p| p.parent()).map(Path::to_path_buf).unwrap_or_else(|| exe_dir.clone())
            } else {
                exe_dir.clone()
            };
            // Windows 10 needs Microsoft's D3D12 Agility runtime next to victory.exe:
            // fetched once from nuget.org (curl and tar ship with Windows 10).
            if !exe_dir.join("D3D12").join("D3D12Core.dll").is_file() {
                {
                    let mut p = progress.lock().unwrap();
                    p.phase = tr(&l, "Downloading Microsoft's D3D12 runtime (once, ~35 MB)", "تنزيل مكتبة D3D12 من مايكروسوفت (مرة واحدة، ~35 ميجا)");
                }
                ctx.request_repaint();
                let tmp = std::env::temp_dir().join("sunline-agility");
                let _ = std::fs::create_dir_all(&tmp);
                let pkg = tmp.join("agility.nupkg");
                let quiet = |c: &mut Command| {
                    c.creation_flags(CREATE_NO_WINDOW).stdin(Stdio::null()).stdout(Stdio::null()).stderr(Stdio::null());
                };
                let mut c = Command::new("curl");
                c.args(["-L", "-s", "-o"]).arg(&pkg).arg(AGILITY_URL);
                quiet(&mut c);
                let _ = c.status();
                let mut t = Command::new("tar");
                t.arg("-xf").arg(&pkg).arg("-C").arg(&tmp);
                quiet(&mut t);
                let _ = t.status();
                let bin = tmp.join("build").join("native").join("bin").join("x64");
                let dst = exe_dir.join("D3D12");
                let _ = std::fs::create_dir_all(&dst);
                for f in ["D3D12Core.dll", "D3D12StateObjectCompiler.dll"] {
                    let _ = std::fs::copy(bin.join(f), dst.join(f));
                }
                if !dst.join("D3D12Core.dll").is_file() {
                    let mut p = progress.lock().unwrap();
                    p.log.push(tr(&l, "Could not download the D3D12 runtime - check the internet connection and press PLAY again.", "تعذر تنزيل مكتبة D3D12 - تأكد من الإنترنت واضغط تشغيل مرة ثانية."));
                    p.running = false;
                    ctx.request_repaint();
                    return;
                }
            }
            let console = if s.series_s { "series-s" } else { "series-x" };
            let logs = PathBuf::from(&s.logs_dir);
            let _ = std::fs::create_dir_all(&logs);
            let tag = match (&profile, s.build) {
                (Some(t), _) => t.name.replace(' ', "-").to_lowercase(),
                (None, Build::Main) => "main".to_string(),
                (None, Build::New) => "new".to_string(),
            };
            let mut steps: Vec<(String, Vec<String>, Option<PathBuf>)> = Vec::new();
            // A cache folder of our own choosing gets its shaders translated once.
            let cache = s.cache_dir();
            // First use of the Sunline cache folder: bring the old cache along, so the
            // shaders and pipelines already built are not lost (no cold, black start).
            let old = Settings::old_cache(s.build);
            if !cache.trim().is_empty() && !Path::new(cache.trim()).join("shaders").is_dir() && old.join("shaders").is_dir() {
                {
                    let mut p = progress.lock().unwrap();
                    p.phase = tr(&l, "Moving the old shader cache into the Sunline folder (once)", "نقل الشيدرات القديمة إلى مجلد Sunline (مرة واحدة)");
                }
                ctx.request_repaint();
                let _ = Command::new("robocopy")
                    .arg(&old)
                    .arg(cache.trim())
                    .args(["/E", "/R:1", "/W:1", "/NFL", "/NDL", "/NJH", "/NJS"])
                    .creation_flags(CREATE_NO_WINDOW)
                    .stdin(Stdio::null())
                    .stdout(Stdio::null())
                    .stderr(Stdio::null())
                    .status();
            }
            if !cache.trim().is_empty() && !Path::new(cache.trim()).join("shaders").is_dir() {
                steps.push((tr(&l, "Translating shaders into the chosen folder (first time, a few minutes)", "ترجمة الشيدرات في المجلد المختار (أول مرة، بضع دقائق)"), vec!["shaders".into()], Some(logs.join(format!("{tag}-shaders.log")))));
            }
            match mode {
                "play" => {
                    if s.warm_before {
                        steps.push((tr(&l, "Preparing the shaders of places already played", "تجهيز شيدرات الأماكن التي لعبت فيها"), vec!["warm".into(), "--used".into()], None));
                    }
                    steps.push((
                        tr(&l, "Playing", "اللعب جارٍ"),
                        vec!["run".into(), s.game_exe.clone(), "--console".into(), console.into(), "--execute".into()],
                        Some(logs.join(format!("{tag}-run.log"))),
                    ));
                    if s.warm_after {
                        steps.push((tr(&l, "Building the shaders this session met", "بناء الشيدرات الجديدة من هذه الجلسة"), vec!["warm".into(), "--used".into()], None));
                    }
                }
                "all" => steps.push((tr(&l, "Building every shader the game ever created", "بناء كل شيدرات اللعبة"), vec!["warm".into()], None)),
                _ => {}
            }
            for (name, args, log) in steps {
                {
                    let mut p = progress.lock().unwrap();
                    p.phase = name.clone();
                    p.done = 0;
                    p.total = 0;
                    p.line.clear();
                    p.log.push(format!("== {name}"));
                }
                ctx.request_repaint();
                let mut cmd = Command::new(&exe);
                cmd.args(&args).current_dir(&work).envs(&env).stdin(Stdio::null());
                #[cfg(windows)]
                cmd.creation_flags(CREATE_NO_WINDOW);
                let piped = log.is_none();
                if let Some(path) = &log {
                    match std::fs::File::create(path) {
                        Ok(f) => {
                            let f2 = f.try_clone().ok();
                            cmd.stdout(Stdio::from(f));
                            if let Some(f2) = f2 {
                                cmd.stderr(Stdio::from(f2));
                            }
                        }
                        Err(e) => {
                            progress.lock().unwrap().log.push(format!("cannot write the log: {e}"));
                        }
                    }
                } else {
                    cmd.stdout(Stdio::piped()).stderr(Stdio::null());
                }
                let child = match cmd.spawn() {
                    Ok(c) => c,
                    Err(e) => {
                        let mut p = progress.lock().unwrap();
                        p.log.push(format!("could not start VIctory: {e}"));
                        p.running = false;
                        ctx.request_repaint();
                        return;
                    }
                };
                *slot.lock().unwrap() = Some(child);
                if piped {
                    let out = slot.lock().unwrap().as_mut().and_then(|c| c.stdout.take());
                    if let Some(mut out) = out {
                        let mut buf = [0u8; 4096];
                        let mut cur = String::new();
                        while let Ok(n) = out.read(&mut buf) {
                            if n == 0 {
                                break;
                            }
                            for ch in String::from_utf8_lossy(&buf[..n]).chars() {
                                if ch == '\r' || ch == '\n' {
                                    if !cur.trim().is_empty() {
                                        take_line(&progress, &cur);
                                        ctx.request_repaint();
                                    }
                                    cur.clear();
                                } else {
                                    cur.push(ch);
                                }
                            }
                        }
                    }
                }
                let status = loop {
                    let mut g = slot.lock().unwrap();
                    match g.as_mut() {
                        Some(c) => match c.try_wait() {
                            Ok(Some(st)) => break Some(st),
                            Ok(None) => {}
                            Err(_) => break None,
                        },
                        None => break None, // stopped by the user
                    }
                    drop(g);
                    std::thread::sleep(std::time::Duration::from_millis(300));
                };
                *slot.lock().unwrap() = None;
                let stopped = status.is_none();
                progress.lock().unwrap().log.push(match status {
                    Some(st) => format!("   ended: {st}"),
                    None => "   stopped".into(),
                });
                ctx.request_repaint();
                if stopped {
                    break;
                }
            }
            let mut p = progress.lock().unwrap();
            p.running = false;
            p.finished_msg = tr(&s.lang, "Done", "تم");
            ctx.request_repaint();
        });
    }

    fn startup(&mut self, ui: &mut egui::Ui) {
        let l = self.s.lang.clone();
        let ok_c = Color32::from_rgb(120, 230, 150);
        let bad_c = Color32::from_rgb(255, 120, 120);
        let step = |ui: &mut egui::Ui, ok: bool, title: &str, what: &str| {
            let (mark, c) = if ok { ("✔", ok_c) } else { ("●", bad_c) };
            ui.label(RichText::new(format!("{mark}  {title}")).size(18.0).strong().color(c));
            if !what.is_empty() {
                ui.label(RichText::new(what).weak());
            }
        };
        card(ui, &tr(&l, "Startup - first time only", "البداية - أول مرة فقط"), |ui| {
            ui.label(RichText::new(tr(&l, "Sunline runs your own copy of the game through VIctory. These steps check everything once. Nothing is ever written into the game folder.", "يشغّل Sunline نسختك من اللعبة عن طريق VIctory. هذه الخطوات تتحقق من كل شيء مرة واحدة. لا يكتب أي شيء داخل مجلد اللعبة أبداً.")).size(15.0));
        });

        let dir_ok = Path::new(self.s.game_dir.trim()).is_dir();
        card(ui, &tr(&l, "1 · Game folder", "1 · مجلد اللعبة"), |ui| {
            step(ui, dir_ok, &tr(&l, "Choose the game folder", "اختر مجلد اللعبة"), &tr(&l, "The folder of your game dump (the one with the game's files). VIctory reads the game's data from here.", "مجلد نسخة اللعبة (الذي فيه ملفات اللعبة). يقرأ VIctory بيانات اللعبة منه."));
            ui.horizontal(|ui| {
                ui.text_edit_singleline(&mut self.s.game_dir);
                if ui.button(tr(&l, "Choose folder…", "اختر المجلد…")).clicked() {
                    self.pick_game_dir();
                }
            });
        });

        let state = exe_state(&self.s.game_exe, &self.s.game_dir);
        let exe_ok = state == ExeState::Ok;
        card(ui, &tr(&l, "2 · Game EXE", "2 · ملف تشغيل اللعبة EXE"), |ui| {
            step(ui, exe_ok, &tr(&l, "Choose the original game EXE", "اختر ملف EXE الأصلي للعبة"), &tr(&l, "The game's own program (game_scarlett_master.exe), inside the game folder. It is never changed.", "برنامج اللعبة نفسه (game_scarlett_master.exe) داخل مجلد اللعبة. لا يتم تعديله أبداً."));
            match state {
                ExeState::Ok => {}
                ExeState::Missing => {
                    ui.label(RichText::new(tr(&l, "No game EXE found. Choose where it is:", "لم يتم العثور على ملف EXE. اختر مكانه:")).color(bad_c));
                }
                ExeState::Outside => {
                    ui.label(RichText::new(tr(&l, "This EXE is not inside the game folder. Choose the one inside it:", "هذا الملف ليس داخل مجلد اللعبة. اختر الملف الذي بداخله:")).color(bad_c));
                }
            }
            ui.horizontal(|ui| {
                ui.text_edit_singleline(&mut self.s.game_exe);
                if ui.button(tr(&l, "Choose EXE…", "اختر الملف…")).clicked() {
                    self.pick_game_exe();
                }
            });
        });

        card(ui, &tr(&l, "3 · Graphics card", "3 · كرت الشاشة"), |ui| {
            step(ui, self.s.gpu_confirmed, &tr(&l, "NVIDIA or AMD", "NVIDIA أو AMD"), &tr(&l, "Each card gets the settings that work best on it.", "كل كرت يأخذ الإعدادات الأنسب له."));
            ui.horizontal(|ui| {
                if ui.button(tr(&l, "Detect", "اكتشاف تلقائي")).clicked() {
                    self.detect();
                }
                if ui.selectable_label(self.s.gpu_confirmed && self.s.gpu == Gpu::Nvidia, "NVIDIA").clicked() {
                    self.set_gpu(Gpu::Nvidia);
                }
                if ui.selectable_label(self.s.gpu_confirmed && self.s.gpu == Gpu::Amd, "AMD").clicked() {
                    self.set_gpu(Gpu::Amd);
                }
            });
            if !self.detected.is_empty() {
                ui.label(RichText::new(&self.detected).color(ok_c));
            }
        });

        let build_ok = Path::new(self.s.victory_exe()).is_file();
        card(ui, &tr(&l, "4 · VIctory", "4 · VIctory"), |ui| {
            step(ui, build_ok, &tr(&l, "Build VIctory (one time)", "بناء VIctory (مرة واحدة)"), &tr(&l, "Builds the compatibility layer with our fixes, in a window of its own: about 10-15 minutes the first time.", "يبني طبقة التوافق مع إصلاحاتنا في نافذة مستقلة: حوالي 10 إلى 15 دقيقة أول مرة."));
            if !build_ok && ui.button(tr(&l, "Build it now", "ابنه الآن")).clicked() {
                run_setup(if self.s.build == Build::Main && Path::new(&here("setup")).join("setup-main.bat").is_file() { "setup-main.bat" } else { "setup-new.bat" });
            }
        });

        card(ui, &tr(&l, "5 · Good to know", "5 · نصائح مهمة"), |ui| {
            ui.label(tr(&l, "•  NVIDIA Control Panel > Shader Cache Size > Unlimited.", "•  لوحة تحكم NVIDIA > Shader Cache Size > Unlimited."));
            ui.label(tr(&l, "•  Windows paging file: 16 to 48 GB on an SSD.", "•  ملف الصفحات في ويندوز: من 16 إلى 48 جيجا على قرص SSD."));
            ui.label(tr(&l, "•  Close the browser and Discord while playing.", "•  أغلق المتصفح وديسكورد أثناء اللعب."));
            ui.label(tr(&l, "•  The first session after a change builds shaders: missing shapes for a minute are normal.", "•  أول جلسة بعد أي تغيير تبني الشيدرات: نقص بعض الأشكال لدقيقة أمر طبيعي."));
        });

        if ui.add(egui::Button::new(RichText::new(tr(&l, "✔  Close", "✔  إغلاق")).size(19.0).strong().color(Color32::WHITE)).fill(Color32::from_rgb(60, 170, 110)).min_size(egui::vec2(170.0, 42.0))).clicked() {
            self.s.setup_done = true;
            self.show_startup = false;
        }
    }

    fn detect(&mut self) {
        let l = self.s.lang.clone();
        match detect_gpu() {
            Some((g, name)) => {
                self.set_gpu(g);
                self.detected = format!("{} {name}", tr(&l, "Found:", "تم العثور على:"));
            }
            None => self.detected = tr(&l, "Could not tell - choose it yourself", "لم يتم التعرف عليه - اختره بنفسك"),
        }
    }

    fn set_gpu(&mut self, g: Gpu) {
        let fast = self.s.own_upscaler;
        self.s.gpu = g;
        self.s.recommended();
        self.s.own_upscaler = fast;
        self.s.gpu_confirmed = true;
    }

    /// The simple view: what is ready, and the one thing to do next.
    fn guide(&mut self, ui: &mut egui::Ui, running: bool) {
        let l = self.s.lang.clone();
        let ok_c = Color32::from_rgb(120, 230, 150);
        let todo_c = Color32::from_rgb(255, 190, 90);
        let dir_ok = Path::new(self.s.game_dir.trim()).is_dir();
        let state = exe_state(&self.s.game_exe, &self.s.game_dir);
        let exe_ok = state == ExeState::Ok;
        let gpu_ok = self.s.gpu_confirmed;
        let build_ok = Path::new(self.s.victory_exe()).is_file();
        let items = [
            (dir_ok, tr(&l, "Game folder", "مجلد اللعبة")),
            (exe_ok, tr(&l, "Game EXE", "ملف اللعبة")),
            (gpu_ok, tr(&l, "Graphics card", "كرت الشاشة")),
            (build_ok, tr(&l, "VIctory built", "VIctory جاهز")),
            (self.s.played_once, tr(&l, "First play", "أول تشغيل")),
        ];
        ui.add_space(6.0);
        card(ui, &tr(&l, "Your progress", "تقدمك"), |ui| {
            ui.horizontal_wrapped(|ui| {
                for (done, name) in &items {
                    let (mark, c) = if *done { ("✔", ok_c) } else { ("○", Color32::from_gray(150)) };
                    ui.label(RichText::new(format!("{mark} {name}")).size(16.0).color(c));
                    ui.add_space(10.0);
                }
            });
        });

        let title = |t: String| RichText::new(t).size(21.0).strong().color(todo_c);
        card(ui, &tr(&l, "Next step", "الخطوة التالية"), |ui| {
            if running {
                ui.label(title(tr(&l, "Working…", "جاري العمل…")));
                ui.label(tr(&l, "Follow the bar at the bottom. Keep Sunline open until it says Done.", "تابع الشريط في الأسفل. اترك Sunline مفتوحاً حتى يكتمل."));
            } else if !dir_ok {
                ui.label(title(tr(&l, "Choose the game folder", "اختر مجلد اللعبة")));
                ui.label(tr(&l, "The folder of your game dump - the one with the game's files.", "مجلد نسخة اللعبة - الذي فيه ملفات اللعبة."));
                if ui.button(RichText::new(tr(&l, "📁  Choose the game folder…", "📁  اختر مجلد اللعبة…")).size(17.0)).clicked() {
                    self.pick_game_dir();
                }
            } else if !exe_ok {
                ui.label(title(tr(&l, "Choose the game EXE", "اختر ملف اللعبة EXE")));
                ui.label(match state {
                    ExeState::Outside => tr(&l, "The EXE you chose is not inside the game folder. Choose the one inside it.", "الملف الذي اخترته ليس داخل مجلد اللعبة. اختر الملف الموجود بداخله."),
                    _ => tr(&l, "No game EXE was found in the game folder. Choose where it is.", "لم يتم العثور على ملف اللعبة داخل المجلد. اختر مكانه."),
                });
                if ui.button(RichText::new(tr(&l, "🎮  Choose the game EXE…", "🎮  اختر ملف اللعبة…")).size(17.0)).clicked() {
                    self.pick_game_exe();
                }
            } else if !gpu_ok {
                ui.label(title(tr(&l, "Tell Sunline your graphics card", "حدد كرت الشاشة")));
                ui.label(tr(&l, "Each card gets the settings that work best on it.", "كل كرت يأخذ الإعدادات الأنسب له."));
                ui.horizontal(|ui| {
                    if ui.button(RichText::new(tr(&l, "🔍  Detect", "🔍  اكتشاف تلقائي")).size(17.0)).clicked() {
                        self.detect();
                    }
                    if ui.button("NVIDIA").clicked() {
                        self.set_gpu(Gpu::Nvidia);
                    }
                    if ui.button("AMD").clicked() {
                        self.set_gpu(Gpu::Amd);
                    }
                });
            } else if !build_ok {
                ui.label(title(tr(&l, "Build VIctory (one time)", "بناء VIctory (مرة واحدة)")));
                ui.label(tr(&l, "A window opens and builds it: about 10-15 minutes. Come back when it says the build is ready.", "تفتح نافذة وتبنيه: حوالي 10 إلى 15 دقيقة. ارجع هنا عندما تقول إن البناء جاهز."));
                if ui.button(RichText::new(tr(&l, "🔧  Build it now", "🔧  ابنه الآن")).size(17.0)).clicked() {
                    run_setup(if self.s.build == Build::Main && Path::new(&here("setup")).join("setup-main.bat").is_file() { "setup-main.bat" } else { "setup-new.bat" });
                }
            } else {
                if self.s.played_once {
                    ui.label(RichText::new(tr(&l, "Ready - press  ▶ PLAY", "جاهز - اضغط  ▶ تشغيل")).size(21.0).strong().color(ok_c));
                } else {
                    ui.label(title(tr(&l, "Press  ▶ PLAY", "اضغط  ▶ تشغيل")));
                    ui.label(tr(&l, "The first time, Sunline prepares the shaders (the bar at the bottom), then the game opens.", "في المرة الأولى يجهز Sunline الشيدرات (الشريط في الأسفل) ثم تفتح اللعبة."));
                    ui.label(tr(&l, "When you close the game, leave Sunline open until the bar is done.", "عند إغلاق اللعبة اترك Sunline مفتوحاً حتى ينتهي الشريط."));
                }
                ui.add_space(6.0);
                ui.label(RichText::new(tr(&l, "Mode", "الوضع")).strong());
                ui.radio_value(&mut self.s.own_upscaler, false, tr(&l, "★ Quality - sharp picture (13 to 14 fps)", "★ الجودة - صورة حادة (13 إلى 14 فريم)"));
                ui.radio_value(&mut self.s.own_upscaler, true, tr(&l, "★ Performance - about 16 fps, picture a little blurry", "★ الأداء - حوالي 16 فريم، الصورة فيها بعض التغبيش"));
            }
        });

        card(ui, &tr(&l, "Tips", "نصائح"), |ui| {
            ui.label(tr(&l, "•  NVIDIA Control Panel > Shader Cache Size > Unlimited.", "•  لوحة تحكم NVIDIA > Shader Cache Size > Unlimited."));
            ui.label(tr(&l, "•  Paging file 16 to 48 GB on an SSD; close the browser and Discord while playing.", "•  ملف الصفحات من 16 إلى 48 جيجا على SSD، وأغلق المتصفح وديسكورد أثناء اللعب."));
            ui.label(tr(&l, "•  Not sure what to do?  🚀 Startup  shows every step.", "•  لست متأكداً؟  🚀 البداية  تشرح كل خطوة."));
        });
    }

    fn pick_game_dir(&mut self) {
        if let Some(p) = rfd::FileDialog::new().pick_folder() {
            self.s.game_dir = p.to_string_lossy().into_owned();
            if exe_state(&self.s.game_exe, &self.s.game_dir) != ExeState::Ok {
                match find_exe(&p) {
                    Some(e) => self.s.game_exe = e.to_string_lossy().into_owned(),
                    None => self.s.game_exe.clear(),
                }
            }
        }
    }

    fn pick_game_exe(&mut self) {
        let mut d = rfd::FileDialog::new().add_filter("Game EXE", &["exe"]);
        if Path::new(self.s.game_dir.trim()).is_dir() {
            d = d.set_directory(self.s.game_dir.trim());
        }
        if let Some(p) = d.pick_file() {
            self.s.game_exe = p.to_string_lossy().into_owned();
            if exe_state(&self.s.game_exe, &self.s.game_dir) == ExeState::Outside {
                if let Some(parent) = p.parent() {
                    self.s.game_dir = parent.to_string_lossy().into_owned();
                }
            }
        }
    }

    fn stop(&mut self) {
        if let Some(mut c) = self.child.lock().unwrap().take() {
            let _ = c.kill();
        }
    }

    fn load_bg(&mut self, ctx: &egui::Context) {
        if self.bg_loaded_for == self.s.background {
            return;
        }
        self.bg_loaded_for = self.s.background.clone();
        self.bg = None;
        if self.s.background.is_empty() {
            return;
        }
        if let Ok(img) = image::open(&self.s.background) {
            let img = img.to_rgba8();
            let size = [img.width() as usize, img.height() as usize];
            let c = egui::ColorImage::from_rgba_unmultiplied(size, img.as_raw());
            self.bg = Some(ctx.load_texture("bg", c, egui::TextureOptions::LINEAR));
        }
    }
}

/// English or Arabic; Arabic is shaped (joined letters) and laid out right to left,
/// which egui does not do by itself.
fn tr(lang: &str, en: &str, ar: &str) -> String {
    if lang != "ar" {
        return en.to_string();
    }
    let shaped = ar_reshaper::ArabicReshaper::default().reshape(ar);
    let bidi = unicode_bidi::BidiInfo::new(&shaped, Some(unicode_bidi::Level::rtl()));
    bidi.paragraphs
        .iter()
        .map(|para| bidi.reorder_line(para, para.range.clone()).into_owned())
        .collect::<Vec<_>>()
        .join("\n")
}

/// Where the game EXE stands against the game folder.
#[derive(PartialEq)]
enum ExeState {
    Ok,
    Missing,
    Outside,
}

fn exe_state(exe: &str, dir: &str) -> ExeState {
    let e = Path::new(exe.trim());
    if exe.trim().is_empty() || !e.is_file() {
        return ExeState::Missing;
    }
    let d = dir.trim();
    if d.is_empty() {
        return ExeState::Ok;
    }
    let norm = |p: &Path| p.to_string_lossy().replace('/', "\\").trim_end_matches('\\').to_lowercase();
    let (e, d) = (norm(e), norm(Path::new(d)));
    if e.starts_with(&format!("{d}\\")) { ExeState::Ok } else { ExeState::Outside }
}

/// The game EXE in a folder, if there is one (the known name first, then any .exe at its top).
fn find_exe(dir: &Path) -> Option<PathBuf> {
    let known = dir.join("game_scarlett_master.exe");
    if known.is_file() {
        return Some(known);
    }
    std::fs::read_dir(dir).ok()?.filter_map(Result::ok).map(|e| e.path()).find(|p| {
        p.extension().is_some_and(|x| x.eq_ignore_ascii_case("exe"))
            && p.file_name().is_some_and(|n| n.to_string_lossy().to_lowercase().starts_with("game"))
    })
}

/// The graphics cards Windows lists; NVIDIA first if there is one (an AMD
/// processor's own graphics is listed too on many PCs).
fn detect_gpu() -> Option<(Gpu, String)> {
    let mut cmd = Command::new("powershell");
    cmd.args(["-NoProfile", "-Command", "(Get-CimInstance Win32_VideoController).Name"]);
    #[cfg(windows)]
    cmd.creation_flags(CREATE_NO_WINDOW);
    let out = cmd.output().ok()?;
    let names = String::from_utf8_lossy(&out.stdout).to_string();
    let lines: Vec<&str> = names.lines().map(str::trim).filter(|l| !l.is_empty()).collect();
    if let Some(n) = lines.iter().find(|l| l.to_lowercase().contains("nvidia")) {
        return Some((Gpu::Nvidia, n.to_string()));
    }
    if let Some(n) = lines.iter().find(|l| { let l = l.to_lowercase(); l.contains("amd") || l.contains("radeon") }) {
        return Some((Gpu::Amd, n.to_string()));
    }
    None
}

/// A path inside the Sunline folder (next to Sunline.exe), so the folder can live anywhere.
fn here(rel: &str) -> String {
    std::env::current_exe()
        .ok()
        .and_then(|p| p.parent().map(|d| d.join(rel)))
        .map(|p| p.to_string_lossy().into_owned())
        .unwrap_or_else(|| rel.to_string())
}

fn run_setup(file: &str) {
    if let Some(dir) = std::env::current_exe().ok().and_then(|p| p.parent().map(Path::to_path_buf)) {
        let bat = dir.join("setup").join(file);
        let _ = Command::new("cmd").arg("/c").arg("start").arg("").arg(bat).spawn();
    }
}

fn take_line(progress: &Arc<Mutex<Progress>>, line: &str) {
    let mut p = progress.lock().unwrap();
    let t = line.trim();
    // "[####----] 1234/3388 36.4%  left ..." from the progress bar
    if let Some(rest) = t.strip_prefix('[') {
        if let Some((_, after)) = rest.split_once("] ") {
            if let Some((d, tot)) = after.split_whitespace().next().and_then(|f| f.split_once('/')) {
                p.done = d.parse().unwrap_or(p.done);
                p.total = tot.parse().unwrap_or(p.total);
            }
            p.line = after.to_string();
            return;
        }
    }
    // "N pipelines known, M made before (..), K bad; T to make, J processes"
    if t.contains("pipelines known") {
        if let Some(to) = t.split("; ").nth(1).and_then(|x| x.split_whitespace().next()) {
            p.total = to.parse().unwrap_or(0);
        }
    }
    p.log.push(format!("   {t}"));
    if p.log.len() > 300 {
        p.log.drain(..100);
    }
}

fn fonts(ctx: &egui::Context) {
    let mut defs = egui::FontDefinitions::default();
    let windir = std::env::var("WINDIR").unwrap_or_else(|_| r"C:\Windows".into());
    for name in ["segoeui.ttf", "tahoma.ttf", "arial.ttf"] {
        if let Ok(bytes) = std::fs::read(Path::new(&windir).join("Fonts").join(name)) {
            defs.font_data.insert("arabic".into(), egui::FontData::from_owned(bytes));
            for fam in [egui::FontFamily::Proportional, egui::FontFamily::Monospace] {
                defs.families.entry(fam).or_default().push("arabic".into());
            }
            break;
        }
    }
    ctx.set_fonts(defs);
}

fn style(ctx: &egui::Context) {
    let mut v = egui::Visuals::dark();
    let accent = Color32::from_rgb(255, 92, 160);
    v.panel_fill = Color32::from_rgb(14, 12, 24);
    v.window_fill = Color32::from_rgb(22, 18, 36);
    v.extreme_bg_color = Color32::from_rgb(10, 8, 18);
    v.widgets.inactive.bg_fill = Color32::from_rgb(36, 30, 58);
    v.widgets.inactive.weak_bg_fill = Color32::from_rgb(36, 30, 58);
    v.widgets.hovered.bg_fill = Color32::from_rgb(60, 44, 92);
    v.widgets.hovered.weak_bg_fill = Color32::from_rgb(60, 44, 92);
    v.widgets.active.bg_fill = accent;
    v.selection.bg_fill = accent;
    v.selection.stroke = Stroke::new(1.0_f32, Color32::WHITE);
    v.hyperlink_color = Color32::from_rgb(255, 170, 90);
    ctx.set_visuals(v);
    let mut st = (*ctx.style()).clone();
    st.spacing.item_spacing = egui::vec2(10.0, 8.0);
    st.spacing.button_padding = egui::vec2(14.0, 7.0);
    for (_, f) in st.text_styles.iter_mut() {
        f.size *= 1.12;
    }
    ctx.set_style(st);
}

fn card(ui: &mut egui::Ui, title: &str, add: impl FnOnce(&mut egui::Ui)) {
    egui::Frame::none()
        .fill(Color32::from_rgba_unmultiplied(28, 22, 46, 230))
        .rounding(12.0)
        .stroke(Stroke::new(1.0_f32, Color32::from_rgb(70, 52, 110)))
        .inner_margin(14.0)
        .show(ui, |ui| {
            ui.set_min_width(ui.available_width());
            ui.label(RichText::new(title).strong().size(17.0).color(Color32::from_rgb(255, 170, 90)));
            ui.add_space(4.0);
            add(ui);
        });
    ui.add_space(8.0);
}

fn pick_file(target: &mut String, what: &str, ext: &[&str]) {
    let mut d = rfd::FileDialog::new().add_filter(what, ext);
    if let Some(dir) = Path::new(target.as_str()).parent() {
        d = d.set_directory(dir);
    }
    if let Some(p) = d.pick_file() {
        *target = p.to_string_lossy().into_owned();
    }
}

impl eframe::App for App {
    fn update(&mut self, ctx: &egui::Context, _frame: &mut eframe::Frame) {
        self.load_bg(ctx);
        let running = self.progress.lock().unwrap().running;

        egui::TopBottomPanel::top("top").frame(egui::Frame::none().fill(Color32::from_rgb(14, 12, 24)).inner_margin(egui::Margin { left: 12.0, right: 12.0, top: 12.0, bottom: 6.0 })).show(ctx, |ui| {
            // Header: the user's own picture if chosen, else a gradient band.
            let (rect, _) = ui.allocate_exact_size(egui::vec2(ui.available_width(), 150.0), egui::Sense::hover());
            if let Some(t) = &self.bg {
                let ts = t.size_vec2();
                let scale = (rect.width() / ts.x).max(rect.height() / ts.y);
                let shown = egui::vec2(rect.width() / (ts.x * scale), rect.height() / (ts.y * scale));
                let uv = egui::Rect::from_center_size(egui::pos2(0.5, 0.5), shown);
                ui.painter().image(t.id(), rect, uv, Color32::WHITE);
            } else {
                let steps = 40;
                for i in 0..steps {
                    let f = i as f32 / steps as f32;
                    let c = Color32::from_rgb(
                        (40.0 + 215.0 * f) as u8,
                        (20.0 + 72.0 * f) as u8,
                        (90.0 + 70.0 * (1.0 - f)) as u8,
                    );
                    let x0 = rect.left() + rect.width() * f;
                    let r = egui::Rect::from_min_max(
                        egui::pos2(x0, rect.top()),
                        egui::pos2(x0 + rect.width() / steps as f32 + 1.0, rect.bottom()),
                    );
                    ui.painter().rect_filled(r, 0.0, c);
                }
            }
            ui.painter().rect_filled(
                egui::Rect::from_min_max(egui::pos2(rect.left(), rect.bottom() - 56.0), rect.right_bottom()),
                0.0,
                Color32::from_black_alpha(150),
            );
            ui.painter().text(
                egui::pos2(rect.left() + 18.0, rect.bottom() - 30.0),
                egui::Align2::LEFT_CENTER,
                "SUNLINE",
                egui::FontId::proportional(30.0),
                Color32::WHITE,
            );
            ui.painter().text(
                egui::pos2(rect.right() - 18.0, rect.bottom() - 30.0),
                egui::Align2::RIGHT_CENTER,
                "for VIctory  ·  NVIDIA build",
                egui::FontId::proportional(15.0),
                Color32::from_gray(210),
            );
            ui.add_space(8.0);

            ui.horizontal(|ui| {
                let l = self.s.lang.clone();
                if ui.button(RichText::new(tr(&l, "🚀  Startup", "🚀  البداية")).size(16.0)).clicked() {
                    self.show_startup = !self.show_startup;
                }
                let label = if self.s.advanced { tr(&l, "◀  Simple view", "◀  العرض البسيط") } else { tr(&l, "⚙  Settings", "⚙  الإعدادات") };
                if ui.button(RichText::new(label).size(16.0)).clicked() {
                    self.s.advanced = !self.s.advanced;
                    self.s.save();
                }
                let other = if l == "ar" { "English" } else { "العربية" };
                if ui.button(RichText::new(format!("🌐 {}", tr(&l, other, other))).size(16.0)).clicked() {
                    self.s.lang = if l == "ar" { "en".into() } else { "ar".into() };
                }
                if !self.s.advanced || self.show_startup {
                    return;
                }
                let mut names = vec!["Play", "Patches", "Graphics", "Files"];
                if self.s.dev_mode {
                    names.push("Tests");
                }
                if self.tab >= names.len() {
                    self.tab = 0;
                }
                for (i, name) in names.iter().enumerate() {
                    if ui.selectable_label(self.tab == i, RichText::new(*name).size(17.0)).clicked() {
                        self.tab = i;
                    }
                }
                ui.with_layout(egui::Layout::right_to_left(egui::Align::Center), |ui| {
                    if ui.button(tr(&l, "★ Performance", "★ الأداء")).on_hover_text(tr(&l, "About 16 fps instead of 13 to 14: the game's own upscaler instead of FSR4. The picture is a little blurry.", "حوالي 16 فريم بدل 13 إلى 14: مكبّر اللعبة بدل FSR4. الصورة فيها بعض التغبيش.")).clicked() {
                        self.s.recommended_fast();
                    }
                    if ui.button(tr(&l, "★ Quality", "★ الجودة")).on_hover_text(tr(&l, "The sharpest picture with the best frame rate measured (13 to 14 fps)", "أوضح صورة مع أفضل أداء تم قياسه (13 إلى 14 فريم)")).clicked() {
                        self.s.recommended();
                    }
                });
            });
            ui.separator();
        });
        egui::TopBottomPanel::bottom("bottom").frame(egui::Frame::none().fill(Color32::from_rgb(18, 14, 30)).inner_margin(12.0)).show(ctx, |ui| {
            {
                let p = self.progress.lock().unwrap();
                if p.running || !p.phase.is_empty() {
                    let frac = if p.total > 0 { p.done as f32 / p.total as f32 } else { 0.0 };
                    let text = if p.total > 0 {
                        format!("{}  —  {}/{}  ({:.1}%)", p.phase, p.done, p.total, frac * 100.0)
                    } else {
                        p.phase.clone()
                    };
                    ui.add(egui::ProgressBar::new(frac).text(text).animate(p.running && p.total == 0));
                    if !p.line.is_empty() {
                        ui.label(RichText::new(&p.line).monospace().size(12.0).weak());
                    }
                    if !p.running && !p.finished_msg.is_empty() {
                        ui.label(RichText::new(&p.finished_msg).color(Color32::from_rgb(120, 230, 150)));
                    }
                    ui.add_space(6.0);
                }
            }
            let problems = self.checks();
            let l = self.s.lang.clone();
            ui.horizontal(|ui| {
                if running {
                    if ui.add(egui::Button::new(RichText::new(tr(&l, "■  STOP", "■  إيقاف")).size(20.0)).min_size(egui::vec2(160.0, 44.0))).clicked() {
                        self.stop();
                    }
                } else {
                    let play = egui::Button::new(RichText::new(tr(&l, "▶  PLAY", "▶  تشغيل")).size(22.0).strong().color(Color32::WHITE))
                        .fill(Color32::from_rgb(230, 70, 140))
                        .min_size(egui::vec2(200.0, 48.0));
                    if ui.add_enabled(problems.is_empty(), play).clicked() {
                        self.s.played_once = true;
                        self.start("play", None, ctx.clone());
                    }
                    if ui
                        .add_enabled(problems.is_empty(), egui::Button::new(tr(&l, "Build all shaders", "بناء كل الشيدرات")).min_size(egui::vec2(160.0, 48.0)))
                        .on_hover_text(tr(&l, "Every shader the game ever created: hours the first time, resumes where it stopped", "كل شيدرات اللعبة: ساعات في المرة الأولى، ويكمل من حيث توقف"))
                        .clicked()
                    {
                        self.start("all", None, ctx.clone());
                    }
                }
            });
            for p in problems {
                ui.label(RichText::new(format!("⚠ {p}")).color(Color32::from_rgb(255, 180, 80)));
            }
        });
        egui::CentralPanel::default().show(ctx, |ui| {
            if self.s.lang.is_empty() {
                ui.add_space(30.0);
                card(ui, "Language  ·  اللغة", |ui| {
                    ui.label(RichText::new("Choose your language").size(18.0));
                    ui.label(RichText::new(tr("ar", "", "اختر لغتك")).size(18.0));
                    ui.add_space(8.0);
                    ui.horizontal(|ui| {
                        if ui.add(egui::Button::new(RichText::new(tr("ar", "", "العربية")).size(22.0)).min_size(egui::vec2(180.0, 56.0))).clicked() {
                            self.s.lang = "ar".into();
                        }
                        if ui.add(egui::Button::new(RichText::new("English").size(22.0)).min_size(egui::vec2(180.0, 56.0))).clicked() {
                            self.s.lang = "en".into();
                        }
                    });
                });
                return;
            }
            egui::ScrollArea::vertical().auto_shrink([false, false]).show(ui, |ui| {
            if self.show_startup {
                self.startup(ui);
            } else if !self.s.advanced {
                self.guide(ui, running);
            } else {
            {
                ui.add_enabled_ui(!running, |ui| match self.tab {
                    0 => {
                        card(ui, "Build", |ui| {
                            ui.radio_value(&mut self.s.build, Build::Main, "Old (VIctory-7900XTX first build + our fixes) - faster, some flicker");
                            ui.radio_value(&mut self.s.build, Build::New, "New (latest 7900XTX + all Belobug ports) - best picture, recommended");
                        });
                        card(ui, "Console profile", |ui| {
                            ui.radio_value(&mut self.s.series_s, true, "Series S — faces with skin, FSR from 720p (recommended)");
                            ui.radio_value(&mut self.s.series_s, false, "Series X — more detail, heavier");
                        });
                        card(ui, "Shaders", |ui| {
                            ui.checkbox(&mut self.s.warm_before, "Build shaders of places already played before the game starts");
                            ui.checkbox(&mut self.s.warm_after, "Build the shaders the session met after the game closes");
                            ui.label(RichText::new("Building while playing costs about 1.5 fps, so it is not done.").weak());
                        });
                    }
                    1 => {
                        card(ui, "Renderer patches (from Belobug)", |ui| {
                            ui.checkbox(&mut self.s.port_descriptors, "Patch 2 · DESCRIPTORS — the game reports changed slots (+2 fps) ✔");
                            ui.checkbox(&mut self.s.port_guard, "Patch 6 · GUARD — shared lock for memory guards (+1.3 fps) ✔");
                            ui.checkbox(&mut self.s.port_constants, "Patch 1 · CONSTANTS — card reads game memory in place (off is faster: 13.4 vs 9.8 fps, card 96% with it)");
                            ui.add_enabled_ui(self.s.build == Build::New && self.s.port_constants, |ui| {
                                ui.checkbox(&mut self.s.port_cbv, "Patch 5 · CBV — shaders read constants in place (needs patch 1, new build)");
                            });
                            ui.add_enabled_ui(self.s.build == Build::New && self.s.port_descriptors, |ui| {
                                ui.checkbox(&mut self.s.port_copies, "Patch 2 + table copies — fewer missed texture updates (new build) ✔");
                            });
                            ui.add_enabled_ui(self.s.build == Build::New, |ui| {
                                ui.checkbox(&mut self.s.port_journal, "Patch 7 · JOURNAL — fewer lock waits at submit (new build)");
                            });
                        });
                        card(ui, "Other fixes", |ui| {
                            ui.checkbox(&mut self.s.view_bytes, "Own copies of off-element buffer views (off = 9.8 fps instead of 5 on the new build)");
                            ui.checkbox(&mut self.s.planes, "Two-plane compose (Series S faces)");
                            ui.checkbox(&mut self.s.bvh_miss, "Skip ray walks (ray tracing off)");
                            ui.checkbox(&mut self.s.safe_paths, "Safe paths (off = the fast set that filled the card)");
                        });
                    }
                    2 => {
                        card(ui, "Resolution", |ui| {
                            if self.s.series_s {
                                ui.label("Series S: output stays 1920x1080 (its FSR4 only has 1080p tables; 720p crashed). It draws 1280x720 inside.");
                            } else {
                                egui::ComboBox::from_label("Output")
                                    .selected_text(self.s.output.clone())
                                    .show_ui(ui, |ui| {
                                        for m in ["1280x720", "1664x936", "1920x1080", "2560x1440"] {
                                            ui.selectable_value(&mut self.s.output, m.to_string(), m);
                                        }
                                    });
                                ui.add(egui::Slider::new(&mut self.s.render_scale, 50..=100).suffix(" %").text("Render scale (67 = the game's own)"));
                                ui.label(RichText::new("Above ~85% the 12 GB card filled up and draws were dropped.").weak());
                            }
                        });
                        card(ui, "Upscaler", |ui| {
                            ui.radio_value(&mut self.s.own_upscaler, false, "FSR4 - sharper picture (13-14 fps)");
                            ui.radio_value(&mut self.s.own_upscaler, true, "The game's own - about 16 fps, picture softer / a little blurry");
                        });
                        card(ui, "Texture store", |ui| {
                            ui.checkbox(&mut self.s.texture_store, "Keep sharp textures on disk (new build): places seen before are sharp at once. Uses RAM and disk.");
                            ui.horizontal(|ui| {
                                ui.label("Folder");
                                ui.text_edit_singleline(&mut self.s.texture_store_dir);
                            });
                        });
                        card(ui, "Lighting", |ui| {
                            ui.add(egui::Slider::new(&mut self.s.ambient, 0.4..=1.5).text("Indirect light stand-in"));
                        });
                        card(ui, "Advanced", |ui| {
                            ui.label("Extra switches (VICTORY_NAME=value, separated by spaces):");
                            ui.text_edit_singleline(&mut self.s.extra);
                        });
                    }
                    3 => {
                        card(ui, "Game", |ui| {
                            ui.horizontal(|ui| {
                                ui.label("Folder");
                                ui.text_edit_singleline(&mut self.s.game_dir);
                                if ui.button("Choose…").clicked() {
                                    if let Some(p) = rfd::FileDialog::new().pick_folder() {
                                        self.s.game_dir = p.to_string_lossy().into_owned();
                                    }
                                }
                            });
                            ui.horizontal(|ui| {
                                ui.label("EXE   ");
                                ui.text_edit_singleline(&mut self.s.game_exe);
                                if ui.button("Browse…").clicked() {
                                    pick_file(&mut self.s.game_exe, "Game", &["exe"]);
                                }
                            });
                            ui.label(RichText::new("Read only — the launcher never writes into the game folder.").weak());
                        });
                        card(ui, "Set up / update the builds", |ui| {
                            ui.label(RichText::new("Applies our patches and builds VIctory in a console window (minutes). Run once, and after an update.").weak());
                            ui.horizontal(|ui| {
                                for (label, file) in [("Set up Main build", "setup-main.bat"), ("Set up New build", "setup-new.bat")] {
                                    if !Path::new(&here("setup")).join(file).is_file() {
                                        continue;
                                    }
                                    if ui.button(label).clicked() {
                                        if let Some(dir) = std::env::current_exe().ok().and_then(|p| p.parent().map(Path::to_path_buf)) {
                                            let bat = dir.join("setup").join(file);
                                            let _ = Command::new("cmd").arg("/c").arg("start").arg("").arg(bat).spawn();
                                        }
                                    }
                                }
                            });
                        });
                        card(ui, "VIctory builds", |ui| {
                            ui.horizontal(|ui| {
                                ui.label("Main");
                                ui.text_edit_singleline(&mut self.s.main_exe);
                                if ui.button("Browse…").clicked() {
                                    pick_file(&mut self.s.main_exe, "victory.exe", &["exe"]);
                                }
                            });
                            ui.horizontal(|ui| {
                                ui.label("New ");
                                ui.text_edit_singleline(&mut self.s.new_exe);
                                if ui.button("Browse…").clicked() {
                                    pick_file(&mut self.s.new_exe, "victory.exe", &["exe"]);
                                }
                            });
                        });
                        card(ui, "Look", |ui| {
                            ui.horizontal(|ui| {
                                ui.label("Header picture (your own image):");
                                ui.text_edit_singleline(&mut self.s.background);
                                if ui.button("Browse…").clicked() {
                                    pick_file(&mut self.s.background, "Image", &["png", "jpg", "jpeg"]);
                                }
                            });
                        });
                        card(ui, "Shader folders (VIctory cache)", |ui| {
                            ui.label(RichText::new("Translated shaders and the list of pipelines are kept here. Empty = VIctory's default. A new folder is filled on the next PLAY (a few minutes, then the first session builds again).").weak());
                            ui.horizontal(|ui| {
                                ui.label("Main");
                                ui.text_edit_singleline(&mut self.s.main_cache);
                                if ui.button("Choose…").clicked() {
                                    if let Some(p) = rfd::FileDialog::new().pick_folder() {
                                        self.s.main_cache = p.to_string_lossy().into_owned();
                                    }
                                }
                            });
                            ui.horizontal(|ui| {
                                ui.label("New ");
                                ui.text_edit_singleline(&mut self.s.new_cache);
                                if ui.button("Choose…").clicked() {
                                    if let Some(p) = rfd::FileDialog::new().pick_folder() {
                                        self.s.new_cache = p.to_string_lossy().into_owned();
                                    }
                                }
                            });
                            ui.label(RichText::new("The compiled shaders themselves are kept by the NVIDIA driver: set Shader Cache Size to Unlimited in the NVIDIA Control Panel.").weak());
                        });
                        card(ui, "Logs", |ui| {
                            ui.text_edit_singleline(&mut self.s.logs_dir);
                            if ui.button("Open logs folder").clicked() {
                                let _ = std::fs::create_dir_all(&self.s.logs_dir);
                                let _ = Command::new("explorer").arg(&self.s.logs_dir).spawn();
                            }
                        });
                        card(ui, "Developer", |ui| {
                            ui.checkbox(&mut self.s.dev_mode, "Developer mode — show the Tests menu (Test 1, Test 2, …)");
                        });
                    }
                    _ => {
                        card(ui, "Developer tests", |ui| {
                            ui.label(RichText::new("Each test runs your current settings plus its own switches, with its own log (test-1-run.log, …). New changes are tried here before they go into the normal settings.").weak());
                        });
                        let mut run: Option<usize> = None;
                        let mut remove: Option<usize> = None;
                        let count = self.s.tests.len();
                        for i in 0..count {
                            let t = &mut self.s.tests[i];
                            card(ui, &t.name.clone(), |ui| {
                                ui.horizontal(|ui| {
                                    ui.label("Name");
                                    ui.text_edit_singleline(&mut t.name);
                                });
                                ui.horizontal(|ui| {
                                    ui.label("Notes");
                                    ui.text_edit_singleline(&mut t.notes);
                                });
                                ui.label("Switches (VICTORY_NAME=value, separated by spaces):");
                                ui.add(egui::TextEdit::multiline(&mut t.extra).desired_rows(2).desired_width(f32::INFINITY).font(egui::TextStyle::Monospace));
                                ui.horizontal(|ui| {
                                    if ui.add(egui::Button::new(RichText::new(format!("▶  Run {}", t.name)).strong()).fill(Color32::from_rgb(120, 60, 200))).clicked() {
                                        run = Some(i);
                                    }
                                    if count > 1 && ui.button("Remove").clicked() {
                                        remove = Some(i);
                                    }
                                });
                            });
                        }
                        if ui.button("+ Add a test").clicked() {
                            let n = self.s.tests.len() + 1;
                            self.s.tests.push(TestProfile { name: format!("Test {n}"), ..Default::default() });
                        }
                        if let Some(i) = remove {
                            self.s.tests.remove(i);
                        }
                        if let Some(i) = run {
                            if self.checks().is_empty() {
                                self.start("play", Some(i), ctx.clone());
                            }
                        }
                    }
                });
            }
            }
            });
        });
        if running {
            ctx.request_repaint_after(std::time::Duration::from_millis(250));
        }
        // Save whenever something changed.
        if let Ok(now) = serde_json::to_string(&self.s) {
            if now != self.saved {
                self.s.save();
                self.saved = now;
            }
        }
    }

    fn on_exit(&mut self, _gl: Option<&eframe::glow::Context>) {
        self.s.save();
    }
}

fn icon() -> egui::IconData {
    let img = image::load_from_memory(include_bytes!("../assets/icon256.png"))
        .map(|i| i.to_rgba8())
        .unwrap_or_else(|_| image::RgbaImage::new(1, 1));
    egui::IconData { width: img.width(), height: img.height(), rgba: img.into_raw() }
}

fn main() -> eframe::Result<()> {
    let opts = eframe::NativeOptions {
        viewport: egui::ViewportBuilder::default()
            .with_inner_size([900.0, 760.0])
            .with_min_inner_size([760.0, 600.0])
            .with_title("Sunline")
            .with_icon(icon()),
        ..Default::default()
    };
    eframe::run_native("Sunline", opts, Box::new(|cc| Ok(Box::new(App::new(cc)))))
}
