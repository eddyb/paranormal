use hsl::HSL;
use image::{Rgb, RgbImage};
use std::f64::consts::PI;
use std::ops::Index;

pub struct Grid<T> {
    width: usize,
    height: usize,
    data: Vec<T>,
}

impl From<RgbImage> for Grid<Rgb<u8>> {
    fn from(img: RgbImage) -> Self {
        Grid {
            width: img.width() as usize,
            height: img.height() as usize,
            // FIXME(eddyb) reuse allocation?
            data: img.pixels().copied().collect(),
        }
    }
}

impl Into<RgbImage> for Grid<Rgb<u8>> {
    fn into(self) -> RgbImage {
        RgbImage::from_fn(self.width as u32, self.height as u32, |x, y| {
            self[(x as usize, y as usize)]
        })
    }
}

impl<T> Index<(usize, usize)> for Grid<T> {
    type Output = T;

    fn index(&self, (x, y): (usize, usize)) -> &T {
        &self.data[y.min(self.height - 1) * self.width + x.min(self.width - 1)]
    }
}

pub struct View<'a, T> {
    grid: &'a Grid<T>,
    x: usize,
    y: usize,
}

impl<T> Index<(isize, isize)> for View<'_, T> {
    type Output = T;

    fn index(&self, (dx, dy): (isize, isize)) -> &T {
        let abs = |base: usize, offset: isize| {
            if offset < 0 {
                base.saturating_sub(-offset as usize)
            } else {
                base + (offset as usize)
            }
        };

        &self.grid[(abs(self.x, dx), abs(self.y, dy))]
    }
}

impl<T> Grid<T> {
    pub fn map<U>(&self, f: impl FnMut(View<'_, T>) -> U) -> Grid<U> {
        let mut data = Vec::with_capacity(self.width * self.height);

        data.extend(
            (0..self.height)
                .flat_map(|y| (0..self.width).map(move |x| View { grid: self, x, y }))
                .map(f),
        );

        Grid {
            width: self.width,
            height: self.height,
            data,
        }
    }
}

pub fn process(img: RgbImage) -> Vec<RgbImage> {
    let color = Grid::from(img);
    let color_maxabs = u8::max_value() as f64;

    let grayscale = color.map(|v| {
        let Rgb([r, g, b]) = v[(0, 0)];
        r as i16 + g as i16 + b as i16
    });
    let grayscale_maxabs = color_maxabs * 3.0;

    let sobel = grayscale.map(|v| {
        (
            -(v[(-1, -1)] + 2 * v[(-1, 0)] + v[(-1, 1)]) + (v[(1, -1)] + 2 * v[(1, 0)] + v[(1, 1)]),
            -(v[(-1, -1)] + 2 * v[(0, -1)] + v[(1, -1)]) + (v[(-1, 1)] + 2 * v[(0, 1)] + v[(1, 1)]),
        )
    });
    let sobel_maxabs_mag = grayscale_maxabs * 3.0;

    let mut sobel_angle_mag = sobel.map(|v| {
        let (gx, gy) = v[(0, 0)];
        let (gx, gy) = (gx as i32, gy as i32);
        (
            (gy as f64).atan2(gx as f64),
            ((gx * gx + gy * gy) as f64).sqrt() / sobel_maxabs_mag,
        )
    });

    let mut frames = vec![];

    // Angles apart by `ANGLE_CLASS` get the same color.
    // Common values: 360, 180 (1D barcodes), 90 (2D barcodes).
    const ANGLE_CLASS: f64 = 180.0;

    loop {
        frames.push(
            sobel_angle_mag
                .map(|v| {
                    let (angle, mag) = v[(0, 0)];
                    let hsl = HSL {
                        h: (((2.0 + angle / PI) * (360.0 / ANGLE_CLASS)) % 2.0) * 180.0,
                        s: 1.0,
                        l: mag / 2.0,
                    };
                    let (r, g, b) = hsl.to_rgb();
                    Rgb([r, g, b])
                })
                .into(),
        );

        let mut changed = false;
        // FIXME(eddyb) do this in-place.
        sobel_angle_mag = sobel_angle_mag.map(|v| {
            let best = (-1..=1)
                .flat_map(|x| (-1..=1).map(move |y| (x, y)))
                .map(|x_y| v[x_y])
                .max_by(|(_, a), (_, b)| a.partial_cmp(b).unwrap());

            if let Some((angle, mag)) = best {
                let r = (angle, mag * 0.9);
                if r.1 > v[(0, 0)].1 {
                    changed |= true;
                    return r;
                }
            }

            v[(0, 0)]
        });
        if !changed {
            break;
        }
    }

    frames
}
