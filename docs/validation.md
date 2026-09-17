# Validation

## Current preview

- 18 automated tests passed on Mac, including sustained CPU/RAM hysteresis, playback priority, pack validation, explicit modes and duplicate desktop prevention.
- Strict workspace Clippy and formatting passed on Mac.
- Mac release compilation and Windows x86_64 GNU cross-compilation passed.
- Native Windows interaction has not been tested locally. GitHub Actions native Mac and Windows tests, strict Clippy, release builds and packaging passed for v0.1.0-preview.1.
- Mac music response was confirmed by the user.
- Earlier native Mac hover, click speech, compact window and speech expiry were visually verified.
- The latest terminal-switch and Health-panel interactions still require manual acceptance checks.
- Mac packages are ad-hoc signed, not notarized. Windows packages are unsigned. The extracted local Mac ZIP passed strict signature verification; both local ZIPs passed archive integrity and SHA-256 checks.
- The bundled Mac terminal passed a plain simulation snapshot from a temporary directory outside the project.

Glitch has 48 generated raster frames across breathing, blinking, walking, waving, sleep, charging, network searching, celebration, low battery, typing, headphones and curiosity. Artwork is inspired by user-supplied Glitch references, not a pixel-perfect extraction or a skeletal rig. Mochi and Pip have fewer source poses.

Quiet mode removes automatic dialogue from mood changes, hovering, periodic chatter and performance warnings. Clicking and explicit activities retain dialogue. CPU/RAM badges remain available.

CPU/RAM profiling and broader real-device acceptance checks remain future validation work. Cross-compilation does not establish native Windows behavior.
