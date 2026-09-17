use anyhow::Result;
use companion_core::{CharacterPack, Director, State, Weather};
use companion_platform::{SensorHub, Settings};
use crossterm::{
    cursor,
    event::{self, Event, KeyCode, KeyEventKind},
    execute, queue,
    style::{Color, Print, ResetColor, SetBackgroundColor, SetForegroundColor},
    terminal::{self, Clear, ClearType, EnterAlternateScreen, LeaveAlternateScreen},
};
use std::{
    io::{self, IsTerminal, Write},
    time::{Duration, Instant},
};

struct Restore;
impl Drop for Restore {
    fn drop(&mut self) {
        let _ = terminal::disable_raw_mode();
        let _ = execute!(io::stdout(), ResetColor, cursor::Show, LeaveAlternateScreen);
    }
}
pub fn run(pack: CharacterPack, settings: Settings, demo: bool, plain: bool) -> Result<()> {
    let mut simulation = demo;
    let mut fake = State::demo();
    let hub = SensorHub::start(settings.clone(), || {});
    let mut director = Director::default();
    if plain || !io::stdout().is_terminal() {
        if !demo {
            let deadline = Instant::now();
            while hub.snapshot().samples == 0 && deadline.elapsed() < Duration::from_secs(5) {
                std::thread::sleep(Duration::from_millis(50));
            }
        }
        let state = if demo { fake } else { hub.snapshot().state };
        let d = director.update(&state, 0);
        println!(
            "{}: {}\n{}\n{}",
            pack.manifest.name,
            d.animation,
            d.reason,
            serde_json::to_string_pretty(&state)?
        );
        return Ok(());
    }
    terminal::enable_raw_mode()?;
    let _restore = Restore;
    let mut out = io::stdout();
    execute!(out, EnterAlternateScreen, cursor::Hide)?;
    let start = Instant::now();
    let mut changed = Instant::now();
    let mut name = String::new();
    let mut paused = false;
    let mut frozen = 0;
    let mut scale: u16 = 1;
    let monochrome =
        std::env::var_os("NO_COLOR").is_some() || std::env::var("TERM").is_ok_and(|t| t == "dumb");
    loop {
        let state = if simulation {
            fake.clone()
        } else {
            hub.snapshot().state
        };
        let now = start.elapsed().as_millis() as u64;
        let decision = director.update(&state, now);
        let (resolved, _) = pack.resolve(decision.animation);
        if !paused && name != resolved {
            name = resolved.into();
            changed = Instant::now();
        }
        let (resolved, animation) = pack.resolve(&name);
        let elapsed = if paused {
            frozen
        } else {
            changed.elapsed().as_millis() as u64
        };
        let (frame, delay) = animation.frame_at(elapsed);
        let (cols, rows) = terminal::size()?;
        queue!(
            out,
            cursor::MoveTo(0, 0),
            Clear(ClearType::All),
            SetForegroundColor(Color::Rgb {
                r: 251,
                g: 211,
                b: 77
            }),
            Print("Scopobot"),
            ResetColor,
            Print(format!(
                "  {}  {}\r\n",
                if simulation {
                    "Simulated"
                } else {
                    "Live device state"
                },
                if paused { "[paused]" } else { "" }
            ))
        )?;
        let [w, h] = pack.manifest.sprite_size;
        if cols >= w as u16 + 4 && rows >= h.div_ceil(2) as u16 + 8 {
            let sheet = &pack.sheets[resolved];
            let actual_scale = scale.min((cols / w as u16).max(1));
            let origin_x = (cols.saturating_sub(w as u16 * actual_scale)) / 2;
            for y in (0..h).step_by(2) {
                queue!(out, cursor::MoveTo(origin_x, 3 + (y / 2) as u16))?;
                for x in 0..w {
                    let color = |yy: u32| {
                        let index = ((yy.min(h - 1) * sheet.width + frame * w + x) * 4) as usize;
                        let p = &sheet.rgba[index..index + 4];
                        if p[3] == 0 {
                            None
                        } else {
                            Some(Color::Rgb {
                                r: p[0],
                                g: p[1],
                                b: p[2],
                            })
                        }
                    };
                    let (glyph, foreground, background) = match (color(y), color(y + 1)) {
                        (None, None) => (" ", Color::Reset, Color::Reset),
                        (Some(top), None) => ("▀", top, Color::Reset),
                        (None, Some(bottom)) => ("▄", bottom, Color::Reset),
                        (Some(top), Some(bottom)) => ("▀", top, bottom),
                    };
                    let glyph = if monochrome && glyph != " " {
                        "#"
                    } else {
                        glyph
                    };
                    queue!(
                        out,
                        SetForegroundColor(foreground),
                        SetBackgroundColor(background),
                        Print(glyph.repeat(actual_scale as usize))
                    )?;
                }
                queue!(out, ResetColor)?;
            }
            queue!(
                out,
                cursor::MoveTo(2, h.div_ceil(2) as u16 + 4),
                Print(format!("{} · {:?}", resolved, decision.intensity))
            )?;
            queue!(
                out,
                cursor::MoveTo(2, h.div_ceil(2) as u16 + 5),
                Print(decision.reason)
            )?;
        } else {
            queue!(
                out,
                cursor::MoveTo(0, 3),
                Print("[ -_- ]"),
                cursor::MoveTo(0, 5),
                Print(resolved)
            )?;
        }
        let controls = if simulation {
            "q quit · space pause · d live · b battery · c charger · n network · h load · r rain"
        } else {
            "q quit · space pause · d simulate · +/- size"
        };
        queue!(
            out,
            cursor::MoveTo(0, rows.saturating_sub(2)),
            ResetColor,
            Print(controls.chars().take(cols as usize).collect::<String>())
        )?;
        out.flush()?;
        let wait = if paused {
            500
        } else {
            delay
                .max(settings.power.effective(&state).frame_floor_ms())
                .min(1000)
        };
        if event::poll(Duration::from_millis(wait))? {
            if let Event::Key(key) = event::read()? {
                if key.kind != KeyEventKind::Press {
                    continue;
                }
                match key.code {
                    KeyCode::Char('q') | KeyCode::Esc => break,
                    KeyCode::Char('c') if key.modifiers.contains(event::KeyModifiers::CONTROL) => {
                        break
                    }
                    KeyCode::Char(' ') => {
                        if !paused {
                            frozen = elapsed;
                        } else {
                            changed = Instant::now() - Duration::from_millis(frozen);
                        }
                        paused = !paused;
                    }
                    KeyCode::Char('d') => {
                        simulation = !simulation;
                        director = Director::default();
                    }
                    KeyCode::Char('b') if simulation => {
                        let b = fake.battery.as_mut().unwrap();
                        b.level = if b.level > 15. { 8. } else { 85. };
                    }
                    KeyCode::Char('c') if simulation => {
                        let b = fake.battery.as_mut().unwrap();
                        b.charging = !b.charging;
                    }
                    KeyCode::Char('n') if simulation => {
                        fake.connected = Some(!fake.connected.unwrap_or(true))
                    }
                    KeyCode::Char('h') if simulation => {
                        fake.cpu = Some(if fake.cpu.unwrap_or(0.) > 80. {
                            15.
                        } else {
                            98.
                        })
                    }
                    KeyCode::Char('r') if simulation => {
                        fake.weather = if fake.weather == Some(Weather::Rain) {
                            None
                        } else {
                            Some(Weather::Rain)
                        }
                    }
                    KeyCode::Char('+') | KeyCode::Char('=') => scale = (scale + 1).min(3),
                    KeyCode::Char('-') => scale = scale.saturating_sub(1).max(1),
                    _ => {}
                }
            }
        }
    }
    Ok(())
}
