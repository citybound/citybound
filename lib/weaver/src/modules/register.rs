
use ::ModWrapper;

/// Used to register mods into the engine.
#[doc(hidden)]
#[derive(Default)]
pub struct Register {
    mods: Vec<Mod>,
}

/// Used to add additional data to a mod.
#[doc(hidden)]
pub struct Mod {
    wrapper: Box<ModWrapper>,
}

impl Register {
    pub fn new() -> Register {
        Register::default()
    }

    pub fn register_mod<M>(&mut self, mod_: M) -> &mut Mod
        where M: ModWrapper + 'static
    {
        let index = self.mods.len();
        let mod_ = Mod { wrapper: Box::new(mod_) };

        self.mods.push(mod_);
        &mut self.mods[index]
    }

    pub fn deconstruct(self) -> (Vec<Mod>,) {
        (self.mods,)
    }
}

impl Mod {
    pub fn deconstruct(self) -> (Box<ModWrapper>,) {
        (self.wrapper,)
    }
}
