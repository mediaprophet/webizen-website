//! Day-lit frames of the WR-01 soil mesh and the three eco-intent tufts.
//! The ground colour is `ground::colour_at`. The plants are the cropped
//! species stills, bottom-pinned to the fitted collar.

#[path = "../../native/world-host/src/runtime/ground.rs"]
mod ground;
#[path = "../../native/world-host/src/runtime/understorey.rs"]
mod understorey;

use ground::GroundMaterial;
use understorey::{sway_tuft, Species, TuftSeed, TUFTS};

const W: i32 = 1280;
const H: i32 = 720;

struct Sprite {
    w: i32,
    h: i32,
    rgba: Vec<u8>,
}

struct Cam {
    eye: [f64; 3],
    forward: [f64; 3],
    right: [f64; 3],
    up: [f64; 3],
    fov_y: f64,
}

#[derive(Clone, Copy)]
struct Hit {
    t: f64,
    p: [f64; 3],
    water: bool,
}

fn main() {
    let out = std::path::Path::new(r"C:\github\webizen-website\evidence\world-recovery\WR-01");
    std::fs::create_dir_all(out).unwrap();
    let sprites = Sprites {
        themeda: load_sprite(r"C:\github\webizen-website\production\ground\sprites\themeda.rgba"),
        poa: load_sprite(r"C:\github\webizen-website\production\ground\sprites\poa.rgba"),
        lomandra: load_sprite(r"C:\github\webizen-website\production\ground\sprites\lomandra.rgba"),
    };
    let fitted: Vec<_> = TUFTS.iter().map(understorey::fit_tuft).collect();
    let posed: Vec<_> = fitted.iter().map(|t| sway_tuft(t, 0.85, 5.5)).collect();

    let pad_y = ground::height_at(-22.0, 28.0);
    render(
        &cam_look([-8.0, pad_y + 5.2, 48.0], [-21.0, pad_y + 0.25, 24.0], 56.0_f64.to_radians()),
        &sprites,
        &fitted,
        &posed,
        out.join("camp-day.png").as_path(),
        false,
    );
    let bank = find_bank();
    let eye = [bank[0] - 8.5, ground::height_at(bank[0] - 8.5, bank[2] + 6.0) + 1.55, bank[2] + 6.0];
    let at = [bank[0] + 1.2, bank[1] + 0.15, bank[2] - 1.5];
    println!("creek target {:.2} {:.2} {:.2} mat {}", bank[0], bank[1], bank[2], ground::material_at(bank[0], bank[2]).as_str());
    render(
        &cam_look(eye, at, 48.0_f64.to_radians()),
        &sprites,
        &fitted,
        &posed,
        out.join("creek-bank.png").as_path(),
        false,
    );
    let cluster: Vec<_> = TUFTS.iter().filter(|t| t.foreground).collect();
    let (cx, cz) = cluster.iter().fold((0.0, 0.0), |a, t| (a.0 + t.trunk_x, a.1 + t.trunk_z));
    let n = cluster.len() as f64;
    let cx = cx / n;
    let cz = cz / n;
    let gy = ground::height_at(cx, cz);
    render(
        &cam_look([cx + 0.4, gy + 1.15, cz - 5.6], [cx, gy + 0.42, cz], 46.0_f64.to_radians()),
        &sprites,
        &fitted,
        &posed,
        out.join("understorey-foreground.png").as_path(),
        true,
    );
}

struct Sprites {
    themeda: Sprite,
    poa: Sprite,
    lomandra: Sprite,
}

impl Sprites {
    fn of(&self, species: Species) -> &Sprite {
        match species {
            Species::ThemedaTriandra => &self.themeda,
            Species::PoaLabillardierei => &self.poa,
            Species::LomandraLongifolia => &self.lomandra,
        }
    }
}

fn load_sprite(path: &str) -> Sprite {
    let bytes = std::fs::read(path).unwrap_or_else(|err| panic!("{path}: {err}"));
    let w = u32::from_le_bytes(bytes[0..4].try_into().unwrap()) as i32;
    let h = u32::from_le_bytes(bytes[4..8].try_into().unwrap()) as i32;
    let rgba = bytes[8..].to_vec();
    assert_eq!(rgba.len(), (w * h * 4) as usize, "{path}");
    trim_sprite(Sprite { w, h, rgba })
}

fn trim_sprite(sprite: Sprite) -> Sprite {
    let mut top = 0;
    let mut bot = sprite.h - 1;
    let opaque = |y: i32| {
        (0..sprite.w).any(|x| sprite.rgba[((y * sprite.w + x) * 4 + 3) as usize] > 40)
    };
    while top < bot && !opaque(top) {
        top += 1;
    }
    while bot > top && !opaque(bot) {
        bot -= 1;
    }
    let nh = bot - top + 1;
    let mut rgba = vec![0u8; (sprite.w * nh * 4) as usize];
    for y in 0..nh {
        let src = ((top + y) * sprite.w * 4) as usize;
        let dst = (y * sprite.w * 4) as usize;
        rgba[dst..dst + (sprite.w * 4) as usize].copy_from_slice(&sprite.rgba[src..src + (sprite.w * 4) as usize]);
    }
    Sprite { w: sprite.w, h: nh, rgba }
}

fn cam_look(eye: [f64; 3], at: [f64; 3], fov_y: f64) -> Cam {
    let forward = norm(sub(at, eye));
    let right = norm(cross(forward, [0.0, 1.0, 0.0]));
    let up = norm(cross(right, forward));
    Cam { eye, forward, right, up, fov_y }
}

fn find_bank() -> [f64; 3] {
    let mut best: Option<(f64, [f64; 3])> = None;
    for j in (0..ground::GRID_SIDE).step_by(2) {
        for i in (0..ground::GRID_SIDE).step_by(2) {
            let x = -ground::HALF_SPAN_M + i as f64 * ground::CELL_M;
            let z = -ground::HALF_SPAN_M + j as f64 * ground::CELL_M;
            if !(-25.0..=45.0).contains(&z) || x.abs() > 35.0 {
                continue;
            }
            if ground::material_at(x, z) != GroundMaterial::Pebble {
                continue;
            }
            let y = ground::height_at(x, z);
            if !(0.35..2.8).contains(&y) {
                continue;
            }
            let wet = [(-2.5, 0.0), (2.5, 0.0), (0.0, -2.5), (0.0, 2.5)].iter().any(|(dx, dz)| {
                ground::material_at(x + dx, z + dz) == GroundMaterial::WetBank
            });
            let score = if wet { 8.0 } else { 0.0 } - (z - 16.0).abs() * 0.03 - x.abs() * 0.02;
            if best.map(|(s, _)| score > s).unwrap_or(true) {
                best = Some((score, [x, y, z]));
            }
        }
    }
    best.expect("pebble bank").1
}

fn render(cam: &Cam, sprites: &Sprites, fitted: &[understorey::FittedTuft], posed: &[Vec<understorey::FittedJoint>], path: &std::path::Path, foreground_only: bool) {
    let mut rgb = vec![0u8; (W * H * 3) as usize];
    let mut zbuf = vec![1.0e9f32; (W * H) as usize];
    let mut counts = [0u32; 8];
    let mut water_px = 0u32;
    for y in 0..H {
        for x in 0..W {
            let dir = ray(cam, x, y);
            if let Some(hit) = march(cam, dir) {
                let (col, mat) = shade(hit);
                put(&mut rgb, x, y, col);
                zbuf[(y * W + x) as usize] = hit.t as f32;
                if hit.water {
                    water_px += 1;
                } else {
                    counts[mat as u8 as usize] += 1;
                }
            } else {
                put(&mut rgb, x, y, sky(dir));
            }
        }
    }
    let mut order: Vec<usize> = (0..fitted.len()).collect();
    order.sort_by(|&a, &b| {
        let da = dist2(cam.eye, [fitted[a].seed.trunk_x, 0.0, fitted[a].seed.trunk_z]);
        let db = dist2(cam.eye, [fitted[b].seed.trunk_x, 0.0, fitted[b].seed.trunk_z]);
        db.partial_cmp(&da).unwrap_or(std::cmp::Ordering::Equal)
    });
    for idx in order {
        let seed = &fitted[idx].seed;
        if foreground_only && !seed.foreground {
            continue;
        }
        if !foreground_only && dist2(cam.eye, [seed.trunk_x, 0.0, seed.trunk_z]) > 75.0 * 75.0 {
            continue;
        }
        draw_tuft(cam, sprites, seed, &fitted[idx].joints, &posed[idx], &mut rgb, &mut zbuf);
    }
    write_png(path, &rgb);
    println!(
        "{} silt {} litter {} sand {} granite {} pebble {} wet {} pad {} water {}",
        path.file_name().unwrap().to_string_lossy(),
        counts[1], counts[2], counts[3], counts[4], counts[5], counts[6], counts[7], water_px
    );
}

fn draw_tuft(cam: &Cam, sprites: &Sprites, seed: &TuftSeed, rest: &[understorey::FittedJoint], posed: &[understorey::FittedJoint], rgb: &mut [u8], zbuf: &mut [f32]) {
    let sprite = sprites.of(seed.species);
    let soil = ground::height_at(seed.trunk_x, seed.trunk_z);
    let height = match seed.species {
        Species::ThemedaTriandra => 1.12,
        Species::PoaLabillardierei => 1.32,
        Species::LomandraLongifolia => 0.96,
    };
    let width = height * sprite.w as f64 / sprite.h as f64;
    let mut shear = [0.0, 0.0];
    let mut best = -1.0;
    for (a, b) in rest.iter().zip(posed.iter()) {
        if a.frozen || a.authored[1] < best {
            continue;
        }
        best = a.authored[1];
        shear = [b.world[0] - a.world[0], b.world[2] - a.world[2]];
    }
    let base = [seed.trunk_x, soil, seed.trunk_z];
    let to_eye = {
        let v = sub(cam.eye, base);
        let h = (v[0] * v[0] + v[2] * v[2]).sqrt().max(1e-4);
        [v[0] / h, 0.0, v[2] / h]
    };
    let right = norm(cross([0.0, 1.0, 0.0], to_eye));
    let hw = width * 0.5;
    let top = [base[0] + shear[0], base[1] + height, base[2] + shear[1]];
    let p0 = add(base, mul(right, -hw));
    let p1 = add(base, mul(right, hw));
    let p2 = add(top, mul(right, hw));
    let p3 = add(top, mul(right, -hw));
    if let (Some(a), Some(b), Some(c), Some(d)) = (project(cam, p0), project(cam, p1), project(cam, p2), project(cam, p3)) {
        blit_tri(rgb, zbuf, sprite, a, b, c, [0.0, 1.0], [1.0, 1.0], [1.0, 0.0]);
        blit_tri(rgb, zbuf, sprite, a, c, d, [0.0, 1.0], [1.0, 0.0], [0.0, 0.0]);
    }
}

fn blit_tri(img: &mut [u8], zbuf: &mut [f32], sprite: &Sprite, a: [f64; 3], b: [f64; 3], c: [f64; 3], ua: [f64; 2], ub: [f64; 2], uc: [f64; 2]) {
    let min_x = a[0].min(b[0]).min(c[0]).floor().max(0.0) as i32;
    let max_x = a[0].max(b[0]).max(c[0]).ceil().min(W as f64 - 1.0) as i32;
    let min_y = a[1].min(b[1]).min(c[1]).floor().max(0.0) as i32;
    let max_y = a[1].max(b[1]).max(c[1]).ceil().min(H as f64 - 1.0) as i32;
    let area = edge(a, b, c);
    if area.abs() < 1e-6 {
        return;
    }
    for y in min_y..=max_y {
        for x in min_x..=max_x {
            let p = [x as f64 + 0.5, y as f64 + 0.5, 0.0];
            let w0 = edge(b, c, p) / area;
            let w1 = edge(c, a, p) / area;
            let w2 = edge(a, b, p) / area;
            if w0 < 0.0 || w1 < 0.0 || w2 < 0.0 {
                continue;
            }
            let z = w0 * a[2] + w1 * b[2] + w2 * c[2];
            if z < 0.2 {
                continue;
            }
            let zi = (y * W + x) as usize;
            if z >= zbuf[zi] as f64 - 0.02 {
                continue;
            }
            let u = w0 * ua[0] + w1 * ub[0] + w2 * uc[0];
            let v = w0 * ua[1] + w1 * ub[1] + w2 * uc[1];
            let sx = (u.clamp(0.0, 0.999) * sprite.w as f64) as i32;
            let sy = (v.clamp(0.0, 0.999) * sprite.h as f64) as i32;
            let o = ((sy * sprite.w + sx) * 4) as usize;
            let alpha = sprite.rgba[o + 3];
            if alpha < 90 {
                continue;
            }
            let cover = alpha as f64 / 255.0;
            let dst = zi * 3;
            for k in 0..3 {
                let src = sprite.rgba[o + k] as f64 * 0.94;
                img[dst + k] = (img[dst + k] as f64 * (1.0 - cover) + src * cover).round().clamp(0.0, 255.0) as u8;
            }
            zbuf[zi] = z as f32;
        }
    }
}

fn edge(a: [f64; 3], b: [f64; 3], c: [f64; 3]) -> f64 {
    (c[0] - a[0]) * (b[1] - a[1]) - (c[1] - a[1]) * (b[0] - a[0])
}

fn project(cam: &Cam, p: [f64; 3]) -> Option<[f64; 3]> {
    let rel = sub(p, cam.eye);
    let x = dot(rel, cam.right);
    let y = dot(rel, cam.up);
    let z = dot(rel, cam.forward);
    if z < 0.25 {
        return None;
    }
    let tan = (cam.fov_y * 0.5).tan();
    let ndc_x = x / (z * tan * (W as f64 / H as f64));
    let ndc_y = y / (z * tan);
    Some([(ndc_x * 0.5 + 0.5) * W as f64, (1.0 - (ndc_y * 0.5 + 0.5)) * H as f64, z])
}

fn ray(cam: &Cam, x: i32, y: i32) -> [f64; 3] {
    let ndc_x = ((x as f64 + 0.5) / W as f64) * 2.0 - 1.0;
    let ndc_y = 1.0 - ((y as f64 + 0.5) / H as f64) * 2.0;
    let tan = (cam.fov_y * 0.5).tan();
    let aspect = W as f64 / H as f64;
    norm(add(cam.forward, add(mul(cam.right, ndc_x * tan * aspect), mul(cam.up, ndc_y * tan))))
}

fn march(cam: &Cam, dir: [f64; 3]) -> Option<Hit> {
    let mut t = 0.35;
    let mut prev = t;
    while t < 220.0 {
        let p = add(cam.eye, mul(dir, t));
        if p[0].abs() > 305.0 || p[2].abs() > 305.0 {
            if dir[1] > 0.05 && p[1] > 40.0 {
                return None;
            }
            prev = t;
            t += 3.0;
            continue;
        }
        let h = ground::height_at(p[0], p[2]);
        if p[1] <= h {
            let mut lo = prev;
            let mut hi = t;
            for _ in 0..7 {
                let mid = 0.5 * (lo + hi);
                let q = add(cam.eye, mul(dir, mid));
                if q[1] <= ground::height_at(q[0], q[2]) {
                    hi = mid;
                } else {
                    lo = mid;
                }
            }
            let q = add(cam.eye, mul(dir, hi));
            let bed = ground::height_at(q[0], q[2]);
            if bed < 0.24 && cam.eye[1] > 0.30 && dir[1].abs() > 1e-5 {
                let tw = (0.24 - cam.eye[1]) / dir[1];
                if tw > 0.3 && tw < hi {
                    let s = add(cam.eye, mul(dir, tw));
                    if ground::height_at(s[0], s[2]) < 0.22 {
                        return Some(Hit { t: tw, p: [s[0], 0.24, s[2]], water: true });
                    }
                }
            }
            return Some(Hit { t: hi, p: [q[0], bed, q[2]], water: false });
        }
        let gap = (p[1] - h).max(0.0);
        prev = t;
        t += (gap * 0.5 + 0.22).clamp(0.18, 4.0);
        if p[1] > 70.0 && dir[1] > 0.0 {
            return None;
        }
    }
    None
}

fn shade(hit: Hit) -> ([u8; 3], GroundMaterial) {
    if hit.water {
        let ripple = (hit.p[0] * 0.35 + hit.p[2] * 0.22).sin();
        let col = [
            (28.0 + ripple * 6.0) as u8,
            (90.0 + ripple * 8.0) as u8,
            (104.0 + ripple * 6.0) as u8,
        ];
        return (fog(col, hit.t), GroundMaterial::WetBank);
    }
    let mat = ground::material_at(hit.p[0], hit.p[2]);
    let base = ground::colour_at(hit.p[0], hit.p[2]);
    let grain = hash01(hit.p[0], hit.p[2]);
    let mut rgb = base;
    match mat {
        GroundMaterial::Pebble => {
            let lift = if grain > 0.72 { 26.0 } else if grain < 0.38 { -24.0 } else { (grain - 0.5) * 10.0 };
            rgb = [rgb[0] + lift, rgb[1] + lift, rgb[2] + lift * 0.8];
        }
        GroundMaterial::LeafLitter => {
            if grain < 0.22 {
                rgb = [rgb[0] - 18.0, rgb[1] - 14.0, rgb[2] - 10.0];
            }
        }
        GroundMaterial::Sandstone => {
            rgb = [rgb[0] + (grain - 0.5) * 16.0, rgb[1] + (grain - 0.5) * 12.0, rgb[2] + (grain - 0.5) * 8.0];
        }
        GroundMaterial::Granite => {
            let lift = if grain > 0.8 { 18.0 } else { (grain - 0.5) * 12.0 };
            rgb = [rgb[0] + lift, rgb[1] + lift, rgb[2] + lift];
        }
        GroundMaterial::WetBank => {
            rgb = [rgb[0] + (grain - 0.6) * 10.0, rgb[1] + (grain - 0.5) * 8.0, rgb[2] + (grain - 0.5) * 6.0];
        }
        _ => {
            rgb = [rgb[0] + (grain - 0.5) * 8.0, rgb[1] + (grain - 0.5) * 7.0, rgb[2] + (grain - 0.5) * 5.0];
        }
    }
    let n = ground::normal_at(hit.p[0], hit.p[2]);
    let sun = norm([0.42, 0.84, 0.34]);
    let ndotl = dot(n, sun).max(0.0);
    let light = 0.46 + 0.68 * ndotl;
    let lit = [
        (rgb[0] * light).clamp(0.0, 255.0) as u8,
        (rgb[1] * light).clamp(0.0, 255.0) as u8,
        (rgb[2] * light).clamp(0.0, 255.0) as u8,
    ];
    (fog(lit, hit.t), mat)
}

fn fog(rgb: [u8; 3], t: f64) -> [u8; 3] {
    let f = (1.0 - (-t / 170.0).exp()).clamp(0.0, 0.55);
    let sky = [206.0, 204.0, 190.0];
    [
        (rgb[0] as f64 * (1.0 - f) + sky[0] * f).round() as u8,
        (rgb[1] as f64 * (1.0 - f) + sky[1] * f).round() as u8,
        (rgb[2] as f64 * (1.0 - f) + sky[2] * f).round() as u8,
    ]
}

fn sky(dir: [f64; 3]) -> [u8; 3] {
    let t = ((dir[1] + 0.05) / 0.7).clamp(0.0, 1.0);
    let horizon = [222.0, 214.0, 196.0];
    let zenith = [138.0, 182.0, 214.0];
    [
        (horizon[0] + (zenith[0] - horizon[0]) * t) as u8,
        (horizon[1] + (zenith[1] - horizon[1]) * t) as u8,
        (horizon[2] + (zenith[2] - horizon[2]) * t) as u8,
    ]
}

fn hash01(x: f64, z: f64) -> f64 {
    let n = (x * 12.9898 + z * 78.233).sin() * 43758.5453;
    n - n.floor()
}

fn put(img: &mut [u8], x: i32, y: i32, rgb: [u8; 3]) {
    let o = ((y * W + x) * 3) as usize;
    img[o] = rgb[0];
    img[o + 1] = rgb[1];
    img[o + 2] = rgb[2];
}

fn dist2(a: [f64; 3], b: [f64; 3]) -> f64 {
    let d = sub(a, b);
    d[0] * d[0] + d[1] * d[1] + d[2] * d[2]
}

fn add(a: [f64; 3], b: [f64; 3]) -> [f64; 3] {
    [a[0] + b[0], a[1] + b[1], a[2] + b[2]]
}
fn sub(a: [f64; 3], b: [f64; 3]) -> [f64; 3] {
    [a[0] - b[0], a[1] - b[1], a[2] - b[2]]
}
fn mul(a: [f64; 3], s: f64) -> [f64; 3] {
    [a[0] * s, a[1] * s, a[2] * s]
}
fn dot(a: [f64; 3], b: [f64; 3]) -> f64 {
    a[0] * b[0] + a[1] * b[1] + a[2] * b[2]
}
fn cross(a: [f64; 3], b: [f64; 3]) -> [f64; 3] {
    [a[1] * b[2] - a[2] * b[1], a[2] * b[0] - a[0] * b[2], a[0] * b[1] - a[1] * b[0]]
}
fn norm(v: [f64; 3]) -> [f64; 3] {
    let len = dot(v, v).sqrt().max(1e-12);
    mul(v, 1.0 / len)
}

fn write_png(path: &std::path::Path, rgb: &[u8]) {
    let mut raw = Vec::with_capacity((H as usize) * (1 + W as usize * 3));
    let stride = W as usize * 3;
    for y in 0..H as usize {
        raw.push(0);
        raw.extend_from_slice(&rgb[y * stride..(y + 1) * stride]);
    }
    let compressed = zlib_store(&raw);
    let mut png = Vec::new();
    png.extend_from_slice(b"\x89PNG\r\n\x1a\n");
    let ihdr = {
        let mut d = Vec::new();
        d.extend_from_slice(&(W as u32).to_be_bytes());
        d.extend_from_slice(&(H as u32).to_be_bytes());
        d.extend_from_slice(&[8, 2, 0, 0, 0]);
        d
    };
    chunk(&mut png, b"IHDR", &ihdr);
    chunk(&mut png, b"IDAT", &compressed);
    chunk(&mut png, b"IEND", &[]);
    std::fs::write(path, png).unwrap();
}

fn chunk(out: &mut Vec<u8>, tag: &[u8; 4], data: &[u8]) {
    out.extend_from_slice(&(data.len() as u32).to_be_bytes());
    out.extend_from_slice(tag);
    out.extend_from_slice(data);
    let mut crc_src = tag.to_vec();
    crc_src.extend_from_slice(data);
    out.extend_from_slice(&crc32(&crc_src).to_be_bytes());
}

fn zlib_store(data: &[u8]) -> Vec<u8> {
    let mut out = vec![0x78, 0x01];
    let mut i = 0;
    while i < data.len() {
        let n = (data.len() - i).min(65535);
        let last = i + n == data.len();
        out.push(if last { 1 } else { 0 });
        out.push((n & 255) as u8);
        out.push((n >> 8) as u8);
        let nlen = (!n as u16).to_le_bytes();
        out.extend_from_slice(&nlen);
        out.extend_from_slice(&data[i..i + n]);
        i += n;
    }
    out.extend_from_slice(&adler32(data).to_be_bytes());
    out
}

fn adler32(data: &[u8]) -> u32 {
    let mut a = 1u32;
    let mut b = 0u32;
    for &byte in data {
        a = (a + byte as u32) % 65521;
        b = (b + a) % 65521;
    }
    (b << 16) | a
}

fn crc32(data: &[u8]) -> u32 {
    let mut c = 0xFFFF_FFFFu32;
    for &b in data {
        c ^= b as u32;
        for _ in 0..8 {
            let bit = c & 1;
            c >>= 1;
            if bit == 1 {
                c ^= 0xEDB8_8320;
            }
        }
    }
    !c
}
