use bytemuck::must_cast;
use parking_lot::Once;

use crate::pixel::{Argb8, Rgb8};

pub(crate) enum YCbCrLookup {
    IDiff555(Box<[f32]>),
    IDiff888(Box<[f32]>),
}

// SAFETY: Only written to once by the closure in instance(), which is mediated by a parking_lot::Once.
static mut LOOKUP_INSTANCE: Option<YCbCrLookup> = None;
static LOOKUP_LOCK: Once = Once::new();

#[inline]
fn dist_ycbcr(r_diff: i16, g_diff: i16, b_diff: i16) -> f64 {
    let r_diff = r_diff as f64;
    let g_diff = g_diff as f64;
    let b_diff = b_diff as f64;

    // using Rec.2020 RGB -> YCbCr conversion
    const K_B: f64 = 0.0593;
    const K_R: f64 = 0.2627;
    const K_G: f64 = 1.0 - K_B - K_R;

    const SCALE_B: f64 = 0.5 / (1.0 - K_B);
    const SCALE_R: f64 = 0.5 / (1.0 - K_R);

    let y = K_R * r_diff + K_G * g_diff + K_B * b_diff;
    let c_b = SCALE_B * (b_diff - y);
    let c_r = SCALE_R * (r_diff - y);

    (y * y + c_b * c_b + c_r * c_r).sqrt()
}

impl YCbCrLookup {
    #[inline]
    pub(crate) fn instance() -> &'static Self {
        Self::initialise();

        unsafe { Self::instance_unchecked() }
    }

    #[inline]
    pub(crate) fn initialise() {
        LOOKUP_LOCK.call_once(|| unsafe {
            #[cfg(feature = "large_lut")]
            {
                LOOKUP_INSTANCE = Some(Self::new_large());
            }
            #[cfg(not(feature = "large_lut"))]
            {
                LOOKUP_INSTANCE = Some(Self::new_small());
            }
        });
    }

    #[inline]
    pub(crate) unsafe fn instance_unchecked() -> &'static Self {
        unsafe { LOOKUP_INSTANCE.as_ref().unwrap_unchecked() }
    }

    pub(crate) fn instance_is_initialised() -> bool {
        unsafe { LOOKUP_INSTANCE.is_some() }
    }

    pub(crate) fn new_small() -> Self {
        let mut lookup = Vec::with_capacity(0x8000);

        for i in 0..0x8000 {
            let r_diff = must_cast::<_, i8>((((i >> 10) & 0x1F) << 3) as u8) as i16 * 2;
            let g_diff = must_cast::<_, i8>((((i >> 5) & 0x1F) << 3) as u8) as i16 * 2;
            let b_diff = must_cast::<_, i8>(((i & 0x1F) << 3) as u8) as i16 * 2;

            lookup.push(dist_ycbcr(r_diff, g_diff, b_diff) as f32);
        }

        Self::IDiff555(lookup.into_boxed_slice())
    }

    pub(crate) fn new_large() -> Self {
        let mut lookup = Vec::with_capacity(0x100_0000);

        for i in 0..0x100_0000 {
            let r_diff = must_cast::<_, i8>(((i >> 16) & 0xFF) as u8) as i16 * 2;
            let g_diff = must_cast::<_, i8>(((i >> 8) & 0xFF) as u8) as i16 * 2;
            let b_diff = must_cast::<_, i8>((i & 0xFF) as u8) as i16 * 2;

            lookup.push(dist_ycbcr(r_diff, g_diff, b_diff) as f32);
        }

        Self::IDiff888(lookup.into_boxed_slice())
    }

    #[inline]
    pub(crate) fn dist_rgb8(&self, pix1: Rgb8, pix2: Rgb8) -> f32 {
        let (r1, g1, b1) = pix1.to_parts();
        let (r2, g2, b2) = pix2.to_parts();
        let r_part: u8 = must_cast((((r1 as i16) - (r2 as i16)) / 2) as i8);
        let g_part: u8 = must_cast((((g1 as i16) - (g2 as i16)) / 2) as i8);
        let b_part: u8 = must_cast((((b1 as i16) - (b2 as i16)) / 2) as i8);

        match self {
            YCbCrLookup::IDiff555(lookup) => {
                lookup[(((r_part as usize) >> 3) << 10)
                    | (((g_part as usize) >> 3) << 5)
                    | ((b_part as usize) >> 3)]
            }
            YCbCrLookup::IDiff888(lookup) => {
                lookup[((r_part as usize) << 16) | ((g_part as usize) << 8) | (b_part as usize)]
            }
        }
    }

    #[inline]
    pub(crate) fn dist_argb8(&self, pix1: Argb8, pix2: Argb8) -> f32 {
        let a1 = pix1.alpha() as f32 / 255.0;
        let a2 = pix2.alpha() as f32 / 255.0;

        let rgb1 = Rgb8::from(pix1);
        let rgb2 = Rgb8::from(pix2);
        let d = self.dist_rgb8(rgb1, rgb2);
        if a1 < a2 {
            a1 * d + 255.0 * (a2 - a1)
        } else {
            a2 * d + 255.0 * (a1 - a2)
        }
    }
}

#[cfg(test)]
mod test {
    use crate::pixel::Rgb8;
    use crate::ycbcr_lookup::{dist_ycbcr, YCbCrLookup};

    fn test_lut(lut: &YCbCrLookup, rgb1: (u8, u8, u8), rgb2: (u8, u8, u8)) {
        let (r1, g1, b1) = rgb1;
        let (r2, g2, b2) = rgb2;
        let r_diff = (r1 as i16) - (r2 as i16);
        let g_diff = (g1 as i16) - (g2 as i16);
        let b_diff = (b1 as i16) - (b2 as i16);

        let dist = dist_ycbcr(r_diff, g_diff, b_diff) as f32;
        let lut_dist = lut.dist_rgb8(Rgb8::from_parts(r1, g1, b1), Rgb8::from_parts(r2, g2, b2));
        assert_eq!(dist, lut_dist)
    }

    fn test_whole_lut(lut: &YCbCrLookup) {
        for r1 in (0..=0xFF).step_by(16) {
            for g1 in (0..=0xFF).step_by(16) {
                for b1 in (0..=0xFF).step_by(16) {
                    for r2 in (0..=0xFF).step_by(16) {
                        for g2 in (0..=0xFF).step_by(16) {
                            for b2 in (0..=0xFF).step_by(16) {
                                test_lut(lut, (r1, g1, b1), (r2, g2, b2))
                            }
                        }
                    }
                }
            }
        }
    }

    #[test]
    fn test_large_lut() {
        let lookup = YCbCrLookup::new_large();
        test_whole_lut(&lookup);
    }

    #[test]
    fn test_small_lut() {
        let lookup = YCbCrLookup::new_small();
        test_whole_lut(&lookup);
    }
}
