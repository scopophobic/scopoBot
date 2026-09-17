# Character pack format

A pack is a folder with `character.json` and horizontal PNG sprite strips. See `characters/hoodie_bot` for a complete example.

```json
{
  "id": "my_character",
  "name": "My Character",
  "author": "Creator",
  "description": "An independent little resident.",
  "spriteSize": [32, 32],
  "scale": 4,
  "animations": {
    "idle": {
      "file": "sprites/idle.png",
      "frames": 4,
      "durationsMs": [1000, 180, 1600, 140],
      "loop": true
    }
  }
}
```

For four 32×32 frames, the strip is 128×32. Frames run left to right. Transparent background, bottom-center character anchor, integer nearest-neighbor scaling. Only `idle` is mandatory; other missing animations fall back to it.

Recognized animation keys: `idle`, `blink`, `sleep`, `tired`, `battery_low`, `battery_high`, `charging_start`, `charging`, `wifi_lost`, `wifi_search`, `wifi_restored`, `cpu_high`, `cpu_critical`, `celebrate`, `wake`, `headphones_reaction`, `headphones_idle`.

The Character Lab can manually play additional animation keys. Normalized signals drive the runtime's Director; packs do not receive code execution privileges or direct system access.

Limits: manifest ≤128 KB; 8–128 px per frame dimension; 1–64 frames per strip; 42–60,000 ms per frame; ≤64 animations; ≤4 MB encoded per PNG; ≤32 MB combined decoded pixels. Asset paths must be relative and stay inside the pack, including after resolving symlinks. Unknown manifest fields are rejected to catch mistakes.

Use **Settings → Character pack → Load folder** or `--pack PATH`. While the app is running, saving a valid asset reloads the entire pack after a short debounce. An invalid edit keeps the previous valid pack and reports the error in the lab.

The built-in art is intentionally a prototype, not a final commissioned sprite set. `tools/make_sprites.py` reproduces every pixel and animation from one shared model and palette.

Animations may also use `regions`: one `[x, y, width, height]` rectangle per frame in a PNG atlas. The loader normalizes these into logical sprite frames with nearest-neighbor scaling. Glitch's assets use four columns and four rows per source atlas, with actual sequential artwork for each frame. `captions` provides friendly text per animation, and `kind` identifies packs whose props and expressions are already in the sprites.
