use std::marker::PhantomData;
use std::ops::Range;
use std::ptr;

use crate::kernel::Kernel4x4;
use crate::pixel::Pixel;

pub(crate) trait OobReader<'src, P: Pixel> {
    fn new(src: &'src [P], width: usize, height: usize, y: isize) -> Self;
    fn fill_dhlp(&self, kernel: &mut Kernel4x4<P>, x: isize);
}

pub(crate) struct OobReaderTransparent<'src, P: Pixel> {
    src_ym1: *const P,
    src_y: *const P,
    src_yp1: *const P,
    src_yp2: *const P,
    x_range: Range<isize>,
    _marker: PhantomData<&'src [P]>,
}

impl<'src, P: Pixel> OobReader<'src, P> for OobReaderTransparent<'src, P> {
    fn new(src: &'src [P], width: usize, height: usize, y: isize) -> Self {
        assert_eq!(src.len(), width * height);
        let src = src.as_ptr();
        let x_range = 0..(width as isize);
        let y_range = 0..(height as isize);
        unsafe {
            Self {
                src_ym1: if y_range.contains(&(y - 1)) {
                    src.offset(width as isize * (y - 1))
                } else {
                    ptr::null()
                },
                src_y: if y_range.contains(&y) {
                    src.offset(width as isize * y)
                } else {
                    ptr::null()
                },
                src_yp1: if y_range.contains(&(y + 1)) {
                    src.offset(width as isize * (y + 1))
                } else {
                    ptr::null()
                },
                src_yp2: if y_range.contains(&(y + 2)) {
                    src.offset(width as isize * (y + 2))
                } else {
                    ptr::null()
                },
                x_range,
                _marker: PhantomData,
            }
        }
    }

    fn fill_dhlp(&self, kernel: &mut Kernel4x4<P>, x: isize) {
        let zero = P::default();
        let x_p2 = x + 2;

        if self.x_range.contains(&x_p2) {
            unsafe {
                kernel.d = if self.src_ym1.is_null() {
                    zero
                } else {
                    *self.src_ym1.offset(x_p2)
                };
                kernel.h = if self.src_y.is_null() {
                    zero
                } else {
                    *self.src_y.offset(x_p2)
                };
                kernel.l = if self.src_yp1.is_null() {
                    zero
                } else {
                    *self.src_yp1.offset(x_p2)
                };
                kernel.p = if self.src_yp2.is_null() {
                    zero
                } else {
                    *self.src_yp2.offset(x_p2)
                };
            }
        } else {
            kernel.d = zero;
            kernel.h = zero;
            kernel.l = zero;
            kernel.p = zero;
        }
    }
}
