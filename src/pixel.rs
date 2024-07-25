use std::fmt::{Debug, Formatter};

#[derive(Default, Copy, Clone)]
pub(crate) struct RGB555(u16);
#[derive(Default, Copy, Clone)]
pub(crate) struct RGB888(u32);

pub(crate) const fn u5_to_u8(v: u8) -> u8 {
    (v << 3) | (v >> 2)
}

impl RGB555 {
    pub(crate) const fn from_parts(r: u8, g: u8, b: u8) -> Self {
        Self(
            (((r as u16) << 7) & 0x7C00)
                | (((g as u16) << 2) & 0x03E0)
                | (((b as u16) >> 3) & 0x001F)
        )
    }

    pub(crate) const fn to_parts(self) -> (u8, u8, u8) {
        (
            u5_to_u8(((self.0 >> 10) & 0x1F) as u8),
            u5_to_u8(((self.0 >> 5) & 0x1F) as u8),
            u5_to_u8((self.0 & 0x1F) as u8)
        )
    }
}

impl Debug for RGB555 {
    fn fmt(&self, f: &mut Formatter<'_>) -> std::fmt::Result {
        let (r, g, b) = self.to_parts();
        f.debug_struct("RGB555")
            .field("repr", &self.0)
            .field("r",  &r)
            .field("g", &g)
            .field("b", &b)
            .finish()
    }
}

impl RGB888 {
    pub(crate) const fn from_parts(r: u8, g: u8, b: u8) -> Self {
        Self(
            ((r as u32) << 16)
            | ((g as u32) << 8)
            | (b as u32)
        )
    }

    pub(crate) const fn to_parts(self) -> (u8, u8, u8) {
        (
            ((self.0 >> 16) & 0xFF) as u8,
            ((self.0 >> 8) & 0xFF) as u8,
            (self.0 & 0xFF) as u8
        )
    }
}

impl Debug for RGB888 {
    fn fmt(&self, f: &mut Formatter<'_>) -> std::fmt::Result {
        let (r, g, b) = self.to_parts();
        f.debug_struct("RGB888")
            .field("repr", &self.0)
            .field("r",  &r)
            .field("g", &g)
            .field("b", &b)
            .finish()
    }
}

impl From<u32> for RGB888 {
    fn from(value: u32) -> Self {
        Self(value)
    }
}

impl From<RGB888> for u32 {
    fn from(value: RGB888) -> Self {
        value.0
    }
}

impl From<RGB555> for RGB888 {
    fn from(value: RGB555) -> Self {
        let (r, g, b) = value.to_parts();
        Self::from_parts(r, g, b)
    }
}

impl From<u16> for RGB555 {
    fn from(value: u16) -> Self {
        Self(value)
    }
}

impl From<RGB555> for u16 {
    fn from(value: RGB555) -> Self {
        value.0
    }
}

impl From<RGB888> for RGB555 {
    fn from(value: RGB888) -> Self {
        let (r, g, b) = value.to_parts();
        Self::from_parts(r, g, b)
    }
}
