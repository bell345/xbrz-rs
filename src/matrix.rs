use crate::kernel::Rotation;
use crate::pixel::Argb8;

#[inline]
const fn rotate_index(i: usize, j: usize, n: usize, rot: Rotation) -> (usize, usize) {
    if rot.is_none() {
        (i, j)
    } else {
        rotate_index(n - 1 - j, i, n, rot.rotate_ccw())
    }
}

pub(crate) struct OutputMatrix<'out, const N: usize, const R: u8> {
    inner: &'out mut [Argb8],
    out_width: usize,
}

impl<'out, const N: usize, const R: u8> OutputMatrix<'out, N, R> {
    #[inline]
    pub(crate) fn new(inner: &'out mut [Argb8], out_width: usize) -> Self {
        debug_assert!(R <= Rotation::Clockwise270 as u8);
        Self { inner, out_width }
    }

    #[inline]
    pub(crate) const fn rotated_index<const I: usize, const J: usize>(&self) -> (usize, usize) {
        rotate_index(I, J, N, Rotation::from_u8(R))
    }

    #[inline]
    pub(crate) fn rotated_ref<'a, const I: usize, const J: usize>(&'a mut self) -> &'a mut Argb8
    where
        'out: 'a,
    {
        let (i, j) = self.rotated_index::<I, J>();
        &mut self.inner[i + j * self.out_width]
    }

    pub(crate) fn into_inner(self) -> &'out mut [Argb8] {
        let Self { inner, .. } = self;
        inner
    }
}

#[cfg(test)]
mod test {
    use crate::kernel::Rotation;
    use crate::matrix::OutputMatrix;
    use crate::pixel::Argb8;

    const TEST_WIDTH: usize = 4;

    const ROT_0: u8 = Rotation::None as u8;
    const ROT_90: u8 = Rotation::Clockwise90 as u8;
    const ROT_180: u8 = Rotation::Clockwise180 as u8;
    const ROT_270: u8 = Rotation::Clockwise270 as u8;

    fn init_buf(it: impl IntoIterator<Item = (u8, u8, u8, u8)>) -> Vec<Argb8> {
        let mut v = vec![];
        for (r, g, b, a) in it {
            v.push(Argb8::from_rgba_parts(r, g, b, a));
        }
        v
    }

    fn init_test_buf() -> Vec<Argb8> {
        init_buf([
            (0, 1, 2, 3),
            (4, 5, 6, 7),
            (8, 9, 10, 11),
            (12, 13, 14, 15),
            (16, 17, 18, 19),
            (20, 21, 22, 23),
            (24, 25, 26, 27),
            (28, 29, 30, 31),
            (32, 33, 34, 35),
            (36, 37, 38, 39),
            (40, 41, 42, 43),
            (44, 45, 46, 47),
            (48, 49, 50, 51),
            (52, 53, 54, 55),
            (56, 57, 58, 59),
            (60, 61, 62, 63),
        ])
    }

    fn test_once<const N: usize, const R: u8, const I: usize, const J: usize>(
        mat: &OutputMatrix<N, R>,
        expected: (usize, usize),
    ) {
        let (i, j) = mat.rotated_index::<I, J>();
        assert_eq!((i, j), expected);
    }

    #[test]
    fn test_3_wide() {
        let mut v = init_test_buf();
        let mat0 = OutputMatrix::<3, { Rotation::None as u8 }>::new(v.as_mut_slice(), TEST_WIDTH);

        test_once::<3, ROT_0, 0, 0>(&mat0, (0, 0));
        test_once::<3, ROT_0, 2, 0>(&mat0, (2, 0));
        test_once::<3, ROT_0, 0, 1>(&mat0, (0, 1));
        test_once::<3, ROT_0, 2, 2>(&mat0, (2, 2));

        let v_mut_slice = mat0.into_inner();
        let mat90 =
            OutputMatrix::<3, { Rotation::Clockwise90 as u8 }>::new(v_mut_slice, TEST_WIDTH);

        test_once::<3, ROT_90, 0, 0>(&mat90, (2, 0));
        test_once::<3, ROT_90, 2, 0>(&mat90, (2, 2));
        test_once::<3, ROT_90, 0, 1>(&mat90, (1, 0));
        test_once::<3, ROT_90, 2, 2>(&mat90, (0, 2));

        let v_mut_slice = mat90.into_inner();
        let mat180 =
            OutputMatrix::<3, { Rotation::Clockwise180 as u8 }>::new(v_mut_slice, TEST_WIDTH);

        test_once::<3, ROT_180, 0, 0>(&mat180, (2, 2));
        test_once::<3, ROT_180, 2, 0>(&mat180, (0, 2));
        test_once::<3, ROT_180, 0, 1>(&mat180, (2, 1));
        test_once::<3, ROT_180, 2, 2>(&mat180, (0, 0));

        let v_mut_slice = mat180.into_inner();
        let mat270 =
            OutputMatrix::<3, { Rotation::Clockwise270 as u8 }>::new(v_mut_slice, TEST_WIDTH);

        test_once::<3, ROT_270, 0, 0>(&mat270, (0, 2));
        test_once::<3, ROT_270, 2, 0>(&mat270, (0, 0));
        test_once::<3, ROT_270, 0, 1>(&mat270, (1, 2));
        test_once::<3, ROT_270, 2, 2>(&mat270, (2, 0));
    }

    #[test]
    fn test_4_wide() {
        let mut v = init_test_buf();
        let mat0 = OutputMatrix::<4, { Rotation::None as u8 }>::new(v.as_mut_slice(), TEST_WIDTH);

        test_once::<4, ROT_0, 0, 0>(&mat0, (0, 0));
        test_once::<4, ROT_0, 2, 0>(&mat0, (2, 0));
        test_once::<4, ROT_0, 0, 1>(&mat0, (0, 1));
        test_once::<4, ROT_0, 2, 2>(&mat0, (2, 2));

        let v_mut_slice = mat0.into_inner();
        let mat90 =
            OutputMatrix::<4, { Rotation::Clockwise90 as u8 }>::new(v_mut_slice, TEST_WIDTH);

        test_once::<4, ROT_90, 0, 0>(&mat90, (3, 0));
        test_once::<4, ROT_90, 2, 0>(&mat90, (3, 2));
        test_once::<4, ROT_90, 0, 1>(&mat90, (2, 0));
        test_once::<4, ROT_90, 2, 2>(&mat90, (1, 2));

        let v_mut_slice = mat90.into_inner();
        let mat180 =
            OutputMatrix::<4, { Rotation::Clockwise180 as u8 }>::new(v_mut_slice, TEST_WIDTH);

        test_once::<4, ROT_180, 0, 0>(&mat180, (3, 3));
        test_once::<4, ROT_180, 2, 0>(&mat180, (1, 3));
        test_once::<4, ROT_180, 0, 1>(&mat180, (3, 2));
        test_once::<4, ROT_180, 2, 2>(&mat180, (1, 1));

        let v_mut_slice = mat180.into_inner();
        let mat270 =
            OutputMatrix::<4, { Rotation::Clockwise270 as u8 }>::new(v_mut_slice, TEST_WIDTH);

        test_once::<4, ROT_270, 0, 0>(&mat270, (0, 3));
        test_once::<4, ROT_270, 2, 0>(&mat270, (0, 1));
        test_once::<4, ROT_270, 0, 1>(&mat270, (1, 3));
        test_once::<4, ROT_270, 2, 2>(&mat270, (2, 1));
    }
}
