/*
Structure to contain information on raster data.
 */

use std::error::Error;
use ndarray::{Array2, Array3};

pub struct Raster {
    pub xmin: f64,
    pub xmax: f64,
    pub ymin: f64,
    pub ymax: f64,
    pub xres: f64,
    pub yres: f64,
    pub nrows: usize,
    pub ncols: usize,
    pub nlyr: usize
}

impl Raster {
    pub fn new(xmin: f64,
               xmax: f64,
               ymin: f64,
               ymax: f64,
               xres: f64,
               yres: f64,
               nlyr: usize) -> Self {
        let nrows = ((ymax - ymin) / yres) as usize;
        let ncols = ((xmax - xmin) / xres) as usize;
        Self {
            xmin,
            xmax,
            ymin,
            ymax,
            xres,
            yres,
            nrows,
            ncols,
            nlyr
        }
    }
}

// construct 2d array
pub fn build_2d_array(raster: &Raster) -> Result<Array2<f64>, &str> {
    let shape_y = (raster.ymax - raster.ymin).ceil() as usize;
    let shape_x = (raster.xmax - raster.xmin).ceil() as usize;
    Ok(Array2::<f64>::zeros((shape_y, shape_x)))
}

pub fn build_3d_array(raster: &Raster) -> Result<Array3<f64>, &str> {
    let shape_y = (raster.ymax - raster.ymin).ceil() as usize;
    let shape_x = (raster.xmax - raster.xmin).ceil() as usize;
    Ok(Array3::<f64>::zeros((raster.nlyr, shape_y, shape_x)))
}

// // possible ndarrays
// pub enum NDArray {
//     A2(Array2<f64>),
//     A3(Array3<f64>),
// }
//
// impl NDArray {
//     pub fn build_array(raster: &Raster,
//                        dim: &str) -> Result<NDArray, Box<dyn Error>> {
//         // get array dimension
//         let shape_y = (raster.ymax - raster.ymin).ceil() as usize;
//         let shape_x = (raster.xmax - raster.xmin).ceil() as usize;
//         // make
//         match dim {
//             "2" => Ok(NDArray::A2(Array2::<f64>::zeros((shape_y, shape_x)))),
//             "3" => Ok(NDArray::A3(Array3::<f64>::zeros((raster.nlyr, shape_y, shape_x)))),
//             _ => unimplemented!("Only 2 and 3-dimensional raster are supported.")
//         }
//     }
//
//     pub fn as_mut(&mut self) -> Result<&mut dyn std::any::Any, &str> {
//         match self {
//             NDArray::A2(array) => Ok(array),
//             NDArray::A3(array) => Ok(array),
//             _ => Err("Can not mutate array. Only 2d and 3d arrays are supported.")
//         }
//     }
// }