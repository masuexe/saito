<p align="center"><img src="docs/logo/saito-icon.svg" width="112" alt="彩斗 saito"></p>

<h1 align="center">彩斗 saito</h1>

<p align="center">Rollback netplay for Mega Man Battle Network 4, 5 and 6 · <b>EXESSR Hub</b> community fork (<code>top.exessrhub.saito</code>).</p>

<p align="center">
  <a href="https://github.com/masuexe/saito/releases"><b>Releases</b></a> ·
  <a href="https://github.com/masuexe/saito">Repository</a>
</p>

**彩斗 saito** is a community-maintained rollback netplay client for Battle Network 4, 5 and 6 by **EXESSR Hub**, forked from the upstream project (`v0.26.1`) under MPL-2.0 with:

- **Strict isolation from upstream services & formats:** upstream endpoints are removed, builds/rulesets/replays use dedicated `.saitotactic` / `.saitoruleset` / `.saitoreplay` formats (`STRP`), and netplay/nearby protocols are isolated so legacy `saito` formats and peers are rejected even if renamed.
- **Bundled offline CJK game content:** self-contained `content/` tables for EXE4, EXE5, and EXE6 without remote catalog fetching.
- **CJK-only localization:** 简体中文 (`zh-Hans`), 日本語 (`ja`), and 한국어 (`ko`, with Japanese battle-text fallback).
- **Online & Direct Connect:** make a room with a friend (with your own signaling server URL or direct IP:port), choose your input delay, reconnect dropped matches seamlessly.
- **Builds & Practice:** pick a navi, customize your folder, NaviCust, patch cards, souls/Crosses, Beast Out, or fight an AI/second-pad practice dummy.
- **Replays:** every online match is recorded to `.saitoreplay`, playable frame-by-frame or exportable to MP4.

## Playing

1. Own [Mega Man Battle Network Legacy Collection Vol. 2](https://store.steampowered.com/app/1798020/) on Steam.
   **saito** extracts graphics and sound directly from your legal copy at launch. No game ROMs or copyrighted assets are included.
2. Download a build from [masuexe/saito Releases](https://github.com/masuexe/saito/releases), or build from source below.
3. Point **saito** to your `exe` folder when prompted (or drag-and-drop it onto the window).

### Controls

| Button | Keyboard | Controller |
| --- | --- | --- |
| Move | Arrow keys | D-pad |
| A | Z | East button |
| B | X | South button |
| L / R | A / S | Shoulders |
| Start / Select | Enter / Backspace | Start / Back |
| Switch navi (party battle) | C / V | West / North button |

Esc opens the menu during a battle. Everything can be rebound in Settings.

## Building from source

You need Rust (stable, edition 2024), CMake (for SDL), and Legacy Collection Vol. 2.

```sh
git clone https://github.com/masuexe/saito.git
cd saito
cargo run --release -p saito --bin saito
```

On Linux, install development packages for ALSA, fontconfig, udev, Wayland and xkbcommon (on Debian/Ubuntu: `cmake pkg-config libasound2-dev libfontconfig1-dev libudev-dev libwayland-dev libxkbcommon-dev`).

### The ROMs

```sh
export SAITO_ROMS=~/path/to/roms
cargo run --release -p saito-content -- check exe6
```

### Tests & CLI Tools

```sh
cargo test --workspace                                 # engine & workspace tests
cargo run -p saito-content-check                      # type-check and lint every content pack
cargo run --release -p saito-tools -- --match match.toml --headless 1-120 --out frames/
cargo run --release -p saito-tools -- --replay set.saitoreplay
cargo run --release -p saito-tools -- --replay set.saitoreplay --mp4 set.mp4
```

### Local signaling server

```sh
cd signaling && npx wrangler dev                      # http://127.0.0.1:8787
SAITO_SIGNAL=ws://127.0.0.1:8787 cargo run --release -p saito --bin saito
```

### In a browser

`bash packaging/web/build.sh` builds the WebAssembly bundle and `python3 packaging/web/serve.py` serves it at `http://localhost:8000/`. It requires nightly Rust, `wasm-bindgen-cli`, and the WASI SDK; see [docs/app.md](docs/app.md) §6.

## Repository layout

| Path | What it is |
| --- | --- |
| `crates/saito-battle` | The battle engine: simulation core and the content model it runs on |
| `crates/saito-content-api`, `saito-luau` | The API content scripts call, and the sandboxed Luau runtime |
| `crates/saito-content`, `saito-content-check` | Loading content into the engine; type-checking and publishing it |
| `crates/saito-match` | Match files and builds, checked against a game's content |
| `crates/saito-extract`, `saito-assets` | Asset extraction from the ROMs, and the graphics in typed form |
| `crates/saito-render`, `saito-audio`, `m4a` | Drawing a battle into frames; sound through the M4A driver |
| `crates/saito-frontend` | Plays battles for a host app: live, online or from a replay (no window) |
| `crates/saito-netplay`, `saito-rtc`, `saito-signaling` | Rollback netplay, its WebRTC transport, and the isolated signaling client |
| `crates/saito-replay`, `saito-video` | The `.saitoreplay` (`STRP`) format; replays exported as MP4 |
| `crates/saito`, `saito-ui` | The `saito` app (Slint), and its CJK UI files |
| `crates/saito-files`, `saito-threads`, `saito-dialogs`, `saito-gamepad` | Platform abstraction crates |
| `crates/saito-updater` | App updates from `masuexe/saito` GitHub releases |
| `crates/saito-tools` | `saito-tool`, the command line |
| `crates/saito-training`, `training/` | AI opponent crate and trainer |
| `crates/exe4-compat`, `exe5-compat`, `exe6-compat` | Original IDs, save import and trace formats |
| `content/` | Bundled game content definitions (`exe4`, `exe5`, `exe6`, `exelib`) |
| `signaling/` | Matchmaking server (Cloudflare Worker) |
| `packaging/` | Installers and release packages (`top.exessrhub.saito`) |
| `docs/` | Documentation |

## License

saito is licensed under the [Mozilla Public License 2.0](LICENSE), forked cleanly from the upstream MPL-2.0 project (`v0.26.1`). Bundled third-party files keep their own licenses: the fonts are under the SIL Open Font License (each folder's `OFL.txt`, including `Noto Sans KR`), and `crates/saito-luau/wasm/Unwind-wasm.c` is LLVM's, under the Apache License 2.0 with LLVM Exceptions. The app's About screen lists every package and font license.

Mega Man Battle Network is a trademark of CAPCOM CO., LTD. saito is an EXESSR Hub community fan project and is not affiliated with, funded by, or in any way associated with Capcom.
