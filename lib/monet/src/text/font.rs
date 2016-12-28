
use rusttype;

pub use descartes::{N, P3, P2, V3, V4, M4, Iso3, Persp3, ToHomogeneous, Norm, Into2d, Into3d,
                    WithUniqueOrthogonal, Inverse, Rotate};

#[derive(Copy, Clone)]
pub enum Font {
    Debug,
}

pub struct FontBank {
    dpi_factor: f32,
}

impl FontBank {
    pub fn new(dpi_factor: f32) -> FontBank {
        FontBank {
            dpi_factor: dpi_factor,
        }
    }

    pub fn font(&self, font: Font) -> FontDescription {
        let (font, scale) = match font {
            Font::Debug => (&FONT_CLEAR_SANS_REGULAR, 14.0),
        };

        FontDescription::new(font, scale, self.dpi_factor)
    }
}

#[derive(Copy, Clone)]
pub struct FontDescription {
    font: &'static rusttype::Font<'static>,
    scale: rusttype::Scale,
}

impl FontDescription {
    fn new(font: &'static rusttype::Font<'static>, scale: f32, dpi_factor: f32) -> FontDescription {
        FontDescription {
            font: font,
            scale: rusttype::Scale::uniform(scale * dpi_factor),
        }
    }

    #[inline]
    pub fn font(&self) -> &'static rusttype::Font<'static> {
        self.font
    }

    #[inline]
    pub fn scale(&self) -> rusttype::Scale {
        self.scale
    }
}

macro_rules! font {
    ( $name:expr ) => {
        {
            let bytes: &[u8] = include_bytes!(concat!("../../../../fonts/", $name));
            rusttype::FontCollection::from_bytes(bytes).into_font().unwrap()
        }
    }
}

lazy_static! {
    static ref FONT_CLEAR_SANS_REGULAR: rusttype::Font<'static> =
        font!("ClearSans-Regular.ttf");
}
