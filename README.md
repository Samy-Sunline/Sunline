# Sunline

A simple Windows launcher for **[VIctory](https://github.com/avatarmanz/VIctory-7900XTX)** (Xbox Series compatibility layer), tuned for NVIDIA and AMD cards.
**Download → choose your game folder → PLAY.**

> ⚠️ Experimental. Expect about 13–16 fps on an RTX 4070 Ti class card, and the first session builds shaders (missing shapes for a while are normal).
> No game files are included. You need your own legally obtained copy of the game.

## How to play
1. Download `Sunline-v1.0.0.zip` from **[Releases](../../releases)** and extract it anywhere (an SSD is best).
2. Open `Sunline.exe` and choose your language.
3. Choose your game folder (Sunline finds the game EXE inside it).
4. Press **▶ PLAY**.

On the first PLAY, Sunline downloads Microsoft's D3D12 runtime by itself (~35 MB, once) and translates the shaders (a few minutes, once).
If Windows SmartScreen warns about the app, click **More info → Run anyway**. The app isn't signed.

## Requirements
- Windows 10 (19045) or Windows 11, 64-bit
- NVIDIA RTX 30/40/50 or AMD RX 6000/7000/9000 (tested: RTX 4070 Ti)
- 32 GB RAM, plus a Windows paging file of 16–48 GB on an SSD
- NVIDIA: Control Panel → Shader Cache Size → **Unlimited**

## Presets
| Preset | What it does | Result (RTX 4070 Ti) |
|---|---|---|
| ★ Quality | Series S mode, FSR4, 1080p output, renderer ports 2/6/7 | ~13.5–14.5 fps, sharp |
| ★ Performance | Same, with the game's own upscaler | ~16 fps, slightly blurry |

## What's in the folder
| Path | What it is |
|---|---|
| `Sunline.exe` | the launcher |
| `victory\victory.exe` | VIctory, built from upstream commit `014d52d` + `patches/newbase-nvidia.patch` |
| `cache\` | shaders and pipelines (made on your PC) |
| `logs\` | run logs, useful when reporting a problem |
| `launcher.json` | your settings |

Sunline never writes into the game folder.

## Building from source (optional)
- Launcher: install Rust, then run `source\build.bat`.
- VIctory: run `setup\setup-new.bat`. It clones VIctory-7900XTX at commit `014d52dedf837d1ac660a0bd256c4704be9eeb40`, applies the patch, and builds.

## License
GPL-3.0-or-later (same as VIctory). The patch is a derivative of VIctory-7900XTX by avatarmanz.
The D3D12 Agility runtime is downloaded from nuget.org under Microsoft's license and isn't redistributed here.

---

## بالعربي

**Sunline** مشغّل بسيط لطبقة التوافق VIctory: **حمّل ← اختر مجلد اللعبة ← تشغيل.**

**طريقة اللعب:**
1. حمّل `Sunline-v1.0.0.zip` من صفحة **Releases** وفك الضغط في أي مكان (يفضّل على قرص SSD).
2. افتح `Sunline.exe` واختر اللغة.
3. اختر مجلد اللعبة، والبرنامج بيلقى ملف اللعبة بنفسه.
4. اضغط **▶ تشغيل**.

أول تشغيل بينزّل البرنامج مكتبة D3D12 من مايكروسوفت تلقائياً (~35 ميجا، مرة وحدة) ويترجم الشيدرات (كم دقيقة، مرة وحدة).
إذا طلع تحذير ويندوز اضغط **More info ثم Run anyway**.

> ⚠️ البرنامج تجريبي، والأداء تقريباً 13 إلى 16 فريم على كرت 4070 Ti.
> ما فيه أي ملفات من اللعبة، ولازم تكون عندك نسختك الخاصة.
