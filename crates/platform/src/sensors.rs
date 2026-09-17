use crate::{weather, Settings};
use chrono::{Datelike, Local, Timelike};
use companion_core::{Battery, BluetoothCategory, State};
use std::{
    sync::{mpsc, Arc, Mutex},
    thread,
    time::{Duration, Instant},
};
use sysinfo::System;

#[derive(Clone, Debug)]
pub struct SensorSnapshot {
    pub state: State,
    pub weather_status: String,
    pub note: String,
    pub samples: u64,
}
impl Default for SensorSnapshot {
    fn default() -> Self {
        Self {
            state: State::default(),
            weather_status: "Weather is off".into(),
            note: "Waiting for the first local reading".into(),
            samples: 0,
        }
    }
}
pub struct SensorHub {
    pub latest: Arc<Mutex<SensorSnapshot>>,
    tx: mpsc::Sender<Option<Settings>>,
}
impl SensorHub {
    pub fn start(settings: Settings, wake: impl Fn() + Send + 'static) -> Self {
        let latest = Arc::new(Mutex::new(SensorSnapshot::default()));
        let output = latest.clone();
        let (tx, rx) = mpsc::channel::<Option<Settings>>();
        thread::spawn(move || {
            let mut config = settings;
            let mut system = System::new();
            let manager = battery::Manager::new().ok();
            let mut first = true;
            let mut audio_until: Option<Instant> = None;
            let mut slow_at = Instant::now();
            let mut weather_at = Instant::now();
            let mut snapshot = SensorSnapshot::default();
            system.refresh_cpu_usage();
            loop {
                if config.sensors_enabled {
                    let time = Local::now();
                    snapshot.state.hour = time.hour() as u8;
                    snapshot.state.day = time.weekday().num_days_from_monday() as u8;
                    system.refresh_cpu_usage();
                    system.refresh_memory();
                    snapshot.state.cpu = if first {
                        None
                    } else {
                        Some(system.global_cpu_usage())
                    };
                    snapshot.state.memory = (system.total_memory() > 0)
                        .then(|| system.used_memory() as f32 / system.total_memory() as f32 * 100.);
                    let interfaces = netdev::get_interfaces();
                    snapshot.state.connected = if interfaces.is_empty() {
                        None
                    } else {
                        Some(interfaces.iter().any(|i| {
                            i.is_up()
                                && i.is_running()
                                && !i.is_loopback()
                                && (!i.ipv4.is_empty() || !i.ipv6.is_empty())
                        }))
                    };
                    let playback = native_playback();
                    if playback == Some(true) {
                        audio_until = Some(Instant::now() + Duration::from_secs(6));
                    }
                    snapshot.state.audio_playing = playback.map(|active| {
                        active || audio_until.is_some_and(|until| Instant::now() < until)
                    });
                    if first || slow_at.elapsed() >= Duration::from_secs(30) {
                        snapshot.state.battery = manager
                            .as_ref()
                            .and_then(|m| m.batteries().ok())
                            .and_then(|iter| iter.filter_map(Result::ok).next())
                            .map(|b| Battery {
                                level: b.state_of_charge().value * 100.,
                                charging: matches!(
                                    b.state(),
                                    battery::State::Charging | battery::State::Full
                                ),
                            });
                        let (volume, headphones, bluetooth) = native_accessories();
                        snapshot.state.volume = volume;
                        snapshot.state.headphones = headphones;
                        snapshot.state.bluetooth = bluetooth;
                        slow_at = Instant::now();
                    }
                    snapshot.note = "Local readings only. Network means an active local interface, not verified internet access.".into();
                } else {
                    snapshot.state = State::default();
                    audio_until = None;
                    snapshot.note = "Device readings paused".into();
                }
                if let Some(city) = &config.weather_city {
                    if first || weather_at.elapsed() >= Duration::from_secs(1800) {
                        match weather::fetch(city) {
                            Ok(condition) => {
                                snapshot.state.weather = Some(condition);
                                snapshot.weather_status =
                                    format!("{}, {} · {:?}", city.name, city.country, condition);
                            }
                            Err(_) => {
                                snapshot.state.weather = None;
                                snapshot.weather_status =
                                    "Weather unavailable. Retrying in 30 minutes.".into();
                            }
                        }
                        weather_at = Instant::now();
                    }
                } else {
                    snapshot.state.weather = None;
                    snapshot.weather_status = "Weather is off".into();
                }
                snapshot.state.normalize();
                snapshot.samples += 1;
                if let Ok(mut out) = output.lock() {
                    *out = snapshot.clone();
                }
                wake();
                first = false;
                let delay = config.power.effective(&snapshot.state).sensor_seconds();
                match rx.recv_timeout(Duration::from_secs(delay)) {
                    Ok(Some(new)) => {
                        if new
                            .weather_city
                            .as_ref()
                            .map(|c| (&c.name, c.latitude, c.longitude))
                            != config
                                .weather_city
                                .as_ref()
                                .map(|c| (&c.name, c.latitude, c.longitude))
                        {
                            weather_at = Instant::now() - Duration::from_secs(1801);
                        }
                        if new.sensors_enabled != config.sensors_enabled {
                            first = true;
                        }
                        config = new;
                    }
                    Ok(None) | Err(mpsc::RecvTimeoutError::Disconnected) => break,
                    Err(mpsc::RecvTimeoutError::Timeout) => {}
                }
            }
        });
        Self { latest, tx }
    }
    pub fn snapshot(&self) -> SensorSnapshot {
        self.latest.lock().map(|v| v.clone()).unwrap_or_default()
    }
    pub fn configure(&self, settings: Settings) {
        let _ = self.tx.send(Some(settings));
    }
}
impl Drop for SensorHub {
    fn drop(&mut self) {
        let _ = self.tx.send(None);
    }
}

#[cfg(target_os = "macos")]
fn native_accessories() -> (Option<f32>, Option<bool>, Option<BluetoothCategory>) {
    unsafe extern "C" {
        fn scopobot_audio(volume: *mut f32, headphones: *mut i32) -> i32;
        fn scopobot_bluetooth() -> i32;
    }
    let mut volume = -1.;
    let mut headphones = -1;
    // Small native adapter reads output settings and already-paired connection state only.
    let bluetooth = unsafe {
        scopobot_audio(&mut volume, &mut headphones);
        scopobot_bluetooth()
    };
    (
        if volume >= 0. { Some(volume) } else { None },
        if headphones >= 0 {
            Some(headphones != 0)
        } else {
            None
        },
        match bluetooth {
            1 => Some(BluetoothCategory::Audio),
            3 => Some(BluetoothCategory::Accessory),
            4 => Some(BluetoothCategory::Generic),
            _ => None,
        },
    )
}
#[cfg(target_os = "windows")]
fn native_accessories() -> (Option<f32>, Option<bool>, Option<BluetoothCategory>) {
    use windows::Win32::{
        Devices::Bluetooth::*,
        Media::Audio::{
            eConsole, eRender, Endpoints::IAudioEndpointVolume, Headphones, Headset,
            IMMDeviceEnumerator, MMDeviceEnumerator, PKEY_AudioEndpoint_FormFactor,
        },
        System::Com::{
            CoCreateInstance, CoInitializeEx, CoUninitialize,
            StructuredStorage::{PropVariantClear, PropVariantToUInt32},
            CLSCTX_ALL, COINIT_MULTITHREADED, STGM_READ,
        },
    };
    unsafe {
        let initialized = CoInitializeEx(None, COINIT_MULTITHREADED).is_ok();
        let audio = (|| -> windows::core::Result<(Option<f32>, Option<bool>)> {
            let enumerator: IMMDeviceEnumerator =
                CoCreateInstance(&MMDeviceEnumerator, None, CLSCTX_ALL)?;
            let device = enumerator.GetDefaultAudioEndpoint(eRender, eConsole)?;
            let volume = (|| -> windows::core::Result<f32> {
                let endpoint: IAudioEndpointVolume = device.Activate(CLSCTX_ALL, None)?;
                if endpoint.GetMute()?.as_bool() {
                    Ok(0.)
                } else {
                    Ok(endpoint.GetMasterVolumeLevelScalar()? * 100.)
                }
            })()
            .ok();
            let headphones = (|| -> windows::core::Result<bool> {
                let store = device.OpenPropertyStore(STGM_READ)?;
                let mut value = store.GetValue(&PKEY_AudioEndpoint_FormFactor)?;
                let form = PropVariantToUInt32(&value);
                let _ = PropVariantClear(&mut value);
                let form = form? as i32;
                Ok(form == Headphones.0 || form == Headset.0)
            })()
            .ok();
            Ok((volume, headphones))
        })()
        .unwrap_or((None, None));
        let search = BLUETOOTH_DEVICE_SEARCH_PARAMS {
            dwSize: std::mem::size_of::<BLUETOOTH_DEVICE_SEARCH_PARAMS>() as u32,
            fReturnConnected: true.into(),
            ..Default::default()
        };
        let mut info = BLUETOOTH_DEVICE_INFO {
            dwSize: std::mem::size_of::<BLUETOOTH_DEVICE_INFO>() as u32,
            ..Default::default()
        };
        let bluetooth = match BluetoothFindFirstDevice(&search, &mut info) {
            Ok(handle) => {
                let _ = BluetoothFindDeviceClose(handle);
                Some(match (info.ulClassofDevice >> 8) & 0x1f {
                    4 => BluetoothCategory::Audio,
                    5 => BluetoothCategory::Accessory,
                    _ => BluetoothCategory::Generic,
                })
            }
            Err(_) => None,
        };
        if initialized {
            CoUninitialize();
        }
        (audio.0, audio.1, bluetooth)
    }
}
#[cfg(not(any(target_os = "macos", target_os = "windows")))]
fn native_accessories() -> (Option<f32>, Option<bool>, Option<BluetoothCategory>) {
    (None, None, None)
}

#[cfg(target_os = "macos")]
fn native_playback() -> Option<bool> {
    unsafe extern "C" {
        fn scopobot_playback() -> i32;
    }
    match unsafe { scopobot_playback() } {
        0 => Some(false),
        1 => Some(true),
        _ => None,
    }
}
#[cfg(target_os = "windows")]
fn native_playback() -> Option<bool> {
    use windows::Win32::{
        Media::Audio::{
            eConsole, eRender, Endpoints::IAudioMeterInformation, IMMDeviceEnumerator,
            MMDeviceEnumerator,
        },
        System::Com::{
            CoCreateInstance, CoInitializeEx, CoUninitialize, CLSCTX_ALL, COINIT_MULTITHREADED,
        },
    };
    unsafe {
        let initialized = CoInitializeEx(None, COINIT_MULTITHREADED).is_ok();
        let result = (|| -> windows::core::Result<bool> {
            let enumerator: IMMDeviceEnumerator =
                CoCreateInstance(&MMDeviceEnumerator, None, CLSCTX_ALL)?;
            let device = enumerator.GetDefaultAudioEndpoint(eRender, eConsole)?;
            let meter: IAudioMeterInformation = device.Activate(CLSCTX_ALL, None)?;
            Ok(meter.GetPeakValue()? > 0.002)
        })()
        .ok();
        if initialized {
            CoUninitialize();
        }
        result
    }
}
#[cfg(not(any(target_os = "macos", target_os = "windows")))]
fn native_playback() -> Option<bool> {
    None
}
