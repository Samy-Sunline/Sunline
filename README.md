# Sunline

A simple Windows launcher for **VIctory** (Xbox Series compatibility layer), built around
**[VIctory-NVIDIA](https://github.com/trustno1-coder/VIctory-NVIDIA)** (on the
[foxhound](https://github.com/readmet3xt-lab/foxhound) line): **DLSS**, a 2560x1440 picture and DLSS frame generation.
**Download → choose your game folder → PLAY.**

> ⚠️ Experimental. Measured on one PC (Windows 10, Ryzen 7 7700X, RTX 4070 Ti, 32 GB) with commit `63b902f`: about
> **16-19 fps** rendered in the world (the card ~96% busy; the picture is complete, faces and eyes included),
> DLSS Quality at 2560x1440, no crash in the measured runs. Frame generation 2x shows about twice as many (30-38).
> No game files are included. You need your own legally obtained copy of the game.

## How to play
1. Download `Sunline-v2.1.0-preview.1.zip` from **[Releases](../../releases)** and extract it anywhere (an SSD is best).
2. Open `Sunline.exe`, choose your language, your game folder and your graphics card.
3. Press **▶ PLAY**.

The first PLAY downloads, once, Microsoft's D3D12 runtime (from nuget.org, checked against Microsoft's signature) and
NVIDIA's DLSS files (from NVIDIA's GitHub), then translates the shaders (a few minutes).
If Windows SmartScreen warns about the app, click **More info → Run anyway** (it isn't signed).

**Important:** NVIDIA Control Panel → Manage 3D Settings → **Shader Cache Size → Unlimited**. A part of the shaders
is kept only in the driver's own cache; with a small cache they are built again at every start.

In the game: **F1** settings menu (DLSS mode, frame generation), **F3** stats panel. The window title shows the
shaders still being built.

## Requirements
- Windows 10 (19045) or Windows 11, 64-bit, a CPU with AVX2
- NVIDIA GeForce RTX (DLSS); frame generation needs an RTX 40/50 and hardware-accelerated GPU scheduling.
  Other cards run with the game's own upscaler (Series S profile), not tested here.
- 32 GB RAM, a Windows paging file of 16-48 GB on an SSD

## Settings
Sunline opens to Home, with Game Settings (F1), Menu Music and Advanced in the sidebar.
Choose the graphics profile and F1 options before pressing Play. The recommended NVIDIA profile is
Series X 1440p, DLSS Quality, and 2× frame generation on supported RTX cards.

Choose a local MP3 or WAV in Menu Music. The controls next to Play change the level or stop the track.
The default is 10% and the range is 0–25%. Music pauses while the game runs. Sunline saves your music
preferences with the other launcher settings. The language switch is at the far right of the header.
Reset All Settings asks for confirmation before restoring defaults, including saved game paths.

## What Sunline adds to VIctory-NVIDIA (`patches/nv-ours.patch`)
- **Shaders ready before the game starts:** the ones earlier runs drew with are built first (`victory ready`, a
  progress bar); the pipeline library is filled once, not at every start (68 s → 6 s here).
- **The shaders still being built in the window title**, while you play.
- Title buffer copies in card memory (2 GB) and 14 pipeline workers: measured faster here.
- Their memory stack (SLOT_EVICT, ALLOCATOR_TRIM, BUDGET_PACK) off: on, new memory was waited for 51,000 times in a
  session (18 s in all) and the game ran at 12 fps here.
- **The water's two tiled volumetrics left out** (`VICTORY_SKIP_PIPELINES`, made to work in a release build): on
  NVIDIA they left dark squares on the sea where boats pass.
- **A progress bar with the time left** while VIctory-NVIDIA is built or updated; Sunline notices an outdated build and
  offers **Update**.
- DLSS keeps working when the driver's DLSS override gives no mode for the render size (DLAA takes it).
- One click: D3D12 runtime, DLSS files, shaders and the game, with logs in `logs\`.

## What's in the folder
| Path | What it is |
|---|---|
| `Sunline.exe` | the launcher |
| `VIctory-NVIDIA\target\release\victory.exe` | VIctory-NVIDIA commit `63b902f` + `patches/nv-ours.patch` |
| `VIctory-NVIDIA\scripts\` | its launcher and its download scripts |
| `logs\`, `launcher.json` | logs and your settings |

Sunline never writes into the game folder.

## Building from source

The Windows GitHub Actions workflow tests the launcher, builds the pinned VIctory-NVIDIA revision
with the patch in this repository, and produces a preview ZIP and SHA-256 checksum. Find the files
in the completed run's `Sunline-Windows-preview` artifact.

- Launcher: install Rust, then `source\build.bat`.
- VIctory-NVIDIA: `setup\setup-nv.bat` (Git, Rust and the Visual Studio C++ build tools): it clones
  `trustno1-coder/VIctory-NVIDIA` at `63b902fe1720a5188ee79d94a88c8de0470f6349`, applies the patch and builds.

## License and credits
GPL-3.0-or-later, as VIctory. VIctory-NVIDIA by trustno1-coder, foxhound by readmet3xt-lab, VIctory and its forks
by their authors. NVIDIA's and Microsoft's files are downloaded from them under their licences and are not
redistributed here.

---

## بالعربي

**Sunline** مشغّل بسيط لـ VIctory، مبني على **VIctory-NVIDIA**: فيه **DLSS** وصورة بدقة 1440p وتوليد الفريمات.

**طريقة اللعب:**
1. حمّل `Sunline-v2.1.0-preview.1.zip` من صفحة **Releases** وفك الضغط في أي مكان (يفضّل على SSD).
2. افتح `Sunline.exe`، واختر اللغة ومجلد اللعبة وكرت الشاشة.
3. اضغط **▶ تشغيل**.

أول تشغيل ينزّل مكتبة D3D12 من مايكروسوفت وملفات DLSS من NVIDIA، ويترجم الشيدرات. هذا يصير مرة وحدة بس.

**مهم:** افتح لوحة تحكم NVIDIA، وخل **Shader Cache Size** على **Unlimited**. بدونه تنبني الشيدرات من جديد كل مرة.

داخل اللعبة: **F1** للإعدادات و**F3** للإحصائيات.

> ⚠️ البرنامج تجريبي. على كرت 4070 Ti الفريمات حوالي 16 إلى 19، والرسوم كاملة بتفاصيل الوجه والعيون. توليد الفريمات يضاعفها في العرض (30 إلى 38).
> أطفأنا شيدرين للماء كانوا يتركون مربعات غامقة على البحر ورا القوارب.
> إعدادات قائمة F1 صارت داخل Sunline: تختارها قبل اللعب بدون ما تشغّل اللعبة وتقفلها، وزر "رجّع إعداداتنا" يرجّع الإعداد اللي قسناه.
> ما فيه أي ملفات من اللعبة، ولازم تكون عندك نسختك الخاصة.

تقدر تختار ملف MP3 أو WAV من صفحة «موسيقى المنيو». التحكم بالصوت والإيقاف جنب زر التشغيل،
والصوت يبدأ على 10% وحدّه الأعلى 25%. يحفظ Sunline تفضيلاتك تلقائيًا.
زر اللغة بأقصى اليمين، وإعادة ضبط الإعدادات تسترجع الإعدادات الافتراضية بعد التأكيد.
