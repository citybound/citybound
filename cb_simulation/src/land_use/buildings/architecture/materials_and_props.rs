#[derive(Copy, Clone, PartialEq, Eq, Debug, Hash, Serialize, Deserialize)]
pub enum BuildingMaterial {
    WhiteWall,
    TiledRoof,
    FlatRoof,
    FieldWheat,
    FieldRows,
    FieldPlant,
    FieldMeadow,
    WoodenFence,
    MetalFence,
    LotAsphalt,
}

impl ::std::fmt::Display for BuildingMaterial {
    fn fmt(&self, f: &mut ::std::fmt::Formatter) -> ::std::fmt::Result {
        ::std::fmt::Debug::fmt(self, f)
    }
}

pub const ALL_MATERIALS: [BuildingMaterial; 10] = [
    BuildingMaterial::WhiteWall,
    BuildingMaterial::TiledRoof,
    BuildingMaterial::FlatRoof,
    BuildingMaterial::FieldWheat,
    BuildingMaterial::FieldRows,
    BuildingMaterial::FieldPlant,
    BuildingMaterial::FieldMeadow,
    BuildingMaterial::WoodenFence,
    BuildingMaterial::MetalFence,
    BuildingMaterial::LotAsphalt,
];

#[derive(Copy, Clone, PartialEq, Eq, Debug, Hash,  Serialize, Deserialize)]
pub enum BuildingProp {
    SmallWindow,
    ShopWindowGlass,
    ShopWindowBanner,
    NarrowDoor,
    WideDoor,
}

impl ::std::fmt::Display for BuildingProp {
    fn fmt(&self, f: &mut ::std::fmt::Formatter) -> ::std::fmt::Result {
        ::std::fmt::Debug::fmt(self, f)
    }
}

pub const ALL_PROP_TYPES: [BuildingProp; 5] = [
    BuildingProp::SmallWindow,
    BuildingProp::ShopWindowGlass,
    BuildingProp::ShopWindowBanner,
    BuildingProp::NarrowDoor,
    BuildingProp::WideDoor,
];