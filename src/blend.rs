use std::fmt::{Debug, Display, Formatter};

use crate::kernel::Rotation;

#[derive(Default, Copy, Clone, PartialEq, Eq)]
pub(crate) enum BlendType {
    #[default]
    None = 0,
    Normal,
    Dominant,
}

impl Debug for BlendType {
    fn fmt(&self, f: &mut Formatter<'_>) -> std::fmt::Result {
        write!(
            f,
            "{}",
            match self {
                BlendType::None => ".",
                BlendType::Normal => "N",
                BlendType::Dominant => "D",
            }
        )
    }
}

#[derive(Default, Copy, Clone, PartialEq, Eq)]
pub(crate) struct Blend2x2 {
    // blend_f
    pub top_left: BlendType,
    // blend_g
    pub top_right: BlendType,
    // blend_j
    pub bottom_left: BlendType,
    // blend_k
    pub bottom_right: BlendType,
}

impl Debug for Blend2x2 {
    fn fmt(&self, f: &mut Formatter<'_>) -> std::fmt::Result {
        writeln!(f, "{:?}{:?}", self.top_left, self.top_right)?;
        write!(f, "{:?}{:?}", self.bottom_left, self.bottom_right)
    }
}

impl Blend2x2 {
    pub(crate) fn clear(&mut self) {
        *self = Self::default();
    }

    pub(crate) fn blending_needed(&self) -> bool {
        self != &Self::default()
    }

    pub(crate) fn rotate(self, rotation: Rotation) -> Self {
        match rotation {
            Rotation::None => self,
            Rotation::Clockwise90 => Self {
                top_left: self.bottom_left,
                top_right: self.top_left,
                bottom_left: self.bottom_right,
                bottom_right: self.top_right,
            },
            Rotation::Clockwise180 => Self {
                top_left: self.bottom_right,
                top_right: self.bottom_left,
                bottom_left: self.top_right,
                bottom_right: self.top_left,
            },
            Rotation::Clockwise270 => Self {
                top_left: self.top_right,
                top_right: self.bottom_right,
                bottom_left: self.top_left,
                bottom_right: self.bottom_left,
            },
        }
    }
}
