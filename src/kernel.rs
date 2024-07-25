use crate::oob_reader::OobReader;
use crate::pixel::ARGB8888;

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
    pub(crate) a: ARGB8888,
    pub(crate) b: ARGB8888,
    pub(crate) c: ARGB8888,

    pub(crate) e: ARGB8888,
    pub(crate) f: ARGB8888,
    pub(crate) g: ARGB8888,

    pub(crate) i: ARGB8888,
    pub(crate) j: ARGB8888,
    pub(crate) k: ARGB8888,
    
    pub(crate) m: ARGB8888,
    pub(crate) n: ARGB8888,
    pub(crate) o: ARGB8888,
    
    pub(crate) d: ARGB8888,
    pub(crate) h: ARGB8888,
    pub(crate) l: ARGB8888,
    pub(crate) p: ARGB8888
}

#[derive(Default)]
pub(crate) enum BlendType {
    #[default]
    None,
    Normal,
    Dominant
}

#[derive(Default)]
pub(crate) struct BlendResult {
    pub blend_f: BlendType,
    pub blend_g: BlendType,
    pub blend_j: BlendType,
    pub blend_k: BlendType
}

impl Kernel4x4 {
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
    
    pub(crate) fn pre_process_corners(&self) -> BlendResult {
        let mut result = BlendResult::default();
        
        if self.f == self.g && self.j == self.k {
            return result;
        }
        
        if self.f == self.j && self.g == self.k {
            return result;
        }
        
        todo!()
        
        result
    }
}