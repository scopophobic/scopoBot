#[path = "../terminal.rs"]
mod terminal;

// This binary only needs the shared pack resolver.
#[allow(dead_code)]
#[path = "../launch.rs"]
mod launch;

fn main() -> anyhow::Result<()> {
    use companion_core::CharacterPack;
    use companion_platform::Settings;
    use std::path::PathBuf;
    let args: Vec<String> = std::env::args().skip(1).collect();
    if args.iter().any(|a| a == "--help") {
        println!("scopobot-terminal [--demo] [--plain] [--pack PATH]\n\nq quit · space pause · d simulate/live · b battery · c charging · n network · h load · r rain · +/- size");
        return Ok(());
    }
    let settings = Settings::load().unwrap_or_default();
    let explicit = args
        .iter()
        .position(|a| a == "--pack")
        .map(|i| {
            args.get(i + 1)
                .map(PathBuf::from)
                .ok_or_else(|| anyhow::anyhow!("Missing pack path"))
        })
        .transpose()?;
    let path = launch::pack_path(explicit, &settings)?;
    terminal::run(
        CharacterPack::load(path)?,
        settings,
        args.iter().any(|a| a == "--demo"),
        args.iter().any(|a| a == "--plain"),
    )
}
