
use rusttype;

pub use descartes::{N, P3, P2, V3, V4, M4, Iso3, Persp3, ToHomogeneous, Norm, Into2d, Into3d,
                    WithUniqueOrthogonal, Inverse, Rotate};

#[derive(Copy, Clone)]
pub enum Font {
    Debug,
}

pub struct FontBank {
    pub dpi_factor: f32,
    text_font: rusttype::Font<'static>,
}

impl FontBank {
    pub fn new(dpi_factor: f32) -> FontBank {
        FontBank {
            dpi_factor: dpi_factor,
            text_font: rusttype::FontCollection::from_bytes(
                include_bytes!("../../../../fonts/ClearSans-Regular.ttf") as &[u8]
            ).into_font().unwrap(),
        }
    }

    pub fn font(&self, font: Font) -> (&rusttype::Font<'static>, rusttype::Scale) {
        let (font, scale) = match font {
            Font::Debug => (&self.text_font, 14.0),
        };

        (font, rusttype::Scale::uniform(scale * self.dpi_factor))
    }
}
