use std::fmt::{Debug, Formatter};
use std::mem;

#[repr(C)]
#[derive(Default, Copy, Clone, PartialEq, Eq)]
pub(crate) struct RGB555(u16);

#[repr(C)]
#[derive(Default, Copy, Clone, PartialEq, Eq)]
pub(crate) struct Rgb8([u8; 4]);

#[repr(C)]
#[derive(Default, Copy, Clone, PartialEq, Eq)]
pub(crate) struct Argb8([u8; 4]);

pub(crate) const fn u5_to_u8(v: u8) -> u8 {
    (v << 3) | (v >> 2)
}

impl RGB555 {
    pub(crate) const fn from_parts(r: u8, g: u8, b: u8) -> Self {
        Self(
            (((r as u16) << 7) & 0x7C00)
                | (((g as u16) << 2) & 0x03E0)
                | (((b as u16) >> 3) & 0x001F),
        )
    }

    pub(crate) const fn to_parts(self) -> (u8, u8, u8) {
        (
            u5_to_u8(((self.0 >> 10) & 0x1F) as u8),
            u5_to_u8(((self.0 >> 5) & 0x1F) as u8),
            u5_to_u8((self.0 & 0x1F) as u8),
        )
    }
}

impl Debug for RGB555 {
    fn fmt(&self, f: &mut Formatter<'_>) -> std::fmt::Result {
        let (r, g, b) = self.to_parts();
        f.debug_struct("RGB555")
            .field("repr", &self.0)
            .field("r", &r)
            .field("g", &g)
            .field("b", &b)
            .finish()
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

impl From<Rgb8> for RGB555 {
    fn from(value: Rgb8) -> Self {
        let (r, g, b) = value.to_parts();
        Self::from_parts(r, g, b)
    }
}

impl Rgb8 {
    pub(crate) const fn from_parts(r: u8, g: u8, b: u8) -> Self {
        Self([0, r, g, b])
    }

    pub(crate) const fn to_parts(self) -> (u8, u8, u8) {
        (self.0[1], self.0[2], self.0[3])
    }
}

impl Debug for Rgb8 {
    fn fmt(&self, f: &mut Formatter<'_>) -> std::fmt::Result {
        let (r, g, b) = self.to_parts();
        f.debug_struct("RGB888")
            .field("repr", &self.0)
            .field("r", &r)
            .field("g", &g)
            .field("b", &b)
            .finish()
    }
}

impl From<RGB555> for Rgb8 {
    fn from(value: RGB555) -> Self {
        let (r, g, b) = value.to_parts();
        Self::from_parts(r, g, b)
    }
}

impl From<Argb8> for Rgb8 {
    fn from(value: Argb8) -> Self {
        // SAFETY: The internal representation of RGB888 and ARGB8888 are compatible
        // as they use the same memory layout. Note that the "A" value exists in a "don't care"
        // portion of the RGB888 backing array.
        unsafe { mem::transmute(value) }
    }
}

impl Argb8 {
    pub(crate) const ZERO: Argb8 = Argb8([0, 0, 0, 0]);

    #[inline(always)]
    pub(crate) const fn red(self) -> u8 {
        self.0[1]
    }
    #[inline(always)]
    pub(crate) const fn green(self) -> u8 {
        self.0[2]
    }
    #[inline(always)]
    pub(crate) const fn blue(self) -> u8 {
        self.0[3]
    }
    #[inline(always)]
    pub(crate) const fn alpha(self) -> u8 {
        self.0[0]
    }

    pub(crate) const fn from_rgba_parts(r: u8, g: u8, b: u8, a: u8) -> Self {
        Self([a, r, g, b])
    }

    pub(crate) const fn to_rgba_parts(self) -> (u8, u8, u8, u8) {
        (self.0[1], self.0[2], self.0[3], self.0[0])
    }

    pub(crate) fn gradient<const M: usize, const N: usize>(front: Self, back: Self) -> Self {
        debug_assert!(0 < M && M < N && N <= 1000);

        let weight_front = front.alpha() as usize * M;
        let weight_back = back.alpha() as usize * (N - M);
        let weight_sum = weight_front + weight_back;

        if weight_sum == 0 {
            return Self::ZERO;
        }

        macro_rules! calc_color {
            ($x:ident) => {
                (front.$x() as usize * weight_front + back.$x() as usize * weight_back) / weight_sum
            };
        }

        Self::from_rgba_parts(
            calc_color!(red) as u8,
            calc_color!(green) as u8,
            calc_color!(blue) as u8,
            (weight_sum / N) as u8,
        )
    }
}

impl Debug for Argb8 {
    fn fmt(&self, f: &mut Formatter<'_>) -> std::fmt::Result {
        let (r, g, b, a) = self.to_rgba_parts();
        f.debug_struct("RGB888")
            .field("repr", &self.0)
            .field("r", &r)
            .field("g", &g)
            .field("b", &b)
            .field("a", &a)
            .finish()
    }
}
