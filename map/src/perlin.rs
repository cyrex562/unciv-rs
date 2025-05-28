use std::f64::consts::PI;

/// Perlin noise generator
///
/// This module provides functions for generating Perlin noise, which is used
/// for creating natural-looking terrain in the game.
///
/// Based on the implementation from https://rosettacode.org/wiki/Perlin_noise#Kotlin
pub struct Perlin;

impl Perlin {
    /// Permutation table for Perlin noise
    const PERMUTATION: [i32; 256] = [
        151, 160, 137,  91,  90,  15, 131,  13, 201,  95,  96,  53, 194, 233,   7, 225,
        140,  36, 103,  30,  69, 142,   8,  99,  37, 240,  21,  10,  23, 190,   6, 148,
        247, 120, 234,  75,   0,  26, 197,  62,  94, 252, 219, 203, 117,  35,  11,  32,
        57 , 177,  33,  88, 237, 149,  56,  87, 174,  20, 125, 136, 171, 168,  68, 175,
        74 , 165,  71, 134, 139,  48,  27, 166,  77, 146, 158, 231,  83, 111, 229, 122,
        60 , 211, 133, 230, 220, 105,  92,  41,  55,  46, 245,  40, 244, 102, 143,  54,
        65 ,  25,  63, 161,   1, 216,  80,  73, 209,  76, 132, 187, 208,  89,  18, 169,
        200, 196, 135, 130, 116, 188, 159,  86, 164, 100, 109, 198, 173, 186,   3,  64,
        52 , 217, 226, 250, 124, 123,   5, 202,  38, 147, 118, 126, 255,  82,  85, 212,
        207, 206,  59, 227,  47,  16,  58,  17, 182, 189,  28,  42, 223, 183, 170, 213,
        119, 248, 152,   2,  44, 154, 163,  70, 221, 153, 101, 155, 167,  43, 172,   9,
        129,  22,  39, 253,  19,  98, 108, 110,  79, 113, 224, 232, 178, 185, 112, 104,
        218, 246,  97, 228, 251,  34, 242, 193, 238, 210, 144,  12, 191, 179, 162, 241,
        81 ,  51, 145, 235, 249,  14, 239, 107,  49, 192, 214,  31, 181, 199, 106, 157,
        184,  84, 204, 176, 115, 121,  50,  45, 127,   4, 150, 254, 138, 236, 205,  93,
        222, 114,  67,  29,  24,  72, 243, 141, 128, 195,  78,  66, 215,  61, 156, 180
    ];

    /// Gradient vectors for 3D Perlin noise
    const GRAD3: [[i32; 3]; 16] = [
        [1, 1, 0], [-1, 1, 0], [1, -1, 0], [-1, -1, 0],
        [1, 0, 1], [-1, 0, 1], [1, 0, -1], [-1, 0, -1],
        [0, 1, 1], [0, -1, 1], [0, 1, -1], [0, -1, -1],
        [1, 0, -1], [-1, 0, -1], [0, -1, 1], [0, 1, 1]
    ];

    /// Extended permutation table
    const P: [i32; 512] = {
        let mut p = [0; 512];
        let mut i = 0;
        while i < 256 {
            p[i] = Self::PERMUTATION[i];
            i += 1;
        }
        while i < 512 {
            p[i] = Self::PERMUTATION[i - 256];
            i += 1;
        }
        p
    };

    /// Generates 3D Perlin noise with multiple octaves
    ///
    /// # Arguments
    ///
    /// * `x` - X coordinate
    /// * `y` - Y coordinate
    /// * `z` - Z coordinate
    /// * `n_octaves` - Number of octaves (default: 3)
    /// * `persistence` - Persistence value (default: 0.5)
    /// * `lacunarity` - Lacunarity value (default: 2.0)
    /// * `scale` - Scale value (default: 10.0)
    ///
    /// # Returns
    ///
    /// A noise value between -1.0 and 1.0
    pub fn noise3d(
        x: f64,
        y: f64,
        z: f64,
        n_octaves: i32,
        persistence: f64,
        lacunarity: f64,
        scale: f64
    ) -> f64 {
        let mut freq = 1.0;
        let mut amp = 1.0;
        let mut max = 0.0;
        let mut total = 0.0;

        for _ in 0..n_octaves {
            total += amp * Self::noise(x * freq / scale, y * freq / scale, z * freq / scale);
            max += amp;
            freq *= lacunarity;
            amp *= persistence;
        }

        total / max
    }

    /// Generates ridged 3D Perlin noise with multiple octaves
    ///
    /// # Arguments
    ///
    /// * `x` - X coordinate
    /// * `y` - Y coordinate
    /// * `z` - Z coordinate
    /// * `n_octaves` - Number of octaves (default: 3)
    /// * `persistence` - Persistence value (default: 0.5)
    /// * `lacunarity` - Lacunarity value (default: 2.0)
    /// * `scale` - Scale value (default: 10.0)
    ///
    /// # Returns
    ///
    /// A noise value between 0.0 and 1.0
    pub fn ridged_noise3d(
        x: f64,
        y: f64,
        z: f64,
        n_octaves: i32,
        persistence: f64,
        lacunarity: f64,
        scale: f64
    ) -> f64 {
        let mut freq = 1.0;
        let mut amp = 1.0;
        let mut max = 0.0;
        let mut total = 0.0;

        for _ in 0..n_octaves {
            let mut value = Self::noise(
                x * freq / scale,
                y * freq / scale,
                z * freq / scale
            );
            value = value.abs();
            total += amp * value;
            max += amp;
            freq *= lacunarity;
            amp *= persistence;
        }

        total / max
    }

    /// Generates 3D Perlin noise
    ///
    /// # Arguments
    ///
    /// * `x` - X coordinate
    /// * `y` - Y coordinate
    /// * `z` - Z coordinate
    ///
    /// # Returns
    ///
    /// A noise value between -1.0 and 1.0
    pub fn noise(x: f64, y: f64, z: f64) -> f64 {
        // Find unit cube that contains point
        let xi = (x.floor() as i32) & 255;
        let yi = (y.floor() as i32) & 255;
        let zi = (z.floor() as i32) & 255;

        // Find relative x, y, z of point in cube
        let xx = x - x.floor();
        let yy = y - y.floor();
        let zz = z - z.floor();

        // Compute fade curves for each of xx, yy, zz
        let u = Self::fade(xx);
        let v = Self::fade(yy);
        let w = Self::fade(zz);

        // Hash co-ordinates of the 8 cube corners
        // and add blended results from 8 corners of cube
        let a = Self::P[xi as usize] + yi;
        let aa = Self::P[a as usize] + zi;
        let ab = Self::P[(a + 1) as usize] + zi;
        let b = Self::P[(xi + 1) as usize] + yi;
        let ba = Self::P[b as usize] + zi;
        let bb = Self::P[(b + 1) as usize] + zi;

        Self::lerp(
            w,
            Self::lerp(
                v,
                Self::lerp(
                    u,
                    Self::grad3(Self::P[aa as usize], xx, yy, zz),
                    Self::grad3(Self::P[ba as usize], xx - 1.0, yy, zz)
                ),
                Self::lerp(
                    u,
                    Self::grad3(Self::P[ab as usize], xx, yy - 1.0, zz),
                    Self::grad3(Self::P[bb as usize], xx - 1.0, yy - 1.0, zz)
                )
            ),
            Self::lerp(
                v,
                Self::lerp(
                    u,
                    Self::grad3(Self::P[(aa + 1) as usize], xx, yy, zz - 1.0),
                    Self::grad3(Self::P[(ba + 1) as usize], xx - 1.0, yy, zz - 1.0)
                ),
                Self::lerp(
                    u,
                    Self::grad3(Self::P[(ab + 1) as usize], xx, yy - 1.0, zz - 1.0),
                    Self::grad3(Self::P[(bb + 1) as usize], xx - 1.0, yy - 1.0, zz - 1.0)
                )
            )
        )
    }

    /// Fade function for Perlin noise
    fn fade(t: f64) -> f64 {
        t * t * t * (t * (t * 6.0 - 15.0) + 10.0)
    }

    /// Linear interpolation function
    fn lerp(t: f64, a: f64, b: f64) -> f64 {
        a + t * (b - a)
    }

    /// Gradient function for 3D Perlin noise
    fn grad3(hash: i32, x: f64, y: f64, z: f64) -> f64 {
        let h = hash & 15;
        x * Self::GRAD3[h as usize][0] as f64 +
        y * Self::GRAD3[h as usize][1] as f64 +
        z * Self::GRAD3[h as usize][2] as f64
    }
}