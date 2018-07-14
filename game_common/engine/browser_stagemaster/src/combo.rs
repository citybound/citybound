#[derive( Hash, PartialEq, Eq, Debug, Copy, Clone)]
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

#[derive(PartialEq, Eq, Debug, Default, Copy, Clone)]
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
        !self.is_empty() && self.0.iter().all(|opt| {
            opt.map(|item| other.0.contains(&Some(item)))
                .unwrap_or(true)
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

#[derive(PartialEq, Eq, Debug, Default, Copy, Clone)]
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

use std::ops::Deref;

impl Deref for ComboListener {
    type Target = Combo;

    fn deref(&self) -> &Combo {
        &self.current
    }
}

#[derive(Clone)]
pub struct Bindings {
    bindings: Vec<(String, Combo2)>,
    rebinding: Option<(String, usize)>,
}

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
