//! Facts a bug report needs about the machine and the player, written to
//! the session log: the device once at startup, and how mpv plays each
//! stream. Probes only print; nothing reads them back. A probe that cannot
//! tell says `unknown` rather than failing. None logs who the user is: no
//! host or user name, no path, no serial.

use std::fmt;

use sysinfo::{CpuRefreshKind, MemoryRefreshKind, RefreshKind, System};

use crate::signals::{DisplayFacts, DisplaysProbe, MpvVersions, PlaybackDrops, PlaybackProbe};

const UNKNOWN: &str = "unknown";

/// Logs the OS, CPU and memory, every GPU adapter and, on Linux, the
/// graphical session.
pub fn log_device() {
    log::info!("os: {}", os());
    log::info!("cpu: {}", cpu());
    let gpus = gpus();
    if gpus.is_empty() {
        log::info!("gpu: {UNKNOWN}");
    }
    for (i, gpu) in gpus.iter().enumerate() {
        log::info!("gpu {} of {}: {gpu}", i + 1, gpus.len());
    }
    if cfg!(target_os = "linux") {
        log::info!("session: {}", session(|name| std::env::var(name).ok()));
    }
}

/// `DisplaysProbe`: one line per screen.
pub fn displays(signal: &DisplaysProbe) {
    for line in display_lines(signal) {
        log::info!("{line}");
    }
}

/// `MpvVersions`.
pub fn mpv_versions(signal: &MpvVersions) {
    log::info!(
        "player: {}, FFmpeg {}",
        or_unknown(&signal.mpv),
        or_unknown(&signal.ffmpeg)
    );
}

/// `PlaybackProbe`.
pub fn playback(signal: &PlaybackProbe) {
    log::info!("{}", playback_line(signal));
}

/// `PlaybackDrops`.
pub fn drops(signal: &PlaybackDrops) {
    log::info!(
        "playback ended: {} frames dropped by the output, {} by the decoder, {} mistimed",
        or_unknown(&signal.output),
        or_unknown(&signal.decoder),
        or_unknown(&signal.mistimed),
    );
}

/// Whether to keep mpv's verbose line `message`: only the OpenGL context
/// it renders with (on Windows ANGLE's, which names the D3D driver).
pub fn keeps_verbose(message: &str) -> bool {
    ["GL_VENDOR=", "GL_RENDERER=", "GL_VERSION="]
        .iter()
        .any(|key| message.trim_start().starts_with(key))
}

fn os() -> String {
    let long = System::long_os_version().unwrap_or_else(|| UNKNOWN.to_owned());
    let version = System::os_version().unwrap_or_else(|| UNKNOWN.to_owned());
    let kernel = System::kernel_version().unwrap_or_else(|| UNKNOWN.to_owned());
    let line = format!("{long}, version {version}, kernel {kernel}");
    match android_api_level() {
        Some(level) => format!("{line}, API level {level}"),
        None => line,
    }
}

#[cfg(target_os = "android")]
fn android_api_level() -> Option<String> {
    let mut value = [0_u8; libc::PROP_VALUE_MAX as usize];
    // SAFETY: the name is NUL-terminated and `value` holds PROP_VALUE_MAX
    // bytes, the most the call writes.
    let len = unsafe {
        libc::__system_property_get(c"ro.build.version.sdk".as_ptr(), value.as_mut_ptr().cast())
    };
    let len = usize::try_from(len).ok().filter(|len| *len > 0)?;
    std::str::from_utf8(value.get(..len)?)
        .ok()
        .map(str::to_owned)
}

#[cfg(not(target_os = "android"))]
fn android_api_level() -> Option<String> {
    None
}

fn cpu() -> String {
    let system = System::new_with_specifics(
        RefreshKind::nothing()
            .with_cpu(CpuRefreshKind::nothing())
            .with_memory(MemoryRefreshKind::nothing().with_ram()),
    );
    let brand = system
        .cpus()
        .first()
        .map(|cpu| cpu.brand().trim())
        .filter(|brand| !brand.is_empty())
        .unwrap_or(UNKNOWN);
    format!(
        "{brand}, {} logical cores, {} MiB RAM",
        system.cpus().len(),
        system.total_memory() / MIB,
    )
}

const MIB: u64 = 1024 * 1024;

/// One GPU adapter, as the OS describes it; `None` where it did not say.
#[derive(Default)]
struct Gpu {
    name: Option<String>,
    vendor_id: Option<u32>,
    device_id: Option<u32>,
    /// The driver, with its version where the OS gives one.
    driver: Option<String>,
    vram_bytes: Option<u64>,
}

impl fmt::Display for Gpu {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        let id = |id: Option<u32>| id.map_or_else(|| UNKNOWN.to_owned(), |id| format!("{id:04x}"));
        write!(
            f,
            "{} [{}:{}], driver {}, ",
            self.name.as_deref().unwrap_or(UNKNOWN),
            id(self.vendor_id),
            id(self.device_id),
            self.driver.as_deref().unwrap_or(UNKNOWN),
        )?;
        match self.vram_bytes {
            Some(bytes) => write!(f, "{} MiB dedicated VRAM", bytes / MIB),
            None => write!(f, "dedicated VRAM {UNKNOWN}"),
        }
    }
}

/// Every adapter DXGI lists, the one apps get by default first. A hybrid
/// laptop lists both its GPUs.
#[cfg(windows)]
fn gpus() -> Vec<Gpu> {
    use windows::Win32::Graphics::Dxgi::{CreateDXGIFactory1, IDXGIDevice, IDXGIFactory1};
    use windows::core::Interface;

    // SAFETY: DXGI calls on a factory and adapters this function owns,
    // with no pointers of ours.
    let Ok(factory) = (unsafe { CreateDXGIFactory1::<IDXGIFactory1>() }) else {
        return Vec::new();
    };
    (0..)
        .map_while(|i| unsafe { factory.EnumAdapters1(i) }.ok())
        .map(|adapter| {
            // The user-mode driver's version, as Device Manager shows it.
            let driver = unsafe { adapter.CheckInterfaceSupport(&IDXGIDevice::IID) }
                .ok()
                .map(umd_version);
            match unsafe { adapter.GetDesc1() } {
                Ok(desc) => Gpu {
                    name: Some(utf16_until_nul(&desc.Description)),
                    vendor_id: Some(desc.VendorId),
                    device_id: Some(desc.DeviceId),
                    driver,
                    vram_bytes: u64::try_from(desc.DedicatedVideoMemory).ok(),
                },
                Err(_) => Gpu {
                    driver,
                    ..Gpu::default()
                },
            }
        })
        .collect()
}

/// Every DRM card in sysfs. sysfs names no GPU, so the PCI ids stand for
/// its name; amdgpu gives its VRAM, NVIDIA's driver its version.
#[cfg(not(windows))]
fn gpus() -> Vec<Gpu> {
    use std::fs;
    use std::path::Path;

    let drm = Path::new("/sys/class/drm");
    let Ok(entries) = fs::read_dir(drm) else {
        return Vec::new();
    };
    let mut cards: Vec<String> = entries
        .filter_map(Result::ok)
        .filter_map(|entry| entry.file_name().into_string().ok())
        .filter(|name| is_card(name))
        .collect();
    cards.sort();
    let read = |path: &Path| {
        fs::read_to_string(path)
            .ok()
            .map(|text| text.trim().to_owned())
    };
    cards
        .iter()
        .map(|card| {
            let device = drm.join(card).join("device");
            let driver = fs::read_link(device.join("driver"))
                .ok()
                .and_then(|link| link.file_name()?.to_str().map(str::to_owned));
            let version = driver
                .as_ref()
                .and_then(|driver| read(&Path::new("/sys/module").join(driver).join("version")));
            Gpu {
                name: None,
                vendor_id: read(&device.join("vendor")).and_then(|id| hex(&id)),
                device_id: read(&device.join("device")).and_then(|id| hex(&id)),
                driver: driver.map(|driver| match version {
                    Some(version) => format!("{driver} {version}"),
                    None => driver,
                }),
                vram_bytes: read(&device.join("mem_info_vram_total")).and_then(|b| b.parse().ok()),
            }
        })
        .collect()
}

/// `card0`, not its connectors (`card0-DP-1`) nor render nodes.
#[cfg(not(windows))]
fn is_card(name: &str) -> bool {
    name.strip_prefix("card")
        .is_some_and(|n| !n.is_empty() && n.bytes().all(|b| b.is_ascii_digit()))
}

/// sysfs's `0x1002`.
#[cfg(not(windows))]
fn hex(text: &str) -> Option<u32> {
    u32::from_str_radix(text.trim_start_matches("0x"), 16).ok()
}

/// DXGI's user-mode driver version: four 16-bit parts, most significant
/// first (`31.0.21921.1000`).
#[cfg_attr(not(windows), allow(dead_code))]
fn umd_version(version: i64) -> String {
    let part = |shift: u32| (version >> shift) & 0xffff;
    format!("{}.{}.{}.{}", part(48), part(32), part(16), part(0))
}

#[cfg_attr(not(windows), allow(dead_code))]
fn utf16_until_nul(text: &[u16]) -> String {
    let len = text.iter().position(|c| *c == 0).unwrap_or(text.len());
    String::from_utf16_lossy(&text[..len])
}

/// The graphical session from the environment's `var`: its type, and
/// which display servers it can reach. Only whether each is set: the
/// values can hold paths.
fn session(var: impl Fn(&str) -> Option<String>) -> String {
    let kind = var("XDG_SESSION_TYPE")
        .filter(|kind| !kind.is_empty())
        .unwrap_or_else(|| UNKNOWN.to_owned());
    let set: Vec<&str> = ["WAYLAND_DISPLAY", "DISPLAY", "GAMESCOPE_WAYLAND_DISPLAY"]
        .into_iter()
        .filter(|name| var(name).is_some())
        .collect();
    let set = if set.is_empty() {
        "none".to_owned()
    } else {
        set.join(", ")
    };
    format!("{kind}, set: {set}")
}

fn display_lines(signal: &DisplaysProbe) -> Vec<String> {
    let count = signal.displays.len();
    if count == 0 {
        return vec![format!("display: {UNKNOWN}")];
    }
    (1..)
        .zip(&signal.displays)
        .map(|(n, display)| format!("display {n} of {count}: {}", display_facts(display)))
        .collect()
}

fn display_facts(d: &DisplayFacts) -> String {
    let window = if d.has_window { ", has the window" } else { "" };
    format!(
        "{}x{} at {:.0} Hz, scale {}{window}",
        d.width, d.height, d.refresh_hz, d.scale
    )
}

fn playback_line(p: &PlaybackProbe) -> String {
    let u = or_unknown;
    format!(
        "playback: hwdec {} (interop {}), vo {} on {}; \
         video {} {}x{} at {} fps, {} (hw {}); audio {} to {}",
        u(&p.hwdec),
        u(&p.hwdec_interop),
        u(&p.vo),
        u(&p.gpu_context),
        u(&p.video_codec),
        u(&p.width),
        u(&p.height),
        u(&p.fps),
        u(&p.pixelformat),
        u(&p.hw_pixelformat),
        u(&p.audio_codec),
        u(&p.ao),
    )
}

/// mpv's text for a property, or `unknown` where it gave none.
fn or_unknown(text: &str) -> &str {
    match text.trim() {
        "" => UNKNOWN,
        text => text,
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn a_gpu_shows_its_ids_in_hex_and_what_it_did_not_say_as_unknown() {
        let gpu = Gpu {
            name: Some("AMD Radeon RX 6800".to_owned()),
            vendor_id: Some(0x1002),
            device_id: Some(0x73bf),
            driver: None,
            vram_bytes: Some(16 * 1024 * MIB),
        };
        assert_eq!(
            gpu.to_string(),
            "AMD Radeon RX 6800 [1002:73bf], driver unknown, 16384 MiB dedicated VRAM"
        );
        assert_eq!(
            Gpu::default().to_string(),
            "unknown [unknown:unknown], driver unknown, dedicated VRAM unknown"
        );
    }

    #[test]
    fn a_driver_version_reads_as_device_manager_shows_it() {
        let version = (31_i64 << 48) | (21921 << 16) | 1000;
        assert_eq!(umd_version(version), "31.0.21921.1000");
    }

    #[test]
    fn an_adapter_name_ends_at_its_nul() {
        let mut text = [0_u16; 8];
        for (slot, c) in text.iter_mut().zip("Intel".encode_utf16()) {
            *slot = c;
        }
        assert_eq!(utf16_until_nul(&text), "Intel");
    }

    #[cfg(not(windows))]
    #[test]
    fn only_cards_are_gpus_and_their_ids_parse() {
        assert!(is_card("card0"));
        assert!(is_card("card12"));
        assert!(!is_card("card0-DP-1"));
        assert!(!is_card("renderD128"));
        assert!(!is_card("card"));
        assert_eq!(hex("0x10de"), Some(0x10de));
    }

    #[test]
    fn a_session_says_which_display_servers_are_set_not_their_values() {
        let env = |name: &str| match name {
            "XDG_SESSION_TYPE" => Some("wayland".to_owned()),
            "WAYLAND_DISPLAY" => Some("wayland-0".to_owned()),
            "DISPLAY" => Some(":0".to_owned()),
            _ => None,
        };
        assert_eq!(session(env), "wayland, set: WAYLAND_DISPLAY, DISPLAY");
        assert_eq!(session(|_| None), "unknown, set: none");
    }

    #[test]
    fn each_display_gets_a_line_and_none_says_unknown() {
        let display = |has_window| DisplayFacts {
            width: 2560,
            height: 1440,
            refresh_hz: 143.998,
            scale: 1.25,
            has_window,
        };
        let probe = DisplaysProbe {
            displays: vec![display(true), display(false)],
        };
        assert_eq!(
            display_lines(&probe),
            [
                "display 1 of 2: 2560x1440 at 144 Hz, scale 1.25, has the window",
                "display 2 of 2: 2560x1440 at 144 Hz, scale 1.25",
            ]
        );
        assert_eq!(
            display_lines(&DisplaysProbe { displays: vec![] }),
            ["display: unknown"]
        );
    }

    #[test]
    fn a_playback_line_says_unknown_where_mpv_did_not_say() {
        let text = |s: &str| s.to_owned();
        let probe = PlaybackProbe {
            hwdec: text("d3d11va"),
            hwdec_interop: text("d3d11"),
            vo: text("libmpv"),
            gpu_context: String::new(),
            video_codec: text("h264"),
            width: text("1920"),
            height: text("1080"),
            fps: text("60.000000"),
            pixelformat: text("d3d11"),
            hw_pixelformat: text("nv12"),
            audio_codec: text("aac"),
            ao: text("wasapi"),
        };
        assert_eq!(
            playback_line(&probe),
            "playback: hwdec d3d11va (interop d3d11), vo libmpv on unknown; \
             video h264 1920x1080 at 60.000000 fps, d3d11 (hw nv12); audio aac to wasapi"
        );
    }

    #[test]
    fn only_the_gl_context_s_verbose_lines_are_kept() {
        assert!(keeps_verbose(
            "GL_RENDERER='ANGLE (NVIDIA, D3D11 31.0.15.5222)'\n"
        ));
        assert!(keeps_verbose("GL_VERSION='OpenGL ES 3.0.0'"));
        assert!(!keeps_verbose(
            "Opening 'https://edge.test/seg.ts' for reading"
        ));
    }
}
