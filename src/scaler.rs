use std::ops::Range;

use crate::blend::{Blend2x2, BlendType};
use crate::config::ScalerConfig;
use crate::kernel::{Kernel4x4, Rotation, RotKernel3x3};
use crate::matrix::OutputMatrix;
use crate::oob_reader::OobReader;
use crate::pixel::Pixel;
use crate::ycbcr_lookup::YCbCrLookup;

fn alpha_grad<P: Pixel, const M: usize, const N: usize>(pix_back: &mut P, pix_front: P) {
    *pix_back = P::gradient::<M, N>(pix_front, *pix_back);
}

fn fill_block<T: Copy>(
    destination: &mut [T],
    row_length: usize,
    value: T,
    block_width: usize,
    block_height: usize,
) {
    let i_range = (0..(block_height * row_length)).step_by(row_length);
    for i in i_range {
        for cell in &mut destination[i..i + block_width] {
            *cell = value;
        }
    }
}

pub(crate) trait Scaler<const SCALE: usize> {
    fn blend_line_shallow<P: Pixel, const R: u8>(col: P, out: &mut OutputMatrix<P, SCALE, R>);
    fn blend_line_steep<P: Pixel, const R: u8>(col: P, out: &mut OutputMatrix<P, SCALE, R>);
    fn blend_line_steep_and_shallow<P: Pixel, const R: u8>(
        col: P,
        out: &mut OutputMatrix<P, SCALE, R>,
    );
    fn blend_line_diagonal<P: Pixel, const R: u8>(col: P, out: &mut OutputMatrix<P, SCALE, R>);
    fn blend_corner<P: Pixel, const R: u8>(col: P, out: &mut OutputMatrix<P, SCALE, R>);

    fn blend_pixel<P: Pixel, const R: u8>(
        kernel: RotKernel3x3<'_, P, R>,
        destination: &mut [P],
        dest_width: usize,
        blend_info: Blend2x2,
        config: &ScalerConfig,
    ) {
        // SAFETY: should be initialised by scale_image()
        debug_assert!(YCbCrLookup::instance_is_initialised());
        let ycbcr = unsafe { YCbCrLookup::instance_unchecked() };
        let blend = blend_info.rotate(Rotation::from_u8(R));

        if blend.bottom_right == BlendType::None {
            return;
        }

        macro_rules! dist {
            ($x:ident, $y:ident) => {
                ycbcr.dist(kernel.$x(), kernel.$y())
            };
        }
        macro_rules! eq {
            ($x:ident, $y:ident) => {
                dist!($x, $y) < config.equal_color_tolerance as f32
            };
        }
        macro_rules! neq {
            ($x:ident, $y:ident) => {
                dist!($x, $y) >= config.equal_color_tolerance as f32
            };
        }

        let do_line_blend = 'a: {
            if blend.bottom_right == BlendType::Dominant {
                break 'a true;
            }

            // make sure there is no second blending in an adjacent rotation for this pixel:
            // handles insular pixels, mario eyes;
            // but support double blending for 90-degree corners
            if blend.top_right != BlendType::None && neq!(e, g) {
                break 'a false;
            }
            if blend.bottom_left != BlendType::None && neq!(e, c) {
                break 'a false;
            }

            // no full blending for L-shapes; blend corner only (handles "mario mushroom eyes")
            if neq!(e, i) && eq!(g, h) && eq!(h, i) && eq!(i, f) && eq!(f, c) {
                break 'a false;
            }

            true
        };

        let px = if dist!(e, f) <= dist!(e, h) {
            kernel.f()
        } else {
            kernel.h()
        };

        let mut out = OutputMatrix::<P, SCALE, R>::new(destination, dest_width);

        if do_line_blend {
            let fg = dist!(f, g);
            let hc = dist!(h, c);

            let shallow_line =
                config.steep_direction_threshold as f32 * fg <= hc && neq!(e, g) && neq!(d, g);
            let steep_line =
                config.steep_direction_threshold as f32 * hc <= fg && neq!(e, c) && neq!(b, c);

            match (shallow_line, steep_line) {
                (true, true) => Self::blend_line_steep_and_shallow(px, &mut out),
                (true, false) => Self::blend_line_shallow(px, &mut out),
                (false, true) => Self::blend_line_steep(px, &mut out),
                (false, false) => Self::blend_line_diagonal(px, &mut out),
            }
        } else {
            Self::blend_corner(px, &mut out);
        }
    }

    fn scale_image<'src, P: Pixel, OOB: OobReader<'src, P>>(
        source: &'src [P],
        destination: &mut [P],
        src_width: usize,
        src_height: usize,
        config: &ScalerConfig,
        y_range: Range<usize>,
    ) {
        let y_first = y_range.start.max(0);
        let y_last = y_range.end.min(src_height);
        assert!(y_first < y_last);
        assert!(src_width > 0);
        assert!(src_height > 0);
        YCbCrLookup::initialise();

        let dest_width = src_width * SCALE;
        let dest_height = src_height * SCALE;
        assert_eq!(destination.len(), dest_width * dest_height);

        let mut pre_proc_buf = vec![Blend2x2::default(); src_width];

        // initialise preprocessing buffer for first row of current stripe:
        // detect upper left and right corner blending
        // this cannot be optimised for adjacent processing stripes; we must not allow for a
        // memory race condition!
        {
            let oob_reader = OOB::new(source, src_width, src_height, y_first as isize - 1);
            let mut kernel = Kernel4x4::init_row(&oob_reader);

            let Blend2x2 { bottom_right, .. } = kernel.pre_process_corners(config);
            pre_proc_buf[0].clear();
            pre_proc_buf[0].top_left = bottom_right;

            for x in 0..src_width {
                kernel.next_column(&oob_reader, x as isize);
                let Blend2x2 {
                    bottom_right,
                    bottom_left,
                    ..
                } = kernel.pre_process_corners(config);
                pre_proc_buf[x].top_right = bottom_left;

                if x + 1 < src_width {
                    pre_proc_buf[x + 1].clear();
                    pre_proc_buf[x + 1].top_left = bottom_right;
                }
            }
        }

        for y in y_first..y_last {
            let row_start = y * SCALE * dest_width;
            let dest_rows = &mut destination[row_start..];

            let oob_reader = OOB::new(source, src_width, src_height, y as isize);
            let mut kernel = Kernel4x4::init_row(&oob_reader);

            // corner blending for current (x, y + 1) position
            let Blend2x2 {
                bottom_right,
                top_right,
                ..
            } = kernel.pre_process_corners(config);
            // set 1st known corner for (0, y + 1) and buffer for use on next column
            let mut blend_xy1 = Blend2x2 {
                top_left: bottom_right,
                ..Default::default()
            };
            // set 3rd known corner for (0, y)
            pre_proc_buf[0].top_left = top_right;

            for x in 0..src_width {
                let out = &mut dest_rows[x * SCALE..];
                kernel.next_column(&oob_reader, x as isize);

                let mut blend_xy = pre_proc_buf[x];
                {
                    let Blend2x2 {
                        top_left,
                        top_right,
                        bottom_left,
                        bottom_right,
                    } = kernel.pre_process_corners(config);

                    // all four corners of (x, y) have been determined at this point
                    blend_xy.bottom_right = top_left;
                    // set 2nd known corner for (x, y + 1)
                    blend_xy1.top_right = bottom_left;
                    pre_proc_buf[x] = blend_xy1;

                    if x + 1 < src_width {
                        blend_xy1.clear();
                        // set 1st known corner for (x + 1, y + 1) and buffer for use on next column
                        blend_xy1.top_left = bottom_right;
                        // set 3rd known corner for (x + 1, y)
                        pre_proc_buf[x + 1].bottom_left = top_right;
                    }
                }

                fill_block(out, dest_width, kernel.f, SCALE, SCALE);

                if blend_xy.blending_needed() {
                    let rot_0 = RotKernel3x3::<P, { Rotation::None as u8 }>::new(&kernel);
                    let rot_90 = RotKernel3x3::<P, { Rotation::Clockwise90 as u8 }>::new(&kernel);
                    let rot_180 = RotKernel3x3::<P, { Rotation::Clockwise180 as u8 }>::new(&kernel);
                    let rot_270 = RotKernel3x3::<P, { Rotation::Clockwise270 as u8 }>::new(&kernel);

                    Self::blend_pixel(rot_0, out, dest_width, blend_xy, config);
                    Self::blend_pixel(rot_90, out, dest_width, blend_xy, config);
                    Self::blend_pixel(rot_180, out, dest_width, blend_xy, config);
                    Self::blend_pixel(rot_270, out, dest_width, blend_xy, config);
                }
            }
        }
    }
}

macro_rules! blend {
    ($m:literal / $n:literal, $out:ident [$x:literal, $y:literal], $col:ident) => {
        alpha_grad::<P, $m, $n>($out.rotated_ref::<$x, $y>(), $col);
    };
}

macro_rules! set {
    ($out:ident [$x:literal, $y:literal], $col:ident) => {
        *$out.rotated_ref::<$x, $y>() = $col;
    };
}

pub(crate) struct Scaler2x;

impl Scaler<2> for Scaler2x {
    fn blend_line_shallow<P: Pixel, const R: u8>(col: P, out: &mut OutputMatrix<P, 2, R>) {
        blend!(1/4, out[1, 0], col);
        blend!(3/4, out[1, 1], col);
    }

    fn blend_line_steep<P: Pixel, const R: u8>(col: P, out: &mut OutputMatrix<P, 2, R>) {
        blend!(1/4, out[0, 1], col);
        blend!(3/4, out[1, 1], col);
    }

    fn blend_line_steep_and_shallow<P: Pixel, const R: u8>(
        col: P,
        out: &mut OutputMatrix<P, 2, R>,
    ) {
        blend!(1/4, out[1, 0], col);
        blend!(1/4, out[0, 1], col);
        blend!(5/6, out[1, 1], col);
    }

    fn blend_line_diagonal<P: Pixel, const R: u8>(col: P, out: &mut OutputMatrix<P, 2, R>) {
        blend!(1/2, out[1, 1], col);
    }

    fn blend_corner<P: Pixel, const R: u8>(col: P, out: &mut OutputMatrix<P, 2, R>) {
        blend!(21/100, out[1, 1], col);
    }
}

pub(crate) struct Scaler3x;

impl Scaler<3> for Scaler3x {
    fn blend_line_shallow<P: Pixel, const R: u8>(col: P, out: &mut OutputMatrix<P, 3, R>) {
        blend!(1/4, out[2, 0], col);
        blend!(1/4, out[1, 2], col);
        blend!(3/4, out[2, 1], col);
        set!(out[2, 2], col);
    }

    fn blend_line_steep<P: Pixel, const R: u8>(col: P, out: &mut OutputMatrix<P, 3, R>) {
        blend!(1/4, out[0, 2], col);
        blend!(1/4, out[2, 1], col);

        blend!(3/4, out[1, 2], col);
        set!(out[2, 2], col);
    }

    fn blend_line_steep_and_shallow<P: Pixel, const R: u8>(
        col: P,
        out: &mut OutputMatrix<P, 3, R>,
    ) {
        blend!(1/4, out[2, 0], col);
        blend!(1/4, out[0, 2], col);

        blend!(3/4, out[2, 1], col);
        blend!(3/4, out[1, 2], col);
        set!(out[2, 2], col);
    }

    fn blend_line_diagonal<P: Pixel, const R: u8>(col: P, out: &mut OutputMatrix<P, 3, R>) {
        blend!(1/8, out[1, 2], col);
        blend!(1/8, out[2, 1], col);
        blend!(7/8, out[2, 2], col);
    }

    fn blend_corner<P: Pixel, const R: u8>(col: P, out: &mut OutputMatrix<P, 3, R>) {
        blend!(45/100, out[2, 2], col);
    }
}

pub(crate) struct Scaler4x;

impl Scaler<4> for Scaler4x {
    fn blend_line_shallow<P: Pixel, const R: u8>(col: P, out: &mut OutputMatrix<P, 4, R>) {
        blend!(1/4, out[3, 0], col);
        blend!(1/4, out[2, 2], col);

        blend!(3/4, out[3, 1], col);
        blend!(3/4, out[2, 3], col);

        set!(out[3, 2], col);
        set!(out[3, 3], col);
    }

    fn blend_line_steep<P: Pixel, const R: u8>(col: P, out: &mut OutputMatrix<P, 4, R>) {
        blend!(1/4, out[0, 3], col);
        blend!(1/4, out[2, 2], col);

        blend!(3/4, out[1, 3], col);
        blend!(3/4, out[3, 2], col);

        set!(out[2, 3], col);
        set!(out[3, 3], col);
    }

    fn blend_line_steep_and_shallow<P: Pixel, const R: u8>(
        col: P,
        out: &mut OutputMatrix<P, 4, R>,
    ) {
        blend!(3/4, out[3, 1], col);
        blend!(3/4, out[1, 3], col);
        blend!(1/4, out[3, 0], col);
        blend!(1/4, out[0, 3], col);

        blend!(1/3, out[2, 2], col);

        set!(out[3, 3], col);
        set!(out[3, 2], col);
        set!(out[2, 3], col);
    }

    fn blend_line_diagonal<P: Pixel, const R: u8>(col: P, out: &mut OutputMatrix<P, 4, R>) {
        blend!(1/2, out[3, 2], col);
        blend!(1/2, out[2, 3], col);
        set!(out[3, 3], col);
    }

    fn blend_corner<P: Pixel, const R: u8>(col: P, out: &mut OutputMatrix<P, 4, R>) {
        blend!(68/100, out[3, 3], col);
        blend!(9/100, out[3, 2], col);
        blend!(9/100, out[2, 3], col);
    }
}

pub(crate) struct Scaler5x;

impl Scaler<5> for Scaler5x {
    fn blend_line_shallow<P: Pixel, const R: u8>(col: P, out: &mut OutputMatrix<P, 5, R>) {
        blend!(1/4, out[4, 0], col);
        blend!(1/4, out[3, 2], col);
        blend!(1/4, out[2, 4], col);

        blend!(3/4, out[4, 1], col);
        blend!(3/4, out[3, 3], col);

        set!(out[4, 2], col);
        set!(out[4, 3], col);
        set!(out[4, 4], col);
        set!(out[3, 4], col);
    }

    fn blend_line_steep<P: Pixel, const R: u8>(col: P, out: &mut OutputMatrix<P, 5, R>) {
        blend!(1/4, out[0, 4], col);
        blend!(1/4, out[2, 3], col);
        blend!(1/4, out[4, 2], col);

        blend!(3/4, out[1, 4], col);
        blend!(3/4, out[3, 3], col);

        set!(out[2, 4], col);
        set!(out[3, 4], col);
        set!(out[4, 4], col);
        set!(out[4, 3], col);
    }

    fn blend_line_steep_and_shallow<P: Pixel, const R: u8>(
        col: P,
        out: &mut OutputMatrix<P, 5, R>,
    ) {
        blend!(1/4, out[0, 4], col);
        blend!(1/4, out[2, 3], col);
        blend!(3/4, out[1, 4], col);

        blend!(1/4, out[4, 0], col);
        blend!(1/4, out[3, 2], col);
        blend!(3/4, out[4, 1], col);

        blend!(2/3, out[3, 3], col);

        set!(out[2, 4], col);
        set!(out[3, 4], col);
        set!(out[4, 4], col);

        set!(out[4, 2], col);
        set!(out[4, 3], col);
    }

    fn blend_line_diagonal<P: Pixel, const R: u8>(col: P, out: &mut OutputMatrix<P, 5, R>) {
        blend!(1/8, out[4, 2], col);
        blend!(1/8, out[3, 3], col);
        blend!(1/8, out[2, 4], col);

        blend!(7/8, out[4, 3], col);
        blend!(7/8, out[3, 4], col);

        set!(out[4, 4], col);
    }

    fn blend_corner<P: Pixel, const R: u8>(col: P, out: &mut OutputMatrix<P, 5, R>) {
        blend!(86/100, out[4, 4], col);
        blend!(23/100, out[4, 3], col);
        blend!(23/100, out[3, 4], col);
    }
}

pub(crate) struct Scaler6x;

impl Scaler<6> for Scaler6x {
    fn blend_line_shallow<P: Pixel, const R: u8>(col: P, out: &mut OutputMatrix<P, 6, R>) {
        blend!(1/4, out[5, 0], col);
        blend!(1/4, out[4, 2], col);
        blend!(1/4, out[3, 4], col);

        blend!(3/4, out[5, 1], col);
        blend!(3/4, out[4, 3], col);
        blend!(3/4, out[3, 5], col);

        set!(out[5, 2], col);
        set!(out[5, 3], col);
        set!(out[5, 4], col);
        set!(out[5, 5], col);

        set!(out[4, 4], col);
        set!(out[4, 5], col);
    }

    fn blend_line_steep<P: Pixel, const R: u8>(col: P, out: &mut OutputMatrix<P, 6, R>) {
        blend!(1/4, out[0, 5], col);
        blend!(1/4, out[2, 4], col);
        blend!(1/4, out[4, 3], col);

        blend!(3/4, out[1, 5], col);
        blend!(3/4, out[3, 4], col);
        blend!(3/4, out[5, 3], col);

        set!(out[2, 5], col);
        set!(out[3, 5], col);
        set!(out[4, 5], col);
        set!(out[5, 5], col);

        set!(out[4, 4], col);
        set!(out[5, 4], col);
    }

    fn blend_line_steep_and_shallow<P: Pixel, const R: u8>(
        col: P,
        out: &mut OutputMatrix<P, 6, R>,
    ) {
        blend!(1/4, out[0, 5], col);
        blend!(1/4, out[2, 4], col);
        blend!(3/4, out[1, 5], col);
        blend!(3/4, out[3, 4], col);

        blend!(1/4, out[5, 0], col);
        blend!(1/4, out[4, 2], col);
        blend!(3/4, out[5, 1], col);
        blend!(3/4, out[4, 3], col);

        set!(out[2, 5], col);
        set!(out[3, 5], col);
        set!(out[4, 5], col);
        set!(out[5, 5], col);

        set!(out[4, 4], col);
        set!(out[5, 4], col);

        set!(out[5, 2], col);
        set!(out[5, 3], col);
    }

    fn blend_line_diagonal<P: Pixel, const R: u8>(col: P, out: &mut OutputMatrix<P, 6, R>) {
        blend!(1/2, out[5, 3], col);
        blend!(1/2, out[4, 4], col);
        blend!(1/2, out[3, 5], col);

        set!(out[4, 5], col);
        set!(out[5, 5], col);
        set!(out[5, 4], col);
    }

    fn blend_corner<P: Pixel, const R: u8>(col: P, out: &mut OutputMatrix<P, 6, R>) {
        blend!(97/100, out[5, 5], col);
        blend!(42/100, out[4, 5], col);
        blend!(42/100, out[5, 4], col);
        blend!(6/100, out[5, 3], col);
        blend!(6/100, out[3, 5], col);
    }
}
