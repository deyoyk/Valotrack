use std::{
    collections::HashSet,
    fs,
    path::{Path, PathBuf},
    sync::{
        mpsc::{self, Sender},
        Arc,
    },
    thread,
    time::Duration,
};

use parking_lot::Mutex;
use rodio::{
    cpal::{
        self,
        traits::HostTrait,
    },
    source::SineWave,
    Decoder, DeviceSinkBuilder, MixerDeviceSink, Player, Source,
};
use serde::{Deserialize, Serialize};
use serde_json::{json, Value};
use uuid::Uuid;

use crate::app::{store, AppState};

const MAX_SOUND_BYTES: u64 = 50 * 1024 * 1024;
const DEFAULT_VOLUME: f32 = 0.8;
const ALLOWED_EXTENSIONS: &[&str] = &["mp3", "wav", "ogg", "flac", "m4a"];

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct SoundEntry {
    pub id: String,
    pub name: String,
    pub file: String,
    pub size: u64,
}

#[derive(Debug, Clone, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct AudioDevice {
    pub name: String,
    pub is_default: bool,
    pub is_virtual: bool,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase", default)]
struct Store {
    sounds: Vec<SoundEntry>,
    selected_device: String,
    volume: f32,
    configured: bool,
}

impl Default for Store {
    fn default() -> Self {
        Self {
            sounds: Vec::new(),
            selected_device: String::new(),
            volume: DEFAULT_VOLUME,
            configured: false,
        }
    }
}

enum AudioCommand {
    Prepare {
        device: String,
        reply: Sender<Result<(), String>>,
    },
    Play {
        sound_id: String,
        path: PathBuf,
        device: String,
        volume: f32,
        reply: Sender<Result<(), String>>,
    },
    Test {
        device: String,
        volume: f32,
        reply: Sender<Result<(), String>>,
    },
    Stop {
        sound_id: String,
        reply: Sender<Result<(), String>>,
    },
    StopAll {
        reply: Sender<Result<(), String>>,
    },
}

struct Active {
    sound_id: String,
    player: Player,
}

#[derive(Default)]
struct AudioEngine {
    device: String,
    sink: Option<MixerDeviceSink>,
    active: Vec<Active>,
}

impl AudioEngine {
    fn ensure_device(&mut self, device: &str) -> Result<(), String> {
        if self.sink.is_some() && self.device == device {
            return Ok(());
        }
        self.active.clear();
        self.sink = None;
        let sink = if device.trim().is_empty() {
            DeviceSinkBuilder::open_default_sink().map_err(|error| error.to_string())?
        } else {
            let host = cpal::default_host();
            let target = host
                .output_devices()
                .map_err(|error| error.to_string())?
                .find(|candidate| candidate.to_string() == device)
                .ok_or_else(|| format!("Audio output '{device}' is no longer available."))?;
            DeviceSinkBuilder::from_device(target)
                .map_err(|error| error.to_string())?
                .open_sink_or_fallback()
                .map_err(|error| error.to_string())?
        };
        self.device = device.to_string();
        self.sink = Some(sink);
        Ok(())
    }

    fn cleanup(&mut self) {
        self.active.retain(|entry| !entry.player.empty());
    }

    fn play(
        &mut self,
        sound_id: String,
        path: PathBuf,
        device: &str,
        volume: f32,
    ) -> Result<(), String> {
        self.ensure_device(device)?;
        self.cleanup();
        let sink = self.sink.as_ref().expect("audio sink");
        let player = Player::connect_new(sink.mixer());
        player.set_volume(volume.clamp(0.0, 1.0));
        let file = fs::File::open(&path)
            .map_err(|error| format!("Could not open {}: {error}", path.display()))?;
        let source = Decoder::try_from(file)
            .map_err(|error| format!("Could not decode {}: {error}", path.display()))?;
        player.append(source);
        self.active.push(Active { sound_id, player });
        Ok(())
    }

    fn test(&mut self, device: &str, volume: f32) -> Result<(), String> {
        self.ensure_device(device)?;
        self.cleanup();
        let sink = self.sink.as_ref().expect("audio sink");
        let player = Player::connect_new(sink.mixer());
        player.set_volume(volume.clamp(0.0, 1.0));
        player.append(
            SineWave::new(660.0)
                .take_duration(Duration::from_millis(650))
                .amplify(0.35),
        );
        Ok(())
    }

    fn stop_sound(&mut self, sound_id: &str) -> Result<(), String> {
        let mut index = 0;
        while index < self.active.len() {
            if self.active[index].sound_id == sound_id {
                let entry = self.active.remove(index);
                entry.player.stop();
            } else {
                index += 1;
            }
        }
        Ok(())
    }

    fn stop_all(&mut self) -> Result<(), String> {
        for entry in self.active.drain(..) {
            entry.player.stop();
        }
        Ok(())
    }
}

fn start_audio_thread() -> Sender<AudioCommand> {
    let (tx, rx) = mpsc::channel::<AudioCommand>();
    let _ = thread::Builder::new()
        .name("valotrack-soundboard".into())
        .spawn(move || {
            let mut engine = AudioEngine::default();
            while let Ok(command) = rx.recv() {
                match command {
                    AudioCommand::Prepare { device, reply } => {
                        let _ = reply.send(engine.ensure_device(&device));
                    }
                    AudioCommand::Play {
                        sound_id,
                        path,
                        device,
                        volume,
                        reply,
                    } => {
                        let _ = reply.send(engine.play(sound_id, path, &device, volume));
                    }
                    AudioCommand::Test {
                        device,
                        volume,
                        reply,
                    } => {
                        let _ = reply.send(engine.test(&device, volume));
                    }
                    AudioCommand::Stop { sound_id, reply } => {
                        let _ = reply.send(engine.stop_sound(&sound_id));
                    }
                    AudioCommand::StopAll { reply } => {
                        let _ = reply.send(engine.stop_all());
                    }
                }
            }
        });
    tx
}

fn is_virtual_device(name: &str) -> bool {
    let name = name.to_ascii_lowercase();
    name.contains("cable input")
        || name.contains("cable output")
        || name.contains("vb-audio")
        || name.contains("voicemeeter")
        || name.contains("virtual")
}

fn device_rows(
    devices: impl Iterator<Item = cpal::Device>,
    default_name: Option<&str>,
) -> Vec<AudioDevice> {
    let mut seen = HashSet::new();
    let mut rows = Vec::new();
    for device in devices {
        let name = device.to_string();
        if name.trim().is_empty() || !seen.insert(name.clone()) {
            continue;
        }
        rows.push(AudioDevice {
            is_default: default_name == Some(name.as_str()),
            is_virtual: is_virtual_device(&name),
            name,
        });
    }
    rows
}

fn output_devices() -> Result<Vec<AudioDevice>, String> {
    let host = cpal::default_host();
    let default_name = host.default_output_device().map(|device| device.to_string());
    Ok(device_rows(
        host.output_devices().map_err(|error| error.to_string())?,
        default_name.as_deref(),
    ))
}

fn input_devices() -> Result<Vec<AudioDevice>, String> {
    let host = cpal::default_host();
    let default_name = host.default_input_device().map(|device| device.to_string());
    Ok(device_rows(
        host.input_devices().map_err(|error| error.to_string())?,
        default_name.as_deref(),
    ))
}

fn valid_extension(path: &Path) -> Option<String> {
    let extension = path.extension()?.to_string_lossy().to_lowercase();
    ALLOWED_EXTENSIONS
        .contains(&extension.as_str())
        .then_some(extension)
}

fn sound_name(path: &Path) -> String {
    path.file_stem()
        .map(|name| name.to_string_lossy().trim().replace('_', " "))
        .filter(|name| !name.is_empty())
        .map(|name| name.chars().take(80).collect())
        .unwrap_or_else(|| "Sound".to_string())
}

pub struct Soundboard {
    data_dir: PathBuf,
    store: Mutex<Store>,
    tx: Sender<AudioCommand>,
}

impl Soundboard {
    pub fn new(data_dir: &Path) -> Self {
        let mut store = load_store(data_dir);
        if store.volume <= 0.0 {
            store.volume = DEFAULT_VOLUME;
        }
        if !store.configured {
            if let Ok(devices) = output_devices() {
                if let Some(device) = devices.into_iter().find(|device| device.is_virtual) {
                    store.selected_device = device.name;
                    store.configured = true;
                    let _ = save_store(data_dir, &store);
                }
            }
        }
        Self {
            data_dir: data_dir.to_path_buf(),
            store: Mutex::new(store),
            tx: start_audio_thread(),
        }
    }

    fn soundboard_dir(&self) -> PathBuf {
        self.data_dir.join("soundboard")
    }

    fn send(&self, build: impl FnOnce(Sender<Result<(), String>>) -> AudioCommand) -> Result<(), String> {
        let (reply, result) = mpsc::channel();
        self.tx
            .send(build(reply))
            .map_err(|_| "Audio thread is not available.".to_string())?;
        result
            .recv_timeout(Duration::from_secs(8))
            .map_err(|_| "Audio device did not respond in time.".to_string())?
    }

    pub fn snapshot(&self) -> Result<Value, String> {
        let outputs = output_devices()?;
        let inputs = input_devices()?;
        let mut store = self.store.lock();
        if !store.configured {
            if let Some(device) = outputs.iter().find(|device| device.is_virtual) {
                store.selected_device = device.name.clone();
                store.configured = true;
                let _ = save_store(&self.data_dir, &store);
            }
        }
        if !store.selected_device.is_empty()
            && !outputs.iter().any(|device| device.name == store.selected_device)
        {
            store.selected_device.clear();
        }
        Ok(json!({
            "sounds": store.sounds,
            "outputs": outputs,
            "inputs": inputs,
            "selectedDevice": store.selected_device,
            "volume": store.volume,
            "folder": self.soundboard_dir().to_string_lossy()
        }))
    }

    pub fn import(&self, paths: Vec<String>) -> Result<Value, String> {
        let mut pending = Vec::new();
        for raw in paths {
            let source = PathBuf::from(&raw);
            let Some(extension) = valid_extension(&source) else {
                return Err(format!(
                    "{} is not a supported audio file. Use mp3, wav, ogg, flac or m4a.",
                    source.display()
                ));
            };
            let metadata = fs::metadata(&source)
                .map_err(|error| format!("Could not read {}: {error}", source.display()))?;
            if !metadata.is_file() {
                return Err(format!("{} is not a file.", source.display()));
            }
            if metadata.len() > MAX_SOUND_BYTES {
                return Err(format!(
                    "{} is larger than 50 MB.",
                    source.display()
                ));
            }
            pending.push((source, extension));
        }
        if pending.is_empty() {
            return Err("No files were selected.".to_string());
        }
        let folder = self.soundboard_dir();
        fs::create_dir_all(&folder).map_err(|error| error.to_string())?;
        let mut imported = Vec::new();
        for (source, extension) in pending {
            let id = Uuid::new_v4().to_string();
            let file = format!("{id}.{extension}");
            let target = folder.join(&file);
            fs::copy(&source, &target)
                .map_err(|error| format!("Could not import {}: {error}", source.display()))?;
            let size = fs::metadata(&target).map(|meta| meta.len()).unwrap_or(0);
            imported.push(SoundEntry {
                id,
                name: sound_name(&source),
                file,
                size,
            });
        }
        {
            let mut store = self.store.lock();
            store.sounds.extend(imported);
            save_store(&self.data_dir, &store)?;
        }
        self.snapshot()
    }

    pub fn remove(&self, id: &str) -> Result<Value, String> {
        let _ = self.send(|reply| AudioCommand::Stop {
            sound_id: id.to_string(),
            reply,
        });
        let removed = {
            let mut store = self.store.lock();
            let Some(index) = store.sounds.iter().position(|sound| sound.id == id) else {
                return Err("Sound not found.".to_string());
            };
            let sound = store.sounds.remove(index);
            save_store(&self.data_dir, &store)?;
            sound
        };
        let _ = fs::remove_file(self.soundboard_dir().join(removed.file));
        self.snapshot()
    }

    pub fn play(&self, id: &str) -> Result<Value, String> {
        let (sound, device, volume) = {
            let store = self.store.lock();
            let sound = store
                .sounds
                .iter()
                .find(|sound| sound.id == id)
                .cloned()
                .ok_or_else(|| "Sound not found.".to_string())?;
            (sound, store.selected_device.clone(), store.volume)
        };
        let path = self.soundboard_dir().join(&sound.file);
        self.send(|reply| AudioCommand::Play {
            sound_id: sound.id.clone(),
            path,
            device,
            volume,
            reply,
        })?;
        Ok(json!({ "ok": true, "id": sound.id }))
    }

    pub fn stop(&self, id: &str) -> Result<Value, String> {
        self.send(|reply| AudioCommand::Stop {
            sound_id: id.to_string(),
            reply,
        })?;
        Ok(json!({ "ok": true }))
    }

    pub fn stop_all(&self) -> Result<Value, String> {
        self.send(|reply| AudioCommand::StopAll { reply })?;
        Ok(json!({ "ok": true }))
    }

    pub fn test(&self) -> Result<Value, String> {
        let (device, volume) = {
            let store = self.store.lock();
            (store.selected_device.clone(), store.volume)
        };
        self.send(|reply| AudioCommand::Test {
            device,
            volume,
            reply,
        })?;
        Ok(json!({ "ok": true }))
    }

    pub fn set_device(&self, device: String) -> Result<Value, String> {
        let device = device.trim().to_string();
        if !device.is_empty() {
            let devices = output_devices()?;
            if !devices.iter().any(|candidate| candidate.name == device) {
                return Err(format!("Audio output '{device}' is not available."));
            }
        }
        self.send(|reply| AudioCommand::Prepare {
            device: device.clone(),
            reply,
        })?;
        {
            let mut store = self.store.lock();
            store.selected_device = device;
            store.configured = true;
            save_store(&self.data_dir, &store)?;
        }
        self.snapshot()
    }

    pub fn set_volume(&self, volume: f32) -> Result<Value, String> {
        {
            let mut store = self.store.lock();
            store.volume = volume.clamp(0.0, 1.0);
            save_store(&self.data_dir, &store)?;
        }
        self.snapshot()
    }
}

fn load_store(data_dir: &Path) -> Store {
    let value = store::read(data_dir, "soundboard");
    serde_json::from_value(value).unwrap_or_default()
}

fn save_store(data_dir: &Path, store: &Store) -> Result<(), String> {
    let value = serde_json::to_value(store).map_err(|error| error.to_string())?;
    store::write(data_dir, "soundboard", &value)
}

pub fn snapshot(state: &Arc<AppState>) -> Result<Value, String> {
    state.soundboard.snapshot()
}

pub fn import(state: &Arc<AppState>, paths: Vec<String>) -> Result<Value, String> {
    state.soundboard.import(paths)
}

pub fn remove(state: &Arc<AppState>, id: String) -> Result<Value, String> {
    state.soundboard.remove(&id)
}

pub fn play(state: &Arc<AppState>, id: String) -> Result<Value, String> {
    state.soundboard.play(&id)
}

pub fn stop(state: &Arc<AppState>, id: String) -> Result<Value, String> {
    state.soundboard.stop(&id)
}

pub fn stop_all(state: &Arc<AppState>) -> Result<Value, String> {
    state.soundboard.stop_all()
}

pub fn test(state: &Arc<AppState>) -> Result<Value, String> {
    state.soundboard.test()
}

pub fn set_device(state: &Arc<AppState>, device: String) -> Result<Value, String> {
    state.soundboard.set_device(device)
}

pub fn set_volume(state: &Arc<AppState>, volume: f32) -> Result<Value, String> {
    state.soundboard.set_volume(volume)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn virtual_devices_are_detected() {
        assert!(is_virtual_device("CABLE Input (VB-Audio Virtual Cable)"));
        assert!(is_virtual_device("VoiceMeeter Input (VB-Audio VoiceMeeter VAIO)"));
        assert!(!is_virtual_device("Speakers (Realtek(R) Audio)"));
    }

    #[test]
    fn supported_audio_files_are_recognized() {
        assert_eq!(valid_extension(Path::new("airhorn.MP3")), Some("mp3".into()));
        assert_eq!(valid_extension(Path::new("voice.wav")), Some("wav".into()));
        assert_eq!(valid_extension(Path::new("clip.txt")), None);
    }

    #[test]
    fn names_are_readable() {
        assert_eq!(sound_name(Path::new("my_cool_sound.mp3")), "my cool sound");
    }
}
