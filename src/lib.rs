//! A high quality image upscaling algorithm designed to preserve key details in low-resolution pixel art.
//!
//! The original version was implemented by C++ by [Zenju](https://sourceforge.net/u/zenju/profile/)
//! and can be found on [SourceForge](https://sourceforge.net/projects/xbrz/).
//!
//! This project is a direct port of xBRZ version 1.8 into Rust.
//!
use std::mem;

use crate::config::ScalerConfig;
use crate::oob_reader::OobReaderTransparent;
use crate::pixel::{Pixel, Rgba8};
use crate::scaler::{Scaler, Scaler2x, Scaler3x, Scaler4x, Scaler5x, Scaler6x};

mod blend;
mod config;
mod kernel;
mod matrix;
mod oob_reader;
mod pixel;
mod scaler;
mod ycbcr_lookup;

/// Use the xBRZ algorithm to scale up an image by an integer factor.
///
/// The `source` is specified as a flat array of pixels, ordered in left to right, then top to bottom order.
/// The subpixels are arranged in RGBA order and each channel is 8 bits, such that each pixel takes up 4 bytes.
///
/// A newly allocated image is returned as a flat RGBA vector, with image dimensions
/// `src_width * factor` by `src_height * factor` and total byte length
/// `src_width * factor * src_height * factor * 4`.
///
/// The `factor` may be one of 1, 2, 3, 4, 5 or 6.
///
/// # Panics
///
/// Panics if the `source` slice length is not exactly equal to `src_width * src_height * 4`,
/// or if `factor` is not one of 1, 2, 3, 4, 5 or 6.
pub fn scale_rgba(source: &[u8], src_width: usize, src_height: usize, factor: usize) -> Vec<u8> {
    scale::<Rgba8>(source, src_width, src_height, factor)
}

fn scale<P: Pixel>(source: &[u8], src_width: usize, src_height: usize, factor: usize) -> Vec<u8> {
    const U8_SIZE: usize = mem::size_of::<u8>();

    if src_width == 0 || src_height == 0 {
        return vec![];
    }

    assert_eq!(source.len(), src_width * src_height * P::SIZE);
    let (_, src_argb, _) = unsafe { source.align_to::<P>() };
    assert_eq!(src_argb.len(), src_width * src_height);

    assert!(factor > 0);
    assert!(factor <= 6);

    let config = ScalerConfig::default();

    let dst_argb = if factor == 1 {
        src_argb.to_owned()
    } else {
        let mut dst_argb = vec![P::default(); src_width * src_height * factor * factor];
        match factor {
            0 => unreachable!(),
            1 => unreachable!(),
            2 => Scaler2x::scale_image::<P, OobReaderTransparent<P>>(
                src_argb,
                dst_argb.as_mut_slice(),
                src_width,
                src_height,
                &config,
                0..src_height,
            ),
            3 => Scaler3x::scale_image::<P, OobReaderTransparent<P>>(
                src_argb,
                dst_argb.as_mut_slice(),
                src_width,
                src_height,
                &config,
                0..src_height,
            ),
            4 => Scaler4x::scale_image::<P, OobReaderTransparent<P>>(
                src_argb,
                dst_argb.as_mut_slice(),
                src_width,
                src_height,
                &config,
                0..src_height,
            ),
            5 => Scaler5x::scale_image::<P, OobReaderTransparent<P>>(
                src_argb,
                dst_argb.as_mut_slice(),
                src_width,
                src_height,
                &config,
                0..src_height,
            ),
            6 => Scaler6x::scale_image::<P, OobReaderTransparent<P>>(
                src_argb,
                dst_argb.as_mut_slice(),
                src_width,
                src_height,
                &config,
                0..src_height,
            ),
            7.. => unreachable!(),
        };
        dst_argb
    };

    unsafe {
        let mut dst_nodrop = mem::ManuallyDrop::new(dst_argb);
        Vec::from_raw_parts(
            dst_nodrop.as_mut_ptr() as *mut u8,
            dst_nodrop.len() * P::SIZE / U8_SIZE,
            dst_nodrop.capacity() * P::SIZE / U8_SIZE,
        )
    }
}

#[cfg(test)]
mod tests {
    use std::mem;

    use crate::pixel::Argb8;

    #[test]
    fn reinterpret_as_argb() {
        let arr = vec![0u8, 1, 2, 3, 4, 5, 6, 7];
        let (p, b, s) = unsafe { arr.align_to::<Argb8>() };
        assert_eq!(p.len(), 0);
        assert_eq!(s.len(), 0);
        assert_eq!(b.len(), 2);
        assert_eq!((1, 2, 3, 0), b[0].to_rgba_parts());
        assert_eq!((5, 6, 7, 4), b[1].to_rgba_parts());
    }

    #[test]
    fn transmute_argb_vec() {
        let original = vec![0u8, 1, 2, 3, 4, 5, 6, 7];
        let new_u8 = {
            let (_, argb_slice, _) = unsafe { original.align_to::<Argb8>() };

            let new_argb = argb_slice.to_owned();
            unsafe {
                let mut argb_nodrop = mem::ManuallyDrop::new(new_argb);
                Vec::from_raw_parts(
                    argb_nodrop.as_mut_ptr() as *mut u8,
                    argb_nodrop.len() * 4,
                    argb_nodrop.capacity() * 4,
                )
            }
        };

        assert_eq!(original, new_u8);
    }
}
