#![cfg_attr(
    all(target_os = "windows", feature = "desktop"),
    windows_subsystem = "windows"
)]
#[cfg(feature = "desktop")]
mod desktop;
#[cfg(feature = "desktop")]
mod dialogue;
#[cfg(feature = "desktop")]
mod instance;
#[cfg(not(all(target_os = "windows", feature = "desktop")))]
mod terminal;
use anyhow::Result;
mod launch;
use companion_core::CharacterPack;
use companion_platform::Settings;
use std::path::PathBuf;

fn main() -> Result<()> {
    let args: Vec<String> = std::env::args().skip(1).collect();
    if args.iter().any(|a| a == "--help" || a == "-h") {
        println!("Scopobot — a tiny desktop roommate\n\nscopobot --mode desktop [--lab] [--pack PATH] [--reset]\nscopobot --mode terminal [--demo] [--plain] [--pack PATH]\nscopobot --validate-pack PATH\n\nDesktop: drag to move; right-click for controls. Menu-bar/tray icon always restores interaction.\nTerminal: q quit, space pause, d simulation, b battery, c charging, n network, h CPU, r rain, + / - scale.");
        return Ok(());
    }
    if let Some(i) = args.iter().position(|a| a == "--validate-pack") {
        let path = args
            .get(i + 1)
            .ok_or_else(|| anyhow::anyhow!("Missing pack path"))?;
        let pack = CharacterPack::load(path)?;
        println!(
            "Valid: {} ({} animations)",
            pack.manifest.name,
            pack.manifest.animations.len()
        );
        return Ok(());
    }
    let mode = launch::mode(&args)?;
    let settings = if args.iter().any(|a| a == "--reset") {
        Settings::default()
    } else {
        Settings::load().unwrap_or_else(|e| {
            eprintln!("Settings could not be read; using defaults: {e}");
            Settings::default()
        })
    };
    let explicit = args
        .iter()
        .position(|a| a == "--pack")
        .map(|i| {
            args.get(i + 1)
                .map(PathBuf::from)
                .ok_or_else(|| anyhow::anyhow!("Missing --pack path"))
        })
        .transpose()?;
    let pack = CharacterPack::load(launch::pack_path(explicit, &settings)?)?;
    if mode == launch::Mode::Terminal {
        #[cfg(all(target_os = "windows", feature = "desktop"))]
        {
            launch::open_terminal(&args)?;
            return Ok(());
        }
        #[cfg(not(all(target_os = "windows", feature = "desktop")))]
        return terminal::run(
            pack,
            settings,
            args.iter().any(|a| a == "--demo"),
            args.iter().any(|a| a == "--plain"),
        );
    }
    #[cfg(feature = "desktop")]
    {
        let Some(_instance_guard) =
            instance::acquire(&Settings::path()?.with_file_name("desktop.lock"))?
        else {
            return Ok(());
        };
        desktop::run(
            pack,
            settings,
            args.iter().any(|a| a == "--lab"),
            args.iter().any(|a| a == "--smoke-test"),
        )
        .map_err(|e| anyhow::anyhow!("{e}"))
    }
    #[cfg(not(feature = "desktop"))]
    terminal::run(pack, settings, args.iter().any(|a| a == "--demo"), false)
}
