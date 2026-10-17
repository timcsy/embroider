#![allow(dead_code)]
//! 測試用的合成圖片。

pub type Rgb = [u8; 3];

pub struct Img {
    pub w: usize,
    pub h: usize,
    pub rgba: Vec<u8>,
}

impl Img {
    pub fn filled(w: usize, h: usize, c: Rgb) -> Self {
        let mut rgba = Vec::with_capacity(w * h * 4);
        for _ in 0..w * h {
            rgba.extend_from_slice(&[c[0], c[1], c[2], 255]);
        }
        Img { w, h, rgba }
    }

    pub fn set(&mut self, x: usize, y: usize, c: Rgb, a: u8) {
        let i = (y * self.w + x) * 4;
        self.rgba[i..i + 4].copy_from_slice(&[c[0], c[1], c[2], a]);
    }

    pub fn rect(&mut self, x0: usize, y0: usize, x1: usize, y1: usize, c: Rgb) {
        for y in y0..y1 {
            for x in x0..x1 {
                self.set(x, y, c, 255);
            }
        }
    }

    pub fn disc(&mut self, cx: f64, cy: f64, r: f64, c: Rgb) {
        for y in 0..self.h {
            for x in 0..self.w {
                let d = ((x as f64 + 0.5 - cx).powi(2) + (y as f64 + 0.5 - cy).powi(2)).sqrt();
                if d <= r {
                    self.set(x, y, c, 255);
                }
            }
        }
    }
}

pub const WHITE: Rgb = [255, 255, 255];
pub const RED: Rgb = [200, 30, 40];
pub const BLUE: Rgb = [40, 80, 180];
pub const GREEN: Rgb = [80, 160, 60];
pub const BLACK: Rgb = [20, 20, 20];

/// 白底、紅色方塊、藍色圓、綠色細條的簡單圖案（三色加背景）。
pub fn logo(size: usize) -> Img {
    let mut img = Img::filled(size, size, WHITE);
    let s = size as f64;
    img.rect(size / 10, size / 10, size * 4 / 10, size * 4 / 10, RED);
    img.disc(s * 0.7, s * 0.3, s * 0.15, BLUE);
    img.rect(
        size / 10,
        size * 6 / 10,
        size * 9 / 10,
        size * 8 / 10,
        GREEN,
    );
    img
}

pub fn dist(a: Rgb, b: Rgb) -> f64 {
    ((a[0] as f64 - b[0] as f64).powi(2)
        + (a[1] as f64 - b[1] as f64).powi(2)
        + (a[2] as f64 - b[2] as f64).powi(2))
    .sqrt()
}
