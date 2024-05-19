//! Event Handler Utilities

use iced_runtime::core::keyboard as core_kb;
use iced_runtime::keyboard::key::Named;
use iced_runtime::keyboard::{self as iced_kb, Key};
use iced_wgpu::core::mouse::Button;
use iced_wgpu::core::{mouse, Point};
use smithay_client_toolkit::seat::keyboard::{self as stk_kb, Keysym};
use smithay_client_toolkit::seat::pointer::{PointerEvent, PointerEventKind};
use smol_str::ToSmolStr;

fn key_convert_key(k: Keysym) -> iced_kb::Key {
    match k {
        Keysym::Tab => Key::Named(Named::Tab),
        Keysym::Escape => Key::Named(Named::Escape),
        Keysym::Return => Key::Named(Named::Enter),
        Keysym::BackSpace => Key::Named(Named::Backspace),
        Keysym::Home => Key::Named(Named::Home),
        Keysym::End => Key::Named(Named::End),
        Keysym::Page_Up => Key::Named(Named::PageUp),
        Keysym::Page_Down => Key::Named(Named::PageDown),
        _ => match k.key_char() {
            Some(c) => Key::Character(c.to_smolstr()),
            None => {
                println!("UNIDENFITIED {k:?}");
                iced_kb::Key::Unidentified
            }
        },
    }
}

fn key_convert_modifiers(m: Option<stk_kb::Modifiers>) -> iced_kb::Modifiers {
    let mut modifiers = iced_kb::Modifiers::default();
    if let Some(mods) = m {
        modifiers.set(iced_kb::Modifiers::CTRL, mods.ctrl);
        modifiers.set(iced_kb::Modifiers::SHIFT, mods.shift);
        modifiers.set(iced_kb::Modifiers::ALT, mods.alt);
        modifiers.set(iced_kb::Modifiers::LOGO, mods.logo);
    }
    modifiers
}

fn mouse_convert_button(button: u32) -> Button {
    match button {
        272 => Button::Left,
        273 => Button::Right,
        274 => Button::Middle,
        275 => Button::Back,
        276 => Button::Forward,
        _ => Button::Other(button as u16),
    }
}

/// Convert Wayland Keyboard-Event into Iced Keyboard-Event
pub fn keypress_event(
    event: stk_kb::KeyEvent,
    modifiers: Option<stk_kb::Modifiers>,
    released: bool,
) -> core_kb::Event {
    if released {
        core_kb::Event::KeyReleased {
            key: key_convert_key(event.keysym),
            location: core_kb::Location::Standard,
            modifiers: key_convert_modifiers(modifiers),
        }
    } else {
        core_kb::Event::KeyPressed {
            key: key_convert_key(event.keysym),
            location: core_kb::Location::Standard,
            modifiers: key_convert_modifiers(modifiers),
            text: event.utf8.map(|s| s.to_smolstr()),
        }
    }
}

/// Convert Wayland Modifiers-Event to Iced Modifiers-Event
pub fn modifiers_event(modifiers: stk_kb::Modifiers) -> core_kb::Event {
    core_kb::Event::ModifiersChanged(key_convert_modifiers(Some(modifiers)))
}

/// Convert Wayland Mouse-Event into Iced Mouse-Event
#[allow(unused_variables)]
pub fn mouse_event(event: &PointerEvent) -> mouse::Event {
    match event.kind {
        PointerEventKind::Enter { serial } => mouse::Event::CursorEntered,
        PointerEventKind::Leave { serial } => mouse::Event::CursorLeft,
        PointerEventKind::Motion { .. } => mouse::Event::CursorMoved {
            position: Point::new(event.position.0 as f32, event.position.1 as f32),
        },
        PointerEventKind::Press {
            time,
            button,
            serial,
        } => mouse::Event::ButtonPressed(mouse_convert_button(button)),
        PointerEventKind::Release {
            time,
            button,
            serial,
        } => mouse::Event::ButtonReleased(mouse_convert_button(button)),
        _ => {
            println!("OTHER MOUSE EVENT: {event:?}");
            mouse::Event::ButtonPressed(Button::Left)
        }
    }
}
