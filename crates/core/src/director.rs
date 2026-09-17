use crate::{State, Weather};
use std::collections::{HashMap, VecDeque};

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Intensity {
    Calm,
    Active,
    Stressed,
    Critical,
}
#[derive(Debug, Clone, PartialEq)]
pub struct Decision {
    pub animation: &'static str,
    pub reason: &'static str,
    pub priority: u8,
    pub intensity: Intensity,
    pub headphones: bool,
    pub rain: bool,
    pub heat: bool,
    pub charging: bool,
    pub cpu_busy: bool,
    pub memory_busy: bool,
}
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
enum Event {
    Lost,
    Restored,
    Charge,
    Full,
    Headphones,
    Wake,
    Muted,
    Loud,
    Accessory,
}
impl Event {
    fn details(self) -> (&'static str, &'static str, u8, u64, u64) {
        match self {
            Self::Lost => (
                "wifi_lost",
                "Connection lost. Looking for a signal.",
                100,
                2400,
                60_000,
            ),
            Self::Restored => (
                "wifi_restored",
                "Connection restored. Obviously I fixed it.",
                90,
                1600,
                30_000,
            ),
            Self::Charge => (
                "charging_start",
                "Power acquired. About time.",
                85,
                1600,
                15_000,
            ),
            Self::Full => (
                "celebrate",
                "Fully charged. Try to keep up.",
                70,
                1800,
                180_000,
            ),
            Self::Headphones => (
                "headphones_reaction",
                "Headphones on. World off.",
                40,
                1200,
                15_000,
            ),
            Self::Wake => ("wake", "Morning already?", 30, 1800, 60_000),
            Self::Muted => ("blink", "Silent treatment. Got it.", 25, 900, 15_000),
            Self::Loud => (
                "headphones_reaction",
                "Okay, we all heard that.",
                35,
                1200,
                30_000,
            ),
            Self::Accessory => (
                "headphones_reaction",
                "Something else moved in.",
                20,
                1000,
                15_000,
            ),
        }
    }
}
#[derive(Default)]
struct Hysteresis {
    active: bool,
    since: Option<u64>,
}
impl Hysteresis {
    fn update(&mut self, value: Option<f32>, enter: f32, exit: f32, now: u64) {
        let Some(value) = value else {
            self.active = false;
            self.since = None;
            return;
        };
        let crossing = if self.active {
            value < exit
        } else {
            value > enter
        };
        if crossing {
            let start = *self.since.get_or_insert(now);
            if now.saturating_sub(start) >= if self.active { 10_000 } else { 5_000 } {
                self.active = !self.active;
                self.since = None;
            }
        } else {
            self.since = None;
        }
    }
}
/// A monotonic-clock, platform-independent director. Events are bounded, expire,
/// and never interrupt another one-shot. State updates are never discarded.
pub struct Director {
    previous: Option<State>,
    queue: VecDeque<(Event, u64)>,
    cooldowns: HashMap<Event, u64>,
    current: Option<(Event, u64)>,
    cpu: Hysteresis,
    memory: Hysteresis,
    last_ambient: u64,
    ambient_until: u64,
    ambient_animation: &'static str,
    last_blink: u64,
    blink_until: u64,
    seed: u64,
}
impl Default for Director {
    fn default() -> Self {
        Self {
            previous: None,
            queue: VecDeque::new(),
            cooldowns: HashMap::new(),
            current: None,
            cpu: Hysteresis::default(),
            memory: Hysteresis::default(),
            last_ambient: 0,
            ambient_until: 0,
            ambient_animation: "wave",
            last_blink: 0,
            blink_until: 0,
            seed: 0xdecafbad,
        }
    }
}
impl Director {
    pub fn new(seed: u64) -> Self {
        Self {
            seed: seed.max(1),
            ..Self::default()
        }
    }
    pub fn queued(&self) -> Vec<&'static str> {
        self.queue.iter().map(|(e, _)| e.details().0).collect()
    }
    pub fn cooldowns(&self, now: u64) -> Vec<(&'static str, u64)> {
        let mut result: Vec<_> = self
            .cooldowns
            .iter()
            .filter(|(_, t)| **t > now)
            .map(|(e, t)| (e.details().0, (t - now).div_ceil(1000)))
            .collect();
        result.sort();
        result
    }
    fn enqueue(&mut self, event: Event, now: u64) {
        if self.cooldowns.get(&event).is_some_and(|until| now < *until)
            || self.current.is_some_and(|(e, _)| e == event)
            || self.queue.iter().any(|(e, _)| *e == event)
        {
            return;
        }
        if self.queue.len() == 4 {
            self.queue.pop_front();
        }
        self.queue.push_back((event, now));
    }
    pub fn update(&mut self, state: &State, now: u64) -> Decision {
        let mut state = state.clone();
        state.normalize();
        if let Some(old) = self.previous.clone() {
            if old.connected == Some(true) && state.connected == Some(false) {
                self.enqueue(Event::Lost, now);
            }
            if old.connected == Some(false) && state.connected == Some(true) {
                self.enqueue(Event::Restored, now);
            }
            if let (Some(a), Some(b)) = (old.battery, state.battery) {
                if !a.charging && b.charging {
                    self.enqueue(Event::Charge, now);
                }
                if a.level < 100. && b.level >= 100. {
                    self.enqueue(Event::Full, now);
                }
            }
            if old.headphones == Some(false) && state.headphones == Some(true) {
                self.enqueue(Event::Headphones, now);
            }
            if old.hour < 7 && (7..12).contains(&state.hour) {
                self.enqueue(Event::Wake, now);
            }
            if old.volume.is_some_and(|v| v > 0.) && state.volume == Some(0.) {
                self.enqueue(Event::Muted, now);
            }
            if old.volume.is_some_and(|v| v <= 85.) && state.volume.is_some_and(|v| v > 85.) {
                self.enqueue(Event::Loud, now);
            }
            if old.bluetooth.is_none() && state.bluetooth.is_some() {
                self.enqueue(Event::Accessory, now);
            }
        }
        self.previous = Some(state.clone());
        self.cpu.update(state.cpu, 80., 60., now);
        self.memory.update(state.memory, 85., 80., now);
        self.queue.retain(|(e, time)| {
            now.saturating_sub(*time) <= 8000
                && match e {
                    Event::Lost => state.connected == Some(false),
                    Event::Restored => state.connected == Some(true),
                    Event::Charge => state.battery.is_some_and(|b| b.charging),
                    Event::Headphones => state.headphones == Some(true),
                    _ => true,
                }
        });
        if self.current.is_some_and(|(_, end)| now >= end) {
            self.current = None;
        }
        if self.current.is_none() && !self.queue.is_empty() {
            let index = self
                .queue
                .iter()
                .enumerate()
                .max_by_key(|(_, (e, _))| e.details().2)
                .unwrap()
                .0;
            let (event, _) = self.queue.remove(index).unwrap();
            self.current = Some((event, now + event.details().3));
            self.cooldowns.insert(event, now + event.details().4);
        }
        let low = state.battery.is_some_and(|b| !b.charging && b.level < 15.);
        let tired = state.battery.is_some_and(|b| !b.charging && b.level < 30.);
        let charging = state.battery.is_some_and(|b| b.charging);
        let stressed = self.cpu.active || self.memory.active;
        let critical = low || (self.cpu.active && state.cpu.is_some_and(|v| v > 95.));
        let intensity = if critical {
            Intensity::Critical
        } else if stressed {
            Intensity::Stressed
        } else if tired || state.cpu.is_some_and(|v| v > 60.) {
            Intensity::Active
        } else {
            Intensity::Calm
        };
        if now.saturating_sub(self.last_ambient) >= 30_000 {
            self.last_ambient = now;
            self.seed ^= self.seed << 13;
            self.seed ^= self.seed >> 7;
            self.seed ^= self.seed << 17;
            self.ambient_animation = match self.seed % 4 {
                0 => "celebrate",
                1 => "walk",
                _ => "wave",
            };
            self.ambient_until = now
                + if self.ambient_animation == "walk" {
                    2880
                } else {
                    1200
                };
        }
        // Start a complete blink at the next update, even if sparse sampling skipped its deadline.
        if now.saturating_sub(self.last_blink) >= 11_000 {
            self.last_blink = now;
            self.blink_until = now + 350;
        }
        let (animation, reason, priority) = if let Some((event, _)) = self.current {
            let (a, r, p, _, _) = event.details();
            (a, r, p)
        } else if low {
            ("battery_low", "Running on fumes. Find a charger.", 80)
        } else if stressed {
            if self.cpu.active && state.cpu.is_some_and(|v| v >= 95.) {
                (
                    "cpu_critical",
                    "CPU is very busy. Heavy work may take longer.",
                    65,
                )
            } else if self.memory.active && state.memory.is_some_and(|v| v >= 95.) {
                (
                    "memory_critical",
                    "RAM is nearly full. Close unused apps or tabs.",
                    65,
                )
            } else if self.cpu.active {
                ("cpu_high", "CPU is busy. Let heavy tasks finish.", 60)
            } else {
                (
                    "memory_high",
                    "RAM use is high. Close unused apps or tabs.",
                    60,
                )
            }
        } else if state.audio_playing == Some(true) && state.volume != Some(0.) {
            ("dance", "Tiny dance party!", 55)
        } else if charging {
            ("charging", "Borrowing some electricity.", 50)
        } else if state.connected == Some(false) {
            ("wifi_search", "Still looking for a connection.", 45)
        } else if (state.hour >= 23 || state.hour < 7) && now % 180_000 >= 150_000 {
            ("sleep", "Do we really need to be awake?", 25)
        } else if tired {
            ("tired", "Energy conservation. Very strategic.", 20)
        } else if state.headphones == Some(true) {
            ("headphones_idle", "In my own little world.", 15)
        } else if now < self.ambient_until {
            (self.ambient_animation, "A little stretch and a hello.", 10)
        } else if now < self.blink_until {
            ("blink", "Just keeping an eye on things.", 5)
        } else if state.battery.is_some_and(|b| b.level >= 80.) {
            (
                "battery_high",
                "Plenty of power. Questionable intentions.",
                2,
            )
        } else {
            ("idle", "This corner is mine now.", 0)
        };
        Decision {
            animation,
            reason,
            priority,
            intensity,
            headphones: state.headphones == Some(true),
            rain: matches!(state.weather, Some(Weather::Rain | Weather::Thunder)),
            heat: stressed || state.weather == Some(Weather::Hot),
            charging,
            cpu_busy: self.cpu.active,
            memory_busy: self.memory.active,
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::Battery;
    #[test]
    fn event_wins_then_returns_to_persistent_state() {
        let mut d = Director::default();
        let mut s = State::demo();
        d.update(&s, 0);
        s.connected = Some(false);
        s.battery = Some(Battery {
            level: 8.,
            charging: false,
        });
        assert_eq!(d.update(&s, 100).animation, "wifi_lost");
        assert_eq!(d.update(&s, 2600).animation, "battery_low");
    }
    #[test]
    fn hysteresis_requires_sustained_load_and_recovery() {
        let mut d = Director::default();
        let mut s = State::demo();
        s.cpu = Some(90.);
        assert_eq!(d.update(&s, 0).animation, "idle");
        s.cpu = Some(79.);
        d.update(&s, 4000);
        s.cpu = Some(90.);
        d.update(&s, 5000);
        assert_eq!(d.update(&s, 9999).animation, "idle");
        assert_eq!(d.update(&s, 10000).animation, "cpu_high");
        s.cpu = Some(30.);
        d.update(&s, 11000);
        assert_eq!(d.update(&s, 20000).animation, "cpu_high");
        assert_eq!(d.update(&s, 21000).animation, "idle");
    }
    #[test]
    fn cooldown_suppresses_animation_but_not_state() {
        let mut d = Director::default();
        let mut s = State::demo();
        d.update(&s, 0);
        s.connected = Some(false);
        d.update(&s, 100);
        s.connected = Some(true);
        d.update(&s, 3000);
        s.connected = Some(false);
        assert_eq!(d.update(&s, 5000).animation, "wifi_search");
    }
    #[test]
    fn queued_events_do_not_interrupt_and_stale_events_drop() {
        let mut d = Director::default();
        let mut s = State::demo();
        d.update(&s, 0);
        s.connected = Some(false);
        d.update(&s, 100);
        s.battery.as_mut().unwrap().charging = true;
        assert_eq!(d.update(&s, 200).animation, "wifi_lost");
        s.battery.as_mut().unwrap().charging = false;
        assert_eq!(d.update(&s, 2600).animation, "wifi_search");
        assert!(d.queued().is_empty());
    }
    #[test]
    fn missing_signals_are_not_low_battery_or_offline() {
        let mut d = Director::default();
        assert_eq!(d.update(&State::default(), 0).animation, "idle");
    }
    #[test]
    fn modifiers_survive_high_priority_events() {
        let mut d = Director::default();
        let mut s = State::demo();
        s.headphones = Some(true);
        s.weather = Some(Weather::Rain);
        d.update(&s, 0);
        s.connected = Some(false);
        let decision = d.update(&s, 100);
        assert_eq!(decision.animation, "wifi_lost");
        assert!(decision.rain && decision.headphones);
    }
    #[test]
    fn sparse_updates_start_a_complete_blink_and_ambient_activity() {
        let mut director = Director::default();
        let state = State {
            hour: 12,
            ..State::default()
        };
        assert_eq!(director.update(&state, 0).animation, "idle");
        assert_eq!(director.update(&state, 12_200).animation, "blink");
        assert_eq!(director.update(&state, 12_400).animation, "blink");
        assert_eq!(director.update(&state, 12_600).animation, "idle");
        assert!(["wave", "walk", "celebrate"].contains(&director.update(&state, 31_000).animation));
    }
    #[test]
    fn playback_dances_but_headphones_and_mute_are_not_playback() {
        let mut director = Director::default();
        let mut state = State::demo();
        state.audio_playing = Some(true);
        assert_eq!(director.update(&state, 0).animation, "dance");
        state.audio_playing = Some(false);
        assert_ne!(director.update(&state, 100).animation, "dance");
        state.headphones = Some(true);
        assert_ne!(director.update(&state, 200).animation, "dance");
        state.audio_playing = Some(true);
        state.volume = Some(0.);
        assert_ne!(director.update(&state, 5000).animation, "dance");
        state.volume = Some(35.);
        state.battery = Some(Battery {
            level: 5.,
            charging: false,
        });
        assert_eq!(director.update(&state, 6000).animation, "battery_low");
    }
    #[test]
    fn ram_warning_is_distinct_sustained_and_recovers_without_flapping() {
        let mut director = Director::default();
        let mut state = State::demo();
        state.memory = Some(90.);
        assert!(!director.update(&state, 0).memory_busy);
        let warning = director.update(&state, 5000);
        assert_eq!(warning.animation, "memory_high");
        assert!(warning.memory_busy && !warning.cpu_busy);
        state.memory = Some(97.);
        assert_eq!(director.update(&state, 6000).animation, "memory_critical");
        state.memory = Some(78.);
        assert!(director.update(&state, 7000).memory_busy);
        state.memory = Some(65.);
        assert!(director.update(&state, 8000).memory_busy);
        assert!(!director.update(&state, 18000).memory_busy);
        state.memory = None;
        assert!(!director.update(&state, 19000).memory_busy);
    }
}
