#![cfg_attr(not(debug_assertions), windows_subsystem = "windows")]

mod anim {
    // Interpolasi ease-out untuk animasi expand / collapse.

    use std::time::Instant;

    /// Ease-out cubic: cepat di awal, melambat halus di akhir.
    pub fn ease_out_cubic(t: f32) -> f32 {
        let u = 1.0 - t.clamp(0.0, 1.0);
        1.0 - u * u * u
    }

    /// Tween 3 nilai sekaligus: [lebar, tinggi, offset-atas].
    #[derive(Clone, Copy)]
    pub struct Tween {
        from: [f32; 3],
        to: [f32; 3],
        start: Instant,
        dur: f32,
    }

    impl Tween {
        pub fn new(v: [f32; 3]) -> Self {
            Self { from: v, to: v, start: Instant::now(), dur: 0.0 }
        }

        fn progress(&self, now: Instant) -> f32 {
            if self.dur <= 0.0 {
                1.0
            } else {
                (now.duration_since(self.start).as_secs_f32() / self.dur).clamp(0.0, 1.0)
            }
        }

        pub fn value(&self, now: Instant) -> [f32; 3] {
            let e = ease_out_cubic(self.progress(now));
            let mut out = [0.0; 3];
            for i in 0..3 {
                out[i] = self.from[i] + (self.to[i] - self.from[i]) * e;
            }
            out
        }

        /// Mulai animasi baru dari posisi saat ini (tidak melompat).
        pub fn retarget(&mut self, now: Instant, to: [f32; 3], dur: f32) {
            self.from = self.value(now);
            self.to = to;
            self.start = now;
            self.dur = dur;
        }

        pub fn running(&self, now: Instant) -> bool {
            self.progress(now) < 1.0
        }
    }
}

mod draw {
    // Primitif gambar: teks, ikon vektor (tanpa font ikon), helper warna,
    // serta kontrol sistem (Settings, screenshot, kecerahan, volume).

    use eframe::egui::{
        epaint::text::{LayoutJob, TextFormat, TextWrapping},
        Align2, Color32, FontFamily, FontId, Painter, Pos2, Rect, Shape, Stroke, vec2,
    };
    use std::os::windows::process::CommandExt;

    /// CREATE_NO_WINDOW: cegah jendela console muncul saat menjalankan proses.
    const NO_WINDOW: u32 = 0x0800_0000;

    pub fn fade(c: Color32, a: f32) -> Color32 {
        c.gamma_multiply(a.clamp(0.0, 1.0))
    }

    fn family(bold: bool) -> FontFamily {
        if bold { FontFamily::Name("bold".into()) } else { FontFamily::Proportional }
    }

    pub fn text(p: &Painter, pos: Pos2, anchor: Align2, s: &str, size: f32, bold: bool, col: Color32) {
        p.text(pos, anchor, s, FontId::new(size, family(bold)), col);
    }

    /// Teks satu baris dengan elipsis ("…") bila melebihi max_w.
    pub fn text_ellipsis(p: &Painter, pos: Pos2, s: &str, size: f32, bold: bool, col: Color32, max_w: f32) {
        let mut job = LayoutJob::single_section(
            s.to_owned(),
            TextFormat { font_id: FontId::new(size, family(bold)), color: col, ..Default::default() },
        );
        job.wrap = TextWrapping {
            max_width: max_w,
            max_rows: 1,
            break_anywhere: true,
            overflow_character: Some('…'),
        };
        let galley = p.layout_job(job);
        p.galley(pos, galley, col);
    }

    pub fn tri(p: &Painter, a: Pos2, b: Pos2, c: Pos2, col: Color32) {
        p.add(Shape::convex_polygon(vec![a, b, c], col, Stroke::NONE));
    }

    pub fn icon_play(p: &Painter, c: Pos2, s: f32, col: Color32) {
        tri(p,
            Pos2::new(c.x - s * 0.34, c.y - s * 0.5),
            Pos2::new(c.x - s * 0.34, c.y + s * 0.5),
            Pos2::new(c.x + s * 0.56, c.y), col);
    }

    pub fn icon_pause(p: &Painter, c: Pos2, s: f32, col: Color32) {
        for dx in [-0.24f32, 0.24] {
            let r = Rect::from_center_size(Pos2::new(c.x + s * dx, c.y), vec2(s * 0.28, s));
            p.rect_filled(r, 1.5, col);
        }
    }

    pub fn icon_next(p: &Painter, c: Pos2, s: f32, col: Color32) {
        tri(p,
            Pos2::new(c.x - s * 0.55, c.y - s * 0.5),
            Pos2::new(c.x - s * 0.55, c.y + s * 0.5),
            Pos2::new(c.x + s * 0.35, c.y), col);
        p.rect_filled(Rect::from_center_size(Pos2::new(c.x + s * 0.5, c.y), vec2(s * 0.16, s)), 1.0, col);
    }

    pub fn icon_prev(p: &Painter, c: Pos2, s: f32, col: Color32) {
        tri(p,
            Pos2::new(c.x + s * 0.55, c.y - s * 0.5),
            Pos2::new(c.x + s * 0.55, c.y + s * 0.5),
            Pos2::new(c.x - s * 0.35, c.y), col);
        p.rect_filled(Rect::from_center_size(Pos2::new(c.x - s * 0.5, c.y), vec2(s * 0.16, s)), 1.0, col);
    }

    pub fn icon_sun(p: &Painter, c: Pos2, s: f32, col: Color32) {
        let st = Stroke::new(1.5_f32, col);
        p.circle_stroke(c, s * 0.22, st);
        for i in 0..8 {
            let a = i as f32 * std::f32::consts::TAU / 8.0;
            let d = vec2(a.cos(), a.sin());
            p.line_segment([c + d * (s * 0.36), c + d * (s * 0.5)], st);
        }
    }

    pub fn icon_moon(p: &Painter, c: Pos2, s: f32, col: Color32, bg: Color32) {
        p.circle_filled(c, s * 0.42, col);
        p.circle_filled(c + vec2(s * 0.22, -s * 0.18), s * 0.36, bg);
    }

    pub fn icon_close(p: &Painter, c: Pos2, s: f32, col: Color32) {
        let st = Stroke::new(1.6_f32, col);
        let h = s * 0.5;
        p.line_segment([c + vec2(-h, -h), c + vec2(h, h)], st);
        p.line_segment([c + vec2(-h, h), c + vec2(h, -h)], st);
    }

    pub fn icon_battery(p: &Painter, tl: Pos2, level: f32, outline: Color32, fill: Color32) {
        let body = Rect::from_min_size(tl, vec2(22.0, 11.0));
        p.rect_stroke(body, 3.0, Stroke::new(1.3_f32, outline));
        let inner = Rect::from_min_size(tl + vec2(2.0, 2.0), vec2(18.0 * level.clamp(0.08, 1.0), 7.0));
        p.rect_filled(inner, 1.6, fill);
        p.rect_filled(Rect::from_min_size(tl + vec2(23.0, 3.5), vec2(2.0, 4.0)), 1.0, outline);
    }

    /// Buka halaman Settings Windows melalui URI ms-settings.
    pub fn open_settings(page: &str) {
        let _ = std::process::Command::new("cmd")
            .args(["/C", "start", "", page])
            .creation_flags(NO_WINDOW)
            .spawn();
    }

    // ---------------- kecerahan ----------------

    fn run_brightness(percent: u8) {
        let script = format!(
            "$b=Get-CimInstance -Namespace root/WMI -ClassName WmiMonitorBrightnessMethods -ErrorAction SilentlyContinue; if ($b) {{ $b | ForEach-Object {{ Invoke-CimMethod -InputObject $_ -MethodName WmiSetBrightness -Arguments @{{Timeout=1; Brightness={}}} | Out-Null }} }}",
            percent
        );
        let _ = std::process::Command::new("powershell")
            .args(["-NoProfile", "-ExecutionPolicy", "Bypass", "-Command", &script])
            .creation_flags(NO_WINDOW)
            .status();
    }

    /// Kirim nilai kecerahan ke worker thread. Bila banyak nilai menumpuk saat
    /// slider digeser, hanya nilai terbaru yang dijalankan.
    /// Hanya bekerja di layar internal yang mendukung WMI.
    pub fn set_brightness(percent: u8) {
        use std::sync::{mpsc, OnceLock};
        static TX: OnceLock<mpsc::Sender<u8>> = OnceLock::new();
        let tx = TX.get_or_init(|| {
            let (tx, rx) = mpsc::channel::<u8>();
            std::thread::spawn(move || {
                while let Ok(mut v) = rx.recv() {
                    while let Ok(n) = rx.try_recv() {
                        v = n;
                    }
                    run_brightness(v);
                }
            });
            tx
        });
        let _ = tx.send(percent.min(100));
    }

    /// Baca kecerahan saat ini (blocking, panggil dari thread terpisah).
    pub fn get_brightness() -> Option<u8> {
        let out = std::process::Command::new("powershell")
            .args([
                "-NoProfile",
                "-Command",
                "(Get-CimInstance -Namespace root/WMI -ClassName WmiMonitorBrightness -ErrorAction SilentlyContinue | Select-Object -First 1).CurrentBrightness",
            ])
            .creation_flags(NO_WINDOW)
            .output()
            .ok()?;
        String::from_utf8_lossy(&out.stdout).trim().parse::<u8>().ok()
    }

    // ---------------- volume ----------------

    fn endpoint_volume() -> Option<windows::Win32::Media::Audio::Endpoints::IAudioEndpointVolume> {
        use windows::Win32::{
            Media::Audio::{
                eConsole, eRender, Endpoints::IAudioEndpointVolume, IMMDeviceEnumerator,
                MMDeviceEnumerator,
            },
            System::Com::{CoCreateInstance, CoInitializeEx, CLSCTX_ALL, COINIT_MULTITHREADED},
        };
        thread_local! {
            static INIT: std::cell::Cell<bool> = std::cell::Cell::new(false);
        }
        unsafe {
            INIT.with(|i| {
                if !i.get() {
                    let _ = CoInitializeEx(None, COINIT_MULTITHREADED);
                    i.set(true);
                }
            });
            let en: IMMDeviceEnumerator = CoCreateInstance(&MMDeviceEnumerator, None, CLSCTX_ALL).ok()?;
            let dev = en.GetDefaultAudioEndpoint(eRender, eConsole).ok()?;
            dev.Activate::<IAudioEndpointVolume>(CLSCTX_ALL, None).ok()
        }
    }

    /// Atur volume master (0-100) langsung lewat Core Audio.
    pub fn set_volume(percent: u8) {
        if let Some(v) = endpoint_volume() {
            unsafe {
                let _ = v.SetMasterVolumeLevelScalar(percent.min(100) as f32 / 100.0, std::ptr::null());
            }
        }
    }

    /// Baca volume master saat ini (0-100).
    pub fn get_volume() -> Option<u8> {
        let v = endpoint_volume()?;
        unsafe { v.GetMasterVolumeLevelScalar().ok().map(|f| (f * 100.0).round() as u8) }
    }

    /// Shortcut Windows + Shift + S untuk membuka Snipping Tool.
    pub fn screenshot_shortcut() {
        use windows::Win32::UI::Input::KeyboardAndMouse::{
            keybd_event, KEYEVENTF_KEYUP, VK_LWIN, VK_SHIFT, VIRTUAL_KEY,
        };
        const VK_S: VIRTUAL_KEY = VIRTUAL_KEY(0x53);
        unsafe {
            keybd_event(VK_LWIN.0 as u8, 0, Default::default(), 0);
            keybd_event(VK_SHIFT.0 as u8, 0, Default::default(), 0);
            keybd_event(VK_S.0 as u8, 0, Default::default(), 0);
            keybd_event(VK_S.0 as u8, 0, KEYEVENTF_KEYUP, 0);
            keybd_event(VK_SHIFT.0 as u8, 0, KEYEVENTF_KEYUP, 0);
            keybd_event(VK_LWIN.0 as u8, 0, KEYEVENTF_KEYUP, 0);
        }
    }
}

mod media {
    // Windows SMTC (GlobalSystemMediaTransportControls).
    // Poller berjalan di thread sendiri dan mengirim pesan ke UI lewat channel.

    use eframe::egui;
    use std::{
        sync::mpsc::Sender,
        thread,
        time::{Duration, SystemTime, UNIX_EPOCH},
    };
    use windows::{
        core::Interface,
        Media::Control::{
            GlobalSystemMediaTransportControlsSession as Session,
            GlobalSystemMediaTransportControlsSessionManager as Manager,
            GlobalSystemMediaTransportControlsSessionMediaProperties as Props,
            GlobalSystemMediaTransportControlsSessionPlaybackStatus as Status,
        },
        Storage::Streams::{DataReader, IInputStream, IRandomAccessStream},
        Win32::System::Com::{CoInitializeEx, COINIT_MULTITHREADED},
    };

    #[derive(Clone, Default)]
    pub struct MediaState {
        pub available: bool,
        pub app: String,
        pub title: String,
        pub artist: String,
        pub playing: bool,
        pub position_ms: i64,
        pub duration_ms: i64,
        pub track_key: String,
    }

    pub enum Msg {
        State(MediaState),
        Cover { key: String, image: egui::ColorImage },
        Lyrics { key: String, lines: Vec<(i64, String)> },
        Brightness(u8),
    }

    /// Ambil lirik bertimestamp dari LRCLIB agar baris bisa mengikuti posisi lagu.
    pub fn fetch_lyrics(tx: Sender<Msg>, key: String, title: String, artist: String, duration_ms: i64) {
        thread::spawn(move || {
            let client = match reqwest::blocking::Client::builder()
                .user_agent("DynamicIslandWin/1.0 (lyrics display)")
                .timeout(Duration::from_secs(8))
                .build() {
                Ok(c) => c,
                Err(_) => return,
            };
            let duration = (duration_ms.max(0) as f64 / 1000.0).round() as i64;
            let response = client.get("https://lrclib.net/api/get")
                .query(&[("track_name", title.as_str()), ("artist_name", artist.as_str())])
                .query(&[("duration", duration.to_string())])
                .send();
            let Ok(response) = response else {
                let _ = tx.send(Msg::Lyrics { key, lines: Vec::new() });
                return;
            };
            let Ok(json) = response.json::<serde_json::Value>() else {
                let _ = tx.send(Msg::Lyrics { key, lines: Vec::new() });
                return;
            };
            let synced = json.get("syncedLyrics").and_then(|v| v.as_str()).unwrap_or("");
            let mut lines = Vec::new();
            for raw in synced.lines() {
                let Some(end) = raw.find(']') else { continue };
                let Some(start) = raw.find('[') else { continue };
                if start != 0 || end <= start + 1 { continue; }
                let stamp = &raw[start + 1..end];
                let mut parts = stamp.split(':');
                let mins = parts.next().and_then(|v| v.parse::<i64>().ok());
                let secs = parts.next().and_then(|v| v.parse::<f64>().ok());
                if let (Some(mins), Some(secs)) = (mins, secs) {
                    let text = raw[end + 1..].trim().to_string();
                    if !text.is_empty() {
                        lines.push((mins * 60_000 + (secs * 1000.0) as i64, text));
                    }
                }
            }
            lines.sort_by_key(|line| line.0);
            let _ = tx.send(Msg::Lyrics { key, lines });
        });
    }

    fn com_init() {
        unsafe {
            let _ = CoInitializeEx(None, COINIT_MULTITHREADED);
        }
    }

    fn request_manager() -> Option<Manager> {
        Manager::RequestAsync().ok()?.get().ok()
    }

    /// Prioritaskan sesi Spotify; kalau tidak ada pakai sesi aktif sistem.
    fn pick_session(mgr: &Manager) -> Option<Session> {
        if let Ok(list) = mgr.GetSessions() {
            if let Ok(n) = list.Size() {
                for i in 0..n {
                    if let Ok(s) = list.GetAt(i) {
                        if let Ok(id) = s.SourceAppUserModelId() {
                            if id.to_string().to_lowercase().contains("spotify") {
                                return Some(s);
                            }
                        }
                    }
                }
            }
        }
        mgr.GetCurrentSession().ok()
    }

    fn read_thumbnail(props: &Props) -> Option<egui::ColorImage> {
        let thumb = props.Thumbnail().ok()?;
        let stream = thumb.OpenReadAsync().ok()?.get().ok()?;
        let ras: IRandomAccessStream = stream.cast().ok()?;
        let size = ras.Size().ok()? as u32;
        if size == 0 {
            return None;
        }
        let input: IInputStream = ras.cast().ok()?;
        let reader = DataReader::CreateDataReader(&input).ok()?;
        reader.LoadAsync(size).ok()?.get().ok()?;
        let mut buf = vec![0u8; size as usize];
        reader.ReadBytes(&mut buf).ok()?;

        let img = image::load_from_memory(&buf).ok()?.thumbnail(160, 160).to_rgba8();
        let dims = [img.width() as usize, img.height() as usize];
        Some(egui::ColorImage::from_rgba_unmultiplied(dims, img.as_raw()))
    }

    fn now_windows_ticks() -> i64 {
        let ns = SystemTime::now()
            .duration_since(UNIX_EPOCH)
            .map(|d| d.as_nanos())
            .unwrap_or(0);
        (ns / 100) as i64 + 116_444_736_000_000_000
    }

    fn build_state(session: &Session, props: &Props) -> windows::core::Result<MediaState> {
        let app = session.SourceAppUserModelId().map(|s| s.to_string()).unwrap_or_default();
        let title = props.Title()?.to_string();
        let artist = props.Artist()?.to_string();
        let playing = session.GetPlaybackInfo()?.PlaybackStatus()? == Status::Playing;

        let tl = session.GetTimelineProperties()?;
        let duration_ms = (tl.EndTime()?.Duration - tl.StartTime()?.Duration) / 10_000;
        let mut position_ms = tl.Position()?.Duration / 10_000;
        if playing {
            // Position = snapshot pada LastUpdatedTime -> ekstrapolasi sampai sekarang.
            let delta = (now_windows_ticks() - tl.LastUpdatedTime()?.UniversalTime) / 10_000;
            position_ms += delta.max(0);
        }
        if duration_ms > 0 {
            position_ms = position_ms.clamp(0, duration_ms);
        }
        Ok(MediaState {
            available: true,
            track_key: format!("{app}|{title}|{artist}"),
            app,
            title,
            artist,
            playing,
            position_ms,
            duration_ms: duration_ms.max(0),
        })
    }

    pub fn spawn_poller(tx: Sender<Msg>, ctx: egui::Context) {
        thread::spawn(move || {
            com_init();
            let mut mgr: Option<Manager> = None;
            let mut last_key = String::new();
            let mut cover_done = false;
            let mut cover_tries = 0u8;

            loop {
                thread::sleep(Duration::from_millis(500));
                if mgr.is_none() {
                    mgr = request_manager();
                }
                let session = mgr.as_ref().and_then(pick_session);
                let snapshot = session.as_ref().and_then(|s| {
                    let props = s.TryGetMediaPropertiesAsync().ok()?.get().ok()?;
                    let st = build_state(s, &props).ok()?;
                    Some((st, props))
                });

                match snapshot {
                    Some((state, props)) => {
                        if state.track_key != last_key {
                            last_key = state.track_key.clone();
                            cover_done = false;
                            cover_tries = 0;
                        }
                        let key = state.track_key.clone();
                        if tx.send(Msg::State(state)).is_err() {
                            return;
                        }
                        // Spotify kadang telat menyediakan cover -> coba ulang beberapa kali.
                        if !cover_done && cover_tries < 8 {
                            cover_tries += 1;
                            if let Some(image) = read_thumbnail(&props) {
                                cover_done = true;
                                let _ = tx.send(Msg::Cover { key, image });
                            }
                        }
                    }
                    None => {
                        last_key.clear();
                        if tx.send(Msg::State(MediaState::default())).is_err() {
                            return;
                        }
                    }
                }
                ctx.request_repaint();
            }
        });
    }

    /// action: "toggle" | "next" | "prev" | "seek" (value = ms)
    pub fn control(action: &'static str, value: Option<i64>) {
        thread::spawn(move || {
            com_init();
            let Some(mgr) = request_manager() else { return };
            let Some(s) = pick_session(&mgr) else { return };
            match action {
                "toggle" => {
                    let _ = s.TryTogglePlayPauseAsync().and_then(|o| o.get());
                }
                "next" => {
                    let _ = s.TrySkipNextAsync().and_then(|o| o.get());
                }
                "prev" => {
                    let _ = s.TrySkipPreviousAsync().and_then(|o| o.get());
                }
                "seek" => {
                    if let Some(ms) = value {
                        let _ = s.TryChangePlaybackPositionAsync(ms * 10_000).and_then(|o| o.get());
                    }
                }
                _ => {}
            }
        });
    }
}

mod sys {
    // Helper Win32: baterai, jam lokal, posisi kursor, lebar layar.

    use windows::Win32::{
        Foundation::POINT,
        System::{
            Power::{GetSystemPowerStatus, SYSTEM_POWER_STATUS},
            SystemInformation::GetLocalTime,
        },
        UI::WindowsAndMessaging::{GetCursorPos, GetSystemMetrics, SM_CXSCREEN},
    };

    #[derive(Clone, Copy)]
    pub struct Battery {
        pub percent: u8,
        pub charging: bool,
        pub has_battery: bool,
    }

    pub fn battery() -> Battery {
        let mut s = SYSTEM_POWER_STATUS::default();
        unsafe {
            let _ = GetSystemPowerStatus(&mut s);
        }
        Battery {
            percent: if s.BatteryLifePercent <= 100 { s.BatteryLifePercent } else { 0 },
            charging: s.ACLineStatus == 1 || (s.BatteryFlag & 8) != 0,
            has_battery: s.BatteryFlag != 128 && s.BatteryLifePercent <= 100,
        }
    }

    pub fn local_hm() -> (u16, u16) {
        let t = unsafe { GetLocalTime() };
        (t.wHour, t.wMinute)
    }

    /// Posisi kursor dalam piksel fisik layar.
    pub fn cursor_pos() -> Option<(f32, f32)> {
        let mut p = POINT::default();
        unsafe { GetCursorPos(&mut p).ok()? };
        Some((p.x as f32, p.y as f32))
    }

    /// Lebar monitor utama dalam piksel fisik.
    pub fn screen_width_px() -> f32 {
        unsafe { GetSystemMetrics(SM_CXSCREEN) as f32 }
    }
}

use anim::Tween;
use draw::*;
use eframe::egui::{
    self, pos2, vec2, Align2, Color32, CursorIcon, FontData, FontDefinitions, FontFamily, Pos2,
    Rect, Sense, ViewportCommand,
};
use media::{MediaState, Msg};
use std::{
    path::PathBuf,
    sync::mpsc::{self, Receiver},
    time::{Duration, Instant},
};

const WIN_W: f32 = 520.0;
const WIN_H: f32 = 470.0;
// Auto-collapse tetap aktif: saat kursor menjauh, panel kembali ke mode Collapsed setelah 350 ms.

#[derive(Clone, Copy, PartialEq)]
enum Mode { Notch, Collapsed, Expanded }

impl Mode {
    /// [lebar, tinggi, offset-atas] pill
    fn geom(self) -> [f32; 3] {
        match self {
            Mode::Notch => [96.0, 8.0, 0.0],
            Mode::Collapsed => [200.0, 38.0, 8.0],
            Mode::Expanded => [470.0, 420.0, 8.0],
        }
    }
    fn hit_w(self) -> f32 {
        match self { Mode::Notch => 150.0, Mode::Collapsed => 200.0, Mode::Expanded => 470.0 }
    }
    fn anim_secs(self) -> f32 {
        match self { Mode::Expanded => 0.45, Mode::Collapsed => 0.32, Mode::Notch => 0.40 }
    }
}

struct Theme { bg: Color32, fg: Color32, muted: Color32, track: Color32, hover: Color32, accent: Color32 }

const DARK: Theme = Theme {
    bg: Color32::from_rgb(24, 26, 31), fg: Color32::WHITE,
    muted: Color32::from_rgb(158, 158, 158), track: Color32::from_rgb(64, 64, 64),
    hover: Color32::from_rgb(34, 34, 34), accent: Color32::from_rgb(30, 215, 96),
};
const LIGHT: Theme = Theme {
    bg: Color32::from_rgb(244, 244, 246), fg: Color32::from_rgb(17, 17, 17),
    muted: Color32::from_rgb(110, 110, 115), track: Color32::from_rgb(210, 210, 214),
    hover: Color32::from_rgb(228, 228, 232), accent: Color32::from_rgb(20, 168, 75),
};

#[derive(Clone, Copy, PartialEq)]
enum Action {
    PillClick, Prev, Toggle, Next, Seek(i64), Theme, Settings, Screenshot,
    BrightnessValue(u8), VolumeValue(u8), Wifi, Bluetooth, Hotspot, LyricsToggle, Quit,
}

struct App {
    rx: Receiver<Msg>,
    tx: mpsc::Sender<Msg>,
    media: MediaState,
    media_at: Instant,
    cover: Option<(String, egui::TextureHandle)>,
    lyrics: Vec<(i64, String)>,
    lyrics_key: String,
    lyrics_enabled: bool,
    battery: sys::Battery,
    battery_at: Instant,
    dark: bool,
    brightness_percent: u8,
    volume_percent: u8,
    vol_sync_at: Instant,

    mode: Mode,
    tween: Tween,
    pinned: bool,
    hovering: bool,
    passthrough: bool,
    pending: Option<(Instant, Mode)>,
    idle_at: Option<Instant>,
    peek_until: Option<Instant>,
    placed_frames: u8,
}

fn theme_path() -> Option<PathBuf> {
    std::env::var_os("APPDATA").map(|a| PathBuf::from(a).join("dynamic-island-win").join("theme.txt"))
}
fn load_dark() -> bool {
    theme_path().and_then(|p| std::fs::read_to_string(p).ok()).map(|s| s.trim() != "light").unwrap_or(true)
}
fn save_dark(dark: bool) {
    if let Some(p) = theme_path() {
        if let Some(dir) = p.parent() { let _ = std::fs::create_dir_all(dir); }
        let _ = std::fs::write(p, if dark { "dark" } else { "light" });
    }
}

fn setup_fonts(ctx: &egui::Context) {
    let mut fonts = FontDefinitions::default();
    let mut prop = fonts.families[&FontFamily::Proportional].clone();
    if let Ok(d) = std::fs::read(r"C:\Windows\Fonts\segoeui.ttf") {
        fonts.font_data.insert("segoe".into(), FontData::from_owned(d));
        prop.insert(0, "segoe".into());
    }
    let mut bold = prop.clone();
    if let Ok(d) = std::fs::read(r"C:\Windows\Fonts\segoeuib.ttf") {
        fonts.font_data.insert("segoe-bold".into(), FontData::from_owned(d));
        bold.insert(0, "segoe-bold".into());
    }
    fonts.families.insert(FontFamily::Proportional, prop);
    fonts.families.insert(FontFamily::Name("bold".into()), bold);
    ctx.set_fonts(fonts);
}

#[allow(dead_code)]
fn fmt_time(ms: i64) -> String {
    let s = (ms.max(0) / 1000) as i64;
    format!("{}:{:02}", s / 60, s % 60)
}

impl App {
    fn new(cc: &eframe::CreationContext<'_>) -> Self {
        setup_fonts(&cc.egui_ctx);
        let (tx, rx) = mpsc::channel();
        media::spawn_poller(tx.clone(), cc.egui_ctx.clone());
        let now = Instant::now();

        // Baca kecerahan awal di thread terpisah (PowerShell lambat).
        {
            let tx2 = tx.clone();
            let ctx2 = cc.egui_ctx.clone();
            std::thread::spawn(move || {
                if let Some(v) = draw::get_brightness() {
                    let _ = tx2.send(Msg::Brightness(v));
                    ctx2.request_repaint();
                }
            });
        }

        Self {
            rx,
            tx,
            media: MediaState::default(),
            media_at: now,
            cover: None,
            lyrics: Vec::new(),
            lyrics_key: String::new(),
            lyrics_enabled: true,
            battery: sys::battery(),
            battery_at: now,
            dark: load_dark(),
            brightness_percent: 50,
            volume_percent: draw::get_volume().unwrap_or(50),
            vol_sync_at: now,
            mode: Mode::Collapsed,
            tween: Tween::new(Mode::Collapsed.geom()),
            pinned: false,
            hovering: false,
            passthrough: true,
            pending: None,
            idle_at: None,
            peek_until: None,
            placed_frames: 0,
        }
    }

    fn theme(&self) -> &'static Theme { if self.dark { &DARK } else { &LIGHT } }

    // ---------------- state machine ----------------
    fn set_mode(&mut self, m: Mode, now: Instant) {
        if m == self.mode { return; }
        self.tween.retarget(now, m.geom(), m.anim_secs());
        self.mode = m;
    }

    fn on_hover(&mut self, h: bool, now: Instant) {
        self.hovering = h;
        if h {
            self.idle_at = None;
            let d = if self.mode == Mode::Notch { 0 } else { 90 };
            self.pending = Some((now + Duration::from_millis(d), Mode::Expanded));
        } else if !self.pinned {
            // Saat kursor menjauh, kembali ke tampilan jam dan baterai (Collapsed).
            self.pending = Some((now + Duration::from_millis(350), Mode::Collapsed));
        }
    }

    #[allow(dead_code)]
    fn peek(&mut self, _now: Instant) {
        // Perubahan lagu tidak mengubah tampilan otomatis; jam dan baterai tetap terlihat.
    }

    fn tick(&mut self, now: Instant) {
        if let Some((t, m)) = self.pending {
            if now >= t {
                self.pending = None;
                self.set_mode(m, now);
            }
        }
    }

    fn position_ms(&self, now: Instant) -> i64 {
        let mut p = self.media.position_ms;
        if self.media.playing { p += now.duration_since(self.media_at).as_millis() as i64; }
        if self.media.duration_ms > 0 { p = p.min(self.media.duration_ms); }
        p
    }

    // ---------------- painting ----------------
    fn paint(&self, ui: &egui::Ui, now: Instant) -> Vec<Action> {
        let mut acts = Vec::new();
        let th = self.theme();
        let [w, h, top] = self.tween.value(now);
        let painter = ui.painter().clone();
        let rect = Rect::from_min_size(pos2((WIN_W - w) / 2.0, top), vec2(w.max(1.0), h.max(1.0)));
        let radius = (h / 2.0).min(28.0);

        let p_open = ((h - 38.0) / (220.0 - 38.0)).clamp(0.0, 1.0);
        let notch_f = ((h - 14.0) / 22.0).clamp(0.0, 1.0);
        // Tanpa gradasi/fade warna saat auto-collapse: elemen berganti secara tegas,
        // sementara animasi ukuran dan posisi pill tetap berjalan seperti sebelumnya.
        let a_col = if p_open < 0.5 { 1.0 } else { 0.0 };
        let a_exp = if p_open >= 0.5 { 1.0 } else { 0.0 };

        // shadow berlapis
        let sh_alpha = (9.0 * notch_f) as u8;
        for i in (1..=8).rev() {
            let g = i as f32 * 2.5;
            painter.rect_filled(rect.expand(g).translate(vec2(0.0, 5.0)), radius + g, Color32::from_black_alpha(sh_alpha));
        }
        painter.rect_filled(rect, radius, th.bg);

        let pill = ui.interact(rect, ui.id().with("pill"), Sense::click());
        let pill_click = pill.clicked();

        let p = painter.with_clip_rect(rect);
        let cx = rect.center().x;
        let y0 = rect.min.y;

        // ------------ collapsed ------------
        if a_col > 0.01 {
            let cy = y0 + 19.0;
            let (hh, mm) = sys::local_hm();
            text(&p, pos2(cx - 82.0, cy), Align2::LEFT_CENTER, &format!("{hh:02}:{mm:02}"), 14.0, true, fade(th.fg, a_col));
            let r = cx + 82.0;
            let b = self.battery;
            let col = if b.charging { Color32::from_rgb(48, 209, 88) }
                      else if b.percent <= 20 && b.has_battery { Color32::from_rgb(255, 69, 58) }
                      else { th.fg };
            let level = if b.has_battery { b.percent as f32 / 100.0 } else { 1.0 };
            icon_battery(&p, pos2(r - 25.0, cy - 5.5), level, fade(th.muted, a_col), fade(col, a_col));
            let label = if b.has_battery { format!("{}%", b.percent) } else { "AC".to_string() };
            text(&p, pos2(r - 31.0, cy), Align2::RIGHT_CENTER, &label, 14.0, true, fade(th.fg, a_col));
        }

        // ------------ expanded: Control Center style panel ------------
        if a_exp > 0.01 {
            let left = rect.min.x + 18.0;
            let right = rect.max.x - 18.0;
            let top = y0 + 18.0;
            let m = &self.media;
            let fg = fade(th.fg, a_exp);
            let muted = fade(th.muted, a_exp);
            let _accent = fade(th.accent, a_exp);

            // Header: title, date/time, theme and close.
            text(&p, pos2(left, top + 2.0), Align2::LEFT_CENTER, "CONTROL CENTER", 10.0, true, muted);
            let (hh, mm) = sys::local_hm();
            text(&p, pos2(left, top + 22.0), Align2::LEFT_CENTER, &format!("{hh:02}:{mm:02}  •  Quick settings"), 16.0, true, fg);
            let head_btn = |id: &str, c: Pos2, label: &str| {
                let r = Rect::from_center_size(c, vec2(30.0, 30.0));
                let resp = ui.interact(r, ui.id().with(id), Sense::click());
                p.circle_filled(c, 15.0, if resp.hovered() { th.hover } else { th.track });
                text(&p, c, Align2::CENTER_CENTER, label, 15.0, true, fg);
                if resp.hovered() { ui.ctx().set_cursor_icon(CursorIcon::PointingHand); }
                resp.clicked()
            };
            if head_btn("theme", pos2(right - 36.0, top + 8.0), if self.dark { "☼" } else { "☾" }) { acts.push(Action::Theme); }
            if head_btn("quit", pos2(right, top + 8.0), "×") { acts.push(Action::Quit); }

            // Hanya dua tombol bulat di atas: Settings dan Screenshot.
            let shortcuts = [
                ("⚙", "Settings", Action::Settings, Color32::from_rgb(24, 170, 220)),
                ("✂", "Screenshot", Action::Screenshot, Color32::from_rgb(181, 91, 237)),
            ];
            for (i, (glyph, label, action, color)) in shortcuts.iter().enumerate() {
                let c = pos2(left + 18.0 + i as f32 * 56.0, top + 66.0);
                let r = Rect::from_center_size(c, vec2(38.0, 38.0));
                let resp = ui.interact(r, ui.id().with(format!("cc_shortcut_{i}")), Sense::click());
                p.circle_filled(c, 19.0, *color);
                text(&p, c, Align2::CENTER_CENTER, glyph, 17.0, true, Color32::WHITE);
                if resp.hovered() { ui.ctx().set_cursor_icon(CursorIcon::PointingHand); }
                if resp.clicked() { acts.push(*action); }
                text(&p, pos2(c.x, c.y + 27.0), Align2::CENTER_CENTER, label, 8.0, false, muted);
            }

            // Main cards: network state and quick action buttons.
            let card_y = top + 104.0;
            let card_h = 112.0;
            let net_w = 268.0;
            let net = Rect::from_min_size(pos2(left, card_y), vec2(net_w, card_h));
            p.rect_filled(net, 20.0, fade(th.track, 0.62));
            let mut tile = |id: &str, c: Pos2, glyph: &str, title: &str, subtitle: &str, color: Color32, action: Action| {
                let r = Rect::from_center_size(c, vec2(124.0, 42.0));
                let resp = ui.interact(r, ui.id().with(id), Sense::click());
                if resp.hovered() { p.rect_filled(r, 12.0, th.hover); ui.ctx().set_cursor_icon(CursorIcon::PointingHand); }
                p.circle_filled(pos2(r.min.x + 17.0, c.y), 12.0, color);
                text(&p, pos2(r.min.x + 17.0, c.y), Align2::CENTER_CENTER, glyph, 12.0, true, Color32::WHITE);
                text(&p, pos2(r.min.x + 35.0, c.y - 6.0), Align2::LEFT_CENTER, title, 11.0, true, fg);
                text(&p, pos2(r.min.x + 35.0, c.y + 8.0), Align2::LEFT_CENTER, subtitle, 9.0, false, muted);
                if resp.clicked() { acts.push(action); }
            };
            tile("wifi_tile", pos2(left + 73.0, card_y + 31.0), "◉", "Wi-Fi", "Buka pengaturan", Color32::from_rgb(30, 164, 220), Action::Wifi);
            tile("bluetooth_tile", pos2(left + 73.0, card_y + 80.0), "ᛒ", "Bluetooth", "Pengaturan perangkat", Color32::from_rgb(35, 155, 224), Action::Bluetooth);
            tile("hotspot_tile", pos2(left + 205.0, card_y + 31.0), "●", "Hotspot", "Pengaturan hotspot", Color32::from_rgb(37, 195, 103), Action::Hotspot);
            tile("lyrics_tile", pos2(left + 205.0, card_y + 80.0), "♪", "Lyrics", if self.lyrics_enabled { "Aktif" } else { "Nonaktif" }, Color32::from_rgb(35, 190, 132), Action::LyricsToggle);

            // Four compact system-style action buttons on the right.
            let quick_x = left + net_w + 35.0;
            let quicks = [("▣", Action::Settings), ("☾", Action::Theme), ("↻", Action::Screenshot), ("⏻", Action::Quit)];
            for (i, (glyph, action)) in quicks.iter().enumerate() {
                let col = i % 2;
                let row = i / 2;
                let c = pos2(quick_x + col as f32 * 47.0, card_y + 31.0 + row as f32 * 48.0);
                let r = Rect::from_center_size(c, vec2(39.0, 39.0));
                let resp = ui.interact(r, ui.id().with(format!("cc_quick_{i}")), Sense::click());
                p.rect_filled(r, 12.0, if resp.hovered() { th.hover } else { fade(th.track, 0.9) });
                text(&p, c, Align2::CENTER_CENTER, glyph, 17.0, true, if i == 3 { Color32::from_rgb(255, 95, 95) } else { fg });
                if resp.hovered() { ui.ctx().set_cursor_icon(CursorIcon::PointingHand); }
                if resp.clicked() { acts.push(*action); }
            }

            // Slider kecerahan & volume: bisa diklik dan digeser kiri-kanan.
            // Closure hanya mengembalikan nilai baru; state diubah di update().
            let slider_row = |id: &str, y: f32, glyph: &str, percent: f32| -> Option<u8> {
                let r = Rect::from_min_size(pos2(left, y), vec2(right - left - 8.0, 38.0));
                let resp = ui.interact(r, ui.id().with(id), Sense::click_and_drag());
                let frac = percent.clamp(0.0, 1.0);

                p.rect_filled(r, 19.0, fade(th.track, 0.95));
                // Isi slider: gambar bentuk penuh lalu di-clip sesuai persentase.
                let fill_clip = Rect::from_min_max(r.min, pos2(r.min.x + r.width() * frac, r.max.y));
                p.with_clip_rect(fill_clip.intersect(rect))
                    .rect_filled(r, 19.0, Color32::from_rgb(228, 231, 231));

                text(&p, pos2(r.min.x + 22.0, r.center().y), Align2::CENTER_CENTER, glyph, 15.0, true, Color32::from_rgb(45, 48, 51));
                text(&p, r.center(), Align2::CENTER_CENTER, &format!("{}%", (frac * 100.0).round() as u32), 11.0, true, Color32::from_rgb(55, 57, 60));

                if resp.hovered() || resp.dragged() {
                    ui.ctx().set_cursor_icon(CursorIcon::PointingHand);
                }
                if resp.clicked() || resp.dragged() {
                    if let Some(pointer) = resp.interact_pointer_pos() {
                        let f = ((pointer.x - r.min.x) / r.width()).clamp(0.0, 1.0);
                        return Some((f * 100.0).round() as u8);
                    }
                }
                None
            };
            if let Some(v) = slider_row("brightness_slider", top + 235.0, "☼", self.brightness_percent as f32 / 100.0) {
                acts.push(Action::BrightnessValue(v));
            }
            if let Some(v) = slider_row("volume_slider", top + 278.0, "◖", self.volume_percent as f32 / 100.0) {
                acts.push(Action::VolumeValue(v));
            }

            // Media / lyrics status footer retains existing player and synced lyric features.
            let footer = Rect::from_min_size(pos2(left, top + 326.0), vec2(right - left - 8.0, 34.0));
            p.rect_filled(footer, 12.0, fade(th.track, 0.72));
            let (title, artist) = if m.available && !m.title.is_empty() {
                (m.title.as_str(), if m.artist.is_empty() { m.app.as_str() } else { m.artist.as_str() })
            } else { ("Tidak ada media", "Putar lagu di Spotify atau pemutar musik") };
            text_ellipsis(&p, pos2(footer.min.x + 12.0, footer.center().y - 6.0), title, 11.0, true, fg, 200.0);
            text_ellipsis(&p, pos2(footer.min.x + 12.0, footer.center().y + 8.0), artist, 9.0, false, muted, 200.0);
            let media_x = footer.max.x - 75.0;
            let media_y = footer.center().y;
            let mut mini = |id: &str, c: Pos2, glyph: &str, action: Action| {
                let r = Rect::from_center_size(c, vec2(28.0, 28.0));
                let resp = ui.interact(r, ui.id().with(id), Sense::click());
                if resp.hovered() { ui.ctx().set_cursor_icon(CursorIcon::PointingHand); p.circle_filled(c, 14.0, th.hover); }
                text(&p, c, Align2::CENTER_CENTER, glyph, 14.0, true, fg);
                if resp.clicked() { acts.push(action); }
            };
            mini("media_prev", pos2(media_x, media_y), "|◀", Action::Prev);
            mini("media_play", pos2(media_x + 29.0, media_y), if m.playing { "Ⅱ" } else { "▶" }, Action::Toggle);
            mini("media_next", pos2(media_x + 58.0, media_y), "▶|", Action::Next);

            // Small hint for lirik mode.
            text(&p, pos2(left, top + 374.0), Align2::LEFT_CENTER,
                if self.lyrics_enabled { "Lirik tersinkron: ON" } else { "Lirik tersinkron: OFF" }, 9.0, false, muted);
            let b = self.battery;
            let battery_label = if b.has_battery { format!("Baterai {}%", b.percent) } else { "Daya AC".to_string() };
            text(&p, pos2(right - 8.0, top + 374.0), Align2::RIGHT_CENTER, &battery_label, 9.0, false, muted);
        }

        // klik di latar pill (bukan tombol) = pin/unpin
        if pill_click && acts.is_empty() { acts.push(Action::PillClick); }
        acts
    }
}

impl eframe::App for App {
    fn clear_color(&self, _v: &egui::Visuals) -> [f32; 4] { egui::Rgba::TRANSPARENT.to_array() }

    fn update(&mut self, ctx: &egui::Context, _f: &mut eframe::Frame) {
        let now = Instant::now();
        let ppp = ctx.pixels_per_point();

        // 1) taruh window di tengah atas layar (beberapa frame pertama, tanpa menggambar)
        if self.placed_frames < 3 {
            let x = (sys::screen_width_px() / ppp - WIN_W) / 2.0;
            ctx.send_viewport_cmd(ViewportCommand::OuterPosition(pos2(x.max(0.0), 0.0)));
            self.placed_frames += 1;
            ctx.request_repaint();
            return;
        }

        // 2) pesan dari thread SMTC / worker lain
        while let Ok(msg) = self.rx.try_recv() {
            match msg {
                Msg::State(s) => {
                    let changed = s.available && s.track_key != self.lyrics_key;
                    self.media = s;
                    self.media_at = now;
                    if changed && self.media.available && !self.media.title.is_empty() {
                        self.lyrics_key = self.media.track_key.clone();
                        self.lyrics.clear();
                        let artist = if self.media.artist.is_empty() { self.media.app.clone() } else { self.media.artist.clone() };
                        media::fetch_lyrics(self.tx.clone(), self.media.track_key.clone(), self.media.title.clone(), artist, self.media.duration_ms);
                    }
                }
                Msg::Cover { key, image } => {
                    let tex = ctx.load_texture("cover", image, egui::TextureOptions::LINEAR);
                    self.cover = Some((key, tex));
                }
                Msg::Lyrics { key, lines } => {
                    if key == self.media.track_key { self.lyrics = lines; }
                }
                Msg::Brightness(v) => self.brightness_percent = v,
            }
        }
        if now.duration_since(self.battery_at) > Duration::from_secs(20) {
            self.battery = sys::battery();
            self.battery_at = now;
        }

        // Sinkronkan slider volume dengan volume sistem (jeda saat slider sedang digeser).
        if self.mode == Mode::Expanded && now.duration_since(self.vol_sync_at) > Duration::from_secs(1) {
            if let Some(v) = draw::get_volume() { self.volume_percent = v; }
            self.vol_sync_at = now;
        }

        // 3) hover manual (window click-through, jadi event mouse tidak datang dari OS)
        let origin = ctx
            .input(|i| i.viewport().outer_rect.or(i.viewport().inner_rect))
            .map(|r| r.min)
            .unwrap_or(pos2(0.0, 0.0));
        let [_, th_h, th_top] = self.mode.geom();
        let hw = self.mode.hit_w();
        let hit = Rect::from_min_max(pos2((WIN_W - hw) / 2.0, 0.0), pos2((WIN_W + hw) / 2.0, th_top + th_h + 2.0));
        let inside = sys::cursor_pos()
            .map(|(x, y)| hit.contains(pos2(x / ppp - origin.x, y / ppp - origin.y)))
            .unwrap_or(false);
        if inside != self.hovering { self.on_hover(inside, now); }
        if self.passthrough == inside {
            self.passthrough = !inside;
            ctx.send_viewport_cmd(ViewportCommand::MousePassthrough(self.passthrough));
        }

        self.tick(now);

        // 4) gambar + kumpulkan aksi
        let mut acts = Vec::new();
        egui::CentralPanel::default().frame(egui::Frame::none()).show(ctx, |ui| {
            acts = self.paint(ui, now);
        });

        for a in acts {
            match a {
                Action::PillClick => {
                    self.pinned = !self.pinned;
                    if self.pinned { self.pending = None; self.set_mode(Mode::Expanded, now); }
                    else { self.pending = None; }
                }
                Action::Prev => media::control("prev", None),
                Action::Next => media::control("next", None),
                Action::Toggle => {
                    media::control("toggle", None);
                    self.media.position_ms = self.position_ms(now);
                    self.media_at = now;
                    self.media.playing = !self.media.playing;
                }
                Action::Seek(ms) => {
                    self.media.position_ms = ms;
                    self.media_at = now;
                    media::control("seek", Some(ms));
                }
                Action::Theme => { self.dark = !self.dark; save_dark(self.dark); }
                Action::Settings => draw::open_settings("ms-settings:"),
                Action::Screenshot => draw::screenshot_shortcut(),
                Action::BrightnessValue(v) => {
                    if v != self.brightness_percent {
                        self.brightness_percent = v;
                        draw::set_brightness(v);
                    }
                }
                Action::VolumeValue(v) => {
                    self.vol_sync_at = now;
                    if v != self.volume_percent {
                        self.volume_percent = v;
                        draw::set_volume(v);
                    }
                }
                Action::Wifi => draw::open_settings("ms-settings:network-wifi"),
                Action::Bluetooth => draw::open_settings("ms-settings:bluetooth"),
                Action::Hotspot => draw::open_settings("ms-settings:network-mobilehotspot"),
                Action::LyricsToggle => { self.lyrics_enabled = !self.lyrics_enabled; }
                Action::Quit => ctx.send_viewport_cmd(ViewportCommand::Close),
            }
        }

        // 5) jadwalkan frame berikutnya
        let visible_viz = self.mode == Mode::Expanded && self.media.playing;
        if self.tween.running(now) || visible_viz {
            ctx.request_repaint_after(Duration::from_millis(16));
        } else {
            ctx.request_repaint_after(Duration::from_millis(40));
        }
    }
}

fn log_error(msg: &str) {
    use std::io::Write;
    if let Some(a) = std::env::var_os("APPDATA") {
        let dir = PathBuf::from(a).join("dynamic-island-win");
        let _ = std::fs::create_dir_all(&dir);
        if let Ok(mut f) = std::fs::OpenOptions::new().create(true).append(true).open(dir.join("error.log")) {
            let _ = writeln!(f, "{msg}");
        }
    }
}

fn main() -> eframe::Result<()> {
    std::panic::set_hook(Box::new(|info| log_error(&format!("PANIC: {info}"))));
    let options = eframe::NativeOptions {
        viewport: egui::ViewportBuilder::default()
            .with_title("Dynamic Island")
            .with_inner_size([WIN_W, WIN_H])
            .with_decorations(false)
            .with_transparent(true)
            .with_always_on_top()
            .with_resizable(false)
            .with_taskbar(false)
            .with_mouse_passthrough(true),
        ..Default::default()
    };
    let result = eframe::run_native("dynamic-island-win", options, Box::new(|cc| Ok(Box::new(App::new(cc)))));
    if let Err(e) = &result {
        log_error(&format!("ERROR: {e}"));
    }
    result
}
