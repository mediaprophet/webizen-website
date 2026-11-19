//! Kangaroo grass, tussock-grass, and mat-rush on the soil mesh.
//!
//! The tufts are the existing eco-intent plants
//! (`themeda-triandra`, `poa-labillardierei`, `lomandra-longifolia`).
//! This module does not rebuild those meshes. It places their authored
//! joints. Joints at or below the collar (`authored y <= 0`) are fitted once
//! with [`super::ground::fit_soil_joint`] and then copied. Wind, from each
//! species' `actuation.vibe` (`evaluate_wind_harmonics`, the tuft-scale
//! `wind_and_root_mechanics`), moves only joints above the collar, rotating
//! them about the frozen crown. The frame loop does not evaluate Vibe.

#![allow(dead_code)]

use super::ground;

/// Understorey species already lofted under `C:\github\eco-intent`.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum Species {
    ThemedaTriandra,
    PoaLabillardierei,
    LomandraLongifolia,
}

impl Species {
    pub const fn as_str(self) -> &'static str {
        match self {
            Species::ThemedaTriandra => "Themeda triandra",
            Species::PoaLabillardierei => "Poa labillardierei",
            Species::LomandraLongifolia => "Lomandra longifolia",
        }
    }

    /// Natural sway from `source/actuation.vibe`.
    const fn harmonics(self) -> (f64, f64, f64, f64) {
        match self {
            // f0 Hz, primary coefficient, tip coefficient, damping ratio.
            Species::ThemedaTriandra => (1.55, 0.34, 0.18, 0.24),
            Species::PoaLabillardierei => (1.35, 0.34, 0.18, 0.24),
            Species::LomandraLongifolia => (1.15, 0.34, 0.18, 0.24),
        }
    }
}

/// One planted tuft. `trunk_x` / `trunk_z` are world metres. `yaw` is radians about +Y.
#[derive(Clone, Copy, Debug)]
pub struct TuftSeed {
    pub species: Species,
    pub trunk_x: f64,
    pub trunk_z: f64,
    pub yaw: f64,
    pub foreground: bool,
}

struct JointDef {
    name: &'static str,
    authored: [f64; 3],
    inv_mass: f64,
}

/// Authored XPBD anchors from each species Vibe. Collar is y = 0.
fn joint_defs(species: Species) -> &'static [JointDef] {
    match species {
        Species::ThemedaTriandra => &[
            JointDef { name: "00_kangaroo_root_crown", authored: [0.0, -0.04, 0.0], inv_mass: 0.0 },
            JointDef { name: "00_fibrous_root_tuft", authored: [0.0, -0.38, 0.0], inv_mass: 0.0 },
            JointDef { name: "01_bronze_leaf_tuft", authored: [0.18, 0.36, 0.10], inv_mass: 0.60 },
            JointDef { name: "02_nodding_spathe_culms", authored: [0.22, 0.78, -0.14], inv_mass: 1.0 },
        ],
        Species::PoaLabillardierei => &[
            JointDef { name: "00_tussock_root_crown", authored: [0.0, -0.05, 0.0], inv_mass: 0.0 },
            JointDef { name: "00_fibrous_root_tuft", authored: [0.0, -0.45, 0.0], inv_mass: 0.0 },
            JointDef { name: "01_basal_sheath_collar", authored: [0.0, 0.14, 0.0], inv_mass: 0.15 },
            JointDef { name: "02_mid_tussock_blades", authored: [0.28, 0.62, 0.12], inv_mass: 0.68 },
            JointDef { name: "03_outer_blade_tips", authored: [0.52, 0.42, -0.22], inv_mass: 0.92 },
            JointDef { name: "04_flowering_panicles", authored: [0.12, 1.18, 0.08], inv_mass: 1.0 },
        ],
        Species::LomandraLongifolia => &[
            JointDef { name: "00_matrush_rhizome_base", authored: [0.0, -0.05, 0.0], inv_mass: 0.0 },
            JointDef { name: "00_rhizome_cord_roots", authored: [0.0, -0.48, 0.0], inv_mass: 0.0 },
            JointDef { name: "01_strap_leaf_rosette", authored: [0.34, 0.52, 0.16], inv_mass: 0.48 },
            JointDef { name: "02_spiky_flower_scapes", authored: [0.14, 0.76, -0.10], inv_mass: 0.65 },
        ],
    }
}

/// Camp and creek-bank tufts. Generated with the soil mesh; not a second height formula.
pub static TUFTS: &[TuftSeed] = &include!("../../../../production/ground/tufts.inc.rs");

/// Wind sample. Wave angles are already scaled by `(1 - damping_ratio)` so the
/// collar holds the tuft. Amplitude and damping come from the species actuation.
#[derive(Clone, Copy, Debug)]
pub struct WindSample {
    pub natural_sway_hz: f64,
    pub primary_wave_rad: f64,
    pub tip_flutter_rad: f64,
    pub damping_ratio: f64,
}

/// Tuft-scale `wind_and_root_mechanics`.
///
/// The grass sources name this `evaluate_wind_harmonics`. The host does not
/// run that Vibe; these are the same coefficients, applied as a runtime pose.
pub fn wind_and_root_mechanics(species: Species, t_s: f64, wind_speed_ms: f64) -> WindSample {
    let (f0, primary_coeff, tip_coeff, damping) = species.harmonics();
    let t_s = if t_s.is_finite() { t_s } else { 0.0 };
    let wind = if wind_speed_ms.is_finite() { wind_speed_ms.max(0.0) } else { 0.0 };
    let w = (wind / 14.0).clamp(0.0, 1.0);
    let hold = 1.0 - damping;
    let two_pi = std::f64::consts::TAU;
    let primary = w * primary_coeff * (two_pi * f0 * t_s).sin();
    let tip = w * tip_coeff * (two_pi * (f0 * 3.1) * t_s + 0.5).sin();
    WindSample {
        natural_sway_hz: f0,
        primary_wave_rad: primary * hold,
        tip_flutter_rad: tip * hold,
        damping_ratio: damping,
    }
}

/// A joint after the soil fit. `world` on a frozen joint is the stored pose.
#[derive(Clone, Copy, Debug)]
pub struct FittedJoint {
    pub name: &'static str,
    pub authored: [f64; 3],
    pub world: [f64; 3],
    pub frozen: bool,
    pub inv_mass: f64,
}

/// One tuft with its base already fitted. Wind must not rebuild these roots.
#[derive(Clone, Debug)]
pub struct FittedTuft {
    pub seed: TuftSeed,
    pub joints: Vec<FittedJoint>,
}

fn yaw_xz(x: f64, z: f64, yaw: f64) -> (f64, f64) {
    let (s, c) = yaw.sin_cos();
    (x * c + z * s, -x * s + z * c)
}

fn crown_index(joints: &[FittedJoint]) -> usize {
    joints
        .iter()
        .enumerate()
        .filter(|(_, j)| j.frozen)
        .max_by(|a, b| {
            a.1.authored[1]
                .partial_cmp(&b.1.authored[1])
                .unwrap_or(std::cmp::Ordering::Equal)
        })
        .map(|(i, _)| i)
        .unwrap_or(0)
}

/// Fit every joint at or below the collar and store the rest pose of the blades.
pub fn fit_tuft(seed: &TuftSeed) -> FittedTuft {
    let trunk = [seed.trunk_x, seed.trunk_z];
    let defs = joint_defs(seed.species);
    let mut joints: Vec<FittedJoint> = defs
        .iter()
        .map(|j| {
            let frozen = j.authored[1] <= 0.0;
            let world = if frozen {
                ground::fit_soil_joint(trunk, seed.yaw, j.authored)
            } else {
                [0.0, 0.0, 0.0]
            };
            FittedJoint {
                name: j.name,
                authored: j.authored,
                world,
                frozen,
                inv_mass: j.inv_mass,
            }
        })
        .collect();
    let crown = crown_index(&joints);
    let crown_authored = joints[crown].authored;
    let crown_world = joints[crown].world;
    for joint in joints.iter_mut() {
        if joint.frozen {
            continue;
        }
        let off = [
            joint.authored[0] - crown_authored[0],
            joint.authored[1] - crown_authored[1],
            joint.authored[2] - crown_authored[2],
        ];
        let (x, z) = yaw_xz(off[0], off[2], seed.yaw);
        joint.world = [crown_world[0] + x, crown_world[1] + off[1], crown_world[2] + z];
    }
    FittedTuft { seed: *seed, joints }
}

fn lean_offset(offset: [f64; 3], angle: f64) -> [f64; 3] {
    let (s, c) = angle.sin_cos();
    [offset[0] * c + offset[1] * s, -offset[0] * s + offset[1] * c, offset[2]]
}

/// Pose at time `t_s`. Frozen joints are copied from `fitted`. Blades rotate about the crown.
pub fn sway_tuft(fitted: &FittedTuft, t_s: f64, wind_speed_ms: f64) -> Vec<FittedJoint> {
    let wind = wind_and_root_mechanics(fitted.seed.species, t_s, wind_speed_ms);
    let crown = crown_index(&fitted.joints);
    let crown_authored = fitted.joints[crown].authored;
    let crown_world = fitted.joints[crown].world;
    fitted
        .joints
        .iter()
        .map(|joint| {
            if joint.frozen {
                return *joint;
            }
            let off = [
                joint.authored[0] - crown_authored[0],
                joint.authored[1] - crown_authored[1],
                joint.authored[2] - crown_authored[2],
            ];
            let reach = (off[1] / 1.2).clamp(0.0, 1.5);
            let angle = (wind.primary_wave_rad + wind.tip_flutter_rad * reach) * reach;
            let leaned = lean_offset(off, angle);
            let (x, z) = yaw_xz(leaned[0], leaned[2], fitted.seed.yaw);
            FittedJoint {
                world: [crown_world[0] + x, crown_world[1] + leaned[1], crown_world[2] + z],
                ..*joint
            }
        })
        .collect()
}

/// Fit every camp tuft once.
pub fn fit_camp() -> Vec<FittedTuft> {
    TUFTS.iter().map(fit_tuft).collect()
}

/// One eco-intent mesh inside the combined homepage vertex buffer.
#[derive(Clone, Debug)]
pub struct TuftSpan {
    pub species: Species,
    pub vertex_start: u32,
    pub vertex_count: usize,
    pub rest_at: usize,
    pub scratch_at: usize,
    pub freeze_y: f32,
    pub height: f32,
}

/// Rest pose and the reusable blade buffer for the three homepage tuft meshes.
///
/// `pose` writes into `scratch` allocated here. The frame loop uploads that
/// slice. Vertices at or below `freeze_y` stay on the rest pose, so the base
/// does not walk while the blades lean.
#[derive(Clone, Debug)]
pub struct ResidentTufts {
    rest: Vec<[f32; 3]>,
    scratch: Vec<f32>,
    spans: Vec<TuftSpan>,
    pub tip_shift_m: f32,
}

impl ResidentTufts {
    /// `meshes` is `(species, vertex_start, rest positions)` for Themeda, Poa, and Lomandra.
    pub fn from_meshes(meshes: Vec<(Species, u32, Vec<[f32; 3]>)>) -> Result<Self, &'static str> {
        if meshes.len() != 3 {
            return Err("homepage understorey needs Themeda, Poa, and Lomandra");
        }
        let mut seen = [false; 3];
        let mut rest = Vec::new();
        let mut spans = Vec::with_capacity(3);
        for (species, vertex_start, positions) in meshes {
            let slot = match species {
                Species::ThemedaTriandra => 0,
                Species::PoaLabillardierei => 1,
                Species::LomandraLongifolia => 2,
            };
            if seen[slot] {
                return Err("duplicate understorey mesh");
            }
            seen[slot] = true;
            if positions.is_empty() || positions.len() > 20_000 {
                return Err("understorey mesh vertex count");
            }
            let mut min_y = f32::MAX;
            let mut max_y = f32::MIN;
            for p in &positions {
                if !p[0].is_finite() || !p[1].is_finite() || !p[2].is_finite() {
                    return Err("understorey vertex is not finite");
                }
                min_y = min_y.min(p[1]);
                max_y = max_y.max(p[1]);
            }
            let height = (max_y - min_y).max(1.0e-3);
            let rest_at = rest.len();
            let count = positions.len();
            rest.extend(positions);
            spans.push(TuftSpan {
                species,
                vertex_start,
                vertex_count: count,
                rest_at,
                scratch_at: rest_at * 3,
                freeze_y: min_y + height * 0.12,
                height,
            });
        }
        if seen.iter().any(|on| !on) {
            return Err("homepage understorey needs Themeda, Poa, and Lomandra");
        }
        let scratch = vec![0.0; rest.len() * 3];
        Ok(Self {
            rest,
            scratch,
            spans,
            tip_shift_m: 0.0,
        })
    }

    pub fn spans(&self) -> &[TuftSpan] {
        &self.spans
    }

    pub fn scratch(&self) -> &[f32] {
        &self.scratch
    }

    /// Lean blades about the frozen base. `scratch` is the only output.
    pub fn pose(&mut self, t_s: f64, wind_speed_ms: f64) {
        let mut tip_shift = 0.0_f32;
        for span in &self.spans {
            let wind = wind_and_root_mechanics(span.species, t_s, wind_speed_ms);
            let freeze = span.freeze_y;
            let height = span.height;
            for i in 0..span.vertex_count {
                let p = self.rest[span.rest_at + i];
                let o = span.scratch_at + i * 3;
                if p[1] <= freeze {
                    self.scratch[o] = p[0];
                    self.scratch[o + 1] = p[1];
                    self.scratch[o + 2] = p[2];
                    continue;
                }
                let off = [p[0], p[1] - freeze, p[2]];
                let reach = (off[1] / height).clamp(0.0, 1.5);
                let angle = (wind.primary_wave_rad + wind.tip_flutter_rad * reach as f64) * reach as f64;
                let leaned = lean_offset([off[0] as f64, off[1] as f64, off[2] as f64], angle);
                self.scratch[o] = leaned[0] as f32;
                self.scratch[o + 1] = freeze + leaned[1] as f32;
                self.scratch[o + 2] = leaned[2] as f32;
                let dx = self.scratch[o] - p[0];
                let dy = self.scratch[o + 1] - p[1];
                let dz = self.scratch[o + 2] - p[2];
                let shift = (dx * dx + dy * dy + dz * dz).sqrt();
                if reach > 0.75 && shift > tip_shift {
                    tip_shift = shift;
                }
            }
        }
        self.tip_shift_m = tip_shift;
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn dist(a: [f64; 3], b: [f64; 3]) -> f64 {
        let d = [a[0] - b[0], a[1] - b[1], a[2] - b[2]];
        (d[0] * d[0] + d[1] * d[1] + d[2] * d[2]).sqrt()
    }

    #[test]
    fn three_species_are_planted_and_foreground_is_one_of_each() {
        assert!(TUFTS.len() > 12);
        for species in [Species::ThemedaTriandra, Species::PoaLabillardierei, Species::LomandraLongifolia] {
            assert!(TUFTS.iter().any(|t| t.species == species));
            assert_eq!(TUFTS.iter().filter(|t| t.foreground && t.species == species).count(), 1);
        }
    }

    #[test]
    fn wind_matches_species_actuation() {
        let sample = wind_and_root_mechanics(Species::ThemedaTriandra, 1.5, 6.5);
        let w = 6.5 / 14.0;
        let hold = 1.0 - 0.24;
        let two_pi = std::f64::consts::TAU;
        let primary = w * 0.34 * (two_pi * 1.55 * 1.5).sin();
        let tip = w * 0.18 * (two_pi * (1.55 * 3.1) * 1.5 + 0.5).sin();
        assert!((sample.primary_wave_rad - primary * hold).abs() < 1e-9);
        assert!((sample.tip_flutter_rad - tip * hold).abs() < 1e-9);
        assert!((sample.damping_ratio - 0.24).abs() < 1e-12);
        assert!((sample.natural_sway_hz - 1.55).abs() < 1e-12);
        let calm = wind_and_root_mechanics(Species::PoaLabillardierei, 0.4, 0.0);
        assert!(calm.primary_wave_rad.abs() < 1e-12);
        assert!(calm.tip_flutter_rad.abs() < 1e-12);
    }

    #[test]
    fn tuft_base_follows_the_contour_and_stays_put_while_blades_move() {
        let seed = TUFTS.iter().find(|t| t.species == Species::ThemedaTriandra).unwrap();
        let fitted = fit_tuft(seed);
        let frozen: Vec<_> = fitted.joints.iter().filter(|j| j.frozen).cloned().collect();
        assert!(frozen.len() >= 2);
        for joint in &frozen {
            let soil = ground::height_at(joint.world[0], joint.world[2]);
            assert!((joint.world[1] - (soil + joint.authored[1])).abs() < 1e-6, "{}", joint.name);
            assert!(joint.world[1] < soil);
            assert_eq!(joint.inv_mass, 0.0);
        }
        let crown = frozen.iter().max_by(|a, b| a.authored[1].partial_cmp(&b.authored[1]).unwrap()).unwrap();
        let deep = frozen.iter().min_by(|a, b| a.authored[1].partial_cmp(&b.authored[1]).unwrap()).unwrap();
        assert!((crown.world[1] - deep.world[1] - (crown.authored[1] - deep.authored[1])).abs() < 1e-6);

        let mut moved = false;
        let calm = sway_tuft(&fitted, 0.2, 0.0);
        for k in 0..8 {
            let posed = sway_tuft(&fitted, k as f64 * 0.17, 7.0);
            for (stored, posed_j) in fitted.joints.iter().zip(posed.iter()) {
                if stored.frozen {
                    assert_eq!(posed_j.world, stored.world);
                }
            }
            for (a, b) in calm.iter().zip(posed.iter()) {
                if !a.frozen && dist(a.world, b.world) > 1e-3 {
                    moved = true;
                }
            }
        }
        assert!(moved, "blades did not leave the frozen base");
    }

    #[test]
    fn two_tufts_on_different_ground_keep_authored_depth() {
        let mut low = &TUFTS[0];
        let mut high = &TUFTS[0];
        let mut low_y = f64::MAX;
        let mut high_y = f64::MIN;
        for tuft in TUFTS {
            let y = ground::height_at(tuft.trunk_x, tuft.trunk_z);
            if y < low_y {
                low_y = y;
                low = tuft;
            }
            if y > high_y {
                high_y = y;
                high = tuft;
            }
        }
        assert!(high_y - low_y > 0.05, "{high_y} {low_y}");
        let a = fit_tuft(low);
        let b = fit_tuft(high);
        let ca = a.joints.iter().find(|j| j.frozen).unwrap();
        let cb = b.joints.iter().find(|j| j.frozen).unwrap();
        let soil_a = ground::height_at(ca.world[0], ca.world[2]);
        let soil_b = ground::height_at(cb.world[0], cb.world[2]);
        assert!((ca.world[1] - soil_a - ca.authored[1]).abs() < 1e-6);
        assert!((cb.world[1] - soil_b - cb.authored[1]).abs() < 1e-6);
        assert!((cb.world[1] - ca.world[1] - (soil_b - soil_a) - (cb.authored[1] - ca.authored[1])).abs() < 1e-6);
    }

    #[test]
    fn posed_mesh_holds_the_base_and_moves_the_tip_without_a_second_buffer() {
        let meshes = vec![
            (Species::ThemedaTriandra, 0, vec![[0.0, 0.0, 0.0], [0.02, 1.0, 0.0]]),
            (Species::PoaLabillardierei, 10, vec![[0.0, -0.1, 0.0], [0.0, 0.9, 0.1]]),
            (Species::LomandraLongifolia, 20, vec![[0.0, 0.0, 0.0], [-0.04, 0.8, 0.0]]),
        ];
        let mut tufts = ResidentTufts::from_meshes(meshes).unwrap();
        let before = tufts.scratch().len();
        tufts.pose(0.2, 0.0);
        assert!(tufts.tip_shift_m < 1.0e-4);
        assert_eq!(tufts.scratch()[0], 0.0);
        assert_eq!(tufts.scratch()[1], 0.0);
        tufts.pose(0.35, 8.5);
        assert_eq!(tufts.scratch().len(), before);
        assert_eq!(tufts.scratch()[0], 0.0);
        assert_eq!(tufts.scratch()[1], 0.0);
        assert_eq!(tufts.scratch()[2], 0.0);
        let tip = (tufts.scratch()[3], tufts.scratch()[4], tufts.scratch()[5]);
        assert!((tip.0 - 0.02).abs() + (tip.1 - 1.0).abs() > 1.0e-3);
        assert!(tufts.tip_shift_m > 1.0e-3);
    }
}
