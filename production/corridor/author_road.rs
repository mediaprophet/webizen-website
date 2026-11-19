//! Build the WR-03 road mesh on ground::height_at and draw the camp frame.
//! The picture is this mesh draped on the soil, not a wasm capture.

#[path = "../../native/world-host/src/runtime/ground.rs"]
mod ground;
#[path = "mesh.rs"]
mod mesh;

use ground::GroundMaterial;
use mesh::RoadSurface;

const W: i32 = 1280;
const H: i32 = 720;

fn main() {
    let built = mesh::author_mesh(|x, z| ground::height_at(x, z));
    check(&built);
    let root = std::path::Path::new(r"C:\github\webizen-website\production\corridor");
    let bin_path = root.join("forestry-road.bin");
    std::fs::write(&bin_path, &built.bytes).unwrap();
    write_json(root, &built);
    let evidence = std::path::Path::new(r"C:\github\webizen-website\evidence\world-recovery\WR-03");
    std::fs::create_dir_all(evidence).unwrap();
    let bytes = std::fs::read(&bin_path).unwrap();
    assert_eq!(bytes, built.bytes);

    let mouth = built
        .samples
        .iter()
        .find(|s| s.chainage_m > 6.0)
        .expect("mouth");
    let ahead = built
        .samples
        .iter()
        .find(|s| s.chainage_m > 48.0)
        .expect("ahead");
    let eye_x = mouth.x as f64 - 7.5;
    let eye_z = mouth.z as f64 - 16.0;
    let eye = [
        eye_x,
        ground::height_at(eye_x, eye_z) + 2.7,
        eye_z,
    ];
    let at = [
        ahead.x as f64,
        ahead.y as f64 + 0.3,
        ahead.z as f64,
    ];
    let camp_counts = render(
        &bytes,
        &cam_look(eye, at, 52.0_f64.to_radians()),
        &evidence.join("track-leaving-camp.png"),
    );
    println!(
        "track-leaving-camp dirt {} gravel {} sealed {} grass {} creek {} water {}",
        camp_counts[0], camp_counts[1], camp_counts[2], camp_counts[3], camp_counts[4], camp_counts[5]
    );

    let high_eye = [-68.0, ground::height_at(-68.0, 70.0) + 42.0, 48.0];
    let high_at = [-21.0, 6.0, 108.0];
    let high_counts = render(
        &bytes,
        &cam_look(high_eye, high_at, 42.0_f64.to_radians()),
        &evidence.join("track-surfaces.png"),
    );
    println!(
        "track-surfaces dirt {} gravel {} sealed {} grass {} creek {} water {}",
        high_counts[0], high_counts[1], high_counts[2], high_counts[3], high_counts[4], high_counts[5]
    );
    println!(
        "samples {} drain {} max_grade {:.3} bytes {}",
        built.samples.len(),
        built.drain_vertices,
        built.max_grade,
        built.bytes.len()
    );
    for sample in &built.samples {
        if sample.chainage_m < 0.1
            || (sample.chainage_m % 10.0) < 2.05 && (sample.chainage_m % 10.0) >= 0.0 && chain_near_step(sample.chainage_m)
            || sample.chainage_m + 2.2 > built.samples.last().unwrap().chainage_m
        {
            println!(
                "s={:6.1} ({:7.2},{:7.2}) y={:6.2} {} half={:.1} cut_right={}",
                sample.chainage_m,
                sample.x,
                sample.z,
                sample.y,
                sample.surface.as_str(),
                sample.half_width_m,
                sample.cut_right
            );
        }
    }
}

fn chain_near_step(chainage: f32) -> bool {
    let m = chainage % 10.0;
    m < 2.05
}

fn check(built: &mesh::BuiltRoad) {
    assert!(built.samples.len() >= 40, "centreline too short");
    assert!(built.drain_vertices > 30, "drain missing from the mesh");
    assert!(
        built.max_grade < 0.16,
        "grade {:.1}% is too steep for the Passat",
        built.max_grade * 100.0
    );
    let bytes = &built.bytes;
    let mut dirt = false;
    let mut gravel = false;
    let mut sealed = false;
    let mut prev_rank = 0;
    for (i, sample) in built.samples.iter().enumerate() {
        let surface = mesh::surface_at_with(bytes, sample.x as f64, sample.z as f64, true);
        assert_eq!(surface, sample.surface, "centreline {i} drifted");
        assert!(
            matches!(
                surface,
                RoadSurface::Dirt | RoadSurface::Gravel | RoadSurface::Sealed
            ),
            "centreline is {}",
            surface.as_str()
        );
        dirt |= surface == RoadSurface::Dirt;
        gravel |= surface == RoadSurface::Gravel;
        sealed |= surface == RoadSurface::Sealed;
        let rank = match surface {
            RoadSurface::Dirt => 0,
            RoadSurface::Gravel => 1,
            RoadSurface::Sealed => 2,
            _ => 9,
        };
        assert!(rank >= prev_rank, "surface went backwards at {i}");
        prev_rank = rank;
        let half = sample.half_width_m;
        assert!(
            (half - 4.5).abs() < 1e-4 || (half - 5.0).abs() < 1e-4 || (half - 6.5).abs() < 1e-4,
            "unexpected half-width {half}"
        );
        let draped = ground::height_at(sample.x as f64, sample.z as f64) + 0.12;
        assert!(
            (draped - sample.y as f64).abs() < 0.02,
            "centreline y is not height_at plus crown"
        );
        assert_ne!(
            ground::material_at(sample.x as f64, sample.z as f64),
            GroundMaterial::WetBank,
            "centreline entered the wet bank"
        );
        assert!(
            ground::height_at(sample.x as f64, sample.z as f64) > 0.5,
            "centreline is in the creek"
        );
        let (dy, role) = mesh::profile_dy(if sample.cut_right { 0.82 } else { -0.82 }, sample.cut_right);
        assert_eq!(role, mesh::ROLE_DRAIN);
        assert!(dy < -0.2);
        let (_dy, other) = mesh::profile_dy(if sample.cut_right { -0.82 } else { 0.82 }, sample.cut_right);
        assert_ne!(other, mesh::ROLE_DRAIN);
        if i > 0 && i + 1 < built.samples.len() {
            let prev = &built.samples[i - 1];
            let next = &built.samples[i + 1];
            let dx = (next.x - prev.x) as f64;
            let dz = (next.z - prev.z) as f64;
            let len = (dx * dx + dz * dz).sqrt().max(1e-8);
            let right = (dz / len, -dx / len);
            let left_y = ground::height_at(
                sample.x as f64 - right.0 * 4.0,
                sample.z as f64 - right.1 * 4.0,
            );
            let right_y = ground::height_at(
                sample.x as f64 + right.0 * 4.0,
                sample.z as f64 + right.1 * 4.0,
            );
            if (right_y - left_y).abs() > 0.08 {
                assert_eq!(sample.cut_right, right_y > left_y, "drain is not on the cut side");
            }
        }
    }
    assert!(dirt && gravel && sealed, "centreline must include dirt, gravel, and sealed");
    assert_eq!(
        mesh::surface_at_with(bytes, -22.0, 28.0, false),
        RoadSurface::Grass,
        "camp pad centre should stay off the carriageway"
    );
    let props = [("passat", -25.5, 29.3), ("jayco", -21.0, 28.0), ("solar", -17.7, 32.1)];
    for (name, x, z) in props {
        assert!(
            mesh::classify(bytes, x, z).is_none(),
            "{name} sits on the road at ({x}, {z})"
        );
    }
    assert_eq!(
        mesh::surface_at_with(bytes, mesh::PAD_X as f64, mesh::PAD_Z as f64, false),
        RoadSurface::Sealed
    );
    assert_eq!(
        mesh::surface_at_with(bytes, mesh::PAD_X as f64 + 11.0, mesh::PAD_Z as f64, false),
        RoadSurface::Gravel,
        "gravel loop"
    );
    let wet_y = ground::height_at(0.0, 56.0);
    let wet_mat = ground::material_at(0.0, 56.0);
    assert!(
        mesh::classify(bytes, 0.0, 56.0).is_none(),
        "creek sample fell on the carriageway"
    );
    assert!(
        wet_y < 0.24
            || wet_mat == GroundMaterial::WetBank
            || (wet_mat == GroundMaterial::Pebble && wet_y < 1.2),
        "creek rule missed ({wet_y}, {})",
        wet_mat.as_str()
    );
    assert!(mesh::plant_blocked(bytes, built.samples[4].x as f64, built.samples[4].z as f64, 0.0));
    assert_eq!(
        sha256(b""),
        "e3b0c44298fc1c149afbf4c8996fb92427ae41e4649b934ca495991b7852b855"
    );
    println!("checks passed");
}

fn write_json(root: &std::path::Path, built: &mesh::BuiltRoad) {
    let mut samples = String::from("[\n");
    for (i, sample) in built.samples.iter().enumerate() {
        let comma = if i + 1 == built.samples.len() { "" } else { "," };
        samples.push_str(&format!(
            "  {{\"chainage_m\": {:.2}, \"x\": {:.3}, \"z\": {:.3}, \"y\": {:.3}, \"surface\": \"{}\", \"half_width_m\": {:.1}, \"cut_right\": {}}}{comma}\n",
            sample.chainage_m, sample.x, sample.z, sample.y, sample.surface.as_str(), sample.half_width_m, sample.cut_right
        ));
    }
    samples.push_str("]\n");
    std::fs::write(root.join("centreline.json"), samples).unwrap();
    let manifest = format!(
        "{{\n  \"schema\": \"webizen.wr03-road/v1\",\n  \"frame\": \"metres, +Y up, same axes as ground::height_at\",\n  \"mesh\": \"production/corridor/forestry-road.bin\",\n  \"bytes\": {},\n  \"centreline\": \"production/corridor/centreline.json\",\n  \"samples\": {},\n  \"max_grade\": {:.4},\n  \"drain_vertices\": {},\n  \"half_width_m\": {{\"dirt\": 4.5, \"gravel\": 5.0, \"sealed\": 6.5}},\n  \"pad\": {{\"x\": {}, \"z\": {}, \"sealed_radius_m\": {}, \"gravel_loop_outer_m\": {}}},\n  \"sha256\": \"{}\",\n  \"note\": \"Vertex y is ground::height_at plus crown 0.12 m, shoulder, or cut-side drain -0.26 m. Tile clearance half-widths in gis_join.rs are unchanged.\"\n}}\n",
        built.bytes.len(),
        built.samples.len(),
        built.max_grade,
        built.drain_vertices,
        mesh::PAD_X,
        mesh::PAD_Z,
        mesh::PAD_INNER_M,
        mesh::PAD_OUTER_M,
        sha256(&built.bytes)
    );
    std::fs::write(root.join("manifest.json"), manifest).unwrap();
}

struct Cam {
    eye: [f64; 3],
    forward: [f64; 3],
    right: [f64; 3],
    up: [f64; 3],
    fov_y: f64,
}

fn cam_look(eye: [f64; 3], at: [f64; 3], fov_y: f64) -> Cam {
    let forward = norm(sub(at, eye));
    let right = norm(cross(forward, [0.0, 1.0, 0.0]));
    let up = norm(cross(right, forward));
    Cam { eye, forward, right, up, fov_y }
}

fn render(mesh_bytes: &[u8], cam: &Cam, path: &std::path::Path) -> [u32; 6] {
    let mut rgb = vec![0u8; (W * H * 3) as usize];
    let mut counts = [0u32; 6];
    for y in 0..H {
        for x in 0..W {
            let dir = ray(cam, x, y);
            if let Some(hit) = march(cam, dir) {
                let (col, kind) = shade(mesh_bytes, hit);
                put(&mut rgb, x, y, col);
                counts[kind] += 1;
            } else {
                put(&mut rgb, x, y, sky(dir));
            }
        }
        if y % 180 == 0 {
            println!("  row {y}");
        }
    }
    write_png(path, &rgb);
    counts
}

#[derive(Clone, Copy)]
struct Hit {
    t: f64,
    p: [f64; 3],
    water: bool,
}

fn march(cam: &Cam, dir: [f64; 3]) -> Option<Hit> {
    let mut t = 0.35;
    let mut prev = t;
    while t < 260.0 {
        let p = add(cam.eye, mul(dir, t));
        if p[0].abs() > 305.0 || p[2].abs() > 305.0 {
            prev = t;
            t += 3.0;
            continue;
        }
        let h = ground::height_at(p[0], p[2]);
        if p[1] <= h {
            let mut lo = prev;
            let mut hi = t;
            for _ in 0..8 {
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
        if p[1] > 80.0 && dir[1] > 0.0 {
            return None;
        }
    }
    None
}

fn shade(mesh_bytes: &[u8], hit: Hit) -> ([u8; 3], usize) {
    if hit.water {
        return (fog([36, 108, 118], hit.t), 5);
    }
    if let Some(road) = mesh::classify(mesh_bytes, hit.p[0], hit.p[2]) {
        let kind = match road.surface {
            RoadSurface::Dirt => 0,
            RoadSurface::Gravel => 1,
            RoadSurface::Sealed => 2,
            RoadSurface::Grass => 3,
            RoadSurface::Creek => 4,
        };
        let rgb = road_colour(&road, hit.p);
        let n = ground::normal_at(hit.p[0], hit.p[2]);
        return (fog(light(rgb, n, hit.t), hit.t), kind);
    }
    let mat = ground::material_at(hit.p[0], hit.p[2]);
    let y = hit.p[1];
    let creek = y < 0.24
        || mat == GroundMaterial::WetBank
        || (mat == GroundMaterial::Pebble && y < 1.2);
    let base = ground::colour_at(hit.p[0], hit.p[2]);
    let grain = hash01(hit.p[0], hit.p[2]);
    let rgb = [
        (base[0] + (grain - 0.5) * 10.0).clamp(0.0, 255.0),
        (base[1] + (grain - 0.5) * 8.0).clamp(0.0, 255.0),
        (base[2] + (grain - 0.5) * 6.0).clamp(0.0, 255.0),
    ];
    let n = ground::normal_at(hit.p[0], hit.p[2]);
    let kind = if creek { 4 } else { 3 };
    (fog(light(rgb, n, hit.t), hit.t), kind)
}

fn road_colour(hit: &mesh::RoadHit, p: [f64; 3]) -> [f64; 3] {
    let hash = hash01(p[0] * 1.7, p[2] * 1.3);
    let ring = hit.on_pad && hit.surface == RoadSurface::Gravel && hit.lateral_m > 2.0;
    let t = if hit.half_width_m > 0.2 && !ring {
        (hit.lateral_m / hit.half_width_m).clamp(-1.0, 1.0)
    } else {
        0.0
    };
    let (_dy, role) = if ring {
        (0.0, mesh::ROLE_EDGE)
    } else {
        mesh::profile_dy(t, hit.cut_right)
    };
    match hit.surface {
        RoadSurface::Dirt => {
            let mut rgb = [188.0, 162.0, 116.0];
            if (hit.lateral_m.abs() - 1.15).abs() < 0.32 {
                rgb = [128.0, 96.0, 58.0];
            }
            if role == mesh::ROLE_SHOULDER {
                rgb = [170.0, 148.0, 108.0];
            }
            if role == mesh::ROLE_DRAIN {
                rgb = [72.0, 56.0, 36.0];
            }
            rgb
        }
        RoadSurface::Gravel => {
            let mut rgb = [154.0, 156.0, 150.0];
            if hash > 0.72 {
                rgb = [188.0, 186.0, 176.0];
            } else if hash < 0.18 {
                rgb = [112.0, 114.0, 108.0];
            }
            if role == mesh::ROLE_DRAIN {
                rgb = [86.0, 84.0, 76.0];
            }
            rgb
        }
        RoadSurface::Sealed => {
            let dash = (hit.chainage_m % 8.0) < 3.2 && hit.lateral_m.abs() < 0.14 && !hit.on_pad;
            if dash {
                [214.0, 208.0, 186.0]
            } else if role == mesh::ROLE_EDGE || hit.lateral_m.abs() > hit.half_width_m * 0.86 {
                [78.0, 80.0, 84.0]
            } else {
                [44.0, 46.0, 50.0]
            }
        }
        RoadSurface::Grass | RoadSurface::Creek => [80.0, 90.0, 50.0],
    }
}

fn light(rgb: [f64; 3], n: [f64; 3], t: f64) -> [u8; 3] {
    let sun = norm([0.42, 0.84, 0.34]);
    let ndotl = (n[0] * sun[0] + n[1] * sun[1] + n[2] * sun[2]).max(0.0);
    let scale = 0.48 + 0.66 * ndotl;
    let _ = t;
    [
        (rgb[0] * scale).clamp(0.0, 255.0) as u8,
        (rgb[1] * scale).clamp(0.0, 255.0) as u8,
        (rgb[2] * scale).clamp(0.0, 255.0) as u8,
    ]
}

fn fog(rgb: [u8; 3], t: f64) -> [u8; 3] {
    let f = (1.0 - (-t / 180.0).exp()).clamp(0.0, 0.5);
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

fn ray(cam: &Cam, x: i32, y: i32) -> [f64; 3] {
    let ndc_x = ((x as f64 + 0.5) / W as f64) * 2.0 - 1.0;
    let ndc_y = 1.0 - ((y as f64 + 0.5) / H as f64) * 2.0;
    let tan = (cam.fov_y * 0.5).tan();
    let aspect = W as f64 / H as f64;
    norm(add(
        cam.forward,
        add(mul(cam.right, ndc_x * tan * aspect), mul(cam.up, ndc_y * tan)),
    ))
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

fn add(a: [f64; 3], b: [f64; 3]) -> [f64; 3] {
    [a[0] + b[0], a[1] + b[1], a[2] + b[2]]
}
fn sub(a: [f64; 3], b: [f64; 3]) -> [f64; 3] {
    [a[0] - b[0], a[1] - b[1], a[2] - b[2]]
}
fn mul(a: [f64; 3], s: f64) -> [f64; 3] {
    [a[0] * s, a[1] * s, a[2] * s]
}
fn cross(a: [f64; 3], b: [f64; 3]) -> [f64; 3] {
    [
        a[1] * b[2] - a[2] * b[1],
        a[2] * b[0] - a[0] * b[2],
        a[0] * b[1] - a[1] * b[0],
    ]
}
fn norm(v: [f64; 3]) -> [f64; 3] {
    let len = (v[0] * v[0] + v[1] * v[1] + v[2] * v[2]).sqrt().max(1e-12);
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
    let mut ihdr = Vec::new();
    ihdr.extend_from_slice(&(W as u32).to_be_bytes());
    ihdr.extend_from_slice(&(H as u32).to_be_bytes());
    ihdr.extend_from_slice(&[8, 2, 0, 0, 0]);
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

fn sha256(message: &[u8]) -> String {
    const K: [u32; 64] = [
        0x428a2f98, 0x71374491, 0xb5c0fbcf, 0xe9b5dba5, 0x3956c25b, 0x59f111f1, 0x923f82a4,
        0xab1c5ed5, 0xd807aa98, 0x12835b01, 0x243185be, 0x550c7dc3, 0x72be5d74, 0x80deb1fe,
        0x9bdc06a7, 0xc19bf174, 0xe49b69c1, 0xefbe4786, 0x0fc19dc6, 0x240ca1cc, 0x2de92c6f,
        0x4a7484aa, 0x5cb0a9dc, 0x76f988da, 0x983e5152, 0xa831c66d, 0xb00327c8, 0xbf597fc7,
        0xc6e00bf3, 0xd5a79147, 0x06ca6351, 0x14292967, 0x27b70a85, 0x2e1b2138, 0x4d2c6dfc,
        0x53380d13, 0x650a7354, 0x766a0abb, 0x81c2c92e, 0x92722c85, 0xa2bfe8a1, 0xa81a664b,
        0xc24b8b70, 0xc76c51a3, 0xd192e819, 0xd6990624, 0xf40e3585, 0x106aa070, 0x19a4c116,
        0x1e376c08, 0x2748774c, 0x34b0bcb5, 0x391c0cb3, 0x4ed8aa4a, 0x5b9cca4f, 0x682e6ff3,
        0x748f82ee, 0x78a5636f, 0x84c87814, 0x8cc70208, 0x90befffa, 0xa4506ceb, 0xbef9a3f7,
        0xc67178f2,
    ];
    let mut h: [u32; 8] = [
        0x6a09e667, 0xbb67ae85, 0x3c6ef372, 0xa54ff53a, 0x510e527f, 0x9b05688c, 0x1f83d9ab,
        0x5be0cd19,
    ];
    let bit_len = (message.len() as u64).wrapping_mul(8);
    let mut data = message.to_vec();
    data.push(0x80);
    while data.len() % 64 != 56 {
        data.push(0);
    }
    data.extend_from_slice(&bit_len.to_be_bytes());
    for chunk in data.chunks(64) {
        let mut w = [0u32; 64];
        for i in 0..16 {
            w[i] = u32::from_be_bytes(chunk[i * 4..i * 4 + 4].try_into().unwrap());
        }
        for i in 16..64 {
            let s0 = w[i - 15].rotate_right(7) ^ w[i - 15].rotate_right(18) ^ (w[i - 15] >> 3);
            let s1 = w[i - 2].rotate_right(17) ^ w[i - 2].rotate_right(19) ^ (w[i - 2] >> 10);
            w[i] = w[i - 16]
                .wrapping_add(s0)
                .wrapping_add(w[i - 7])
                .wrapping_add(s1);
        }
        let mut a: u32 = h[0];
        let mut b: u32 = h[1];
        let mut c: u32 = h[2];
        let mut d: u32 = h[3];
        let mut e: u32 = h[4];
        let mut f: u32 = h[5];
        let mut g: u32 = h[6];
        let mut hh: u32 = h[7];
        for i in 0..64 {
            let s1 = e.rotate_right(6) ^ e.rotate_right(11) ^ e.rotate_right(25);
            let ch = (e & f) ^ (!e & g);
            let t1 = hh
                .wrapping_add(s1)
                .wrapping_add(ch)
                .wrapping_add(K[i])
                .wrapping_add(w[i]);
            let s0 = a.rotate_right(2) ^ a.rotate_right(13) ^ a.rotate_right(22);
            let maj = (a & b) ^ (a & c) ^ (b & c);
            let t2 = s0.wrapping_add(maj);
            hh = g;
            g = f;
            f = e;
            e = d.wrapping_add(t1);
            d = c;
            c = b;
            b = a;
            a = t1.wrapping_add(t2);
        }
        h[0] = h[0].wrapping_add(a);
        h[1] = h[1].wrapping_add(b);
        h[2] = h[2].wrapping_add(c);
        h[3] = h[3].wrapping_add(d);
        h[4] = h[4].wrapping_add(e);
        h[5] = h[5].wrapping_add(f);
        h[6] = h[6].wrapping_add(g);
        h[7] = h[7].wrapping_add(hh);
    }
    h.iter().map(|word| format!("{word:08x}")).collect()
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
