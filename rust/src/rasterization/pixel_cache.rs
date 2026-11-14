use crate::geo::{edges::LineEdge, raster::RasterInfo};

/// Pixels already burned by the current geometry, so [`PixelFunction`](crate::PixelFunction) `Sum` and `Count`
/// add each pixel once per geometry.
/// Lines: every written pixel is recorded and repeats are skipped.
/// Polygons with `all_touched`: pass 1 burns the rings and records their pixels, pass 2 fills the interior
/// and skips the pixels recorded in pass 1.
#[derive(Default)]
pub(crate) struct PixelCache {
    blocks: Vec<u64>,
    burned: Vec<usize>,
    width: usize,
    height: usize,
    xmin: usize,
    ymin: usize,
}

impl PixelCache {
    pub(crate) fn reset(&mut self, linedges: &[LineEdge], raster_info: &RasterInfo) {
        // clear visited blocks
        for &block in &self.burned {
            self.blocks[block] = 0;
        }
        self.burned.clear();

        // resize to the bbox of `linedges` and at most the raster bounding box.
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

        let xmin = xmin.floor().max(0.0) as usize;
        let ymin = ymin.floor().max(0.0) as usize;
        let xmax = (xmax.floor().max(0.0) as usize).min(raster_info.ncols.saturating_sub(1));
        let ymax = (ymax.floor().max(0.0) as usize).min(raster_info.nrows.saturating_sub(1));
        let width = (xmax + 1).saturating_sub(xmin);
        let height = (ymax + 1).saturating_sub(ymin);

        // reset blocks size if needed
        let n_blocks = (width * height).div_ceil(64);
        if self.blocks.len() < n_blocks {
            self.blocks = vec![0_u64; n_blocks];
        }

        (self.width, self.height, self.xmin, self.ymin) = (width, height, xmin, ymin);
    }

    /// Bit index of a pixel, `None` if it is outside the cached box.
    #[inline]
    fn index(&self, x: usize, y: usize) -> Option<usize> {
        let (local_x, local_y) = (x.wrapping_sub(self.xmin), y.wrapping_sub(self.ymin));
        (local_x < self.width && local_y < self.height).then(|| local_y * self.width + local_x)
    }

    /// Record a pixel. Returns `true` if it was not visited.
    #[inline]
    pub(crate) fn insert(&mut self, x: usize, y: usize) -> bool {
        let Some(idx) = self.index(x, y) else {
            return true;
        };

        // get the burned bit
        let (block_idx, bit) = (idx / 64, 1u64 << (idx % 64));
        let block = &mut self.blocks[block_idx];

        // if already burned do not insert
        if *block & bit != 0 {
            return false;
        }

        // first bit to burn, remember to clean it later
        if *block == 0 {
            self.burned.push(block_idx);
        }

        // else record the burn
        *block |= bit;
        true
    }

    #[inline]
    pub(crate) fn contains(&self, x: usize, y: usize) -> bool {
        self.index(x, y)
            .is_some_and(|idx| self.blocks[idx / 64] & (1 << (idx % 64)) != 0)
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn raster(n: usize) -> RasterInfo {
        RasterInfo {
            ncols: n,
            nrows: n,
            xmin: 0.0,
            xmax: n as f64,
            ymin: 0.0,
            ymax: n as f64,
            xres: 1.0,
            yres: 1.0,
            epsg: None,
        }
    }

    // pixel-space edge spanning (x0, y0) -> (x1, y1)
    fn edge(x0: f64, y0: f64, x1: f64, y1: f64) -> LineEdge {
        LineEdge { x0, y0, x1, y1 }
    }

    #[test]
    fn dedups_within_a_geometry_and_clears_between() {
        let ri = raster(200);
        let mut cache = PixelCache::default();

        cache.reset(&[edge(0.5, 0.5, 150.5, 150.5)], &ri);
        assert!(cache.insert(3, 3));
        assert!(cache.insert(140, 140));
        assert!(!cache.insert(3, 3), "second visit must be rejected");
        assert!(cache.contains(140, 140));
        assert!(cache.insert(199, 199), "outside the bbox is never cached");
        assert!(!cache.contains(199, 199));

        cache.reset(&[edge(0.5, 0.5, 150.5, 150.5)], &ri);
        assert!(
            !cache.contains(3, 3) && !cache.contains(140, 140),
            "reset must clear burned blocks"
        );
        assert!(cache.insert(3, 3));

        cache.reset(&[edge(2.5, 2.5, 5.5, 5.5)], &ri);
        assert!((2..=5).all(|y| (2..=5).all(|x| !cache.contains(x, y))));
    }
}
