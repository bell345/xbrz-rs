/// # xbrz
///
/// This project is a Rust port of the C++ implementation of the xBRZ pixel scaling algorithm
/// authored by Zenju. You can download the original C++ version on
/// [SourceForge](https://sourceforge.net/projects/xbrz/). Both the C++ version and this port are
/// licensed under the [GNU General Public License v3](https://www.gnu.org/licenses/gpl-3.0).
///
use std::mem;

use crate::config::ScalerConfig;
use crate::oob_reader::OobReaderTransparent;
use crate::pixel::Argb8;
use crate::scaler::{Scaler, Scaler2x};

mod blend;
mod config;
mod kernel;
mod matrix;
mod oob_reader;
mod pixel;
mod scaler;
mod ycbcr_lookup;

pub fn scale_argb(source: &[u8], src_width: usize, src_height: usize, factor: usize) -> Vec<u8> {
    const ARGB_SIZE: usize = mem::size_of::<Argb8>();
    const U8_SIZE: usize = mem::size_of::<u8>();

    if src_width == 0 || src_height == 0 {
        return vec![];
    }

    assert_eq!(source.len(), src_width * src_height * ARGB_SIZE);
    let (_, src_argb, _) = unsafe { source.align_to::<Argb8>() };
    assert_eq!(src_argb.len(), src_width * src_height);

    assert!(factor > 0);
    assert!(factor <= 6);

    let config = ScalerConfig::default();

    let dst_argb = if factor == 1 {
        src_argb.to_owned()
    } else {
        let mut dst_argb = vec![Argb8::ZERO; src_width * src_height * factor * factor];
        match factor {
            1 => unreachable!(),
            2 => Scaler2x::scale_image::<OobReaderTransparent>(
                src_argb,
                dst_argb.as_mut_slice(),
                src_width,
                src_height,
                &config,
                0..src_height,
            ),
            3..=6 => todo!(),
            _ => unreachable!(),
        };
        dst_argb
    };

    unsafe {
        let mut dst_nodrop = mem::ManuallyDrop::new(dst_argb);
        Vec::from_raw_parts(
            dst_nodrop.as_mut_ptr() as *mut u8,
            dst_nodrop.len() * ARGB_SIZE / U8_SIZE,
            dst_nodrop.capacity() * ARGB_SIZE / U8_SIZE,
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
