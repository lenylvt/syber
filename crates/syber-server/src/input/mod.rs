//! Input injection on the server (Host) side.

use anyhow::Result;
use kymux_types::input::InputPacket as WireInputPacket;
use kynput::{InputPacket, InputTarget};

pub fn inject(wire: &WireInputPacket) -> Result<()> {
    let pkt = InputPacket::deserialize(wire.type_, InputTarget::Host, &wire.payload)
        .map_err(|e| anyhow::anyhow!("deserialize input: {e:?}"))?;
    platform_inject(pkt)
}

// ── Windows ───────────────────────────────────────────────────────────────────

#[cfg(target_os = "windows")]
fn platform_inject(pkt: InputPacket) -> Result<()> {
    use kynput::Payload;
    match pkt.payload {
        Payload::Keyboard(k)      => win_keyboard(k.scancode, k.pressed),
        Payload::MouseMove(m)     => win_mouse_move(m.dx as i32, m.dy as i32),
        Payload::MouseButton(m)   => win_mouse_btn(&m),
        Payload::MouseWheel(w)    => win_mouse_wheel(w.dy),
        Payload::MousePosition(p) => win_mouse_pos(p.x as i32, p.y as i32),
        _ => Ok(()),
    }
}

#[cfg(target_os = "windows")]
fn win_keyboard(scancode: u16, pressed: bool) -> Result<()> {
    use windows_sys::Win32::UI::Input::KeyboardAndMouse::*;
    unsafe {
        let input = INPUT {
            r#type: INPUT_KEYBOARD,
            Anonymous: INPUT_0 {
                ki: KEYBDINPUT {
                    wVk:         0,
                    wScan:       scancode,
                    dwFlags:     KEYEVENTF_SCANCODE | if pressed { 0 } else { KEYEVENTF_KEYUP },
                    time:        0,
                    dwExtraInfo: 0,
                },
            },
        };
        SendInput(1, &input, std::mem::size_of::<INPUT>() as i32);
    }
    Ok(())
}

#[cfg(target_os = "windows")]
fn win_mouse_move(dx: i32, dy: i32) -> Result<()> {
    use windows_sys::Win32::UI::Input::KeyboardAndMouse::*;
    unsafe {
        let input = INPUT {
            r#type: INPUT_MOUSE,
            Anonymous: INPUT_0 {
                mi: MOUSEINPUT {
                    dx, dy, mouseData: 0,
                    dwFlags: MOUSEEVENTF_MOVE, time: 0, dwExtraInfo: 0,
                },
            },
        };
        SendInput(1, &input, std::mem::size_of::<INPUT>() as i32);
    }
    Ok(())
}

#[cfg(target_os = "windows")]
fn win_mouse_btn(m: &kynput::types::MouseButton) -> Result<()> {
    use kynput::types::MouseButtonType;
    use windows_sys::Win32::UI::Input::KeyboardAndMouse::*;
    let flags = match (m.button, m.pressed) {
        (MouseButtonType::Left,   true)  => MOUSEEVENTF_LEFTDOWN,
        (MouseButtonType::Left,   false) => MOUSEEVENTF_LEFTUP,
        (MouseButtonType::Right,  true)  => MOUSEEVENTF_RIGHTDOWN,
        (MouseButtonType::Right,  false) => MOUSEEVENTF_RIGHTUP,
        (MouseButtonType::Middle, true)  => MOUSEEVENTF_MIDDLEDOWN,
        (MouseButtonType::Middle, false) => MOUSEEVENTF_MIDDLEUP,
        _ => return Ok(()),
    };
    unsafe {
        let input = INPUT {
            r#type: INPUT_MOUSE,
            Anonymous: INPUT_0 {
                mi: MOUSEINPUT {
                    dx: 0, dy: 0, mouseData: 0,
                    dwFlags: flags, time: 0, dwExtraInfo: 0,
                },
            },
        };
        SendInput(1, &input, std::mem::size_of::<INPUT>() as i32);
    }
    Ok(())
}

#[cfg(target_os = "windows")]
fn win_mouse_wheel(dy: f32) -> Result<()> {
    use windows_sys::Win32::UI::Input::KeyboardAndMouse::*;
    unsafe {
        let input = INPUT {
            r#type: INPUT_MOUSE,
            Anonymous: INPUT_0 {
                mi: MOUSEINPUT {
                    dx: 0, dy: 0,
                    mouseData: (dy * 120.0) as u32,
                    dwFlags: MOUSEEVENTF_WHEEL, time: 0, dwExtraInfo: 0,
                },
            },
        };
        SendInput(1, &input, std::mem::size_of::<INPUT>() as i32);
    }
    Ok(())
}

#[cfg(target_os = "windows")]
fn win_mouse_pos(x: i32, y: i32) -> Result<()> {
    use windows_sys::Win32::UI::Input::KeyboardAndMouse::*;
    unsafe {
        let input = INPUT {
            r#type: INPUT_MOUSE,
            Anonymous: INPUT_0 {
                mi: MOUSEINPUT {
                    dx: x, dy: y, mouseData: 0,
                    dwFlags: MOUSEEVENTF_MOVE | MOUSEEVENTF_ABSOLUTE,
                    time: 0, dwExtraInfo: 0,
                },
            },
        };
        SendInput(1, &input, std::mem::size_of::<INPUT>() as i32);
    }
    Ok(())
}

// ── Linux ─────────────────────────────────────────────────────────────────────

#[cfg(target_os = "linux")]
fn platform_inject(pkt: InputPacket) -> Result<()> {
    use kynput::Payload;
    match pkt.payload {
        Payload::Keyboard(k) => {
            let key = scancode_to_xdotool(k.scancode);
            let act = if k.pressed { "keydown" } else { "keyup" };
            let _ = std::process::Command::new("xdotool").args([act, &key]).spawn();
        }
        Payload::MouseMove(m) => {
            let _ = std::process::Command::new("xdotool")
                .args(["mousemove_relative", "--", &m.dx.to_string(), &m.dy.to_string()])
                .spawn();
        }
        Payload::MouseButton(m) => {
            let btn = match m.button {
                kynput::types::MouseButtonType::Left   => "1",
                kynput::types::MouseButtonType::Middle => "2",
                kynput::types::MouseButtonType::Right  => "3",
                _ => return Ok(()),
            };
            let act = if m.pressed { "mousedown" } else { "mouseup" };
            let _ = std::process::Command::new("xdotool").args([act, btn]).spawn();
        }
        Payload::MouseWheel(w) => {
            let btn = if w.dy > 0.0 { "4" } else { "5" };
            let _ = std::process::Command::new("xdotool").args(["click", btn]).spawn();
        }
        Payload::MousePosition(p) => {
            let _ = std::process::Command::new("xdotool")
                .args(["mousemove", &p.x.to_string(), &p.y.to_string()])
                .spawn();
        }
        _ => {}
    }
    Ok(())
}

#[cfg(target_os = "linux")]
fn scancode_to_xdotool(sc: u16) -> String {
    match sc {
        0x01 => "Escape",   0x0E => "BackSpace", 0x0F => "Tab",
        0x1C => "Return",   0x39 => "space",
        0x1E => "a", 0x30 => "b", 0x2E => "c", 0x20 => "d",
        0x12 => "e", 0x21 => "f", 0x22 => "g", 0x23 => "h",
        0x17 => "i", 0x24 => "j", 0x25 => "k", 0x26 => "l",
        0x32 => "m", 0x31 => "n", 0x18 => "o", 0x19 => "p",
        0x10 => "q", 0x13 => "r", 0x1F => "s", 0x14 => "t",
        0x16 => "u", 0x2F => "v", 0x11 => "w", 0x2D => "x",
        0x15 => "y", 0x2C => "z",
        0x02..=0x0B => &["1","2","3","4","5","6","7","8","9","0"][(sc-0x02) as usize],
        0x3B => "F1",  0x3C => "F2",  0x3D => "F3",  0x3E => "F4",
        0x3F => "F5",  0x40 => "F6",  0x41 => "F7",  0x42 => "F8",
        0x43 => "F9",  0x44 => "F10", 0x57 => "F11", 0x58 => "F12",
        _ => "space",
    }.to_string()
}

// ── macOS (server non supporté) ───────────────────────────────────────────────

#[cfg(not(any(target_os = "windows", target_os = "linux")))]
fn platform_inject(_pkt: InputPacket) -> Result<()> { Ok(()) }
