# Architecture

```text
OS adapters ──> normalized State ──> Director ──> Decision
                     ↑                              │
                Character Lab                       ├─> desktop sprite renderer
                                                    └─> terminal sprite renderer
JSON / PNG pack ──> validated animation resolver ──────┘
```

`companion-core` has no windowing, OS sensor, or networking dependencies. It contains the state model, Director, manifest validator, PNG pack loading, and frame timing. The Director takes an explicit monotonic timestamp, allowing deterministic tests without wall-clock sleeps.

`companion-platform` owns safe sensor reads, optional weather, settings storage, and native accessory adapters. A worker thread samples aggregate system data and notifies the UI. It never refreshes the process list. Mac accessory support is a small Objective-C adapter over CoreAudio and IOBluetooth. Windows uses COM audio endpoints and the Bluetooth connection API. Unknown values stay `None`.

`scopobot` hosts both presentations. The desktop uses an eframe/winit native transparent window and egui's OpenGL renderer. The Character Lab is a separate native viewport. The renderer receives no OS sensor handles. Character artwork is data, not code.

The terminal uses crossterm, including a drop guard to restore terminal modes on normal exit, errors, or unwind. It shares the pack and Director. No LLM, command-execution chatbot, or shell monitoring is involved.

## Timing

Each animation has per-frame durations. Idle holds can be long, with short blinks. The desktop requests a repaint at the next frame boundary rather than running continuously. Input, sensor changes, tray actions, and file-watch notifications wake the event loop. Saver and Ultra set lower frame-rate ceilings. The lab is destroyed when closed.

Director one-shots have fixed behavioral hold times, independent of artwork timing; short non-looping strips hold their final frame until the reaction ends. Every state update still applies during a cooldown. Events expire after eight seconds and incompatible pending events are dropped. The queue is capped at four entries.

CPU enters stress above 80% for five seconds, leaves below 60% for ten. RAM enters above 85%, leaves below 70%, with the same durations. Events take precedence over persistent states. Rain, headphones, and heat are separate visual overlays.

## Boundaries

The current renderer uses normalized positions for accessory overlays, suitable for the bundled humanoid. A different silhouette can supply all base animations but may need a future manifest extension for accessory anchors. Rule priorities are currently runtime-owned; arbitrary pack scripts are deliberately unsupported.

Platform parity needs native acceptance testing. Multi-monitor placement recovery, OS power-saving notifications, BLE and broader audio-route classification, creator tools, and final production artwork remain follow-up work. Linux desktop is not a supported release target yet.

Audio activity is an optional aggregate boolean. On macOS 14.2+, Core Audio output-stream activity is reduced immediately to one boolean; no application names, PIDs, taps, or audio samples are read. Earlier macOS versions return unavailable. Windows reads the default render endpoint's peak meter; exclusive mode can report zero when software metering is unavailable. Samples are checked at the existing adaptive sensor interval, with a six-second activity hold to avoid stopping the dance between readings. This detects output activity, including videos and calls, rather than identifying music or matching its beat. Unknown remains unknown. Muted output and low battery do not select automatic dancing. Manual Dance party works without sensors and yields to higher-priority states.

API references: [Apple Core Audio process objects](https://developer.apple.com/documentation/coreaudio/kaudiohardwarepropertyprocessobjectlist), [Microsoft endpoint peak meter](https://learn.microsoft.com/en-us/windows/win32/api/endpointvolume/nn-endpointvolume-iaudiometerinformation).

Performance: CPU entry >80% for five seconds and exit <60% for ten seconds; RAM entry >85% for five seconds and exit <80% for ten seconds. Director exposes CPU and RAM warning flags independently of one-shot event priority. Desktop badges and numeric alerts use those flags; alerts repeat at most once per minute, and recovery messages require known readings. Health opens the system monitor only on a click and never terminates tasks.
