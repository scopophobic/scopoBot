use anyhow::{bail, Result};
use companion_platform::Settings;
use std::path::PathBuf;

#[derive(Debug, PartialEq)]
pub enum Mode {
    Desktop,
    Terminal,
}
pub fn mode(args: &[String]) -> Result<Mode> {
    let explicit = args
        .iter()
        .position(|a| a == "--mode")
        .map(|i| {
            args.get(i + 1)
                .map(String::as_str)
                .ok_or_else(|| anyhow::anyhow!("Use --mode desktop or --mode terminal"))
        })
        .transpose()?;
    match explicit {
        Some("desktop") if args.iter().any(|a| a == "--terminal" || a == "--plain") => {
            bail!("Choose one mode: desktop or terminal")
        }
        Some("desktop") | None if !args.iter().any(|a| a == "--terminal" || a == "--plain") => {
            Ok(Mode::Desktop)
        }
        Some("terminal") | None => Ok(Mode::Terminal),
        Some(_) => bail!("Use --mode desktop or --mode terminal"),
    }
}
pub fn pack_path(explicit: Option<PathBuf>, settings: &Settings) -> Result<PathBuf> {
    if let Some(path) = explicit {
        return Ok(path);
    }
    if let Some(path) = &settings.character {
        if path.join("character.json").is_file() {
            return Ok(path.clone());
        }
    }
    let exe = std::env::current_exe()?;
    let parent = exe
        .parent()
        .ok_or_else(|| anyhow::anyhow!("Missing executable directory"))?;
    let roots = [
        parent.join("characters"),
        parent.join("../Resources/characters"),
        PathBuf::from("characters"),
        PathBuf::from(env!("CARGO_MANIFEST_DIR")).join("../../characters"),
    ];
    let saved_id = settings.character.as_ref().and_then(|p| p.file_name());
    for root in roots {
        if let Some(id) = saved_id {
            let candidate = root.join(id);
            if candidate.join("character.json").is_file() {
                return Ok(candidate);
            }
        }
        let candidate = root.join("glitch");
        if candidate.join("character.json").is_file() {
            return Ok(candidate);
        }
    }
    bail!("Character pack not found. Keep the characters folder with the app, or use --pack PATH.")
}

#[cfg(feature = "desktop")]
pub fn open_terminal(args: &[String]) -> Result<()> {
    let exe = std::env::current_exe()?;
    let parent = exe
        .parent()
        .ok_or_else(|| anyhow::anyhow!("Missing executable directory"))?;
    #[cfg(target_os = "macos")]
    let mut child = {
        let script = parent.join("../Resources/StartTerminal.command");
        if !script.is_file() {
            bail!("Run scopobot-terminal from your terminal, or use the packaged Mac app.");
        }
        std::process::Command::new("/usr/bin/open")
            .arg(script)
            .spawn()?
    };
    #[cfg(target_os = "windows")]
    let mut child = {
        use std::os::windows::process::CommandExt;
        let terminal = parent.join("scopobot-terminal.exe");
        if !terminal.is_file() {
            bail!("Keep scopobot-terminal.exe next to scopobot.exe.");
        }
        std::process::Command::new(terminal)
            .args(args)
            .creation_flags(0x00000010)
            .spawn()?
    };
    #[cfg(not(any(target_os = "macos", target_os = "windows")))]
    {
        let _ = (parent, args);
        bail!("Run scopobot-terminal in your terminal.");
    }
    #[cfg(target_os = "macos")]
    let _ = args;
    #[cfg(any(target_os = "macos", target_os = "windows"))]
    {
        std::thread::spawn(move || {
            let _ = child.wait();
        });
        Ok(())
    }
}
#[cfg(test)]
mod tests {
    use super::*;
    #[test]
    fn modes_are_explicit_and_conflicts_fail() {
        let a = |values: &[&str]| values.iter().map(|s| s.to_string()).collect::<Vec<_>>();
        assert_eq!(mode(&a(&[])).unwrap(), Mode::Desktop);
        assert_eq!(mode(&a(&["--mode", "terminal"])).unwrap(), Mode::Terminal);
        assert_eq!(mode(&a(&["--terminal"])).unwrap(), Mode::Terminal);
        assert!(mode(&a(&["--mode", "desktop", "--terminal"])).is_err());
        assert!(mode(&a(&["--mode"])).is_err());
        assert!(mode(&a(&["--mode", "unknown"])).is_err());
    }
}
