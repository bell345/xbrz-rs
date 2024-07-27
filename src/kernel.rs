use std::mem;

use crate::blend::{Blend2x2, BlendType};
use crate::config::ScalerConfig;
use crate::oob_reader::OobReader;
use crate::pixel::Argb8;
use crate::ycbcr_lookup::YCbCrLookup;

/// 4x4 kernel with logical positions:
/// ```text
/// -----------------
/// | A | B | C | D |
/// -----------------
/// | E | F | G | H |
/// -----------------
/// | I | J | K | L |
/// -----------------
/// | M | N | O | P |
/// -----------------
/// ```
/// F is the center pixel.
#[repr(C)]
#[derive(Default)]
pub(crate) struct Kernel4x4 {
    pub(crate) a: Argb8,
    pub(crate) b: Argb8,
    pub(crate) c: Argb8,

    pub(crate) e: Argb8,
    pub(crate) f: Argb8,
    pub(crate) g: Argb8,

    pub(crate) i: Argb8,
    pub(crate) j: Argb8,
    pub(crate) k: Argb8,

    pub(crate) m: Argb8,
    pub(crate) n: Argb8,
    pub(crate) o: Argb8,

    pub(crate) d: Argb8,
    pub(crate) h: Argb8,
    pub(crate) l: Argb8,
    pub(crate) p: Argb8,
}

impl Kernel4x4 {
    #[inline]
    pub(crate) fn init_row<'src>(oob: &impl OobReader<'src>) -> Self {
        let mut kernel = Self::default();

        oob.fill_dhlp(&mut kernel, -4);
        kernel.a = kernel.d;
        kernel.e = kernel.h;
        kernel.i = kernel.l;
        kernel.m = kernel.p;

        oob.fill_dhlp(&mut kernel, -3);
        kernel.b = kernel.d;
        kernel.f = kernel.h;
        kernel.j = kernel.l;
        kernel.n = kernel.p;

        oob.fill_dhlp(&mut kernel, -2);
        kernel.c = kernel.d;
        kernel.g = kernel.h;
        kernel.k = kernel.l;
        kernel.o = kernel.p;

        oob.fill_dhlp(&mut kernel, -1);

        kernel
    }

    #[inline]
    pub(crate) fn next_column<'src>(&mut self, oob: &impl OobReader<'src>, x: isize) {
        self.a = self.b;
        self.e = self.f;
        self.i = self.j;
        self.m = self.n;

        self.b = self.c;
        self.f = self.g;
        self.j = self.k;
        self.n = self.o;

        self.c = self.d;
        self.g = self.h;
        self.k = self.l;
        self.o = self.p;

        oob.fill_dhlp(self, x);
    }

    #[inline]
    pub(crate) fn pre_process_corners(&self, cfg: &ScalerConfig) -> Blend2x2 {
        let mut result = Blend2x2::default();
        let ycbcr_lookup = YCbCrLookup::instance();

        if self.f == self.g && self.j == self.k {
            return result;
        }

        if self.f == self.j && self.g == self.k {
            return result;
        }

        macro_rules! dist {
            ($x:ident, $y:ident) => {
                ycbcr_lookup.dist_argb8(self.$x, self.$y)
            };
        }

        let c_bias = cfg.center_direction_bias as f32;
        let dir_thresh = cfg.dominant_direction_threshold as f32;

        let jg = dist!(i, f) + dist!(f, c) + dist!(n, k) + dist!(k, h) + c_bias * dist!(j, g);
        let fk = dist!(e, j) + dist!(j, o) + dist!(b, g) + dist!(g, l) + c_bias * dist!(f, k);

        if jg < fk {
            let blend_mode = if dir_thresh * jg < fk {
                BlendType::Dominant
            } else {
                BlendType::Normal
            };

            if self.f != self.g && self.f != self.j {
                result.top_left = blend_mode;
            }

            if self.k != self.j && self.k != self.g {
                result.bottom_right = blend_mode;
            }
        } else if fk < jg {
            let blend_mode = if dir_thresh * fk < jg {
                BlendType::Dominant
            } else {
                BlendType::Normal
            };

            if self.j != self.f && self.j != self.k {
                result.bottom_left = blend_mode;
            }

            if self.g != self.f && self.g != self.k {
                result.top_right = blend_mode;
            }
        }

        result
    }
}

#[repr(u8)]
#[derive(Debug, Default, Copy, Clone, PartialEq, Eq)]
pub(crate) enum Rotation {
    #[default]
    None,
    Clockwise90,
    Clockwise180,
    Clockwise270,
}

impl Rotation {
    #[inline]
    pub(crate) const fn from_u8(value: u8) -> Self {
        assert!(value <= Rotation::Clockwise270 as u8);
        unsafe { mem::transmute(value) }
    }

    #[inline]
    pub(crate) const fn is_none(self) -> bool {
        matches!(self, Rotation::None)
    }

    /*
    #[inline]
    pub(crate) const fn rotate_cw(self) -> Self {
        match self {
            Rotation::None => Rotation::Clockwise90,
            Rotation::Clockwise90 => Rotation::Clockwise180,
            Rotation::Clockwise180 => Rotation::Clockwise270,
            Rotation::Clockwise270 => Rotation::None,
        }
    }*/

    #[inline]
    pub(crate) const fn rotate_ccw(self) -> Self {
        match self {
            Rotation::None => Rotation::Clockwise270,
            Rotation::Clockwise90 => Rotation::None,
            Rotation::Clockwise180 => Rotation::Clockwise90,
            Rotation::Clockwise270 => Rotation::Clockwise180,
        }
    }
}

pub(crate) struct RotKernel3x3<'ker, const R: u8>(&'ker Kernel4x4);

macro_rules! impl_getter {
    ($x:ident, $rot90:ident, $rot180:ident, $rot270:ident) => {
        #[inline]
        pub(crate) fn $x(&self) -> Argb8 {
            if R == Rotation::None as u8 {
                self.0.$x
            } else if R == Rotation::Clockwise90 as u8 {
                self.0.$rot90
            } else if R == Rotation::Clockwise180 as u8 {
                self.0.$rot180
            } else {
                self.0.$rot270
            }
        }
    };
}

impl<'ker, const R: u8> RotKernel3x3<'ker, R> {
    #[inline]
    pub(crate) fn new(kernel: &'ker Kernel4x4) -> Self {
        assert!(R <= Rotation::Clockwise270 as u8);
        Self(kernel)
    }

    /*impl_getter!(a, g, i, c);*/
    impl_getter!(b, d, h, f);
    impl_getter!(c, a, g, i);
    impl_getter!(d, h, f, b);
    impl_getter!(e, e, e, e);
    impl_getter!(f, b, d, h);
    impl_getter!(g, i, c, a);
    impl_getter!(h, f, b, d);
    impl_getter!(i, c, a, g);
}
