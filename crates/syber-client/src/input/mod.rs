//! Input capture on the client side.
//! Translates egui raw events → kynput InputPackets → kymux wire format.

use kymux_types::input::InputPacket as WirePacket;
use kynput::{
    InputPacket, InputTarget, Payload,
    types::{KeyboardKey, MouseButton, MouseButtonType, MouseMove, MousePosition, MouseWheel},
};
use bytes::Bytes;

/// Convert an egui key event to a wire InputPacket.
pub fn egui_key_to_wire(key: egui::Key, pressed: bool, _mods: &egui::Modifiers) -> Option<WirePacket> {
    let scancode = egui_key_to_scancode(key)?;
    let kp = InputPacket::new(InputTarget::Host, Payload::Keyboard(
        KeyboardKey::new(scancode, pressed)
    ));
    Some(to_wire(kp))
}

/// Convert egui pointer delta to a wire MouseMove packet.
pub fn egui_mouse_move_to_wire(delta: egui::Vec2) -> WirePacket {
    let kp = InputPacket::new(InputTarget::Host, Payload::MouseMove(
        MouseMove::new(delta.x as i16, delta.y as i16)
    ));
    to_wire(kp)
}

/// Convert egui absolute pointer position to a MousePosition packet.
pub fn egui_mouse_pos_to_wire(pos: egui::Pos2, display_id: u32) -> WirePacket {
    let kp = InputPacket::new(InputTarget::Host, Payload::MousePosition(
        MousePosition::new(display_id, pos.x as i16, pos.y as i16)
    ));
    to_wire(kp)
}

/// Convert egui pointer button event to a wire MouseButton packet.
pub fn egui_mouse_button_to_wire(btn: egui::PointerButton, pressed: bool) -> Option<WirePacket> {
    let btn_type = match btn {
        egui::PointerButton::Primary   => MouseButtonType::Left,
        egui::PointerButton::Secondary => MouseButtonType::Right,
        egui::PointerButton::Middle    => MouseButtonType::Middle,
        _ => return None,
    };
    let kp = InputPacket::new(InputTarget::Host, Payload::MouseButton(
        MouseButton::new(btn_type, pressed)
    ));
    Some(to_wire(kp))
}

/// Convert egui scroll delta to wire MouseWheel packets.
pub fn egui_scroll_to_wire(delta: egui::Vec2) -> Vec<WirePacket> {
    let mut pkts = Vec::new();
    if delta.x.abs() > 0.1 || delta.y.abs() > 0.1 {
        let kp = InputPacket::new(InputTarget::Host, Payload::MouseWheel(
            MouseWheel::new(delta.x, delta.y)
        ));
        pkts.push(to_wire(kp));
    }
    pkts
}

// ─────────────────────────────────────────────────────────────────────────────

fn to_wire(pkt: InputPacket) -> WirePacket {
    WirePacket {
        type_:   pkt.get_type() as u8,
        payload: Bytes::from(pkt.serialize()),
    }
}

/// egui Key → PS/2 scan code set 1 (used by kynput as cross-platform basis)
fn egui_key_to_scancode(key: egui::Key) -> Option<u16> {
    let sc: u16 = match key {
        // Letters (scan codes set 1)
        egui::Key::A => 0x1E, egui::Key::B => 0x30, egui::Key::C => 0x2E,
        egui::Key::D => 0x20, egui::Key::E => 0x12, egui::Key::F => 0x21,
        egui::Key::G => 0x22, egui::Key::H => 0x23, egui::Key::I => 0x17,
        egui::Key::J => 0x24, egui::Key::K => 0x25, egui::Key::L => 0x26,
        egui::Key::M => 0x32, egui::Key::N => 0x31, egui::Key::O => 0x18,
        egui::Key::P => 0x19, egui::Key::Q => 0x10, egui::Key::R => 0x13,
        egui::Key::S => 0x1F, egui::Key::T => 0x14, egui::Key::U => 0x16,
        egui::Key::V => 0x2F, egui::Key::W => 0x11, egui::Key::X => 0x2D,
        egui::Key::Y => 0x15, egui::Key::Z => 0x2C,
        // Digits
        egui::Key::Num0 => 0x0B, egui::Key::Num1 => 0x02, egui::Key::Num2 => 0x03,
        egui::Key::Num3 => 0x04, egui::Key::Num4 => 0x05, egui::Key::Num5 => 0x06,
        egui::Key::Num6 => 0x07, egui::Key::Num7 => 0x08, egui::Key::Num8 => 0x09,
        egui::Key::Num9 => 0x0A,
        // Special
        egui::Key::Space     => 0x39, egui::Key::Enter      => 0x1C,
        egui::Key::Backspace => 0x0E, egui::Key::Tab        => 0x0F,
        egui::Key::Escape    => 0x01, egui::Key::Delete      => 0xE053,
        egui::Key::ArrowLeft => 0xE04B, egui::Key::ArrowRight => 0xE04D,
        egui::Key::ArrowUp   => 0xE048, egui::Key::ArrowDown  => 0xE050,
        egui::Key::Home  => 0xE047, egui::Key::End       => 0xE04F,
        egui::Key::PageUp => 0xE049, egui::Key::PageDown  => 0xE051,
        // F-keys
        egui::Key::F1  => 0x3B, egui::Key::F2  => 0x3C, egui::Key::F3  => 0x3D,
        egui::Key::F4  => 0x3E, egui::Key::F5  => 0x3F, egui::Key::F6  => 0x40,
        egui::Key::F7  => 0x41, egui::Key::F8  => 0x42, egui::Key::F9  => 0x43,
        egui::Key::F10 => 0x44, egui::Key::F11 => 0x57, egui::Key::F12 => 0x58,
        _ => return None,
    };
    Some(sc)
}
