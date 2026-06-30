use crate::geo::edges::LineEdge;
use fixedbitset::FixedBitSet;

/// Cache pixels when `all_touched` is the burn strategy and [`PixelFunction`] is `Sum` or `Count`.
/// Pass 1 -> burn interior and exterior lines and record visited pixels.
/// Pass 2 -> fill inner values and skip visited from pass 1.
pub(crate) struct PixelCache {
    bits: FixedBitSet,
    width: usize,
    xmin: isize,
    ymin: isize,
}

impl PixelCache {
    pub(crate) fn new(linedges: &[LineEdge]) -> Self {
        let (xmin, ymin, xmax, ymax) = linedges.iter().fold(
            (f64::MAX, f64::MAX, f64::MIN, f64::MIN),
            |(xmin, ymin, xmax, ymax), edge| {
                (
                    xmin.min(edge.x0).min(edge.x1),
                    ymin.min(edge.y0).min(edge.y1),
                    xmax.max(edge.x0).max(edge.x1),
                    ymax.max(edge.y0).max(edge.y1),
                )
            },
        );

        let width = (xmax.floor() - xmin.floor()) as usize + 1;
        let length = (ymax.floor() - ymin.floor()) as usize + 1;

        Self {
            bits: FixedBitSet::with_capacity(width * length),
            width,
            xmin: xmin as isize,
            ymin: ymin as isize,
        }
    }

    #[inline]
    fn unravel_index(&self, x: usize, y: usize) -> usize {
        let local_x = (x as isize - self.xmin) as usize;
        let local_y = (y as isize - self.ymin) as usize;
        local_y * self.width + local_x
    }

    pub(crate) fn insert(&mut self, x: usize, y: usize) -> bool {
        let idx = self.unravel_index(x, y);
        if self.bits.contains(idx) {
            return false;
        }
        self.bits.insert(idx);
        true
    }

    pub(crate) fn contains(&self, x: usize, y: usize) -> bool {
        let idx = self.unravel_index(x, y);
        self.bits.contains(idx)
    }
}
