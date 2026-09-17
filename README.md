



# Scopobot


A friendly pixel mascot in a compact transparent desktop window. Glitch is the default companion; Mochi the puppy and Pip are selectable alternatives. Mac and Windows share a Rust runtime; the terminal companion uses the same character pack and Director. Linux terminal mode is included; Linux desktop remains experimental.

This is a local desktop application, not a website, AI assistant, or cloud service.

<img width="800" height="559" alt="ScreenRecording2026-09-17at6 00 29AM-ezgif com-video-to-gif-converter" src="https://github.com/user-attachments/assets/61e5630c-f024-476d-ae43-74c3ee73a50a" />

## Download and install

[Download the latest preview](https://github.com/scopophobic/scopoBot/releases/tag/v0.1.0-preview.1): [Mac Apple Silicon](https://github.com/scopophobic/scopoBot/releases/download/v0.1.0-preview.1/Scopobot-macos-arm64.zip) · [Windows x64](https://github.com/scopophobic/scopoBot/releases/download/v0.1.0-preview.1/Scopobot-windows.zip). Both packages include the terminal companion. Intel Mac users can build from source. This is an early preview.

- **Mac:** extract the Mac ZIP and move Scopobot.app to Applications. The filename indicates its CPU architecture. The preview is ad-hoc signed, without Apple notarization; macOS may require approval in System Settings → Privacy & Security.
- **Windows:** extract the entire Windows ZIP, keeping both executables and the characters folder together. Open scopobot.exe for desktop mode or scopobot-terminal.exe for terminal mode. The preview is unsigned.
- **Linux:** build the terminal binary from source. Linux desktop support remains experimental.

## Modes and controls

Desktop mode floats above your apps on the current desktop. It starts with only the mascot. Drag to move, hover for a curious reaction, click for a wave and a short line, and right-click or double-click for settings. The tray menu also opens settings. Speech appears only after your interaction, never as random chatter or automatic performance warnings.

Choose **Switch to terminal only** in settings or the tray to close the desktop companion and open its terminal counterpart. On Mac this uses the system Terminal application. For a source build, run the terminal command below directly. Quit the terminal with q before opening the desktop app again.

```sh
scopobot --mode desktop
scopobot-terminal
```

`scopobot --mode terminal` also selects terminal mode; Windows opens the separate console executable. Desktop mode stays above apps on the active desktop; pinning to every virtual desktop and every full-screen app is not implemented.

## Build from source

Install [Rust](https://rustup.rs/) and the native compiler tools for your OS (Xcode Command Line Tools on Mac; Visual Studio C++ Build Tools on Windows).

```sh
cargo run -p scopobot --bin scopobot --release -- --lab
```

The companion appears in a borderless, always-on-top window. Click Glitch for a wave. Character Lab has quick activities and animation previews. Drag to move it. Double-click or right-click opens the Character Lab. The menu-bar/tray icon provides pause, size, power mode, click-through, reset position, and quit. The menu remains accessible when click-through is on.

The Character Lab opens on first launch. Close it or choose **Back to desktop** to leave only the character running. Preferences are saved locally. Use `--reset` to recover default position and interaction settings.

### Terminal companion

```sh
cargo run -p scopobot --no-default-features --bin scopobot-terminal --release
cargo run -p scopobot --no-default-features --bin scopobot-terminal -- --demo
cargo run -p scopobot --no-default-features --bin scopobot-terminal -- --plain
```

The terminal uses true-color half-block sprites, with a text fallback in small windows. It does not require a desktop session or graphics library. `--plain` emits a single readable status snapshot, suitable for scripts. On Windows, use the separate `scopobot-terminal.exe` console binary.

| Key | Action |
| --- | --- |
| `q`, `Esc`, `Ctrl+C` | Quit and restore terminal |
| `Space` | Pause / resume animation |
| `d` | Switch live / simulation |
| `b` | Simulate low / high battery |
| `c` | Simulate charging |
| `n` | Simulate network loss / recovery |
| `h` | Simulate high / normal CPU (high requires 5 seconds) |
| `r` | Simulate rain |
| `+`, `-` | Adjust horizontal sprite size |

## Included

- Shared Rust Director: bounded, expiring event queue, priorities, cooldowns, non-interruptible reactions, persistent-state fallback, CPU/memory hysteresis, intensity, ambient probability, and layered effects.
- Glitch artwork adapted from the supplied mascot references, plus Mochi and Pip. Atlas regions resolve to logical 64×64 frames with nearest-neighbor scaling.
- Native desktop windows using eframe/egui + OpenGL, no browser renderer or full game engine. Repaints are scheduled for frames, input, or state changes, with no permanent 60 FPS loop.
- Character Lab: simulated battery, charger, CPU, RAM, network, time/day, weather, volume, headphones, Bluetooth; forced animation; frame step; speed; backdrop; live readings; Director diagnostics.
- Pack validation, asset hot reload, custom pack path, last-good-pack recovery.
- Local settings, adaptive power modes, menu-bar/tray controls, optional manual-city weather.
- Mac and Windows sensor adapters, plus a Linux terminal foundation.

## Sensors and limitations

| Signal | Mac | Windows | Linux terminal |
| --- | --- | --- | --- |
| Battery / charging | Battery API | Battery API | Battery API where supported |
| Aggregate CPU / RAM | Supported | Supported | Supported |
| Network interface state | Supported | Supported | Supported |
| Local time / day | Supported | Supported | Supported |
| Output volume | CoreAudio where exposed | Default endpoint volume | Not implemented |
| Headphones | Built-in output data source where exposed | Endpoint form factor where exposed | Not implemented |
| Bluetooth | Connected paired classic devices | Connected paired classic devices | Not implemented |
| Weather | Optional Open-Meteo | Optional Open-Meteo | Saved city only |

Missing readings are not replaced with fake values. Bluetooth is reduced to generic categories and does not expose names or identifiers. BLE-only accessories and some headphone routes are not covered. A connected local interface does **not** prove internet reachability; no traffic inspection or connectivity probe is performed.

CPU/RAM/network polling is every 3 seconds normally, 10 in Saver, 30 in Ultra. Battery and accessories refresh around every 30 seconds. These are conservative polling adapters, not an implementation of every OS notification API. Weather refreshes every 30 minutes; failures clear stale weather and retry later.

Auto power mode selects Saver below 20% and Ultra below 8% while discharging. Ultra allows at most one scheduled animation frame every 5 seconds. OS battery-saver settings are not yet integrated. Input and sensor events can still trigger a redraw.

## Privacy

No screen capture, content inspection, keystroke monitoring, microphone, camera, audio-stream inspection, packet inspection, AI service, account, analytics, or cloud sync. Runtime reads its own settings and the selected character pack. Device readings are held in memory and are not logged.

Weather is off by default. Searching sends the typed city name to Open-Meteo's geocoding service. Selecting a city saves its city-level coordinates and requests weather periodically. No GPS is used. Turn weather off to stop future requests; an in-flight request may finish. [Weather attribution](https://open-meteo.com/).

## Character packs

```sh
cargo run -p scopobot --bin scopobot -- --validate-pack characters/glitch
cargo run -p scopobot --bin scopobot -- --pack /path/to/character
```

See [the pack format](docs/character-packs.md). Packs contain only JSON and PNG files. Arbitrary code, path traversal, external symlinks, oversized textures, and invalid frame timings are rejected. Missing animation names fall back to `idle`.

To regenerate the original placeholder pack (this does not regenerate Glitch):

```sh
python3 tools/make_sprites.py
```

## Test and package

```sh
cargo fmt --all --check
cargo test --workspace --locked
cargo clippy --workspace --all-targets --locked -- -D warnings
cargo build --release --workspace --locked
python3 tools/package.py
```

Mac packaging creates `dist/Scopobot.app` and an architecture-labeled ZIP, with the terminal binary and packs inside the app. The app is locally ad-hoc signed, not Developer ID signed or notarized. Windows packaging creates a portable ZIP containing both binaries and character packs. Archives include SHA-256 checksums. A tag-triggered release workflow builds downloadable preview packages; production signing is a separate step.

The checked-in CI workflow builds and tests on Mac and Windows and checks terminal mode on Linux when the repository is hosted on GitHub. A workflow file by itself does not mean those remote jobs have run.

See [architecture](docs/architecture.md) and [manual acceptance checks](docs/acceptance.md). Performance and platform verification results are recorded in [validation](docs/validation.md).

Glitch includes 48 source animation frames: breathing, blinking, walking in place, waving, sleeping, charging, signal searching, celebration hops, low battery, typing on a laptop, headphones, and curiosity. Ambient activities run occasionally; manual reactions finish and return to live behavior. The companion defaults to a compact 128×128 transparent window. Existing furnished-scene preferences migrate to this smaller size.

Hover over the mascot for a friendly lift, glow, sparkles, and a curious reaction. Click for a wave and a varied line of dialogue. More than 50 short lines are available for explicit interactions. Automatic mood changes do not display dialogue. Speech bubbles disappear after five seconds or when clicked; they keep the character's feet anchored and the window returns to its compact size. Character Lab opens on double-click or right-click, rather than opening a second window on first launch. An operating-system lock prevents duplicate desktop companions and releases automatically on exit or crash; terminal companions run independently.

Music reactions: Glitch dances when output audio activity is detected (macOS 14.2+ and Windows), including speaker playback. It reacts to activity rather than identifying music or tracking the beat, so video or call audio can also trigger dancing. Reactions stop after the activity hold expires; unavailable sensors stay unavailable. Reactions has a manual Dance party switch. A tiny battery badge appears only while charging or below 20%; the dialogue bubble uses a smaller 180-pixel width and 40-pixel body.

Computer health: sustained CPU use above 80% or used RAM above 85% triggers a distinct reaction and a percentage badge. Click a badge for the Health panel. Brief spikes do not warn. CPU clears after 10 seconds below 60%, RAM below 80%; Automatic speech is disabled; open Health for practical guidance. Used RAM is not a memory-pressure diagnosis.

Health includes “See which apps are busy,” opening Activity Monitor on Mac or Task Manager on Windows only when clicked. Glitch does not collect application names or stop tasks automatically.
