pub use monet::glium::glutin::{WindowEvent, KeyboardInput, VirtualKeyCode, MouseButton};

#[derive(Serialize, Deserialize, Hash, PartialEq, Eq, Debug, Copy, Clone)]
#[cfg_attr(rustfmt, rustfmt_skip)]
pub enum Button {
    NumberKey(u8),
    A, B, C, D, E, F, G, H, I, J, K, L, M, N, O, P, Q, R, S, T, U, V, W, X, Y, Z,
    Escape, F1, F2, F3, F4, F5, F6, F7, F8, F9, F10, F11, F12, F13, F14, F15,
    Snapshot, Scroll, Pause, Insert, Home, Delete, End, PageDown, PageUp,
    Left, Up, Right, Down,
    Back, Return, Space, Compose,
    Numlock,
    NumpadNumberKey(u8),
    AbntC1, AbntC2, Add, Apostrophe, Apps, At, Ax, Backslash, Calculator,
    Capital, Colon, Comma, Convert, Decimal, Divide, Equals,
    Grave, Kana, Kanji, LAlt, LBracket, LControl, LMenu, LShift, LWin, Mail,
    MediaSelect, MediaStop, Minus, Multiply, Mute, MyComputer, NavigateForward,
    NavigateBackward, NextTrack, NoConvert, NumpadComma, NumpadEnter, NumpadEquals,
    OEM102, Period, PlayPause, Power, PrevTrack, RAlt, RBracket, RControl,
    RMenu, RShift, RWin, Semicolon, Slash, Sleep, Stop, Subtract, Sysrq, Tab,
    Underline, Unlabeled, VolumeDown, VolumeUp, Wake, WebBack, WebFavorites,
    WebForward, WebHome, WebRefresh, WebSearch, WebStop, Yen,
    LeftMouseButton,
    MiddleMouseButton,
    RightMouseButton,
    OtherMouseButton(u8)
}

pub const MAX_COMBO_LEN: usize = 10;

#[derive(Serialize, Deserialize, PartialEq, Eq, Debug, Default, Copy, Clone)]
pub struct Combo([Option<Button>; MAX_COMBO_LEN]);

impl Combo {
    pub fn new(buttons: &[Button]) -> Self {
        let mut fixed_buttons = [None; 10];
        for (i, button) in buttons.iter().take(10).enumerate() {
            fixed_buttons[i] = Some(*button);
        }
        Combo(fixed_buttons)
    }

    pub fn len(&self) -> usize {
        self.0.iter().filter(|b| b.is_some()).count()
    }

    pub fn is_empty(&self) -> bool {
        self.len() == 0
    }

    pub fn is_in(&self, other: &Combo) -> bool {
        !self.is_empty() &&
            self.0.iter().all(|opt| {
                opt.map(|item| other.0.contains(&Some(item))).unwrap_or(
                    true,
                )
            })
    }

    pub fn is_freshly_in(&self, listener: &ComboListener) -> bool {
        self.is_in(&listener.current) && !self.is_in(&listener.previous)
    }

    pub fn insert(&mut self, button: Button) {
        if !self.0.contains(&Some(button)) {
            for i in 0..MAX_COMBO_LEN {
                if self.0[i].is_none() {
                    self.0[i] = Some(button);
                    return;
                }
            }
        }
    }

    pub fn remove(&mut self, button: &Button) {
        for i in 0..MAX_COMBO_LEN {
            if self.0[i] == Some(*button) {
                self.0[i] = None;
                return;
            }
        }
    }
}

impl ::std::fmt::Display for Combo {
    fn fmt(&self, f: &mut ::std::fmt::Formatter) -> ::std::fmt::Result {
        write!(
            f,
            "{}",
            self.0
                .iter()
                .filter_map(|mb| mb.map(|b| format!("{:?}", b)))
                .collect::<Vec<_>>()
                .join(" + ")
        )
    }
}

#[derive(Serialize, Deserialize, PartialEq, Eq, Debug, Default, Copy, Clone)]
pub struct Combo2(pub [Combo; 2]);

impl Combo2 {
    pub fn new(a: &[Button], b: &[Button]) -> Self {
        Combo2([Combo::new(a), Combo::new(b)])
    }

    pub fn is_in(&self, other: &Combo) -> bool {
        self.0[0].is_in(other) || self.0[1].is_in(other)
    }

    pub fn is_freshly_in(&self, other: &ComboListener) -> bool {
        self.0[0].is_freshly_in(other) || self.0[1].is_freshly_in(other)
    }
}

#[derive(Default, Copy, Clone)]
pub struct ComboListener {
    pub previous: Combo,
    pub current: Combo,
}

use monet::glium::glutin::{Event, ElementState};

impl From<VirtualKeyCode> for Button {
    fn from(source: VirtualKeyCode) -> Self {
        match source {
            VirtualKeyCode::Key1 => Button::NumberKey(1),
            VirtualKeyCode::Key2 => Button::NumberKey(2),
            VirtualKeyCode::Key3 => Button::NumberKey(3),
            VirtualKeyCode::Key4 => Button::NumberKey(4),
            VirtualKeyCode::Key5 => Button::NumberKey(5),
            VirtualKeyCode::Key6 => Button::NumberKey(6),
            VirtualKeyCode::Key7 => Button::NumberKey(7),
            VirtualKeyCode::Key8 => Button::NumberKey(8),
            VirtualKeyCode::Key9 => Button::NumberKey(9),
            VirtualKeyCode::Key0 => Button::NumberKey(0),
            VirtualKeyCode::A => Button::A,
            VirtualKeyCode::B => Button::B,
            VirtualKeyCode::C => Button::C,
            VirtualKeyCode::D => Button::D,
            VirtualKeyCode::E => Button::E,
            VirtualKeyCode::F => Button::F,
            VirtualKeyCode::G => Button::G,
            VirtualKeyCode::H => Button::H,
            VirtualKeyCode::I => Button::I,
            VirtualKeyCode::J => Button::J,
            VirtualKeyCode::K => Button::K,
            VirtualKeyCode::L => Button::L,
            VirtualKeyCode::M => Button::M,
            VirtualKeyCode::N => Button::N,
            VirtualKeyCode::O => Button::O,
            VirtualKeyCode::P => Button::P,
            VirtualKeyCode::Q => Button::Q,
            VirtualKeyCode::R => Button::R,
            VirtualKeyCode::S => Button::S,
            VirtualKeyCode::T => Button::T,
            VirtualKeyCode::U => Button::U,
            VirtualKeyCode::V => Button::V,
            VirtualKeyCode::W => Button::W,
            VirtualKeyCode::X => Button::X,
            VirtualKeyCode::Y => Button::Y,
            VirtualKeyCode::Z => Button::Z,
            VirtualKeyCode::Escape => Button::Escape,
            VirtualKeyCode::F1 => Button::F1,
            VirtualKeyCode::F2 => Button::F2,
            VirtualKeyCode::F3 => Button::F3,
            VirtualKeyCode::F4 => Button::F4,
            VirtualKeyCode::F5 => Button::F5,
            VirtualKeyCode::F6 => Button::F6,
            VirtualKeyCode::F7 => Button::F7,
            VirtualKeyCode::F8 => Button::F8,
            VirtualKeyCode::F9 => Button::F9,
            VirtualKeyCode::F10 => Button::F10,
            VirtualKeyCode::F11 => Button::F11,
            VirtualKeyCode::F12 => Button::F12,
            VirtualKeyCode::F13 => Button::F13,
            VirtualKeyCode::F14 => Button::F14,
            VirtualKeyCode::F15 => Button::F15,
            VirtualKeyCode::Snapshot => Button::Snapshot,
            VirtualKeyCode::Scroll => Button::Scroll,
            VirtualKeyCode::Pause => Button::Pause,
            VirtualKeyCode::Insert => Button::Insert,
            VirtualKeyCode::Home => Button::Home,
            VirtualKeyCode::Delete => Button::Delete,
            VirtualKeyCode::End => Button::End,
            VirtualKeyCode::PageDown => Button::PageDown,
            VirtualKeyCode::PageUp => Button::PageUp,
            VirtualKeyCode::Left => Button::Left,
            VirtualKeyCode::Up => Button::Up,
            VirtualKeyCode::Right => Button::Right,
            VirtualKeyCode::Down => Button::Down,
            VirtualKeyCode::Back => Button::Back,
            VirtualKeyCode::Return => Button::Return,
            VirtualKeyCode::Space => Button::Space,
            VirtualKeyCode::Compose => Button::Compose,
            VirtualKeyCode::Numlock => Button::Numlock,
            VirtualKeyCode::Numpad0 => Button::NumpadNumberKey(0),
            VirtualKeyCode::Numpad1 => Button::NumpadNumberKey(1),
            VirtualKeyCode::Numpad2 => Button::NumpadNumberKey(2),
            VirtualKeyCode::Numpad3 => Button::NumpadNumberKey(3),
            VirtualKeyCode::Numpad4 => Button::NumpadNumberKey(4),
            VirtualKeyCode::Numpad5 => Button::NumpadNumberKey(5),
            VirtualKeyCode::Numpad6 => Button::NumpadNumberKey(6),
            VirtualKeyCode::Numpad7 => Button::NumpadNumberKey(7),
            VirtualKeyCode::Numpad8 => Button::NumpadNumberKey(8),
            VirtualKeyCode::Numpad9 => Button::NumpadNumberKey(9),
            VirtualKeyCode::AbntC1 => Button::AbntC1,
            VirtualKeyCode::AbntC2 => Button::AbntC2,
            VirtualKeyCode::Add => Button::Add,
            VirtualKeyCode::Apostrophe => Button::Apostrophe,
            VirtualKeyCode::Apps => Button::Apps,
            VirtualKeyCode::At => Button::At,
            VirtualKeyCode::Ax => Button::Ax,
            VirtualKeyCode::Backslash => Button::Backslash,
            VirtualKeyCode::Calculator => Button::Calculator,
            VirtualKeyCode::Capital => Button::Capital,
            VirtualKeyCode::Colon => Button::Colon,
            VirtualKeyCode::Comma => Button::Comma,
            VirtualKeyCode::Convert => Button::Convert,
            VirtualKeyCode::Decimal => Button::Decimal,
            VirtualKeyCode::Divide => Button::Divide,
            VirtualKeyCode::Equals => Button::Equals,
            VirtualKeyCode::Grave => Button::Grave,
            VirtualKeyCode::Kana => Button::Kana,
            VirtualKeyCode::Kanji => Button::Kanji,
            VirtualKeyCode::LAlt => Button::LAlt,
            VirtualKeyCode::LBracket => Button::LBracket,
            VirtualKeyCode::LControl => Button::LControl,
            VirtualKeyCode::LMenu => Button::LMenu,
            VirtualKeyCode::LShift => Button::LShift,
            VirtualKeyCode::LWin => Button::LWin,
            VirtualKeyCode::Mail => Button::Mail,
            VirtualKeyCode::MediaSelect => Button::MediaSelect,
            VirtualKeyCode::MediaStop => Button::MediaStop,
            VirtualKeyCode::Minus => Button::Minus,
            VirtualKeyCode::Multiply => Button::Multiply,
            VirtualKeyCode::Mute => Button::Mute,
            VirtualKeyCode::MyComputer => Button::MyComputer,
            VirtualKeyCode::NavigateForward => Button::NavigateForward,
            VirtualKeyCode::NavigateBackward => Button::NavigateBackward,
            VirtualKeyCode::NextTrack => Button::NextTrack,
            VirtualKeyCode::NoConvert => Button::NoConvert,
            VirtualKeyCode::NumpadComma => Button::NumpadComma,
            VirtualKeyCode::NumpadEnter => Button::NumpadEnter,
            VirtualKeyCode::NumpadEquals => Button::NumpadEquals,
            VirtualKeyCode::OEM102 => Button::OEM102,
            VirtualKeyCode::Period => Button::Period,
            VirtualKeyCode::PlayPause => Button::PlayPause,
            VirtualKeyCode::Power => Button::Power,
            VirtualKeyCode::PrevTrack => Button::PrevTrack,
            VirtualKeyCode::RAlt => Button::RAlt,
            VirtualKeyCode::RBracket => Button::RBracket,
            VirtualKeyCode::RControl => Button::RControl,
            VirtualKeyCode::RMenu => Button::RMenu,
            VirtualKeyCode::RShift => Button::RShift,
            VirtualKeyCode::RWin => Button::RWin,
            VirtualKeyCode::Semicolon => Button::Semicolon,
            VirtualKeyCode::Slash => Button::Slash,
            VirtualKeyCode::Sleep => Button::Sleep,
            VirtualKeyCode::Stop => Button::Stop,
            VirtualKeyCode::Subtract => Button::Subtract,
            VirtualKeyCode::Sysrq => Button::Sysrq,
            VirtualKeyCode::Tab => Button::Tab,
            VirtualKeyCode::Underline => Button::Underline,
            VirtualKeyCode::Unlabeled => Button::Unlabeled,
            VirtualKeyCode::VolumeDown => Button::VolumeDown,
            VirtualKeyCode::VolumeUp => Button::VolumeUp,
            VirtualKeyCode::Wake => Button::Wake,
            VirtualKeyCode::WebBack => Button::WebBack,
            VirtualKeyCode::WebFavorites => Button::WebFavorites,
            VirtualKeyCode::WebForward => Button::WebForward,
            VirtualKeyCode::WebHome => Button::WebHome,
            VirtualKeyCode::WebRefresh => Button::WebRefresh,
            VirtualKeyCode::WebSearch => Button::WebSearch,
            VirtualKeyCode::WebStop => Button::WebStop,
            VirtualKeyCode::Yen => Button::Yen,
        }
    }
}

impl From<MouseButton> for Button {
    fn from(source: MouseButton) -> Self {
        match source {
            MouseButton::Left => Button::LeftMouseButton,
            MouseButton::Middle => Button::MiddleMouseButton,
            MouseButton::Right => Button::RightMouseButton,
            MouseButton::Other(code) => Button::OtherMouseButton(code),
        }
    }
}

impl ComboListener {
    pub fn update(&mut self, event: &Event) {
        let old_current = self.current;
        let something_changed = match *event {
            Event::WindowEvent {
                event: WindowEvent::KeyboardInput {
                    device_id: _,
                    input: KeyboardInput {
                        state: state,
                        virtual_keycode: Some(glutin_code),
                        modifiers: _,
                        scancode: _,
                    },
                },
                ..
            } => {
                let pressed = state == ElementState::Pressed;
                if pressed {
                    self.current.insert(glutin_code.into());
                } else {
                    self.current.remove(&(glutin_code.into()));
                }
                true
            }
            Event::WindowEvent {
                event: WindowEvent::MouseInput {
                    device_id: _,
                    state: state,
                    button: glutin_button,
                },
                ..
            } => {
                let pressed = state == ElementState::Pressed;
                if pressed {
                    self.current.insert(glutin_button.into());
                } else {
                    self.current.remove(&glutin_button.into());
                }
                true
            }
            _ => false,
        };
        if something_changed {
            self.previous = old_current;
        }
    }
}

use std::ops::Deref;

impl Deref for ComboListener {
    type Target = Combo;

    fn deref(&self) -> &Combo {
        &self.current
    }
}

#[derive(Serialize, Deserialize, Clone)]
pub struct Bindings {
    bindings: Vec<(String, Combo2)>,
    #[serde(skip_serializing)]
    #[serde(skip_deserializing)]
    rebinding: Option<(String, usize)>,
}

use imgui::{ImGuiSelectableFlags, ImVec2};

impl Bindings {
    pub fn new(bindings: Vec<(&str, Combo2)>) -> Self {
        Bindings {
            bindings: bindings
                .into_iter()
                .map(|(name, combo)| (name.to_owned(), combo))
                .collect(),
            rebinding: None,
        }
    }

    fn pos_of(&self, name: &str) -> usize {
        self.bindings
            .iter()
            .position(|&(ref item_name, _)| item_name == name)
            .expect("Expected binding to exist")
    }

    pub fn settings_ui(&mut self, ui: &::imgui::Ui) -> bool {
        let mut new_target = self.rebinding.clone();
        let mut finished_changing = false;
        for (name, combos) in self.bindings.clone() {
            ui.text(im_str!("{:?}", name));
            ui.same_line(150.0);
            let target_is = match self.rebinding {
                Some((ref target_name, idx)) if target_name == &name => {
                    if idx == 0 {
                        (true, false)
                    } else {
                        (false, true)
                    }
                }
                _ => (false, false),
            };

            if ui.selectable(
                im_str!("{}", combos.0[0]),
                target_is.0,
                ImGuiSelectableFlags::empty(),
                ImVec2::new(200.0, 0.0),
            )
            {
                if target_is.0 {
                    new_target = None;
                    finished_changing = true;
                } else {
                    self[name.as_str()].0[0] = Combo::new(&[]);
                    new_target = Some((name.clone(), 0))
                }
            }
            ui.same_line(350.0);
            if ui.selectable(
                im_str!("(2nd: {})", combos.0[1]),
                target_is.1,
                ImGuiSelectableFlags::empty(),
                ImVec2::new(200.0, 0.0),
            )
            {
                if target_is.1 {
                    new_target = None;
                    finished_changing = true;
                } else {
                    self[name.as_str()].0[1] = Combo::new(&[]);
                    new_target = Some((name.clone(), 1))
                }
            }

        }
        self.rebinding = new_target;
        finished_changing
    }

    pub fn do_rebinding(&mut self, combo: &Combo) {
        if let Some((ref name, idx)) = self.rebinding.clone() {
            if combo.len() > self[name.as_str()].0[idx].len() {
                self[name.as_str()].0[idx] = *combo;
            }
        }
    }
}

impl<'a> ::std::ops::Index<&'a str> for Bindings {
    type Output = Combo2;

    fn index(&self, name: &'a str) -> &Combo2 {
        &self.bindings[self.pos_of(name)].1
    }
}

impl<'a> ::std::ops::IndexMut<&'a str> for Bindings {
    fn index_mut(&mut self, name: &'a str) -> &mut Combo2 {
        let pos = self.pos_of(name);
        &mut self.bindings[pos].1
    }
}
