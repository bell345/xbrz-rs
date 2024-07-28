use crate::kernel::Rotation;
use crate::pixel::Pixel;

#[inline]
const fn rotate_index(i: usize, j: usize, n: usize, rot: Rotation) -> (usize, usize) {
    if rot.is_none() {
        (i, j)
    } else {
        rotate_index(n - 1 - j, i, n, rot.rotate_ccw())
    }
}

pub(crate) struct OutputMatrix<'out, P: Pixel, const N: usize, const R: u8> {
    inner: &'out mut [P],
    out_width: usize,
}

impl<'out, P: Pixel, const N: usize, const R: u8> OutputMatrix<'out, P, N, R> {
    #[inline]
    pub(crate) fn new(inner: &'out mut [P], out_width: usize) -> Self {
        debug_assert!(R <= Rotation::Clockwise270 as u8);
        Self { inner, out_width }
    }

    #[inline]
    pub(crate) const fn rotated_index<const I: usize, const J: usize>(&self) -> (usize, usize) {
        rotate_index(I, J, N, Rotation::from_u8(R))
    }

    #[inline]
    pub(crate) fn rotated_ref<'a, const I: usize, const J: usize>(&'a mut self) -> &'a mut P
    where
        'out: 'a,
    {
        let (i, j) = self.rotated_index::<I, J>();
        &mut self.inner[j + i * self.out_width]
    }

    pub(crate) fn into_inner(self) -> &'out mut [P] {
        let Self { inner, .. } = self;
        inner
    }
}
