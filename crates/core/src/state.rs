use serde::{Deserialize, Serialize};

#[derive(Debug, Clone, Copy, PartialEq, Serialize, Deserialize)]
pub struct Battery {
    pub level: f32,
    pub charging: bool,
}

#[derive(Debug, Default, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum Weather {
    #[default]
    Clear,
    Cloudy,
    Rain,
    Snow,
    Thunder,
    Hot,
    Cold,
}

#[derive(Debug, Default, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum BluetoothCategory {
    #[default]
    Generic,
    Audio,
    Controller,
    Accessory,
}

/// Unknown readings stay unknown. No pack ever receives OS handles or device names.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
#[serde(default)]
pub struct State {
    pub battery: Option<Battery>,
    pub cpu: Option<f32>,
    pub memory: Option<f32>,
    pub connected: Option<bool>,
    pub hour: u8,
    /// Monday = 0, Sunday = 6.
    pub day: u8,
    pub weather: Option<Weather>,
    pub volume: Option<f32>,
    pub headphones: Option<bool>,
    /// Aggregate output activity, not music recognition or recorded audio.
    #[serde(default)]
    pub audio_playing: Option<bool>,
    pub bluetooth: Option<BluetoothCategory>,
}
impl Default for State {
    fn default() -> Self {
        Self {
            battery: None,
            cpu: None,
            memory: None,
            connected: None,
            hour: 12,
            day: 0,
            weather: None,
            volume: None,
            headphones: None,
            audio_playing: None,
            bluetooth: None,
        }
    }
}
impl State {
    pub fn demo() -> Self {
        Self {
            battery: Some(Battery {
                level: 72.0,
                charging: false,
            }),
            cpu: Some(12.0),
            memory: Some(40.0),
            connected: Some(true),
            day: 2,
            volume: Some(35.0),
            headphones: Some(false),
            ..Self::default()
        }
    }
    pub fn normalize(&mut self) {
        fn percent(v: &mut Option<f32>) {
            *v = v.filter(|v| v.is_finite()).map(|v| v.clamp(0., 100.));
        }
        percent(&mut self.cpu);
        percent(&mut self.memory);
        percent(&mut self.volume);
        if let Some(b) = &mut self.battery {
            if b.level.is_finite() {
                b.level = b.level.clamp(0., 100.);
            } else {
                self.battery = None;
            }
        }
        self.hour = self.hour.min(23);
        self.day = self.day.min(6);
    }
}

#[derive(Debug, Default, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
pub enum PowerMode {
    #[default]
    Auto,
    Normal,
    Saver,
    Ultra,
}
impl PowerMode {
    pub fn effective(self, state: &State) -> Self {
        if self != Self::Auto {
            return self;
        }
        match state.battery {
            Some(Battery {
                level,
                charging: false,
            }) if level < 8. => Self::Ultra,
            Some(Battery {
                level,
                charging: false,
            }) if level < 20. => Self::Saver,
            _ => Self::Normal,
        }
    }
    pub fn sensor_seconds(self) -> u64 {
        match self {
            Self::Ultra => 30,
            Self::Saver => 10,
            _ => 3,
        }
    }
    pub fn frame_floor_ms(self) -> u64 {
        match self {
            Self::Ultra => 5000,
            Self::Saver => 750,
            _ => 50,
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    #[test]
    fn invalid_readings_do_not_become_real_conditions() {
        let mut s = State {
            cpu: Some(f32::NAN),
            memory: Some(300.),
            battery: Some(Battery {
                level: f32::INFINITY,
                charging: false,
            }),
            hour: 255,
            ..State::default()
        };
        s.normalize();
        assert_eq!(s.cpu, None);
        assert_eq!(s.battery, None);
        assert_eq!(s.memory, Some(100.));
        assert_eq!(s.hour, 23);
    }
    #[test]
    fn automatic_power_respects_charging_and_manual_override() {
        let mut s = State::demo();
        s.battery = Some(Battery {
            level: 5.,
            charging: false,
        });
        assert_eq!(PowerMode::Auto.effective(&s), PowerMode::Ultra);
        assert_eq!(PowerMode::Normal.effective(&s), PowerMode::Normal);
        s.battery.as_mut().unwrap().charging = true;
        assert_eq!(PowerMode::Auto.effective(&s), PowerMode::Normal);
    }
}
