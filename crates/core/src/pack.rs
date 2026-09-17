use anyhow::{bail, Context, Result};
use serde::Deserialize;
use std::{
    collections::BTreeMap,
    fs,
    path::{Component, Path, PathBuf},
};

#[derive(Debug, Clone, Deserialize)]
#[serde(deny_unknown_fields, rename_all = "camelCase")]
pub struct Animation {
    pub file: String,
    pub frames: u32,
    pub durations_ms: Vec<u64>,
    #[serde(rename = "loop")]
    pub looping: bool,
    /// Optional atlas rectangles, normalized into logical sprite frames at load time.
    #[serde(default)]
    pub regions: Option<Vec<[u32; 4]>>,
}
impl Animation {
    pub fn frame_at(&self, elapsed_ms: u64) -> (u32, u64) {
        let total: u64 = self.durations_ms.iter().sum();
        if !self.looping && elapsed_ms >= total {
            return (self.frames - 1, 1000);
        }
        let mut cursor = if self.looping {
            elapsed_ms % total
        } else {
            elapsed_ms
        };
        for (frame, &duration) in self.durations_ms.iter().enumerate() {
            if cursor < duration {
                return (frame as u32, duration - cursor);
            }
            cursor -= duration;
        }
        (0, self.durations_ms[0])
    }
}
#[derive(Debug, Deserialize)]
#[serde(deny_unknown_fields, rename_all = "camelCase")]
pub struct Manifest {
    pub id: String,
    pub name: String,
    pub author: String,
    pub description: String,
    pub sprite_size: [u32; 2],
    pub scale: u32,
    pub animations: BTreeMap<String, Animation>,
    #[serde(default)]
    pub captions: BTreeMap<String, String>,
    #[serde(default)]
    pub kind: String,
}
pub struct SpriteSheet {
    pub width: u32,
    pub height: u32,
    pub rgba: Vec<u8>,
}
pub struct CharacterPack {
    pub root: PathBuf,
    pub manifest: Manifest,
    pub sheets: BTreeMap<String, SpriteSheet>,
}
impl CharacterPack {
    pub fn load(root: impl AsRef<Path>) -> Result<Self> {
        let root = root
            .as_ref()
            .canonicalize()
            .context("Character folder not found")?;
        let manifest_path = safe_path(&root, "character.json")?;
        if fs::metadata(&manifest_path)?.len() > 128 * 1024 {
            bail!("Manifest exceeds 128 KB");
        }
        let manifest: Manifest = serde_json::from_slice(&fs::read(manifest_path)?)?;
        if manifest.id.is_empty() || manifest.name.len() > 100 || manifest.animations.len() > 64 {
            bail!("Invalid pack metadata");
        }
        let [w, h] = manifest.sprite_size;
        if !(8..=128).contains(&w) || !(8..=128).contains(&h) || !(1..=8).contains(&manifest.scale)
        {
            bail!("Invalid sprite dimensions or scale");
        }
        if !manifest.animations.contains_key("idle") {
            bail!("Every pack must provide an idle animation");
        }
        let mut sheets = BTreeMap::new();
        let mut total_bytes = 0;
        let mut decoded = BTreeMap::new();
        for (name, anim) in &manifest.animations {
            if anim.frames == 0
                || anim.frames > 64
                || anim.durations_ms.len() != anim.frames as usize
                || anim.durations_ms.iter().any(|d| !(42..=60_000).contains(d))
            {
                bail!("Invalid timing in {name}");
            }
            let file = safe_path(&root, &anim.file)?;
            if file.extension().and_then(|s| s.to_str()) != Some("png") {
                bail!("Only PNG sprite assets are accepted");
            }
            if fs::metadata(&file)?.len() > 4 * 1024 * 1024 {
                bail!("Sprite file is too large");
            }
            let dimensions = image::image_dimensions(&file)?;
            if anim.regions.is_none() && dimensions != (w * anim.frames, h) {
                bail!(
                    "{name}: expected {}×{h} horizontal strip, got {dimensions:?}",
                    w * anim.frames
                );
            }
            if !decoded.contains_key(&file) {
                total_bytes += dimensions.0 as usize * dimensions.1 as usize * 4;
            }
            if total_bytes > 32 * 1024 * 1024 {
                bail!("Decoded pack exceeds 32 MB");
            }
            if !decoded.contains_key(&file) {
                decoded.insert(file.clone(), image::open(&file)?.into_rgba8());
            }
            let source = &decoded[&file];
            let image = if let Some(regions) = &anim.regions {
                if regions.len() != anim.frames as usize {
                    bail!("{name}: one atlas rectangle is required per frame");
                }
                let mut strip = image::RgbaImage::new(w * anim.frames, h);
                for (index, &[x, y, rw, rh]) in regions.iter().enumerate() {
                    if rw == 0
                        || rh == 0
                        || x.checked_add(rw).is_none_or(|v| v > source.width())
                        || y.checked_add(rh).is_none_or(|v| v > source.height())
                    {
                        bail!("{name}: atlas rectangle outside image");
                    }
                    let region = image::imageops::crop_imm(source, x, y, rw, rh).to_image();
                    let frame = image::imageops::resize(
                        &region,
                        w,
                        h,
                        image::imageops::FilterType::Nearest,
                    );
                    image::imageops::replace(&mut strip, &frame, (index as u32 * w) as i64, 0);
                }
                strip
            } else {
                source.clone()
            };
            sheets.insert(
                name.clone(),
                SpriteSheet {
                    width: image.width(),
                    height: image.height(),
                    rgba: image.into_raw(),
                },
            );
        }
        Ok(Self {
            root,
            manifest,
            sheets,
        })
    }
    pub fn resolve<'a>(&'a self, requested: &'a str) -> (&'a str, &'a Animation) {
        if let Some(animation) = self.manifest.animations.get(requested) {
            (requested, animation)
        } else {
            ("idle", &self.manifest.animations["idle"])
        }
    }
}
fn safe_path(root: &Path, relative: &str) -> Result<PathBuf> {
    let path = Path::new(relative);
    if path.is_absolute()
        || relative.contains('\\')
        || relative.contains(':')
        || path
            .components()
            .any(|part| !matches!(part, Component::Normal(_)))
    {
        bail!("Asset paths must stay inside the character folder");
    }
    let resolved = root.join(path).canonicalize()?;
    if !resolved.starts_with(root) {
        bail!("Asset symlinks must stay inside the character folder");
    }
    Ok(resolved)
}

#[cfg(test)]
mod tests {
    use super::*;
    #[test]
    fn variable_duration_and_oneshot_hold() {
        let a = Animation {
            file: "".into(),
            frames: 3,
            durations_ms: vec![900, 100, 1800],
            looping: true,
            regions: None,
        };
        assert_eq!(a.frame_at(950), (1, 50));
        assert_eq!(a.frame_at(2800), (0, 900));
        let b = Animation {
            looping: false,
            ..a
        };
        assert_eq!(b.frame_at(9000).0, 2);
    }
    #[test]
    fn rejects_path_traversal() {
        assert!(safe_path(Path::new("/tmp"), "../secret.png").is_err());
        assert!(safe_path(Path::new("/tmp"), "C:\\secret.png").is_err());
        assert!(safe_path(Path::new("/tmp"), "/secret.png").is_err());
    }
    #[test]
    fn bundled_pack_is_valid() {
        let pack = CharacterPack::load(
            Path::new(env!("CARGO_MANIFEST_DIR")).join("../../characters/hoodie_bot"),
        )
        .unwrap();
        assert_eq!(pack.manifest.sprite_size, [32, 32]);
        assert_eq!(pack.resolve("not_installed").0, "idle");
    }
    #[test]
    fn glitch_uses_distinct_transparent_animation_frames() {
        let pack = CharacterPack::load(
            Path::new(env!("CARGO_MANIFEST_DIR")).join("../../characters/glitch"),
        )
        .unwrap();
        for name in [
            "idle",
            "blink",
            "walk",
            "wave",
            "sleep",
            "charging",
            "wifi_search",
            "celebrate",
            "battery_low",
            "cpu_high",
            "headphones_idle",
            "curious",
        ] {
            let sheet = &pack.sheets[name];
            assert_eq!(pack.manifest.animations[name].frames, 4);
            assert!(
                sheet
                    .rgba
                    .as_chunks::<4>()
                    .0
                    .iter()
                    .any(|pixel| pixel[3] == 0),
                "{name} must be transparent"
            );
            // Compare first and second frame row by row in the normalized horizontal strip.
            let stride = 64 * 4 * 4;
            assert!(
                (0..64).any(|y| sheet.rgba[y * stride..y * stride + 256]
                    != sheet.rgba[y * stride + 256..y * stride + 512]),
                "{name} must animate"
            );
        }
    }
}
