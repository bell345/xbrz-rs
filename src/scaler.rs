use std::ops::Range;
use crate::config::ScalerConfig;
use crate::kernel::Kernel4x4;
use crate::oob_reader::OobReader;
use crate::pixel::ARGB8888;

pub(crate) trait Scaler {
    const SCALE: usize;

    fn scale_image<'src, OOB: OobReader<'src>>(
        &self,
        source: &'src [ARGB8888],
        destination: &mut [ARGB8888],
        src_width: isize,
        src_height: isize,
        config: &ScalerConfig,
        y_range: Range<isize>,
    ) {
        let y_first = y_range.start.max(0);
        let y_last = y_range.end.min(src_height);
        assert!(y_first < y_last);
        assert!(src_width > 0);
        assert!(src_height > 0);

        let dest_width = src_width * Self::SCALE as isize;
        let pre_proc_buf = vec![0u8; src_width as usize];
        
        {
            let oob_reader = OOB::new(source, src_width, src_height, y_first - 1);
            let mut kernel = Kernel4x4::init_row(&oob_reader);
            
            
        }
    }
}

pub(crate) struct Scaler2x;

impl Scaler for Scaler2x {
    const SCALE: usize = 2;
}