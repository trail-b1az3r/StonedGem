//! Liquid Glass with real refraction, on the CPU: the same model as OyNIx's
//! shader (assets/shaders/liquidglass.frag there). The page frame behind a
//! glass shape is bent inward near the rounded rim like a lens (with a
//! little colour dispersion), frosted, tinted, and lit along the rim from
//! the pointer. The window uses it for glass that floats over the page
//! (menus, the right-click menu, notifications).

/// A glass material.
#[derive(Clone, Copy, Debug)]
pub struct Glass {
    pub radius: f32,
    /// Refraction at the rim, pixels.
    pub bend: f32,
    /// Frost (blur) radius, pixels.
    pub frost: f32,
    /// Straight RGBA 0..1; `[3]` = how much tint.
    pub tint: [f32; 4],
    /// Rim colour; `[3]` = brightness.
    pub rim: [f32; 4],
}

/// The page frame behind the glass: straight RGBA, `w`×`h`.
pub struct Backdrop<'a> {
    pub rgba: &'a [u8],
    pub w: u32,
    pub h: u32,
    /// Colour outside the frame (e.g. the toolbar above the page).
    pub outside: [f32; 3],
}

impl Backdrop<'_> {
    fn at(&self, x: f32, y: f32) -> [f32; 3] {
        let (xi, yi) = (x.floor() as i64, y.floor() as i64);
        if xi < 0 || yi < 0 || xi >= self.w as i64 || yi >= self.h as i64 {
            return self.outside;
        }
        let i = ((yi as u32 * self.w + xi as u32) * 4) as usize;
        [self.rgba[i] as f32 / 255.0, self.rgba[i + 1] as f32 / 255.0, self.rgba[i + 2] as f32 / 255.0]
    }

    fn frosted(&self, x: f32, y: f32, r: f32) -> [f32; 3] {
        let mut c = self.at(x, y).map(|v| v * 0.2);
        for k in 0..8 {
            let a = k as f32 * std::f32::consts::FRAC_PI_4;
            let s = self.at(x + a.cos() * r, y + a.sin() * r);
            for i in 0..3 {
                c[i] += s[i] * 0.1;
            }
        }
        c
    }
}

fn sd_round_rect(px: f32, py: f32, bx: f32, by: f32, r: f32) -> f32 {
    let qx = px.abs() - bx + r;
    let qy = py.abs() - by + r;
    let ox = qx.max(0.0);
    let oy = qy.max(0.0);
    (ox * ox + oy * oy).sqrt() + qx.max(qy).min(0.0) - r
}

/// Render glass of size `w`×`h` whose top-left sits at (`ox`, `oy`) in the
/// backdrop's coordinates. `pointer` is in glass coordinates. Returns
/// straight RGBA (transparent outside the rounded shape).
pub fn refract(bd: &Backdrop, ox: f32, oy: f32, w: u32, h: u32, g: &Glass, pointer: Option<(f32, f32)>) -> Vec<u8> {
    let (hw, hh) = (w as f32 / 2.0, h as f32 / 2.0);
    let r = g.radius.min(hw).min(hh);
    let band = r.clamp(6.0, 22.0);
    let light = match pointer {
        Some((px, py)) => {
            let (dx, dy) = (px - hw, py - hh);
            let l = (dx * dx + dy * dy).sqrt().max(1e-3);
            (dx / l, dy / l)
        }
        None => {
            let l = (0.36f32 + 1.0).sqrt();
            (-0.6 / l, -1.0 / l)
        }
    };
    let mut out = vec![0u8; (w * h * 4) as usize];
    for y in 0..h {
        for x in 0..w {
            let (px, py) = (x as f32 + 0.5 - hw, y as f32 + 0.5 - hh);
            let d = sd_round_rect(px, py, hw, hh, r);
            let inside = (0.5 - d).clamp(0.0, 1.0);
            if inside <= 0.0 {
                continue;
            }
            let nx = sd_round_rect(px + 1.0, py, hw, hh, r) - sd_round_rect(px - 1.0, py, hw, hh, r);
            let ny = sd_round_rect(px, py + 1.0, hw, hh, r) - sd_round_rect(px, py - 1.0, hw, hh, r);
            let nl = (nx * nx + ny * ny).sqrt().max(1e-6);
            let (nx, ny) = (nx / nl, ny / nl);
            let t = (1.0 + d / band).clamp(0.0, 1.0);
            let lens = t * t * t;
            let (dx, dy) = (-nx * lens * g.bend, -ny * lens * g.bend);
            let (sx, sy) = (ox + x as f32 + 0.5, oy + y as f32 + 0.5);
            let mut c = [0.0f32; 3];
            for (ch, k) in [(0usize, 0.92f32), (1, 1.0), (2, 1.08)] {
                c[ch] = bd.frosted(sx + dx * k, sy + dy * k, g.frost)[ch];
            }
            for (ci, ti) in c.iter_mut().zip(g.tint) {
                *ci += (ti - *ci) * g.tint[3];
            }
            let edge = {
                let e = (d + 1.0).abs();
                let t = (1.0 - e / 2.5).clamp(0.0, 1.0);
                t * t * (3.0 - 2.0 * t)
            };
            let facing_raw = (nx * light.0 + ny * light.1).max(0.0);
            let facing = 0.35 + 0.65 * facing_raw;
            for (ci, ri) in c.iter_mut().zip(g.rim) {
                *ci += ri * g.rim[3] * (edge * facing + 0.18 * lens * facing_raw);
            }
            let o = ((y * w + x) * 4) as usize;
            for i in 0..3 {
                out[o + i] = (c[i].clamp(0.0, 1.0) * 255.0).round() as u8;
            }
            out[o + 3] = (inside * 255.0).round() as u8;
        }
    }
    out
}

#[cfg(test)]
mod tests {
    use super::*;

    fn glass() -> Glass {
        Glass { radius: 12.0, bend: 10.0, frost: 0.0, tint: [0.0, 0.0, 0.0, 0.0], rim: [1.0, 1.0, 1.0, 0.0] }
    }

    #[test]
    fn shape_tint_and_lens() {
        // Left half red, right half blue.
        let (w, h) = (100u32, 60u32);
        let mut px = vec![0u8; (w * h * 4) as usize];
        for y in 0..h {
            for x in 0..w {
                let i = ((y * w + x) * 4) as usize;
                if x < 50 {
                    px[i] = 255;
                } else {
                    px[i + 2] = 255;
                }
                px[i + 3] = 255;
            }
        }
        let bd = Backdrop { rgba: &px, w, h, outside: [0.0, 0.0, 0.0] };
        // Glass from x=46: its left rim sits on red, but the lens pulls the
        // colour from further inside (blue) near the rim.
        let out = refract(&bd, 46.0, 10.0, 40, 40, &glass(), None);
        let p = |x: u32, y: u32| {
            let i = ((y * 40 + x) * 4) as usize;
            [out[i], out[i + 1], out[i + 2], out[i + 3]]
        };
        assert_eq!(p(0, 0)[3], 0, "corner is outside the rounded shape");
        assert_eq!(p(20, 20), [0, 0, 255, 255], "centre shows what's behind, unbent");
        assert_eq!(p(1, 20)[2], 255, "near the rim the lens pulls in content from inside (blue, not the red behind)");

        // Tint and a lit rim.
        let g = Glass { tint: [1.0, 1.0, 1.0, 0.5], rim: [1.0, 1.0, 1.0, 1.0], ..glass() };
        let out = refract(&bd, 60.0, 10.0, 30, 30, &g, Some((15.0, -100.0)));
        let c = ((15 * 30 + 15) * 4) as usize;
        assert_eq!([out[c], out[c + 1], out[c + 2]], [128, 128, 255], "half-white tint over blue");
        let top = ((30 + 15) * 4) as usize;
        let bottom = ((28 * 30 + 15) * 4) as usize;
        assert!(out[top] > out[bottom], "the rim facing the pointer (above) is brighter");
    }
}
