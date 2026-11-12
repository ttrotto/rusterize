use crate::geo::{edges::LineEdge, raster::RasterInfo};
use fixedbitset::FixedBitSet;

/// Cache pixels when `all_touched` is the burn strategy and [`PixelFunction`] is `Sum` or `Count`.
/// Pass 1 -> burn interior and exterior lines and record visited pixels.
/// Pass 2 -> fill inner values and skip visited from pass 1.
#[derive(Default)]
pub(crate) struct PixelCache {
    bits: FixedBitSet,
    width: usize,
    height: usize,
    xmin: usize,
    ymin: usize,
}

impl PixelCache {
    /// Resize to the bbox of `linedges` and clear all visited pixels.
    pub(crate) fn reset(&mut self, linedges: &[LineEdge], raster_info: &RasterInfo) {
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

        // only pixels inside the raster are ever burned, so never size the bitset past it
        let xmin = xmin.floor().max(0.0) as usize;
        let ymin = ymin.floor().max(0.0) as usize;
        let xmax = (xmax.floor().max(0.0) as usize).min(raster_info.ncols.saturating_sub(1));
        let ymax = (ymax.floor().max(0.0) as usize).min(raster_info.nrows.saturating_sub(1));
        let width = (xmax + 1).saturating_sub(xmin);
        let height = (ymax + 1).saturating_sub(ymin);

        // never shrink, and only zero the blocks this geometry uses (fill is a memset; set_range goes word by word)
        let len = width * height;
        if self.bits.len() < len {
            self.bits.grow(len);
        }
        self.bits.as_mut_slice()[..len.div_ceil(usize::BITS as usize)].fill(0);
        (self.width, self.height, self.xmin, self.ymin) = (width, height, xmin, ymin);
    }

    /// Bit index of a pixel, `None` if it is outside the cached box.
    #[inline]
    fn index(&self, x: usize, y: usize) -> Option<usize> {
        let (local_x, local_y) = (x.wrapping_sub(self.xmin), y.wrapping_sub(self.ymin));
        (local_x < self.width && local_y < self.height).then(|| local_y * self.width + local_x)
    }

    /// Record a pixel. Returns `true` if it was not visited yet (pixels outside the box are never cached).
    pub(crate) fn insert(&mut self, x: usize, y: usize) -> bool {
        self.index(x, y).is_none_or(|idx| !self.bits.put(idx))
    }

    pub(crate) fn contains(&self, x: usize, y: usize) -> bool {
        self.index(x, y).is_some_and(|idx| self.bits.contains(idx))
    }
}
