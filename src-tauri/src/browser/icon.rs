// The Claudia browser wears the orange Chrome logo, so it is never mistaken for the user's own Chrome.
//
// How, and why this way: the icon is set on the browser's WINDOWS (`WM_SETICON`), only for the process
// the app launched itself. The `chrome.exe` is never edited, so its signature, its integrity and its
// updater (there is none) are untouched; nothing persists in the binary, and not calling this code
// brings the original icon back. A window the app does not own is never touched.

/// Embedded PNGs: 256 px for the taskbar / Alt+Tab, 32 px for the small slot.
const BIG: &[u8] = include_bytes!("../../assets/browser-icon/browser-icon-256.png");
const SMALL: &[u8] = include_bytes!("../../assets/browser-icon/browser-icon-32.png");

/// (width, height) of a PNG, read from its header. None if the bytes are not a PNG.
pub(crate) fn png_dimensions(bytes: &[u8]) -> Option<(u32, u32)> {
    const SIG: [u8; 8] = [0x89, b'P', b'N', b'G', 0x0d, 0x0a, 0x1a, 0x0a];
    if bytes.len() < 24 || bytes[..8] != SIG || &bytes[12..16] != b"IHDR" {
        return None;
    }
    let w = u32::from_be_bytes(bytes[16..20].try_into().ok()?);
    let h = u32::from_be_bytes(bytes[20..24].try_into().ok()?);
    Some((w, h))
}

#[cfg(windows)]
mod win {
    use super::{png_dimensions, BIG, SMALL};
    use std::sync::OnceLock;
    use windows_sys::Win32::Foundation::{BOOL, HWND, LPARAM};
    use windows_sys::Win32::UI::WindowsAndMessaging::{
        CreateIconFromResourceEx, EnumWindows, GetClassNameW, GetWindowThreadProcessId, IsWindowVisible, SendMessageTimeoutW,
        ICON_BIG, ICON_SMALL, SMTO_ABORTIFHUNG, WM_GETICON, WM_SETICON,
    };

    /// The handles are created once and live as long as the process (a few KB): Windows keeps a window's
    /// icon only as a handle, so it must outlive the windows that use it.
    static ICONS: OnceLock<Option<(isize, isize)>> = OnceLock::new();

    fn make(bytes: &[u8]) -> Option<isize> {
        let (w, h) = png_dimensions(bytes)?;
        // dwVer 0x00030000: the bytes are a PNG-compressed icon image (supported since Windows Vista).
        let handle = unsafe { CreateIconFromResourceEx(bytes.as_ptr(), bytes.len() as u32, 1, 0x0003_0000, w as i32, h as i32, 0) };
        if handle.is_null() { None } else { Some(handle as isize) }
    }

    pub fn icons() -> Option<(isize, isize)> {
        *ICONS.get_or_init(|| Some((make(BIG)?, make(SMALL)?)))
    }

    struct Ctx {
        pid: u32,
        big: isize,
        small: isize,
        windows: usize,
        changed: usize,
    }

    unsafe extern "system" fn each(hwnd: HWND, lparam: LPARAM) -> BOOL {
        let ctx = &mut *(lparam as *mut Ctx);
        let mut pid = 0u32;
        GetWindowThreadProcessId(hwnd, &mut pid);
        if pid != ctx.pid || IsWindowVisible(hwnd) == 0 {
            return 1;
        }
        // Only Chrome's own top-level window class: never another window that happens to share the pid.
        let mut class = [0u16; 64];
        let n = GetClassNameW(hwnd, class.as_mut_ptr(), class.len() as i32) as usize;
        if !String::from_utf16_lossy(&class[..n]).starts_with("Chrome_WidgetWin") {
            return 1;
        }
        ctx.windows += 1;
        // Either slot: Chrome can reset just one of them.
        if send(hwnd, WM_GETICON, ICON_BIG as usize, 0) != Some(ctx.big) || send(hwnd, WM_GETICON, ICON_SMALL as usize, 0) != Some(ctx.small) {
            send(hwnd, WM_SETICON, ICON_BIG as usize, ctx.big);
            send(hwnd, WM_SETICON, ICON_SMALL as usize, ctx.small);
            ctx.changed += 1;
        }
        1
    }

    /// A cross-process `SendMessage` that gives up after 200 ms and never waits on a hung window (the
    /// watcher calls this every few seconds: a frozen Chrome UI must not freeze the watcher with it).
    /// None when the window did not answer.
    unsafe fn send(hwnd: HWND, msg: u32, wparam: usize, lparam: isize) -> Option<isize> {
        let mut out: usize = 0;
        let ok = SendMessageTimeoutW(hwnd, msg, wparam, lparam, SMTO_ABORTIFHUNG, 200, &mut out);
        if ok == 0 { None } else { Some(out as isize) }
    }

    /// Returns (windows of the process, windows whose icon was changed by this call).
    pub fn apply(pid: u32) -> (usize, usize) {
        let Some((big, small)) = icons() else { return (0, 0) };
        let mut ctx = Ctx { pid, big, small, windows: 0, changed: 0 };
        unsafe { EnumWindows(Some(each), &mut ctx as *mut Ctx as LPARAM) };
        (ctx.windows, ctx.changed)
    }

    /// The icon currently shown in the big slot of each Chrome window of `pid` (test support).
    pub fn current_icons(pid: u32, small: bool) -> Vec<isize> {
        struct Probe {
            pid: u32,
            small: bool,
            found: Vec<isize>,
        }
        unsafe extern "system" fn each_probe(hwnd: HWND, lparam: LPARAM) -> BOOL {
            let p = &mut *(lparam as *mut Probe);
            let mut pid = 0u32;
            GetWindowThreadProcessId(hwnd, &mut pid);
            if pid == p.pid && IsWindowVisible(hwnd) != 0 {
                let mut class = [0u16; 64];
                let n = GetClassNameW(hwnd, class.as_mut_ptr(), class.len() as i32) as usize;
                if String::from_utf16_lossy(&class[..n]).starts_with("Chrome_WidgetWin") {
                    let slot = if p.small { ICON_SMALL } else { ICON_BIG };
                    p.found.push(send(hwnd, WM_GETICON, slot as usize, 0).unwrap_or(0));
                }
            }
            1
        }
        let mut p = Probe { pid, small, found: vec![] };
        unsafe { EnumWindows(Some(each_probe), &mut p as *mut Probe as LPARAM) };
        p.found
    }
}

/// Sets the orange icon on the visible Chrome windows of `pid`. Returns how many windows were found.
/// A no-op (0) where there is nothing to do or on platforms without Win32.
pub fn apply_to_process(pid: u32) -> usize {
    #[cfg(windows)]
    {
        win::apply(pid).0
    }
    #[cfg(not(windows))]
    {
        let _ = pid;
        0
    }
}

/// How many windows of `pid` were changed by the call (0 when they already wear the orange icon).
#[doc(hidden)]
pub fn apply_to_process_counting_changes(pid: u32) -> usize {
    #[cfg(windows)]
    {
        win::apply(pid).1
    }
    #[cfg(not(windows))]
    {
        let _ = pid;
        0
    }
}

/// The handle of the big orange icon, and the icons currently on `pid`'s windows (live tests).
#[doc(hidden)]
pub fn orange_big_handle() -> Option<isize> {
    #[cfg(windows)]
    {
        win::icons().map(|(b, _)| b)
    }
    #[cfg(not(windows))]
    {
        None
    }
}

#[doc(hidden)]
pub fn orange_small_handle() -> Option<isize> {
    #[cfg(windows)]
    {
        win::icons().map(|(_, s)| s)
    }
    #[cfg(not(windows))]
    {
        None
    }
}

#[doc(hidden)]
pub fn current_big_icons(pid: u32) -> Vec<isize> {
    current_icons(pid, false)
}

#[doc(hidden)]
pub fn current_small_icons(pid: u32) -> Vec<isize> {
    current_icons(pid, true)
}

fn current_icons(pid: u32, small: bool) -> Vec<isize> {
    #[cfg(windows)]
    {
        win::current_icons(pid, small)
    }
    #[cfg(not(windows))]
    {
        let _ = (pid, small);
        vec![]
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn the_embedded_icons_are_real_pngs_of_the_expected_sizes() {
        assert_eq!(png_dimensions(BIG), Some((256, 256)));
        assert_eq!(png_dimensions(SMALL), Some((32, 32)));
    }

    #[test]
    fn png_dimensions_rejects_anything_that_is_not_a_png() {
        assert_eq!(png_dimensions(b""), None);
        assert_eq!(png_dimensions(b"GIF89a not a png at all, but long enough to pass a length check"), None);
        let mut broken = BIG.to_vec();
        broken[12] = b'X'; // corrupt the IHDR tag
        assert_eq!(png_dimensions(&broken), None);
        assert_eq!(png_dimensions(&BIG[..10]), None, "truncated");
    }

    /// (r, g, b) average of the clearly visible pixels of a PNG.
    fn average_color(bytes: &[u8]) -> (f64, f64, f64, usize) {
        let mut reader = png::Decoder::new(std::io::Cursor::new(bytes)).read_info().unwrap();
        let mut buf = vec![0; reader.output_buffer_size()];
        let info = reader.next_frame(&mut buf).unwrap();
        assert_eq!(info.color_type, png::ColorType::Rgba, "the icons are RGBA with transparency");
        let (mut r, mut g, mut b, mut n) = (0f64, 0f64, 0f64, 0usize);
        for px in buf[..info.buffer_size()].chunks_exact(4) {
            if px[3] > 200 && !(px[0] > 235 && px[1] > 235 && px[2] > 235) {
                // opaque and not the white ring in the middle of the logo
                r += px[0] as f64;
                g += px[1] as f64;
                b += px[2] as f64;
                n += 1;
            }
        }
        (r / n as f64, g / n as f64, b / n as f64, n)
    }

    #[test]
    fn both_embedded_icons_are_orange_not_the_regular_chrome_logo() {
        for (name, bytes) in [("256", BIG), ("32", SMALL)] {
            let (r, g, b, n) = average_color(bytes);
            assert!(n > 100, "{name}: the logo has visible pixels");
            // Orange: strong red, medium green, little blue. The regular logo (red/green/blue/yellow) averages near grey.
            assert!(r > 230.0 && (120.0..200.0).contains(&g) && b < 110.0, "{name}: average colour ({r:.0},{g:.0},{b:.0}) is not orange");
        }
    }

    #[test]
    fn the_icon_has_transparent_corners() {
        let mut reader = png::Decoder::new(std::io::Cursor::new(BIG)).read_info().unwrap();
        let mut buf = vec![0; reader.output_buffer_size()];
        let info = reader.next_frame(&mut buf).unwrap();
        assert_eq!(buf[3], 0, "the top-left corner pixel is transparent: the logo is a round shape, not a square");
        assert_eq!(info.width, 256);
    }

    #[cfg(windows)]
    #[test]
    fn the_icon_handles_can_be_created_from_the_embedded_pngs() {
        let (big, small) = win::icons().expect("Windows accepts the PNG icons");
        assert!(big != 0 && small != 0 && big != small);
    }

    #[test]
    fn a_pid_with_no_windows_changes_nothing() {
        // pid 0 / an unused pid owns no Chrome window: the call must simply report none.
        assert_eq!(apply_to_process(u32::MAX - 3), 0);
        assert_eq!(apply_to_process_counting_changes(u32::MAX - 3), 0);
    }
}
