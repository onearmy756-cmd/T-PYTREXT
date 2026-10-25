//! forge_rust3d.rs — 3D KWA RUST: generator ya PRODUCTION (std-only).
//!
//! Huzalisha mradi KAMILI wa 3D kwa Rust bila dependencies za nje (std pekee):
//! hesabu ya vekta/matriksi, meshes (cube/sphere/plane), kamera, na RASTERIZER
//! ya software yenye z-buffer inayoandika PPM. Kila faili ni HALISI na
//! inacompile; `cargo test` inapita. Hii ndiyo "3D kwa Rust" ya FORGE.

use crate::forge_domain::{to_snake, Domain};

/// Je, kind hii ni 3D ya Rust?
pub fn is_rust3d(kind: &str) -> bool {
    matches!(kind, "rust-3d" | "rust3d" | "rust-3d-game" | "3d-rust")
}

/// Zalisha mradi kamili wa Rust 3D (files, note).
pub fn generate_rust3d(domain: &Domain) -> (Vec<(String, String)>, String) {
    let name = to_snake(&domain.project);
    let name = if name.is_empty() { "scene3d".to_string() } else { name };
    let mut files: Vec<(String, String)> = Vec::new();
    files.push(("Cargo.toml".into(), cargo_toml(&name)));
    files.push(("README.md".into(), readme(&name)));
    files.push(("src/lib.rs".into(), lib_rs()));
    files.push(("src/vec3.rs".into(), vec3_rs()));
    files.push(("src/mat4.rs".into(), mat4_rs()));
    files.push(("src/mesh.rs".into(), mesh_rs()));
    files.push(("src/camera.rs".into(), camera_rs()));
    files.push(("src/raster.rs".into(), raster_rs()));
    files.push(("src/main.rs".into(), main_rs(&name)));
    (files, "rust-3d (std-only: vec3/mat4/mesh/camera/software-rasterizer) production".to_string())
}

fn cargo_toml(name: &str) -> String {
    format!(
        r#"[package]
name = "{name}"
version = "0.1.0"
edition = "2021"
description = "3D scene renderer (software rasterizer, std-only)"

[lib]
name = "{name}"
path = "src/lib.rs"

[[bin]]
name = "{name}"
path = "src/main.rs"

[profile.release]
opt-level = 3
"#
    )
}

fn readme(name: &str) -> String {
    format!(
        r#"# {name} — 3D kwa Rust

Renderer ya 3D ya software (std-only, hakuna crates za nje).

## Kukimbia

```bash
cargo run --release     # huzalisha out.ppm (picha ya 3D)
cargo test              # testi za hesabu, mesh, na rasterizer
```

## Faili

- `src/vec3.rs`   — vekta za 3D (dot, cross, normalize)
- `src/mat4.rs`   — matriksi 4x4 (perspective, look_at, transform)
- `src/mesh.rs`   — meshes: cube, sphere, plane
- `src/camera.rs` — kamera
- `src/raster.rs` — framebuffer + z-buffer rasterizer + PPM writer
- `src/main.rs`   — scene → render → `out.ppm`
"#
    )
}

fn lib_rs() -> String {
    r#"//! Library crate (3D, std-only).
pub mod camera;
pub mod mat4;
pub mod mesh;
pub mod raster;
pub mod vec3;

pub use camera::Camera;
pub use mat4::Mat4;
pub use mesh::Mesh;
pub use raster::Framebuffer;
pub use vec3::Vec3;
"#
    .to_string()
}

fn vec3_rs() -> String {
    r#"//! vec3.rs — 3D vector math (std-only).
use std::ops::{Add, Mul, Neg, Sub};

#[derive(Debug, Clone, Copy, PartialEq, Default)]
pub struct Vec3 {
    pub x: f64,
    pub y: f64,
    pub z: f64,
}

impl Vec3 {
    pub const fn new(x: f64, y: f64, z: f64) -> Self {
        Self { x, y, z }
    }

    pub const fn zero() -> Self {
        Self { x: 0.0, y: 0.0, z: 0.0 }
    }

    pub fn dot(self, o: Vec3) -> f64 {
        self.x * o.x + self.y * o.y + self.z * o.z
    }

    pub fn cross(self, o: Vec3) -> Vec3 {
        Vec3::new(
            self.y * o.z - self.z * o.y,
            self.z * o.x - self.x * o.z,
            self.x * o.y - self.y * o.x,
        )
    }

    pub fn len(self) -> f64 {
        self.dot(self).sqrt()
    }

    /// Rejesha vekta yenye urefu 1 (zero kama urefu ni 0).
    pub fn normalize(self) -> Vec3 {
        let l = self.len();
        if l <= f64::EPSILON {
            Vec3::zero()
        } else {
            self * (1.0 / l)
        }
    }
}

impl Add for Vec3 {
    type Output = Vec3;
    fn add(self, o: Vec3) -> Vec3 {
        Vec3::new(self.x + o.x, self.y + o.y, self.z + o.z)
    }
}

impl Sub for Vec3 {
    type Output = Vec3;
    fn sub(self, o: Vec3) -> Vec3 {
        Vec3::new(self.x - o.x, self.y - o.y, self.z - o.z)
    }
}

impl Mul<f64> for Vec3 {
    type Output = Vec3;
    fn mul(self, s: f64) -> Vec3 {
        Vec3::new(self.x * s, self.y * s, self.z * s)
    }
}

impl Neg for Vec3 {
    type Output = Vec3;
    fn neg(self) -> Vec3 {
        Vec3::new(-self.x, -self.y, -self.z)
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn dot_na_cross() {
        let a = Vec3::new(1.0, 0.0, 0.0);
        let b = Vec3::new(0.0, 1.0, 0.0);
        assert_eq!(a.dot(b), 0.0);
        assert_eq!(a.cross(b), Vec3::new(0.0, 0.0, 1.0));
    }

    #[test]
    fn normalize_urefu_mmoja() {
        let v = Vec3::new(3.0, 4.0, 0.0).normalize();
        assert!((v.len() - 1.0).abs() < 1e-9);
    }

    #[test]
    fn normalize_zero_salama() {
        assert_eq!(Vec3::zero().normalize(), Vec3::zero());
    }
}
"#
    .to_string()
}

fn mat4_rs() -> String {
    r#"//! mat4.rs — 4x4 matrices (column-major, std-only).
use crate::vec3::Vec3;

#[derive(Debug, Clone, Copy, PartialEq)]
pub struct Mat4 {
    /// m[col][row]
    pub m: [[f64; 4]; 4],
}

impl Mat4 {
    pub const fn identity() -> Self {
        Self {
            m: [
                [1.0, 0.0, 0.0, 0.0],
                [0.0, 1.0, 0.0, 0.0],
                [0.0, 0.0, 1.0, 0.0],
                [0.0, 0.0, 0.0, 1.0],
            ],
        }
    }

    /// Matriksi ya mwonekano (perspective). `fov_y` ni radians.
    pub fn perspective(fov_y: f64, aspect: f64, near: f64, far: f64) -> Self {
        let f = 1.0 / (fov_y / 2.0).tan();
        let mut r = Mat4::identity();
        r.m[0][0] = f / aspect;
        r.m[1][1] = f;
        r.m[2][2] = (far + near) / (near - far);
        r.m[2][3] = -1.0;
        r.m[3][2] = (2.0 * far * near) / (near - far);
        r.m[3][3] = 0.0;
        r
    }

    /// Matriksi ya kamera (world → view).
    pub fn look_at(eye: Vec3, target: Vec3, up: Vec3) -> Self {
        let f = (target - eye).normalize();
        let s = f.cross(up).normalize();
        let u = s.cross(f);
        let mut r = Mat4::identity();
        r.m[0][0] = s.x;
        r.m[1][0] = s.y;
        r.m[2][0] = s.z;
        r.m[0][1] = u.x;
        r.m[1][1] = u.y;
        r.m[2][1] = u.z;
        r.m[0][2] = -f.x;
        r.m[1][2] = -f.y;
        r.m[2][2] = -f.z;
        r.m[3][0] = -s.dot(eye);
        r.m[3][1] = -u.dot(eye);
        r.m[3][2] = f.dot(eye);
        r
    }

    pub fn translate(x: f64, y: f64, z: f64) -> Self {
        let mut r = Mat4::identity();
        r.m[3][0] = x;
        r.m[3][1] = y;
        r.m[3][2] = z;
        r
    }

    pub fn scale(s: f64) -> Self {
        let mut r = Mat4::identity();
        r.m[0][0] = s;
        r.m[1][1] = s;
        r.m[2][2] = s;
        r
    }

    pub fn rotate_y(angle: f64) -> Self {
        let (s, c) = angle.sin_cos();
        let mut r = Mat4::identity();
        r.m[0][0] = c;
        r.m[0][2] = -s;
        r.m[2][0] = s;
        r.m[2][2] = c;
        r
    }

    /// Kuzidisha matriksi (self * rhs).
    pub fn mul(&self, rhs: &Mat4) -> Mat4 {
        let mut r = Mat4::identity();
        for c in 0..4 {
            for row in 0..4 {
                let mut sum = 0.0;
                for k in 0..4 {
                    sum += self.m[k][row] * rhs.m[c][k];
                }
                r.m[c][row] = sum;
            }
        }
        r
    }

    /// Badilisha pointi (inajumuisha mgawano wa w).
    pub fn transform_point(&self, p: Vec3) -> Vec3 {
        let x = self.m[0][0] * p.x + self.m[1][0] * p.y + self.m[2][0] * p.z + self.m[3][0];
        let y = self.m[0][1] * p.x + self.m[1][1] * p.y + self.m[2][1] * p.z + self.m[3][1];
        let z = self.m[0][2] * p.x + self.m[1][2] * p.y + self.m[2][2] * p.z + self.m[3][2];
        let w = self.m[0][3] * p.x + self.m[1][3] * p.y + self.m[2][3] * p.z + self.m[3][3];
        if w.abs() <= f64::EPSILON {
            Vec3::new(x, y, z)
        } else {
            Vec3::new(x / w, y / w, z / w)
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn identity_haibadilishi() {
        let p = Vec3::new(1.0, 2.0, 3.0);
        assert_eq!(Mat4::identity().transform_point(p), p);
    }

    #[test]
    fn translate_huhamisha() {
        let p = Vec3::new(0.0, 0.0, 0.0);
        assert_eq!(Mat4::translate(1.0, 2.0, 3.0).transform_point(p), Vec3::new(1.0, 2.0, 3.0));
    }

    #[test]
    fn look_at_huweka_eye_kwenye_view() {
        let eye = Vec3::new(0.0, 0.0, 5.0);
        let view = Mat4::look_at(eye, Vec3::zero(), Vec3::new(0.0, 1.0, 0.0));
        let v = view.transform_point(eye);
        assert!(v.len() < 1e-9, "eye inapaswa kuwa asili ya view");
    }
}
"#
    .to_string()
}

fn mesh_rs() -> String {
    r#"//! mesh.rs — meshes za msingi (std-only).
use crate::vec3::Vec3;

#[derive(Debug, Clone, Default)]
pub struct Mesh {
    pub vertices: Vec<Vec3>,
    /// Nyuso za pembetatu (indices).
    pub faces: Vec<[usize; 3]>,
}

impl Mesh {
    pub fn new(vertices: Vec<Vec3>, faces: Vec<[usize; 3]>) -> Self {
        Self { vertices, faces }
    }

    /// Cube yenye kipimo `size` (upande mmoja) iliyowekwa katikati.
    pub fn cube(size: f64) -> Self {
        let h = size / 2.0;
        let v = vec![
            Vec3::new(-h, -h, -h),
            Vec3::new(h, -h, -h),
            Vec3::new(h, h, -h),
            Vec3::new(-h, h, -h),
            Vec3::new(-h, -h, h),
            Vec3::new(h, -h, h),
            Vec3::new(h, h, h),
            Vec3::new(-h, h, h),
        ];
        let f = vec![
            [0, 1, 2], [0, 2, 3], // back
            [4, 6, 5], [4, 7, 6], // front
            [0, 4, 5], [0, 5, 1], // bottom
            [3, 2, 6], [3, 6, 7], // top
            [1, 5, 6], [1, 6, 2], // right
            [0, 3, 7], [0, 7, 4], // left
        ];
        Mesh::new(v, f)
    }

    /// Sphere (UV) yenye `segments` (mzunguko) na `rings` (juu-chini).
    pub fn sphere(radius: f64, segments: usize, rings: usize) -> Self {
        let seg = segments.max(3);
        let ring = rings.max(2);
        let mut vertices = Vec::new();
        for r in 0..=ring {
            let phi = std::f64::consts::PI * (r as f64) / (ring as f64);
            for s in 0..=seg {
                let theta = 2.0 * std::f64::consts::PI * (s as f64) / (seg as f64);
                let x = radius * phi.sin() * theta.cos();
                let y = radius * phi.cos();
                let z = radius * phi.sin() * theta.sin();
                vertices.push(Vec3::new(x, y, z));
            }
        }
        let mut faces = Vec::new();
        let stride = seg + 1;
        for r in 0..ring {
            for s in 0..seg {
                let a = r * stride + s;
                let b = a + stride;
                faces.push([a, b, a + 1]);
                faces.push([a + 1, b, b + 1]);
            }
        }
        Mesh::new(vertices, faces)
    }

    /// Plane (grid) ya `n x n` yenye kipimo `size` kwenye ndege ya XZ.
    pub fn plane(size: f64, n: usize) -> Self {
        let n = n.max(1);
        let step = size / (n as f64);
        let half = size / 2.0;
        let mut vertices = Vec::new();
        for i in 0..=n {
            for j in 0..=n {
                let x = -half + (i as f64) * step;
                let z = -half + (j as f64) * step;
                vertices.push(Vec3::new(x, 0.0, z));
            }
        }
        let mut faces = Vec::new();
        let stride = n + 1;
        for i in 0..n {
            for j in 0..n {
                let a = i * stride + j;
                faces.push([a, a + 1, a + stride]);
                faces.push([a + 1, a + stride + 1, a + stride]);
            }
        }
        Mesh::new(vertices, faces)
    }

    /// Kituo cha mesh (wastani wa vertices).
    pub fn center(&self) -> Vec3 {
        if self.vertices.is_empty() {
            return Vec3::zero();
        }
        let mut sum = Vec3::zero();
        for v in &self.vertices {
            sum = sum + *v;
        }
        sum * (1.0 / self.vertices.len() as f64)
    }

    /// Normal ya uso (pembetatu) kwa index ya face.
    pub fn face_normal(&self, face: &[usize; 3]) -> Vec3 {
        let a = self.vertices[face[0]];
        let b = self.vertices[face[1]];
        let c = self.vertices[face[2]];
        (b - a).cross(c - a).normalize()
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn cube_ina_vertices_na_faces() {
        let c = Mesh::cube(2.0);
        assert_eq!(c.vertices.len(), 8);
        assert_eq!(c.faces.len(), 12);
    }

    #[test]
    fn sphere_ina_vertices() {
        let s = Mesh::sphere(1.0, 8, 6);
        assert_eq!(s.vertices.len(), (8 + 1) * (6 + 1));
        assert!(!s.faces.is_empty());
    }

    #[test]
    fn plane_ina_grid() {
        let p = Mesh::plane(10.0, 4);
        assert_eq!(p.vertices.len(), 25);
        assert_eq!(p.faces.len(), 4 * 4 * 2);
    }

    #[test]
    fn face_normal_urefu_mmoja() {
        let c = Mesh::cube(2.0);
        let n = c.face_normal(&c.faces[0]);
        assert!((n.len() - 1.0).abs() < 1e-9);
    }
}
"#
    .to_string()
}

fn camera_rs() -> String {
    r#"//! camera.rs — kamera (std-only).
use crate::mat4::Mat4;
use crate::vec3::Vec3;

#[derive(Debug, Clone, Copy)]
pub struct Camera {
    pub eye: Vec3,
    pub target: Vec3,
    pub up: Vec3,
    pub fov_deg: f64,
    pub aspect: f64,
}

impl Camera {
    pub fn new(eye: Vec3, target: Vec3, up: Vec3, fov_deg: f64, aspect: f64) -> Self {
        Self { eye, target, up, fov_deg, aspect }
    }

    pub fn view(&self) -> Mat4 {
        Mat4::look_at(self.eye, self.target, self.up)
    }

    pub fn projection(&self, near: f64, far: f64) -> Mat4 {
        Mat4::perspective(self.fov_deg.to_radians(), self.aspect, near, far)
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn view_na_projection_ni_matriksi() {
        let cam = Camera::new(Vec3::new(0.0, 1.0, 5.0), Vec3::zero(), Vec3::new(0.0, 1.0, 0.0), 60.0, 16.0 / 9.0);
        let v = cam.view();
        let p = cam.projection(0.1, 100.0);
        assert_eq!(v.m[3][3], 1.0);
        assert_eq!(p.m[2][3], -1.0);
    }
}
"#
    .to_string()
}

fn raster_rs() -> String {
    r#"//! raster.rs — framebuffer + z-buffer software rasterizer + PPM writer (std-only).
use std::fs::File;
use std::io::{self, Write};

pub struct Framebuffer {
    pub width: usize,
    pub height: usize,
    color: Vec<[u8; 3]>,
    depth: Vec<f64>,
}

impl Framebuffer {
    pub fn new(width: usize, height: usize) -> Self {
        let n = width.saturating_mul(height);
        Self {
            width,
            height,
            color: vec![[0, 0, 0]; n],
            depth: vec![f64::INFINITY; n],
        }
    }

    pub fn clear(&mut self, bg: [u8; 3]) {
        for c in self.color.iter_mut() {
            *c = bg;
        }
        for d in self.depth.iter_mut() {
            *d = f64::INFINITY;
        }
    }

    /// Weka pikseli kama iko mbele zaidi (z ndogo = karibu).
    pub fn plot(&mut self, x: i32, y: i32, z: f64, c: [u8; 3]) {
        if x < 0 || y < 0 || x as usize >= self.width || y as usize >= self.height {
            return;
        }
        let idx = y as usize * self.width + x as usize;
        if z < self.depth[idx] {
            self.depth[idx] = z;
            self.color[idx] = c;
        }
    }

    /// Chora pembetatu (screen coords + depth) kwa barycentric + z-buffer.
    pub fn triangle(
        &mut self,
        a: (f64, f64, f64),
        b: (f64, f64, f64),
        c: (f64, f64, f64),
        color: [u8; 3],
    ) {
        let area = edge(a, b, c);
        if area.abs() < 1e-9 {
            return;
        }
        let min_x = a.0.min(b.0).min(c.0).floor().max(0.0) as i32;
        let max_x = a.0.max(b.0).max(c.0).ceil().min((self.width - 1) as f64) as i32;
        let min_y = a.1.min(b.1).min(c.1).floor().max(0.0) as i32;
        let max_y = a.1.max(b.1).max(c.1).ceil().min((self.height - 1) as f64) as i32;
        for y in min_y..=max_y {
            for x in min_x..=max_x {
                let p = (x as f64 + 0.5, y as f64 + 0.5, 0.0);
                let w0 = edge(b, c, p) / area;
                let w1 = edge(c, a, p) / area;
                let w2 = edge(a, b, p) / area;
                if w0 >= 0.0 && w1 >= 0.0 && w2 >= 0.0 {
                    let z = w0 * a.2 + w1 * b.2 + w2 * c.2;
                    self.plot(x, y, z, color);
                }
            }
        }
    }

    pub fn pixel(&self, x: usize, y: usize) -> [u8; 3] {
        self.color[y * self.width + x]
    }

    /// Idadi ya pikseli zisizo za background (zilizochorwa).
    pub fn filled_count(&self, bg: [u8; 3]) -> usize {
        self.color.iter().filter(|c| **c != bg).count()
    }

    /// Andika picha kama PPM (P6).
    pub fn write_ppm(&self, path: &str) -> io::Result<()> {
        let mut f = File::create(path)?;
        write!(f, "P6\n{} {}\n255\n", self.width, self.height)?;
        let mut buf = Vec::with_capacity(self.color.len() * 3);
        for c in &self.color {
            buf.extend_from_slice(c);
        }
        f.write_all(&buf)?;
        Ok(())
    }
}

/// Edge function ya barycentric.
fn edge(a: (f64, f64, f64), b: (f64, f64, f64), c: (f64, f64, f64)) -> f64 {
    (b.0 - a.0) * (c.1 - a.1) - (b.1 - a.1) * (c.0 - a.0)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn clear_inaweka_background() {
        let mut fb = Framebuffer::new(4, 4);
        fb.clear([10, 20, 30]);
        assert_eq!(fb.pixel(0, 0), [10, 20, 30]);
        assert_eq!(fb.filled_count([10, 20, 30]), 0);
    }

    #[test]
    fn triangle_inachora_pikseli() {
        let mut fb = Framebuffer::new(32, 32);
        fb.clear([0, 0, 0]);
        fb.triangle((1.0, 1.0, 1.0), (30.0, 1.0, 1.0), (1.0, 30.0, 1.0), [255, 0, 0]);
        assert!(fb.filled_count([0, 0, 0]) > 100, "pembetatu haikuchorwa");
    }

    #[test]
    fn z_buffer_huzuia_nyuma() {
        let mut fb = Framebuffer::new(8, 8);
        fb.clear([0, 0, 0]);
        fb.plot(2, 2, 5.0, [255, 0, 0]);
        fb.plot(2, 2, 9.0, [0, 255, 0]); // nyuma zaidi — haipaswi kubadilisha
        assert_eq!(fb.pixel(2, 2), [255, 0, 0]);
        fb.plot(2, 2, 1.0, [0, 0, 255]); // mbele — hubadilisha
        assert_eq!(fb.pixel(2, 2), [0, 0, 255]);
    }
}
"#
    .to_string()
}

fn main_rs(name: &str) -> String {
    format!(
        r#"//! {name} — render scene ya 3D → out.ppm (std-only).
use {name}::{{Camera, Framebuffer, Mat4, Mesh, Vec3}};

fn main() {{
    let width = 640usize;
    let height = 480usize;
    let bg = [12u8, 16, 28];

    let mut fb = Framebuffer::new(width, height);
    fb.clear(bg);

    // Scene: plane + cube + sphere (kila moja na rangi yake).
    let plane = Mesh::plane(20.0, 20);
    let cube = Mesh::cube(2.5);
    let sphere = Mesh::sphere(1.6, 24, 16);

    let cam = Camera::new(
        Vec3::new(6.0, 5.0, 9.0),
        Vec3::new(0.0, 1.0, 0.0),
        Vec3::new(0.0, 1.0, 0.0),
        55.0,
        width as f64 / height as f64,
    );
    let view = cam.view();
    let proj = cam.projection(0.1, 200.0);
    let light = Vec3::new(0.4, 1.0, 0.3).normalize();

    let mut triangles = 0usize;

    // Plane (kijani) — imezungushwa kwa mzunguko mdogo ili ionekane.
    let model = Mat4::rotate_y(0.0).mul(&Mat4::translate(0.0, -1.6, 0.0));
    triangles += draw_mesh(&mut fb, &plane, &model, &view, &proj, &light, [70, 150, 90], width, height);

    // Cube (bluu) — imezungushwa.
    let model = Mat4::rotate_y(0.6).mul(&Mat4::translate(-2.6, 0.4, 0.0));
    triangles += draw_mesh(&mut fb, &cube, &model, &view, &proj, &light, [90, 150, 235], width, height);

    // Sphere (dhahabu).
    let model = Mat4::translate(2.6, 0.6, 0.0);
    triangles += draw_mesh(&mut fb, &sphere, &model, &view, &proj, &light, [225, 180, 70], width, height);

    let filled = fb.filled_count(bg);
    match fb.write_ppm("out.ppm") {{
        Ok(()) => println!("Rendered {{triangles}} triangles → out.ppm ({{filled}} pixels)"),
        Err(e) => eprintln!("write_ppm imeshindwa: {{e}}"),
    }}
}}

/// Chora mesh: badilisha → project → rasterize (flat shading).
fn draw_mesh(
    fb: &mut Framebuffer,
    mesh: &Mesh,
    model: &Mat4,
    view: &Mat4,
    proj: &Mat4,
    light: &Vec3,
    base: [u8; 3],
    width: usize,
    height: usize,
) -> usize {{
    let mut drawn = 0usize;
    for face in &mesh.faces {{
        let mut screen = [(0.0f64, 0.0f64, 0.0f64); 3];
        let mut ok = true;
        for (i, vi) in face.iter().enumerate() {{
            let world = model.transform_point(mesh.vertices[*vi]);
            let vp = view.transform_point(world);
            if vp.z >= -0.1 {{
                ok = false;
                break;
            }}
            let ndc = proj.transform_point(vp);
            let sx = (ndc.x * 0.5 + 0.5) * width as f64;
            let sy = (1.0 - (ndc.y * 0.5 + 0.5)) * height as f64;
            screen[i] = (sx, sy, -vp.z);
        }}
        if !ok {{
            continue;
        }}
        // Flat shading kwa normal ya uso (world space).
        let a = model.transform_point(mesh.vertices[face[0]]);
        let b = model.transform_point(mesh.vertices[face[1]]);
        let c = model.transform_point(mesh.vertices[face[2]]);
        let n = (b - a).cross(c - a).normalize();
        let mut intensity = n.dot(*light);
        if intensity < 0.0 {{
            intensity = -intensity;
        }}
        intensity = 0.15 + 0.85 * intensity;
        let col = [
            (base[0] as f64 * intensity).min(255.0) as u8,
            (base[1] as f64 * intensity).min(255.0) as u8,
            (base[2] as f64 * intensity).min(255.0) as u8,
        ];
        fb.triangle(screen[0], screen[1], screen[2], col);
        drawn += 1;
    }}
    drawn
}}
"#
    )
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::forge_domain::parse_domain;

    fn write_project(files: &[(String, String)], dir: &std::path::Path) {
        for (path, content) in files {
            let full = dir.join(path);
            if let Some(parent) = full.parent() {
                std::fs::create_dir_all(parent).expect("mkdir");
            }
            std::fs::write(&full, content).expect("write");
        }
    }

    #[test]
    fn rust3d_inazalisha_nyaraka_zote() {
        let domain = parse_domain("mradi wa scene3d");
        let (files, note) = generate_rust3d(&domain);
        assert!(note.contains("production"));
        let paths: Vec<&str> = files.iter().map(|(p, _)| p.as_str()).collect();
        for want in [
            "Cargo.toml", "README.md", "src/lib.rs", "src/vec3.rs", "src/mat4.rs",
            "src/mesh.rs", "src/camera.rs", "src/raster.rs", "src/main.rs",
        ] {
            assert!(paths.contains(&want), "haipo: {want}");
        }
        let raster = files.iter().find(|(p, _)| p == "src/raster.rs").expect("raster");
        assert!(raster.1.contains("z-buffer") || raster.1.contains("depth"));
        assert!(raster.1.contains("fn triangle"));
        let mesh = files.iter().find(|(p, _)| p == "src/mesh.rs").expect("mesh");
        assert!(mesh.1.contains("pub fn cube"));
        assert!(mesh.1.contains("pub fn sphere"));
    }

    #[test]
    fn rust3d_hupita_linter_ya_repo() {
        let domain = parse_domain("mradi wa scene3d");
        let (files, _) = generate_rust3d(&domain);
        let mut violations = 0;
        for (p, c) in &files {
            let lang = crate::code_audit::lang_for(p);
            let v = crate::code_audit::analyze_content(lang, c);
            if !v.is_empty() {
                violations += v.len();
                println!("VIOLATION path={p} lang={lang} -> {v:?}");
            }
        }
        assert_eq!(violations, 0, "3D ya Rust inavunja kanuni za repo");
    }

    /// Uthibitisho HALISI: mradi wa 3D unacompile + testi zinapita + huzalisha picha.
    #[test]
    #[ignore = "inahitaji cargo; endesha kwa --ignored"]
    fn rust3d_unacompile_na_kuzalisha_picha() {
        let domain = parse_domain("mradi wa scene3d");
        let (files, _) = generate_rust3d(&domain);
        let dir = std::env::temp_dir().join(format!("forge_3d_{}", std::process::id()));
        let _ = std::fs::remove_dir_all(&dir);
        write_project(&files, &dir);
        let test = std::process::Command::new("cargo")
            .args(["test", "--quiet"])
            .current_dir(&dir)
            .output()
            .expect("cargo haipo");
        println!("CARGO TEST:\n{}\n{}", String::from_utf8_lossy(&test.stdout), String::from_utf8_lossy(&test.stderr));
        assert!(test.status.success(), "cargo test imeshindwa");
        let run = std::process::Command::new("cargo")
            .args(["run", "--release", "--quiet"])
            .current_dir(&dir)
            .output()
            .expect("cargo haipo");
        println!("CARGO RUN:\n{}\n{}", String::from_utf8_lossy(&run.stdout), String::from_utf8_lossy(&run.stderr));
        assert!(run.status.success(), "cargo run imeshindwa");
        let ppm = dir.join("out.ppm");
        let meta = std::fs::metadata(&ppm).expect("out.ppm haipo");
        assert!(meta.len() > 1000, "picha ni ndogo mno: {} bytes", meta.len());
        let _ = std::fs::remove_dir_all(&dir);
    }
}
