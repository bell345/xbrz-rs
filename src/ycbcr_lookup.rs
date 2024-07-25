use std::sync::OnceLock;

use bytemuck::must_cast;

use crate::pixel::RGB888;

pub(crate) enum YCbCrLookup {
    IDiff555(Box<[f32]>),
    IDiff888(Box<[f32]>)
}

static LOOKUP_INSTANCE: OnceLock<YCbCrLookup> = OnceLock::new();

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
    pub(crate) fn instance() -> &'static Self {
        LOOKUP_INSTANCE.get_or_init(|| {
            #[cfg(feature = "large_lut")]
            {
                Self::new_large()
            }
            #[cfg(not(feature = "large_lut"))]
            {
                Self::new_small()
            }
        })
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

    pub(crate) fn dist_ycbcr(&self, pix1: RGB888, pix2: RGB888) -> f32 {
        let (r1, g1, b1) = pix1.to_parts();
        let (r2, g2, b2) = pix2.to_parts();
        let r_part: u8 = must_cast((((r1 as i16) - (r2 as i16)) / 2) as i8);
        let g_part: u8  = must_cast((((g1 as i16) - (g2 as i16)) / 2) as i8);
        let b_part: u8 = must_cast((((b1 as i16) - (b2 as i16)) / 2) as i8);

        match self {
            YCbCrLookup::IDiff555(lookup) => lookup[(((r_part as usize) >> 3) << 10) | (((g_part as usize) >> 3) << 5) | ((b_part as usize) >> 3)],
            YCbCrLookup::IDiff888(lookup) => lookup[((r_part as usize) << 16) | ((g_part as usize) << 8) | (b_part as usize)]
        }
    }
}

#[cfg(test)]
mod test {
    use crate::pixel::RGB888;
    use crate::ycbcr_lookup::{dist_ycbcr, YCbCrLookup};

    fn test_lut(lut: &YCbCrLookup, rgb1: (u8, u8, u8), rgb2: (u8, u8, u8)) {
        let (r1, g1, b1) = rgb1;
        let (r2, g2, b2) = rgb2;
        let r_diff = (r1 as i16) - (r2 as i16);
        let g_diff = (g1 as i16) - (g2 as i16);
        let b_diff = (b1 as i16) - (b2 as i16);

        let dist = dist_ycbcr(r_diff, g_diff, b_diff) as f32;
        let lut_dist = lut.dist_ycbcr(RGB888::from_parts(r1, g1, b1), RGB888::from_parts(r2, g2, b2));
        assert_eq!(dist, lut_dist)
    }
    
    fn test_whole_lut(lut: &YCbCrLookup) {
        for r1 in (0..=0xFF).step_by(16) {
            for g1 in (0..=0xFF).step_by(16) {
                for b1 in (0..=0xFF).step_by(16)  {
                    for r2 in (0..=0xFF).step_by(16)  {
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
