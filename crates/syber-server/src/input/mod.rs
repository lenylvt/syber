//! Input injection on the server (Host) side.
//! Receives kymux_types::InputPacket from the network, deserializes using kynput,
//! and injects via kynput's host-side handlers.

use anyhow::Result;
use kymux_types::input::InputPacket as WireInputPacket;
use kynput::{InputPacket, InputTarget};

/// Inject a received wire input packet into the OS.
/// The caller provides the platform-specific injection closures
/// via the kynput pipeline.
pub fn inject(wire_pkt: &WireInputPacket) -> Result<()> {
    let kynput_pkt = InputPacket::deserialize(
        wire_pkt.type_,
        InputTarget::Host,
        &wire_pkt.payload,
    ).map_err(|e| anyhow::anyhow!("input deserialize: {e:?}"))?;

    platform_inject(kynput_pkt)
}

#[cfg(target_os = "windows")]
fn platform_inject(pkt: InputPacket) -> Result<()> {
    use kynput::{Payload, types::*};
    match &pkt.payload {
        Payload::Keyboard(k)      => inject_keyboard_windows(k),
        Payload::MouseMove(m)     => inject_mouse_move_windows(m),
        Payload::MouseButton(m)   => inject_mouse_button_windows(m),
        Payload::MouseWheel(w)    => inject_mouse_wheel_windows(w),
        Payload::MousePosition(p) => inject_mouse_pos_windows(p),
        _ => Ok(()), // cursor, clipboard, etc. handled separately
    }
}

#[cfg(target_os = "windows")]
fn inject_keyboard_windows(k: &kynput::types::KeyboardKey) -> Result<()> {
    use windows_sys::Win32::UI::Input::KeyboardAndMouse::*;

    let vk = k.key as u16;
    let flags = if k.pressed { 0 } else { KEYEVENTF_KEYUP };
    unsafe {
        let input = INPUT {
            r#type: INPUT_KEYBOARD,
            Anonymous: INPUT_0 {
                ki: KEYBDINPUT { wVk: vk, wScan: 0, dwFlags: flags, time: 0, dwExtraInfo: 0 },
            },
        };
        SendInput(1, &input, std::mem::size_of::<INPUT>() as i32);
    }
    Ok(())
}

#[cfg(target_os = "windows")]
fn inject_mouse_move_windows(m: &kynput::types::MouseMove) -> Result<()> {
    use windows_sys::Win32::UI::Input::KeyboardAndMouse::*;
    unsafe {
        let input = INPUT {
            r#type: INPUT_MOUSE,
            Anonymous: INPUT_0 {
                mi: MOUSEINPUT {
                    dx: m.x as i32, dy: m.y as i32,
                    mouseData: 0,
                    dwFlags: MOUSEEVENTF_MOVE,
                    time: 0, dwExtraInfo: 0,
                },
            },
        };
        SendInput(1, &input, std::mem::size_of::<INPUT>() as i32);
    }
    Ok(())
}

#[cfg(target_os = "windows")]
fn inject_mouse_button_windows(m: &kynput::types::MouseButton) -> Result<()> {
    use windows_sys::Win32::UI::Input::KeyboardAndMouse::*;
    let flags = match (m.button, m.pressed) {
        (kynput::types::MouseButtonType::Left,   true)  => MOUSEEVENTF_LEFTDOWN,
        (kynput::types::MouseButtonType::Left,   false) => MOUSEEVENTF_LEFTUP,
        (kynput::types::MouseButtonType::Right,  true)  => MOUSEEVENTF_RIGHTDOWN,
        (kynput::types::MouseButtonType::Right,  false) => MOUSEEVENTF_RIGHTUP,
        (kynput::types::MouseButtonType::Middle, true)  => MOUSEEVENTF_MIDDLEDOWN,
        (kynput::types::MouseButtonType::Middle, false) => MOUSEEVENTF_MIDDLEUP,
        _ => return Ok(()),
    };
    unsafe {
        let input = INPUT {
            r#type: INPUT_MOUSE,
            Anonymous: INPUT_0 {
                mi: MOUSEINPUT {
                    dx: 0, dy: 0, mouseData: 0, dwFlags: flags, time: 0, dwExtraInfo: 0,
                },
            },
        };
        SendInput(1, &input, std::mem::size_of::<INPUT>() as i32);
    }
    Ok(())
}

#[cfg(target_os = "windows")]
fn inject_mouse_wheel_windows(w: &kynput::types::MouseWheel) -> Result<()> {
    use windows_sys::Win32::UI::Input::KeyboardAndMouse::*;
    unsafe {
        let input = INPUT {
            r#type: INPUT_MOUSE,
            Anonymous: INPUT_0 {
                mi: MOUSEINPUT {
                    dx: 0, dy: 0,
                    mouseData: (w.delta * 120) as u32,
                    dwFlags: MOUSEEVENTF_WHEEL,
                    time: 0, dwExtraInfo: 0,
                },
            },
        };
        SendInput(1, &input, std::mem::size_of::<INPUT>() as i32);
    }
    Ok(())
}

#[cfg(target_os = "windows")]
fn inject_mouse_pos_windows(p: &kynput::types::MousePosition) -> Result<()> {
    use windows_sys::Win32::UI::Input::KeyboardAndMouse::*;
    unsafe {
        let input = INPUT {
            r#type: INPUT_MOUSE,
            Anonymous: INPUT_0 {
                mi: MOUSEINPUT {
                    dx: p.x as i32, dy: p.y as i32,
                    mouseData: 0,
                    dwFlags: MOUSEEVENTF_MOVE | MOUSEEVENTF_ABSOLUTE,
                    time: 0, dwExtraInfo: 0,
                },
            },
        };
        SendInput(1, &input, std::mem::size_of::<INPUT>() as i32);
    }
    Ok(())
}

// ── Linux ────────────────────────────────────────────────────────────────────

#[cfg(target_os = "linux")]
fn platform_inject(pkt: InputPacket) -> Result<()> {
    use kynput::{Payload, types::*};
    match &pkt.payload {
        Payload::Keyboard(k)      => inject_keyboard_linux(k),
        Payload::MouseMove(m)     => inject_mouse_move_linux(m),
        Payload::MouseButton(m)   => inject_mouse_button_linux(m),
        Payload::MouseWheel(w)    => inject_mouse_wheel_linux(w),
        Payload::MousePosition(p) => inject_mouse_pos_linux(p),
        _ => Ok(()),
    }
}

#[cfg(target_os = "linux")]
fn inject_keyboard_linux(k: &kynput::types::KeyboardKey) -> Result<()> {
    // Use xdotool or XTest extension via xcb
    // For now: use the kynput xcb-based injection by spawning xdotool
    let action = if k.pressed { "keydown" } else { "keyup" };
    let keyname = keycode_to_x11_name(k.key as u32);
    let _ = std::process::Command::new("xdotool")
        .args([action, "--clearmodifiers", &keyname])
        .spawn();
    Ok(())
}

#[cfg(target_os = "linux")]
fn inject_mouse_move_linux(m: &kynput::types::MouseMove) -> Result<()> {
    let _ = std::process::Command::new("xdotool")
        .args(["mousemove_relative", "--", &m.x.to_string(), &m.y.to_string()])
        .spawn();
    Ok(())
}

#[cfg(target_os = "linux")]
fn inject_mouse_button_linux(m: &kynput::types::MouseButton) -> Result<()> {
    let btn = match m.button {
        kynput::types::MouseButtonType::Left   => "1",
        kynput::types::MouseButtonType::Middle => "2",
        kynput::types::MouseButtonType::Right  => "3",
        _ => return Ok(()),
    };
    let action = if m.pressed { "mousedown" } else { "mouseup" };
    let _ = std::process::Command::new("xdotool")
        .args([action, btn])
        .spawn();
    Ok(())
}

#[cfg(target_os = "linux")]
fn inject_mouse_wheel_linux(w: &kynput::types::MouseWheel) -> Result<()> {
    let btn = if w.delta > 0 { "4" } else { "5" };
    let _ = std::process::Command::new("xdotool")
        .args(["click", btn])
        .spawn();
    Ok(())
}

#[cfg(target_os = "linux")]
fn inject_mouse_pos_linux(p: &kynput::types::MousePosition) -> Result<()> {
    let _ = std::process::Command::new("xdotool")
        .args(["mousemove", &p.x.to_string(), &p.y.to_string()])
        .spawn();
    Ok(())
}

#[cfg(target_os = "linux")]
fn keycode_to_x11_name(keycode: u32) -> String {
    // Very simplified mapping — kynput provides full keycode crate
    match keycode {
        8 => "BackSpace", 9 => "Tab", 13 => "Return",
        27 => "Escape", 32 => "space", 37 => "ctrl",
        _ => "space",
    }.to_string()
}

// ── macOS (server not supported) ─────────────────────────────────────────────

#[cfg(not(any(target_os = "windows", target_os = "linux")))]
fn platform_inject(_pkt: InputPacket) -> Result<()> {
    Ok(()) // macOS server not yet implemented
}
