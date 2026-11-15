//! Render a quick orthographic side view (left) and top view (right) of the
//! tree to a PNG, without any graphics dependency. For eyeballing the model.
//!
//! cargo run -p dendro-core --release --example snapshot -- --years 60 --doy 200 --out tree.png
//!     [--seed N] [--no-leaves] [--no-roots] [--side] [--genome FILE.ron]

use dendro_core::{Genome, MeshData, Simulation};

fn main() {
    let mut years = 60u32;
    let mut seed = 42u64;
    let mut doy = 200u32;
    let mut out = String::from("tree.png");
    let mut leaves_on = true;
    let mut roots_on = true;
    let mut side_only = false;
    let mut genome = Genome::quercus_robur();
    let mut args = std::env::args().skip(1);
    while let Some(a) = args.next() {
        match a.as_str() {
            "--years" => years = args.next().unwrap().parse().unwrap(),
            "--seed" => seed = args.next().unwrap().parse().unwrap(),
            "--doy" => doy = args.next().unwrap().parse().unwrap(),
            "--out" => out = args.next().unwrap(),
            "--no-leaves" => leaves_on = false,
            "--no-roots" => roots_on = false,
            "--side" => side_only = true,
            "--genome" => {
                let path = args.next().unwrap();
                genome = Genome::from_ron(&std::fs::read_to_string(&path).expect("read genome")).expect("parse genome");
            }
            other => panic!("unknown argument {other}"),
        }
    }
    let mut sim = Simulation::new(genome.clone(), seed);
    sim.advance_days(years * 365 + doy);
    let wood = sim.wood_mesh();
    let leaves = sim.leaf_mesh();
    let roots = sim.root_mesh();
    println!(
        "age {:.2} phase {:?} H {:.2} DBH {:.1} tris wood {} leaves {} roots {}",
        sim.age_years(),
        sim.phase(),
        sim.height_m(),
        sim.dbh_cm(),
        wood.triangle_count(),
        leaves.triangle_count(),
        roots.triangle_count()
    );
    let mut all = MeshData::default();
    all.append(&wood);
    if leaves_on {
        all.append(&leaves);
    }
    if roots_on {
        all.append(&roots);
    }

    // Two panels (side + top), or one large side view with --side.
    let (w, h) = if side_only { (3200usize, 1600usize) } else { (1400usize, 800usize) };
    let mut img = vec![[235u8, 240, 245]; w * h];
    let mut zbuf = vec![f32::MAX; w * h];
    // Extents.
    let (mut minx, mut maxx, mut miny, mut maxy) = (f32::MAX, f32::MIN, f32::MAX, f32::MIN);
    for p in &all.positions {
        minx = minx.min(p[0]).min(p[2]);
        maxx = maxx.max(p[0]).max(p[2]);
        miny = miny.min(p[1]);
        maxy = maxy.max(p[1]);
    }
    let span = (maxx - minx).max(maxy - miny).max(0.5) * 1.05;
    let panel = if side_only { w } else { w / 2 };
    let scale = (h.min(panel) as f32) / span;
    let cx = (minx + maxx) * 0.5;
    let cy = (miny + maxy) * 0.5;
    let light = norm([0.4, 0.8, 0.45]);
    // Ground line (side view).
    let gy = (h as f32 * 0.5 - (0.0 - cy) * scale) as usize;
    if gy < h {
        for x in 0..panel {
            img[gy * w + x] = [120, 100, 80];
        }
    }
    for view in 0..if side_only { 1 } else { 2 } {
        let x_off = if view == 0 { 0.0 } else { (w / 2) as f32 };
        for t in all.indices.as_chunks::<3>().0 {
            let mut pts = [[0.0f32; 3]; 3];
            for k in 0..3 {
                let p = all.positions[t[k] as usize];
                pts[k] = if view == 0 {
                    // side: x right, y up, depth z
                    [x_off + (panel / 2) as f32 + (p[0] - cx) * scale, h as f32 * 0.5 - (p[1] - cy) * scale, p[2]]
                } else {
                    // top: x right, z down, depth -y
                    [x_off + (panel / 2) as f32 + (p[0] - cx) * scale, h as f32 * 0.5 + (p[2]) * scale, -p[1]]
                };
            }
            let n = all.normals[t[0] as usize];
            let c = all.colors[t[0] as usize];
            let lam = (n[0] * light[0] + n[1] * light[1] + n[2] * light[2]).abs() * 0.7 + 0.3;
            let col = [
                (c[0] * lam * 255.0).min(255.0) as u8,
                (c[1] * lam * 255.0).min(255.0) as u8,
                (c[2] * lam * 255.0).min(255.0) as u8,
            ];
            raster(&mut img, &mut zbuf, w, h, &pts, col, panel, x_off);
        }
    }
    write_png(&out, w, h, &img);
    println!("wrote {out}");
}

fn norm(v: [f32; 3]) -> [f32; 3] {
    let l = (v[0] * v[0] + v[1] * v[1] + v[2] * v[2]).sqrt();
    [v[0] / l, v[1] / l, v[2] / l]
}

#[allow(clippy::too_many_arguments)]
fn raster(img: &mut [[u8; 3]], zb: &mut [f32], w: usize, h: usize, p: &[[f32; 3]; 3], col: [u8; 3], panel: usize, x_off: f32) {
    let x_lo = x_off.max(0.0);
    let x_hi = (x_off + panel as f32).min(w as f32) - 1.0;
    let minx = p.iter().map(|q| q[0]).fold(f32::MAX, f32::min).floor().max(x_lo) as i32;
    let maxx = p.iter().map(|q| q[0]).fold(f32::MIN, f32::max).ceil().min(x_hi) as i32;
    let miny = p.iter().map(|q| q[1]).fold(f32::MAX, f32::min).floor().max(0.0) as i32;
    let maxy = p.iter().map(|q| q[1]).fold(f32::MIN, f32::max).ceil().min(h as f32 - 1.0) as i32;
    let area = (p[1][0] - p[0][0]) * (p[2][1] - p[0][1]) - (p[2][0] - p[0][0]) * (p[1][1] - p[0][1]);
    if area.abs() < 1e-6 {
        // Degenerate/sub-pixel: plot a point.
        let (x, y) = (p[0][0] as i32, p[0][1] as i32);
        if x >= minx && x <= maxx && y >= 0 && y < h as i32 {
            let i = y as usize * w + x as usize;
            if p[0][2] < zb[i] {
                zb[i] = p[0][2];
                img[i] = col;
            }
        }
        return;
    }
    for y in miny..=maxy {
        for x in minx..=maxx {
            let (fx, fy) = (x as f32 + 0.5, y as f32 + 0.5);
            let w0 = ((p[1][0] - fx) * (p[2][1] - fy) - (p[2][0] - fx) * (p[1][1] - fy)) / area;
            let w1 = ((p[2][0] - fx) * (p[0][1] - fy) - (p[0][0] - fx) * (p[2][1] - fy)) / area;
            let w2 = 1.0 - w0 - w1;
            if w0 < 0.0 || w1 < 0.0 || w2 < 0.0 {
                continue;
            }
            let z = w0 * p[0][2] + w1 * p[1][2] + w2 * p[2][2];
            let i = y as usize * w + x as usize;
            if z < zb[i] {
                zb[i] = z;
                img[i] = col;
            }
        }
    }
}

fn crc32(data: &[u8]) -> u32 {
    let mut c = 0xFFFF_FFFFu32;
    for &b in data {
        c ^= b as u32;
        for _ in 0..8 {
            c = if c & 1 != 0 { 0xEDB8_8320 ^ (c >> 1) } else { c >> 1 };
        }
    }
    !c
}

fn write_png(path: &str, w: usize, h: usize, img: &[[u8; 3]]) {
    let mut raw = Vec::with_capacity((w * 3 + 1) * h);
    for y in 0..h {
        raw.push(0u8);
        for x in 0..w {
            raw.extend_from_slice(&img[y * w + x]);
        }
    }
    // zlib with stored (uncompressed) deflate blocks.
    let mut z = vec![0x78u8, 0x01];
    for (i, chunk) in raw.chunks(65535).enumerate() {
        let last = (i + 1) * 65535 >= raw.len();
        z.push(last as u8);
        let len = chunk.len() as u16;
        z.extend_from_slice(&len.to_le_bytes());
        z.extend_from_slice(&(!len).to_le_bytes());
        z.extend_from_slice(chunk);
    }
    let (mut a, mut b) = (1u32, 0u32);
    for &x in &raw {
        a = (a + x as u32) % 65521;
        b = (b + a) % 65521;
    }
    z.extend_from_slice(&((b << 16) | a).to_be_bytes());
    let mut png = vec![0x89, b'P', b'N', b'G', 0x0D, 0x0A, 0x1A, 0x0A];
    let mut chunk = |ty: &[u8], data: &[u8]| {
        png.extend_from_slice(&(data.len() as u32).to_be_bytes());
        let mut c = ty.to_vec();
        c.extend_from_slice(data);
        png.extend_from_slice(&c);
        png.extend_from_slice(&crc32(&c).to_be_bytes());
    };
    let mut ihdr = Vec::new();
    ihdr.extend_from_slice(&(w as u32).to_be_bytes());
    ihdr.extend_from_slice(&(h as u32).to_be_bytes());
    ihdr.extend_from_slice(&[8, 2, 0, 0, 0]);
    chunk(b"IHDR", &ihdr);
    chunk(b"IDAT", &z);
    chunk(b"IEND", &[]);
    std::fs::write(path, png).expect("write png");
}
