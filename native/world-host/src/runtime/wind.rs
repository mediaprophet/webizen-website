//! Runtime pose for the four eco-intent eucalypts.
//!
//! The frame loop does not evaluate Vibe. Each root joint is fitted once with
//! [`ground::fit_soil_joint`] and then copied. Wind, from each species'
//! `wind_and_root_mechanics` / `evaluate_wind_harmonics`, rotates only the
//! joints above the collar about that fitted plate.

#![allow(dead_code)]

#[cfg(flora_standalone)]
#[path = "ground.rs"]
mod ground;
#[cfg(all(not(flora_standalone), target_arch = "wasm32"))]
use super::super::ground;
#[cfg(all(not(flora_standalone), not(target_arch = "wasm32")))]
use super::ground;

#[path = "instruments/rhizosphere.rs"]
mod rhizosphere;

pub use rhizosphere::DrawView;

use std::sync::OnceLock;

/// Eucalypts whose lofted roots already exist under `C:\github\eco-intent`.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
#[repr(u8)]
pub enum WoodySpecies {
    Camaldulensis = 0,
    Melliodora = 1,
    Dalrympleana = 2,
    Regnans = 3,
}

impl WoodySpecies {
    pub const fn as_str(self) -> &'static str {
        match self {
            WoodySpecies::Camaldulensis => "Eucalyptus camaldulensis",
            WoodySpecies::Melliodora => "Eucalyptus melliodora",
            WoodySpecies::Dalrympleana => "Eucalyptus dalrympleana",
            WoodySpecies::Regnans => "Eucalyptus regnans",
        }
    }

    pub const fn crown_ref_m(self) -> f64 {
        match self {
            WoodySpecies::Camaldulensis => 19.0,
            WoodySpecies::Melliodora => 16.0,
            WoodySpecies::Dalrympleana => 18.0,
            WoodySpecies::Regnans => 40.0,
        }
    }

    pub const fn all() -> [WoodySpecies; 4] {
        [
            WoodySpecies::Camaldulensis,
            WoodySpecies::Melliodora,
            WoodySpecies::Dalrympleana,
            WoodySpecies::Regnans,
        ]
    }
}

/// One planted gum. `trunk_x` / `trunk_z` are world metres. `yaw` is radians about +Y.
#[derive(Clone, Copy, Debug)]
pub struct GumSeed {
    pub species: WoodySpecies,
    pub trunk_x: f64,
    pub trunk_z: f64,
    pub yaw: f64,
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum JointRole {
    Root,
    Bole,
    Bough,
    Sprig,
}

impl JointRole {
    fn from_u8(value: u8) -> Self {
        match value {
            1 => JointRole::Bole,
            2 => JointRole::Bough,
            3 => JointRole::Sprig,
            _ => JointRole::Root,
        }
    }

    fn as_u8(self) -> u8 {
        match self {
            JointRole::Root => 0,
            JointRole::Bole => 1,
            JointRole::Bough => 2,
            JointRole::Sprig => 3,
        }
    }
}

/// A joint after the soil fit. Frozen `world` is the stored pose.
#[derive(Clone, Copy, Debug)]
pub struct FittedJoint {
    pub part: &'static str,
    pub part_index: u16,
    pub section: u16,
    pub role: JointRole,
    pub order: u8,
    pub authored: [f64; 3],
    pub world: [f64; 3],
    pub inv_mass: f64,
    pub frozen: bool,
}

/// One gum with its roots already fitted. Wind must not rebuild those roots.
#[derive(Clone, Debug)]
pub struct FittedTree {
    pub seed: GumSeed,
    pub joints: Vec<FittedJoint>,
    pub plate: usize,
}

/// Wind sample. Bend angles are still multiplied by `(1 - damping_ratio)` when posed,
/// so the plate holds the tree. Coefficients come from the species actuation.
#[derive(Clone, Copy, Debug)]
pub struct WindMechanics {
    pub bough_bend_rad: f64,
    pub sprig_flutter_rad: f64,
    pub bole_deflection_rad: f64,
    pub damping_ratio: f64,
    pub bough_hz: f64,
    pub anchorage: f64,
}

/// Published river-red-gum sample in `actuation.vibe`: `wind_and_root_mechanics(2.5, 8.5, 0.35, 0.42)`.
pub const DEMO_TIME_S: f64 = 2.5;
pub const DEMO_WIND_MS: f64 = 8.5;
pub const DEMO_BIAS: f64 = 0.35;
pub const DEMO_SATURATION: f64 = 0.42;

const PLATE_NAMES: &[&str] = &[
    "lignotuber_root_plate",
    "00_lignotuber_anchor",
    "lignotuber_anchor",
    "00_buttress_anchor",
];

fn clamp01(value: f64) -> f64 {
    if value.is_finite() {
        value.clamp(0.0, 1.0)
    } else {
        0.0
    }
}

fn finite(value: f64, default: f64) -> f64 {
    if value.is_finite() {
        value
    } else {
        default
    }
}

/// Species wind. River red gum matches `wind_and_root_mechanics`. The other three
/// map `evaluate_wind_harmonics` into the same fields. Soil saturation is the red
/// gum's anchorage term; the other species do not take it.
pub fn wind_and_root_mechanics(
    species: WoodySpecies,
    t_s: f64,
    wind_speed_ms: f64,
    wind_bias: f64,
    soil_saturation_01: f64,
) -> WindMechanics {
    let t = finite(t_s, 0.0);
    let wind = finite(wind_speed_ms, 0.0).max(0.0);
    let bias = finite(wind_bias, 0.0);
    let sat = finite(soil_saturation_01, 0.0);
    let tau = std::f64::consts::TAU;
    match species {
        WoodySpecies::Camaldulensis => {
            let w = clamp01(wind / 18.0);
            let saturation = clamp01(sat);
            let anchorage = 1.0 - 0.18 * saturation;
            let damping = 0.24 + (0.16 - 0.24) * saturation;
            let bough_hz = 0.22;
            let bough_sway = (w / anchorage)
                * (0.38 * (tau * bough_hz * t).sin() + 0.11 * (tau * 0.66 * t + 0.4).sin());
            let sprig =
                w * (0.14 * (tau * 3.80 * t + 1.2).sin() + 0.05 * (tau * 5.7 * t).cos());
            WindMechanics {
                bough_bend_rad: 0.16 * clamp01(bias) + bough_sway * 0.22,
                sprig_flutter_rad: sprig * 0.48,
                bole_deflection_rad: 0.0,
                damping_ratio: damping,
                bough_hz,
                anchorage,
            }
        }
        WoodySpecies::Melliodora => harmonics(
            wind,
            bias,
            t,
            18.0,
            12.0,
            0.68,
            0.32,
            0.16,
            0.32,
            0.34,
            0.09,
            0.72,
            0.4,
            3.8,
            0.9,
            5.2,
            0.18,
            0.06,
            0.014,
            0.060,
            0.16,
            0.34,
            0.42,
            0.16,
            0.32,
        ),
        WoodySpecies::Dalrympleana => harmonics(
            wind, bias, t, 18.0, 12.0, 0.65, 0.35, 0.18, 0.25, 0.36, 0.10, 0.65, 0.3, 4.2, 1.1,
            6.3, 0.16, 0.05, 0.018, 0.065, 0.18, 0.32, 0.45, 0.14, 0.25,
        ),
        WoodySpecies::Regnans => harmonics(
            wind, bias, t, 22.0, 14.0, 0.65, 0.35, 0.14, 0.34, 0.34, 0.12, 0.72, 0.25, 4.1, 0.9,
            5.4, 0.18, 0.06, 0.024, 0.068, 0.16, 0.34, 0.46, 0.16, 0.34,
        ),
    }
}

fn harmonics(
    wind: f64,
    bias: f64,
    t: f64,
    wind_cap: f64,
    load_div: f64,
    gust_base: f64,
    gust_amp: f64,
    gust_hz: f64,
    bough_hz: f64,
    sway_a: f64,
    sway_b: f64,
    sway_b_hz: f64,
    sway_b_phase: f64,
    sprig_hz: f64,
    sprig_phase: f64,
    sprig_cos_hz: f64,
    sprig_a: f64,
    sprig_b: f64,
    bole_k: f64,
    bough_load_k: f64,
    bough_sway_k: f64,
    leaf_load_k: f64,
    leaf_sprig_k: f64,
    damping: f64,
    reported_hz: f64,
) -> WindMechanics {
    let tau = std::f64::consts::TAU;
    let w = clamp01(wind / wind_cap);
    let gust = gust_base + gust_amp * (0.5 + 0.5 * (tau * gust_hz * t).sin());
    let load = (wind / load_div) * (0.6 + 0.4 * clamp01(bias)) * gust;
    let bough_sway =
        w * (sway_a * (tau * bough_hz * t).sin() + sway_b * (tau * sway_b_hz * t + sway_b_phase).sin());
    let sprig =
        w * (sprig_a * (tau * sprig_hz * t + sprig_phase).sin() + sprig_b * (tau * sprig_cos_hz * t).cos());
    WindMechanics {
        bough_bend_rad: bough_load_k * load + bough_sway_k * bough_sway,
        sprig_flutter_rad: leaf_load_k * load + leaf_sprig_k * sprig,
        bole_deflection_rad: bole_k * load,
        damping_ratio: damping,
        bough_hz: reported_hz,
        anchorage: 1.0,
    }
}

fn yaw_xz(x: f64, z: f64, yaw: f64) -> (f64, f64) {
    let (s, c) = yaw.sin_cos();
    (x * c + z * s, -x * s + z * c)
}

fn lean_xy(offset: [f64; 3], angle: f64) -> [f64; 3] {
    let (s, c) = angle.sin_cos();
    [
        offset[0] * c + offset[1] * s,
        -offset[0] * s + offset[1] * c,
        offset[2],
    ]
}

fn plate_index(joints: &[FittedJoint]) -> usize {
    if let Some(index) = joints.iter().position(|joint| PLATE_NAMES.contains(&joint.part)) {
        return index;
    }
    joints
        .iter()
        .enumerate()
        .filter(|(_, joint)| {
            joint.frozen && joint.authored[0].abs() < 0.25 && joint.authored[2].abs() < 0.25
        })
        .min_by(|a, b| {
            a.1.authored[1]
                .abs()
                .partial_cmp(&b.1.authored[1].abs())
                .unwrap_or(std::cmp::Ordering::Equal)
        })
        .map(|(index, _)| index)
        .unwrap_or(0)
}

/// Fit every root joint onto the soil contour and store the crown on that plate.
///
/// Root `world` is [`ground::fit_soil_joint`]. Crown `world` is the rigid offset
/// from the fitted plate, yawed about the trunk. A later [`pose_wind`] recomputes
/// the crown from those authored offsets and copies the roots.
pub fn fit_tree(seed: GumSeed) -> FittedTree {
    let trunk = [seed.trunk_x, seed.trunk_z];
    let yaw = finite(seed.yaw, 0.0);
    let mut joints = Vec::new();
    for record in rhizosphere::JOINTS {
        if record.0 != seed.species as u8 {
            continue;
        }
        let authored = [record.5 as f64, record.6 as f64, record.7 as f64];
        let role = JointRole::from_u8(record.1);
        let inv_mass = record.8 as f64 / 1000.0;
        let frozen = role == JointRole::Root || inv_mass == 0.0 || authored[1] <= 0.0;
        let world = if frozen {
            ground::fit_soil_joint(trunk, yaw, authored)
        } else {
            [0.0, 0.0, 0.0]
        };
        joints.push(FittedJoint {
            part: rhizosphere::PARTS[record.3 as usize],
            part_index: record.3,
            section: record.4,
            role,
            order: record.2,
            authored,
            world,
            inv_mass,
            frozen,
        });
    }
    assert!(!joints.is_empty(), "no joints for {}", seed.species.as_str());
    let plate = plate_index(&joints);
    let plate_authored = joints[plate].authored;
    let plate_world = joints[plate].world;
    for joint in joints.iter_mut() {
        if joint.frozen {
            continue;
        }
        let off = [
            joint.authored[0] - plate_authored[0],
            joint.authored[1] - plate_authored[1],
            joint.authored[2] - plate_authored[2],
        ];
        let (x, z) = yaw_xz(off[0], off[2], yaw);
        joint.world = [plate_world[0] + x, plate_world[1] + off[1], plate_world[2] + z];
    }
    FittedTree {
        seed: GumSeed { yaw, ..seed },
        joints,
        plate,
    }
}

/// Pose at time `t_s`. Frozen joints are copied from `fitted`. The crown rotates
/// about the fitted plate. This does not integrate the previous pose.
pub fn pose_wind(
    fitted: &FittedTree,
    t_s: f64,
    wind_speed_ms: f64,
    wind_bias: f64,
    soil_saturation_01: f64,
) -> Vec<FittedJoint> {
    let wind = wind_and_root_mechanics(
        fitted.seed.species,
        t_s,
        wind_speed_ms,
        wind_bias,
        soil_saturation_01,
    );
    let plate = fitted.joints[fitted.plate];
    let hold = 1.0 - wind.damping_ratio;
    let crown_ref = fitted.seed.species.crown_ref_m();
    fitted
        .joints
        .iter()
        .map(|joint| {
            if joint.frozen {
                return *joint;
            }
            let off = [
                joint.authored[0] - plate.authored[0],
                joint.authored[1] - plate.authored[1],
                joint.authored[2] - plate.authored[2],
            ];
            let reach = (off[1] / crown_ref).clamp(0.0, 1.6);
            let bend = match joint.role {
                JointRole::Bole => 0.35 * wind.bough_bend_rad + wind.bole_deflection_rad,
                JointRole::Bough => wind.bough_bend_rad,
                JointRole::Sprig => wind.bough_bend_rad + wind.sprig_flutter_rad,
                JointRole::Root => 0.0,
            };
            let leaned = lean_xy(off, bend * hold * reach);
            let (x, z) = yaw_xz(leaned[0], leaned[2], fitted.seed.yaw);
            FittedJoint {
                world: [plate.world[0] + x, plate.world[1] + leaned[1], plate.world[2] + z],
                ..*joint
            }
        })
        .collect()
}

pub fn view_joints(joints: &[FittedJoint], view: DrawView) -> Vec<&FittedJoint> {
    joints
        .iter()
        .filter(|joint| {
            rhizosphere::visible(view, joint.role.as_u8(), joint.authored[1], joint.inv_mass)
        })
        .collect()
}

#[derive(Clone, Copy)]
struct Tip {
    part: &'static str,
    authored: [f64; 3],
}

fn primary_tips(species: WoodySpecies) -> Vec<Tip> {
    let mut tips: Vec<(u16, u16, Tip)> = Vec::new();
    for record in rhizosphere::JOINTS {
        if record.0 != species as u8 || record.2 != 2 {
            continue;
        }
        let tip = Tip {
            part: rhizosphere::PARTS[record.3 as usize],
            authored: [record.5 as f64, record.6 as f64, record.7 as f64],
        };
        if let Some(slot) = tips.iter_mut().find(|slot| slot.0 == record.3) {
            if record.4 >= slot.1 {
                *slot = (record.3, record.4, tip);
            }
        } else {
            tips.push((record.3, record.4, tip));
        }
    }
    tips.into_iter().map(|slot| slot.2).collect()
}

fn most_opposite(tips: &[Tip]) -> (Tip, Tip) {
    let mut best: Option<(f64, usize, usize)> = None;
    for i in 0..tips.len() {
        for j in (i + 1)..tips.len() {
            let dot = tips[i].authored[0] * tips[j].authored[0] + tips[i].authored[2] * tips[j].authored[2];
            if best.map(|(so_far, _, _)| dot < so_far).unwrap_or(true) {
                best = Some((dot, i, j));
            }
        }
    }
    let (_, i, j) = best.expect("a gum needs two primary laterals");
    (tips[i], tips[j])
}

/// Where one river-red-gum lateral runs onto a rise and the opposite lateral into a gully.
#[derive(Clone, Copy, Debug)]
pub struct ContourSite {
    pub seed: GumSeed,
    pub rise_m: f64,
    pub fall_m: f64,
    pub plane_gap_m: f64,
    pub collar_y: f64,
    pub gully_tip: [f64; 3],
    pub rise_tip: [f64; 3],
    pub gully_authored_y: f64,
    pub rise_authored_y: f64,
    pub gully_part: &'static str,
    pub rise_part: &'static str,
}

/// Trunk where the two opposite red-gum laterals leave a shared tangent plane.
pub fn contour_demonstration_site() -> ContourSite {
    static SITE: OnceLock<ContourSite> = OnceLock::new();
    *SITE.get_or_init(search_contour_site)
}

fn search_contour_site() -> ContourSite {
    let (tip_a, tip_b) = most_opposite(&primary_tips(WoodySpecies::Camaldulensis));
    let yaws = [0.0, 0.45, 0.9, 1.35, 1.8, 2.25, 2.7, 3.15];
    let mut best: Option<(f64, ContourSite)> = None;
    let mut near_rise = 0.0_f64;
    let mut near_gap = 0.0_f64;
    let mut x = -270.0;
    while x <= 270.0 {
        let mut z = -270.0;
        while z <= 270.0 {
            let collar = ground::height_at(x, z);
            if collar < 1.0 {
                z += 5.0;
                continue;
            }
            let normal = ground::normal_at(x, z);
            if normal[1] < 0.72 {
                z += 5.0;
                continue;
            }
            for &yaw in &yaws {
                let world_a = ground::fit_soil_joint([x, z], yaw, tip_a.authored);
                let world_b = ground::fit_soil_joint([x, z], yaw, tip_b.authored);
                let soil_a = ground::height_at(world_a[0], world_a[2]);
                let soil_b = ground::height_at(world_b[0], world_b[2]);
                let delta_a = soil_a - collar;
                let delta_b = soil_b - collar;
                if delta_a * delta_b >= 0.0 {
                    continue;
                }
                let (gully, rise, gully_tip, rise_tip, gully_soil, rise_soil) = if soil_a < soil_b {
                    (world_a, world_b, tip_a, tip_b, soil_a, soil_b)
                } else {
                    (world_b, world_a, tip_b, tip_a, soil_b, soil_a)
                };
                let fall = collar - gully_soil;
                let rise_m = rise_soil - collar;
                let plane = |point: [f64; 3], authored_y: f64| {
                    collar - (normal[0] * (point[0] - x) + normal[2] * (point[2] - z)) / normal[1]
                        + authored_y
                };
                let fitted_delta = gully[1] - rise[1];
                let plane_delta = plane(gully, gully_tip.authored[1]) - plane(rise, rise_tip.authored[1]);
                let gap = (fitted_delta - plane_delta).abs();
                if rise_m + fall + gap > near_rise + near_gap {
                    near_rise = rise_m + fall;
                    near_gap = gap;
                }
                if rise_m < 0.40 || fall < 0.40 || gap < 0.18 || gap > 6.0 {
                    continue;
                }
                let dist = ((x + 22.0).powi(2) + (z - 28.0).powi(2)).sqrt();
                let score = gap + rise_m + fall - dist * 0.002;
                let site = ContourSite {
                    seed: GumSeed {
                        species: WoodySpecies::Camaldulensis,
                        trunk_x: x,
                        trunk_z: z,
                        yaw,
                    },
                    rise_m,
                    fall_m: fall,
                    plane_gap_m: gap,
                    collar_y: collar,
                    gully_tip: gully,
                    rise_tip: rise,
                    gully_authored_y: gully_tip.authored[1],
                    rise_authored_y: rise_tip.authored[1],
                    gully_part: gully_tip.part,
                    rise_part: rise_tip.part,
                };
                if best.map(|(so_far, _)| score > so_far).unwrap_or(true) {
                    best = Some((score, site));
                }
            }
            z += 5.0;
        }
        x += 5.0;
    }
    best.map(|(_, site)| site).unwrap_or_else(|| {
        panic!(
            "no red-gum site with a rise, a gully, and a contour gap (nearest relief {near_rise:.2} m, gap {near_gap:.2} m)"
        )
    })
}

/// Two times where the red-gum crown bend differs. Roots are not consulted.
pub fn wind_contrast_times(species: WoodySpecies) -> (f64, f64) {
    let mut best = (0.15_f64, 2.6_f64, 0.0_f64);
    let mut t = 0.0;
    while t <= 10.0 {
        let mut u = t + 0.2;
        while u <= 10.0 {
            let a = wind_and_root_mechanics(species, t, DEMO_WIND_MS, DEMO_BIAS, DEMO_SATURATION);
            let b = wind_and_root_mechanics(species, u, DEMO_WIND_MS, DEMO_BIAS, DEMO_SATURATION);
            let delta = (a.bough_bend_rad - b.bough_bend_rad).abs()
                + (a.sprig_flutter_rad - b.sprig_flutter_rad).abs();
            if delta > best.2 {
                best = (t, u, delta);
            }
            u += 0.25;
        }
        t += 0.25;
    }
    (best.0, best.1)
}

#[cfg(test)]
mod tests {
    use super::ground;
    use super::*;
    use std::collections::BTreeSet;

    fn dist(a: [f64; 3], b: [f64; 3]) -> f64 {
        let d = [a[0] - b[0], a[1] - b[1], a[2] - b[2]];
        (d[0] * d[0] + d[1] * d[1] + d[2] * d[2]).sqrt()
    }

    #[test]
    fn red_gum_wind_matches_the_actuation_sample() {
        let sample = wind_and_root_mechanics(
            WoodySpecies::Camaldulensis,
            DEMO_TIME_S,
            DEMO_WIND_MS,
            DEMO_BIAS,
            DEMO_SATURATION,
        );
        let tau = std::f64::consts::TAU;
        let w = DEMO_WIND_MS / 18.0;
        let sat = DEMO_SATURATION;
        let anchorage = 1.0 - 0.18 * sat;
        let damping = 0.24 + (0.16 - 0.24) * sat;
        let bough_sway = (w / anchorage)
            * (0.38 * (tau * 0.22 * DEMO_TIME_S).sin()
                + 0.11 * (tau * 0.66 * DEMO_TIME_S + 0.4).sin());
        let sprig = w
            * (0.14 * (tau * 3.80 * DEMO_TIME_S + 1.2).sin()
                + 0.05 * (tau * 5.7 * DEMO_TIME_S).cos());
        let bend = 0.16 * DEMO_BIAS + bough_sway * 0.22;
        assert!((sample.bough_bend_rad - bend).abs() < 1e-12);
        assert!((sample.sprig_flutter_rad - sprig * 0.48).abs() < 1e-12);
        assert!((sample.damping_ratio - damping).abs() < 1e-12);
        assert!((sample.anchorage - anchorage).abs() < 1e-12);
        assert!((sample.bough_hz - 0.22).abs() < 1e-12);
        assert_eq!(sample.bole_deflection_rad, 0.0);
        let calm = wind_and_root_mechanics(WoodySpecies::Melliodora, 0.4, 0.0, 0.0, 0.2);
        assert!(calm.bough_bend_rad.abs() < 1e-12);
        assert!(calm.sprig_flutter_rad.abs() < 1e-12);
        assert!((calm.damping_ratio - 0.16).abs() < 1e-12);
    }

    #[test]
    fn red_gum_anchors_are_the_rigid_xpbd_bodies() {
        let fitted = fit_tree(GumSeed {
            species: WoodySpecies::Camaldulensis,
            trunk_x: 0.0,
            trunk_z: 0.0,
            yaw: 0.0,
        });
        let deep = fitted
            .joints
            .iter()
            .find(|joint| joint.part == "taproot_deep_anchor")
            .unwrap();
        let plate = fitted
            .joints
            .iter()
            .find(|joint| joint.part == "lignotuber_root_plate")
            .unwrap();
        assert!(deep.authored[0].abs() < 1e-6 && deep.authored[2].abs() < 1e-6);
        assert!((deep.authored[1] + 5.2).abs() < 1e-6);
        assert_eq!(deep.inv_mass, 0.0);
        assert!(deep.frozen);
        assert!(plate.authored[0].abs() < 1e-6 && plate.authored[2].abs() < 1e-6);
        assert!((plate.authored[1] + 0.15).abs() < 1e-6);
        assert_eq!(plate.inv_mass, 0.0);
        assert!(plate.frozen);
        assert_eq!(fitted.joints[fitted.plate].part, "lignotuber_root_plate");
    }

    #[test]
    fn opposite_laterals_follow_the_contour_not_a_shared_plane() {
        let site = contour_demonstration_site();
        assert!(site.gully_authored_y < 0.0 && site.rise_authored_y < 0.0, "{site:?}");
        assert!(site.fall_m >= 0.40 && site.rise_m >= 0.40, "{site:?}");
        assert!(site.plane_gap_m > 0.15, "{site:?}");
        let gully_soil = ground::height_at(site.gully_tip[0], site.gully_tip[2]);
        let rise_soil = ground::height_at(site.rise_tip[0], site.rise_tip[2]);
        assert!((site.gully_tip[1] - (gully_soil + site.gully_authored_y)).abs() < 1e-6);
        assert!((site.rise_tip[1] - (rise_soil + site.rise_authored_y)).abs() < 1e-6);
        assert!(site.gully_tip[1] < gully_soil);
        assert!(site.rise_tip[1] < rise_soil);
        assert!(gully_soil < site.collar_y);
        assert!(rise_soil > site.collar_y);
        let normal = ground::normal_at(site.seed.trunk_x, site.seed.trunk_z);
        let plane = |point: [f64; 3], authored_y: f64| {
            site.collar_y
                - (normal[0] * (point[0] - site.seed.trunk_x) + normal[2] * (point[2] - site.seed.trunk_z))
                    / normal[1]
                + authored_y
        };
        let fitted_delta = site.gully_tip[1] - site.rise_tip[1];
        let plane_delta =
            plane(site.gully_tip, site.gully_authored_y) - plane(site.rise_tip, site.rise_authored_y);
        assert!((fitted_delta - plane_delta).abs() > 0.15);
        let fitted = fit_tree(site.seed);
        for name in [
            "taproot_sinker",
            "primary_lateral_root_1",
            "primary_lateral_root_2",
            "primary_lateral_root_3",
            "primary_lateral_root_4",
            "primary_lateral_root_5",
            "primary_lateral_root_6",
            "primary_lateral_root_7",
        ] {
            assert!(fitted.joints.iter().any(|joint| joint.part == name), "{name}");
        }
        assert!(fitted.joints.iter().any(|joint| joint.part.starts_with("secondary_")));
        assert!(fitted
            .joints
            .iter()
            .any(|joint| joint.part.contains("sinker") || joint.part.contains("feeder")));
    }

    #[test]
    fn four_species_keep_authored_depth_under_their_own_soil() {
        let site = contour_demonstration_site();
        for species in WoodySpecies::all() {
            let fitted = fit_tree(GumSeed {
                species,
                trunk_x: site.seed.trunk_x,
                trunk_z: site.seed.trunk_z,
                yaw: site.seed.yaw,
            });
            let mut primaries = BTreeSet::new();
            let mut deeper = false;
            let mut buried = 0_usize;
            for joint in &fitted.joints {
                if joint.order == 2 {
                    primaries.insert(joint.part);
                }
                if (3..=5).contains(&joint.order) {
                    deeper = true;
                }
                if joint.authored[1] < 0.0 {
                    buried += 1;
                    assert!(joint.frozen, "{}", joint.part);
                    assert_eq!(joint.inv_mass, 0.0, "{}", joint.part);
                    let soil = ground::height_at(joint.world[0], joint.world[2]);
                    assert!(
                        (joint.world[1] - (soil + joint.authored[1])).abs() < 1e-6,
                        "{} world {} soil {} authored {}",
                        joint.part,
                        joint.world[1],
                        soil,
                        joint.authored[1]
                    );
                    assert!(joint.world[1] < soil, "{}", joint.part);
                }
            }
            assert!(primaries.len() >= 7, "{} {:?}", species.as_str(), primaries);
            assert!(deeper, "{}", species.as_str());
            assert!(buried > 20, "{} buried {buried}", species.as_str());
        }
    }

    #[test]
    fn wind_moves_the_crown_and_leaves_the_fitted_roots() {
        let site = contour_demonstration_site();
        let fitted = fit_tree(site.seed);
        let stored: Vec<_> = fitted.joints.iter().map(|joint| joint.world).collect();
        let (t0, t1) = wind_contrast_times(WoodySpecies::Camaldulensis);
        let early = pose_wind(&fitted, t0, DEMO_WIND_MS, DEMO_BIAS, DEMO_SATURATION);
        let later = pose_wind(&fitted, t1, DEMO_WIND_MS, DEMO_BIAS, DEMO_SATURATION);
        assert_eq!(
            stored,
            fitted.joints.iter().map(|joint| joint.world).collect::<Vec<_>>()
        );
        let calm = pose_wind(&fitted, 1.25, 0.0, 0.0, DEMO_SATURATION);
        for (joint, posed) in fitted.joints.iter().zip(&calm) {
            assert_eq!(joint.world, posed.world);
        }
        let mut crown = 0.0_f64;
        for (index, (left, right)) in early.iter().zip(&later).enumerate() {
            if fitted.joints[index].frozen {
                assert_eq!(left.world, right.world);
                assert_eq!(left.world, fitted.joints[index].world);
                assert_eq!(left.inv_mass, 0.0);
            } else {
                crown = crown.max(dist(left.world, right.world));
            }
        }
        assert!(crown > 0.15, "crown shift {crown} m between {t0} s and {t1} s");
        for name in ["bole_collar", "bole_mid", "crown_fork", "bough_east", "bough_west"] {
            let shift = part_shift(&fitted, &early, &later, name);
            assert!(shift > 1.0e-4, "{name} shift {shift}");
        }
        let sprig = part_shift(&fitted, &early, &later, "canopy_apex")
            .max(part_shift(&fitted, &early, &later, "pendulous_sprig_flutter"));
        assert!(sprig > 0.05, "sprig shift {sprig}");
    }

    fn part_shift(fitted: &FittedTree, early: &[FittedJoint], later: &[FittedJoint], part: &str) -> f64 {
        fitted
            .joints
            .iter()
            .zip(early)
            .zip(later)
            .filter(|((joint, _), _)| joint.part == part)
            .map(|((_, left), right)| dist(left.world, right.world))
            .fold(0.0, f64::max)
    }

    #[test]
    fn normal_view_and_rhizosphere_are_different_lenses() {
        let fitted = fit_tree(contour_demonstration_site().seed);
        let normal = view_joints(&fitted.joints, DrawView::Normal);
        let rhizosphere = view_joints(&fitted.joints, DrawView::Rhizosphere);
        assert!(normal.iter().all(|joint| joint.role != JointRole::Root));
        assert!(normal.iter().all(|joint| joint.authored[1] > 0.0));
        assert!(normal.iter().any(|joint| joint.part == "bole_collar"));
        assert!(normal.iter().any(|joint| joint.part == "crown_fork"));
        assert!(rhizosphere.iter().any(|joint| joint.part == "taproot_sinker"));
        assert!(rhizosphere.iter().any(|joint| joint.part == "primary_lateral_root_1"));
        assert!(rhizosphere.iter().any(|joint| joint.part.starts_with("secondary_")));
        assert_ne!(normal.len(), rhizosphere.len());
        for joint in &normal {
            assert!(!rhizosphere::visible(
                DrawView::Rhizosphere,
                joint.role.as_u8(),
                joint.authored[1],
                joint.inv_mass
            ));
        }
    }
}

#[cfg(all(flora_evidence, not(flora_standalone)))]
compile_error!("the evidence binary needs --cfg flora_standalone");

#[cfg(flora_evidence)]
fn main() {
    evidence::render();
}

#[cfg(flora_evidence)]
mod evidence {
    use super::ground::{self, GroundMaterial};
    use super::*;

    const W: i32 = 1280;
    const H: i32 = 720;

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

    struct Rect {
        x: f64,
        y: f64,
        w: f64,
        h: f64,
    }

    struct Axis {
        origin_x: f64,
        origin_z: f64,
        ux: f64,
        uz: f64,
        span: f64,
        y_min: f64,
        y_max: f64,
    }

    pub fn render() {
        let out = std::path::Path::new(r"C:\github\webizen-website\evidence\world-recovery\WR-02");
        std::fs::create_dir_all(out).unwrap();
        let site = contour_demonstration_site();
        let fitted = fit_tree(site.seed);
        let (t0, t1) = wind_contrast_times(WoodySpecies::Camaldulensis);
        let early = pose_wind(&fitted, t0, DEMO_WIND_MS, DEMO_BIAS, DEMO_SATURATION);
        let later = pose_wind(&fitted, t1, DEMO_WIND_MS, DEMO_BIAS, DEMO_SATURATION);
        let root_delta = max_frozen_delta(&early, &later);
        let crown_delta = max_free_delta(&early, &later);
        assert!(root_delta == 0.0, "root joints moved {root_delta} m");
        assert!(crown_delta > 0.15, "crown shift {crown_delta} m");

        let normal_cam = normal_camera(&site);
        let (mut rgb, counts, water, sky, mut zbuf) = raster(&normal_cam);
        let crown_px = draw_crown(&normal_cam, &fitted.joints, &mut rgb, &mut zbuf, false);
        assert!(sky > 80_000, "normal view has no sky ({sky} px); the camera is in the hill");
        assert!(crown_px > 2_000, "normal view drew {crown_px} crown pixels");
        write_png(&out.join("normal-uneven.png"), &rgb);

        let mut cut = vec![0u8; (W * H * 3) as usize];
        for pixel in cut.chunks_exact_mut(3) {
            pixel.copy_from_slice(&[186, 206, 214]);
        }
        paint_profile(
            &mut cut,
            &full_rect(),
            &profile_axis(&site, &fitted.joints),
            &[&fitted.joints],
            &[[122, 78, 46]],
        );
        write_png(&out.join("rhizosphere-contour.png"), &cut);

        let wind_cam = wind_camera(&site, &early, &later);
        let apex_px = apex_pixels(&wind_cam, &early, &later);
        assert!(apex_px > 36.0, "crown screen shift {apex_px} px");
        let (mut wind_rgb, wind_counts, wind_water, wind_sky, mut wind_z) = raster(&wind_cam);
        let _ = wind_sky;
        let ghost_px = draw_crown(&wind_cam, &early, &mut wind_rgb, &mut wind_z, true);
        let solid_px = draw_crown(&wind_cam, &later, &mut wind_rgb, &mut wind_z, false);
        assert!(ghost_px > 800 && solid_px > 800, "wind crown pixels {ghost_px} {solid_px}");
        let inset = Rect {
            x: 36.0,
            y: (H - 280) as f64,
            w: 620.0,
            h: 244.0,
        };
        paint_profile(
            &mut wind_rgb,
            &inset,
            &profile_axis(&site, &fitted.joints),
            &[&later, &early],
            &[[24, 116, 112], [122, 78, 46]],
        );
        write_png(&out.join("wind-crown.png"), &wind_rgb);

        println!(
            "site trunk {:.2} {:.2} yaw {:.3} collar {:.3} material {}",
            site.seed.trunk_x,
            site.seed.trunk_z,
            site.seed.yaw,
            site.collar_y,
            ground::material_at(site.seed.trunk_x, site.seed.trunk_z).as_str()
        );
        println!(
            "gully {} soil {:.3} root {:.3} authored {:.3}",
            site.gully_part,
            ground::height_at(site.gully_tip[0], site.gully_tip[2]),
            site.gully_tip[1],
            site.gully_authored_y
        );
        println!(
            "rise {} soil {:.3} root {:.3} authored {:.3}",
            site.rise_part,
            ground::height_at(site.rise_tip[0], site.rise_tip[2]),
            site.rise_tip[1],
            site.rise_authored_y
        );
        println!(
            "relief rise {:.3} fall {:.3} plane_gap {:.3} wind {t0:.2}s -> {t1:.2}s crown {:.3} m roots {root_delta:.3} m apex {apex_px:.1} px",
            site.rise_m, site.fall_m, site.plane_gap_m, crown_delta
        );
        println!(
            "normal silt {} litter {} sand {} granite {} pebble {} wet {} pad {} water {water}",
            counts[1], counts[2], counts[3], counts[4], counts[5], counts[6], counts[7]
        );
        println!(
            "wind-ground silt {} litter {} sand {} granite {} pebble {} wet {} pad {} water {wind_water}",
            wind_counts[1],
            wind_counts[2],
            wind_counts[3],
            wind_counts[4],
            wind_counts[5],
            wind_counts[6],
            wind_counts[7]
        );
    }

    fn full_rect() -> Rect {
        Rect {
            x: 40.0,
            y: 28.0,
            w: (W - 80) as f64,
            h: (H - 56) as f64,
        }
    }

    fn max_frozen_delta(early: &[FittedJoint], later: &[FittedJoint]) -> f64 {
        early
            .iter()
            .zip(later)
            .filter(|(joint, _)| joint.frozen)
            .map(|(left, right)| {
                let d = [
                    left.world[0] - right.world[0],
                    left.world[1] - right.world[1],
                    left.world[2] - right.world[2],
                ];
                d[0] * d[0] + d[1] * d[1] + d[2] * d[2]
            })
            .fold(0.0, f64::max)
            .sqrt()
    }

    fn max_free_delta(early: &[FittedJoint], later: &[FittedJoint]) -> f64 {
        early
            .iter()
            .zip(later)
            .filter(|(joint, _)| !joint.frozen)
            .map(|(left, right)| {
                let d = [
                    left.world[0] - right.world[0],
                    left.world[1] - right.world[1],
                    left.world[2] - right.world[2],
                ];
                (d[0] * d[0] + d[1] * d[1] + d[2] * d[2]).sqrt()
            })
            .fold(0.0, f64::max)
    }

    fn normal_camera(site: &ContourSite) -> Cam {
        let (s, c) = site.seed.yaw.sin_cos();
        let side = [s, 0.0, c];
        let along = [c, 0.0, -s];
        let collar = [site.seed.trunk_x, site.collar_y + 0.15, site.seed.trunk_z];
        let bole = [site.seed.trunk_x, site.collar_y + 6.5, site.seed.trunk_z];
        let mut chosen: Option<(f64, Cam)> = None;
        for &dist in &[12.0, 16.0, 20.0, 26.0, 34.0] {
            for &flip in &[1.0, -1.0] {
                for &along_m in &[0.0, 6.0, -6.0] {
                    let ex = site.seed.trunk_x + side[0] * dist * flip + along[0] * along_m;
                    let ez = site.seed.trunk_z + side[2] * dist * flip + along[2] * along_m;
                    let eye_ground = ground::height_at(ex, ez);
                    if eye_ground > site.collar_y + 0.4 {
                        continue;
                    }
                    let eye = [ex, eye_ground + 1.7, ez];
                    let cam = cam_look(eye, bole, 48.0_f64.to_radians());
                    if !reaches(&cam, collar) || !reaches(&cam, bole) {
                        continue;
                    }
                    let Some(projected) = project(&cam, collar) else { continue };
                    let on_screen = projected[0] > 280.0
                        && projected[0] < (W as f64 - 280.0)
                        && projected[1] > 220.0
                        && projected[1] < (H as f64 - 120.0);
                    if on_screen {
                        let score = 800.0 - (site.collar_y - eye_ground) - (projected[1] - H as f64 * 0.62).abs();
                        if chosen.as_ref().map(|(so_far, _)| score > *so_far).unwrap_or(true) {
                            chosen = Some((score, cam));
                        }
                    }
                }
            }
        }
        chosen.map(|(_, cam)| cam).unwrap_or_else(|| {
            let ex = site.seed.trunk_x + side[0] * 18.0;
            let ez = site.seed.trunk_z + side[2] * 18.0;
            cam_look(
                [ex, ground::height_at(ex, ez) + 2.2, ez],
                bole,
                50.0_f64.to_radians(),
            )
        })
    }

    fn reaches(cam: &Cam, point: [f64; 3]) -> bool {
        let delta = sub(point, cam.eye);
        let reach = dot(delta, delta).sqrt();
        let dir = mul(delta, 1.0 / reach.max(1e-6));
        match march(cam, dir) {
            None => true,
            Some(hit) => hit.t + 1.2 > reach,
        }
    }

    fn wind_camera(site: &ContourSite, early: &[FittedJoint], later: &[FittedJoint]) -> Cam {
        let a = highest_free(early);
        let b = highest_free(later);
        let target = [(a[0] + b[0]) * 0.5, (a[1] + b[1]) * 0.5 - 0.8, (a[2] + b[2]) * 0.5];
        let (s, c) = site.seed.yaw.sin_cos();
        let side = [s, 0.0, c];
        let mut dist = 14.0;
        let mut cam = cam_look(wind_eye(target, side, dist), target, 34.0_f64.to_radians());
        for _ in 0..8 {
            let px = apex_pixels(&cam, early, later);
            if px < 48.0 {
                dist *= 0.78;
            } else if px > 140.0 {
                dist *= 1.16;
            } else {
                break;
            }
            cam = cam_look(wind_eye(target, side, dist), target, 34.0_f64.to_radians());
        }
        cam
    }

    fn wind_eye(target: [f64; 3], side: [f64; 3], dist: f64) -> [f64; 3] {
        let x = target[0] + side[0] * dist;
        let z = target[2] + side[2] * dist;
        let y = (target[1] + 0.2).max(ground::height_at(x, z) + 1.6);
        [x, y, z]
    }

    fn highest_free(joints: &[FittedJoint]) -> [f64; 3] {
        joints
            .iter()
            .filter(|joint| !joint.frozen)
            .max_by(|a, b| {
                a.authored[1]
                    .partial_cmp(&b.authored[1])
                    .unwrap_or(std::cmp::Ordering::Equal)
            })
            .map(|joint| joint.world)
            .unwrap_or([0.0, 0.0, 0.0])
    }

    fn apex_pixels(cam: &Cam, early: &[FittedJoint], later: &[FittedJoint]) -> f64 {
        let a = project(cam, highest_free(early));
        let b = project(cam, highest_free(later));
        match (a, b) {
            (Some(p), Some(q)) => ((p[0] - q[0]).powi(2) + (p[1] - q[1]).powi(2)).sqrt(),
            _ => 0.0,
        }
    }

    fn profile_axis(site: &ContourSite, joints: &[FittedJoint]) -> Axis {
        let gully = site.gully_tip;
        let rise = site.rise_tip;
        let dx = rise[0] - gully[0];
        let dz = rise[2] - gully[2];
        let len = (dx * dx + dz * dz).sqrt().max(1e-6);
        let ux = dx / len;
        let uz = dz / len;
        let pad = 2.0;
        let mut y_min = gully[1].min(rise[1]);
        let mut y_max = site.collar_y;
        for joint in joints {
            if joint.role != JointRole::Root || joint.authored[1] > 0.0 {
                continue;
            }
            y_min = y_min.min(joint.world[1]);
        }
        let steps = 64;
        for step in 0..=steps {
            let s = -pad + (len + 2.0 * pad) * (step as f64 / steps as f64);
            let h = ground::height_at(gully[0] + ux * s, gully[2] + uz * s);
            y_max = y_max.max(h);
            y_min = y_min.min(h);
        }
        y_max += 1.8;
        y_min -= 0.6;
        Axis {
            origin_x: gully[0] - ux * pad,
            origin_z: gully[2] - uz * pad,
            ux,
            uz,
            span: len + 2.0 * pad,
            y_min,
            y_max,
        }
    }

    fn paint_profile(img: &mut [u8], rect: &Rect, axis: &Axis, poses: &[&[FittedJoint]], colours: &[[u8; 3]]) {
        frame(img, rect);
        for x in 0..rect.w as i32 {
            let px = rect.x + x as f64;
            let s = (px - rect.x) / rect.w * axis.span;
            let world_x = axis.origin_x + axis.ux * s;
            let world_z = axis.origin_z + axis.uz * s;
            let soil = ground::height_at(world_x, world_z);
            let (_, sy) = to_px(axis, rect, s, soil);
            let top = rect.y as i32;
            let bot = (rect.y + rect.h) as i32;
            let surface = sy.round() as i32;
            let colour = ground::colour_at(world_x, world_z);
            for y in top..bot {
                if y < 0 || y >= H || px < 0.0 || px >= W as f64 {
                    continue;
                }
                let rgb = if y < surface {
                    [176, 198, 206]
                } else {
                    let depth = ((y - surface) as f64 / 140.0).clamp(0.0, 0.55);
                    [
                        (colour[0] * (1.0 - depth) + 42.0 * depth).round() as u8,
                        (colour[1] * (1.0 - depth) + 30.0 * depth).round() as u8,
                        (colour[2] * (1.0 - depth) + 22.0 * depth).round() as u8,
                    ]
                };
                put(img, px as i32, y, rgb);
            }
            if (0..H).contains(&surface) {
                put(img, px as i32, surface, [236, 224, 176]);
                if surface > 0 {
                    put(img, px as i32, surface - 1, [236, 224, 176]);
                }
            }
        }
        for (pose, colour) in poses.iter().zip(colours.iter()) {
            draw_roots(img, rect, axis, pose, *colour);
        }
    }

    fn draw_roots(img: &mut [u8], rect: &Rect, axis: &Axis, joints: &[FittedJoint], primary: [u8; 3]) {
        let mut groups: Vec<(u16, Vec<usize>)> = Vec::new();
        for (index, joint) in joints.iter().enumerate() {
            if joint.role != JointRole::Root || joint.authored[1] > 0.0 {
                continue;
            }
            let soil = ground::height_at(joint.world[0], joint.world[2]);
            assert!(
                joint.world[1] <= soil + 1e-4,
                "{} authored {} sits {} m above the soil",
                joint.part,
                joint.authored[1],
                joint.world[1] - soil
            );
            if let Some(slot) = groups.iter_mut().find(|slot| slot.0 == joint.part_index) {
                slot.1.push(index);
            } else {
                groups.push((joint.part_index, vec![index]));
            }
        }
        for group in &mut groups {
            group.1.sort_by_key(|&index| joints[index].section);
            let mut previous: Option<usize> = None;
            for &index in &group.1 {
                let joint = joints[index];
                if joint.authored[1] > -0.01 {
                    previous = None;
                    continue;
                }
                let (x, y) = project_joint(axis, rect, joint.world);
                let (radius, rgb) = root_style(joint.order, primary);
                if let Some(prev) = previous {
                    if joint.section == joints[prev].section + 1 {
                        let (x0, y0) = project_joint(axis, rect, joints[prev].world);
                        stamp_seg(img, x0, y0, x, y, radius, rgb);
                    }
                }
                stamp_disc(img, x, y, radius + 0.6, rgb);
                previous = Some(index);
            }
        }
    }

    fn root_style(order: u8, primary: [u8; 3]) -> (f64, [u8; 3]) {
        match order {
            1 => (4.2, [72, 44, 30]),
            2 => (3.1, primary),
            3 => (2.0, [98, 64, 42]),
            4 => (2.3, [64, 40, 28]),
            5 => (1.3, [116, 86, 58]),
            _ => (1.8, [88, 58, 40]),
        }
    }

    fn project_joint(axis: &Axis, rect: &Rect, world: [f64; 3]) -> (f64, f64) {
        let s = (world[0] - axis.origin_x) * axis.ux + (world[2] - axis.origin_z) * axis.uz;
        to_px(axis, rect, s, world[1])
    }

    fn to_px(axis: &Axis, rect: &Rect, s: f64, y: f64) -> (f64, f64) {
        let px = rect.x + (s / axis.span) * rect.w;
        let py = rect.y + (axis.y_max - y) / (axis.y_max - axis.y_min) * rect.h;
        (px, py)
    }

    fn frame(img: &mut [u8], rect: &Rect) {
        let x0 = rect.x as i32;
        let y0 = rect.y as i32;
        let x1 = (rect.x + rect.w) as i32;
        let y1 = (rect.y + rect.h) as i32;
        for x in x0..=x1 {
            put(img, x, y0, [36, 28, 22]);
            put(img, x, y1, [36, 28, 22]);
        }
        for y in y0..=y1 {
            put(img, x0, y, [36, 28, 22]);
            put(img, x1, y, [36, 28, 22]);
        }
    }

    fn draw_crown(cam: &Cam, joints: &[FittedJoint], img: &mut [u8], zbuf: &mut [f32], ghost: bool) -> u32 {
        let bark = if ghost { [198, 186, 162] } else { [228, 212, 186] };
        let stocking = if ghost { [154, 112, 96] } else { [132, 64, 48] };
        let leaf = if ghost { [176, 184, 156] } else { [86, 104, 68] };
        let leaf_deep = if ghost { [160, 168, 142] } else { [62, 82, 50] };
        let collar = collar_point(joints);
        let mut boles: Vec<usize> = joints
            .iter()
            .enumerate()
            .filter(|(_, joint)| joint.role == JointRole::Bole && !joint.frozen && joint.authored[1] > 0.0)
            .map(|(index, _)| index)
            .collect();
        boles.sort_by(|&a, &b| {
            joints[a].authored[1]
                .partial_cmp(&joints[b].authored[1])
                .unwrap_or(std::cmp::Ordering::Equal)
        });
        let mut painted = 0u32;
        let planted = [collar[0], collar[1] - 1.4, collar[2]];
        let mut previous = planted;
        let mut previous_y = planted[1];
        for (step, &index) in boles.iter().enumerate() {
            let joint = joints[index];
            assert_ne!(joint.role, JointRole::Root);
            assert!(joint.authored[1] > 0.0, "{}", joint.part);
            let radius = 0.46 - step as f64 * 0.08;
            let colour = if (previous_y + joint.world[1]) * 0.5 - collar[1] < 2.3 {
                stocking
            } else {
                bark
            };
            painted += stroke_world(cam, img, zbuf, previous, joint.world, radius.max(0.18), colour);
            previous = joint.world;
            previous_y = joint.world[1];
        }
        let fork = boles
            .iter()
            .copied()
            .map(|index| joints[index].world)
            .last()
            .unwrap_or(collar);
        for joint in joints {
            if joint.frozen || joint.role == JointRole::Root || joint.authored[1] <= 0.0 {
                continue;
            }
            if joint.role == JointRole::Bough {
                let attach = attachment(joints, &boles, joint.authored[1]).unwrap_or(fork);
                painted += stroke_world(cam, img, zbuf, attach, joint.world, 0.16, bark);
                painted += foliage(cam, img, zbuf, joint.world, leaf, leaf_deep);
            } else if joint.role == JointRole::Sprig {
                let from = nearest_bough(joints, joint).unwrap_or(fork);
                painted += stroke_world(cam, img, zbuf, from, joint.world, 0.06, leaf_deep);
                painted += foliage(cam, img, zbuf, joint.world, leaf, leaf_deep);
            }
        }
        painted
    }

    fn collar_point(joints: &[FittedJoint]) -> [f64; 3] {
        let plate = joints.iter().find(|joint| PLATE_NAMES.contains(&joint.part));
        if let Some(plate) = plate {
            let dy = -plate.authored[1];
            return [plate.world[0], plate.world[1] + dy, plate.world[2]];
        }
        joints.first().map(|joint| joint.world).unwrap_or([0.0, 0.0, 0.0])
    }

    fn attachment(joints: &[FittedJoint], boles: &[usize], authored_y: f64) -> Option<[f64; 3]> {
        boles
            .iter()
            .copied()
            .filter(|&index| joints[index].authored[1] <= authored_y)
            .max_by(|&a, &b| {
                joints[a].authored[1]
                    .partial_cmp(&joints[b].authored[1])
                    .unwrap_or(std::cmp::Ordering::Equal)
            })
            .map(|index| joints[index].world)
    }

    fn nearest_bough(joints: &[FittedJoint], sprig: &FittedJoint) -> Option<[f64; 3]> {
        joints
            .iter()
            .filter(|joint| joint.role == JointRole::Bough)
            .min_by(|a, b| {
                let da = authored_dist(a.authored, sprig.authored);
                let db = authored_dist(b.authored, sprig.authored);
                da.partial_cmp(&db).unwrap_or(std::cmp::Ordering::Equal)
            })
            .map(|joint| joint.world)
    }

    fn authored_dist(a: [f64; 3], b: [f64; 3]) -> f64 {
        let d = [a[0] - b[0], a[1] - b[1], a[2] - b[2]];
        d[0] * d[0] + d[1] * d[1] + d[2] * d[2]
    }

    fn foliage(cam: &Cam, img: &mut [u8], zbuf: &mut [f32], at: [f64; 3], leaf: [u8; 3], deep: [u8; 3]) -> u32 {
        let mut painted = 0u32;
        for (dy, radius, colour) in [(0.2, 1.35, leaf), (-0.7, 1.05, deep), (-1.35, 0.72, leaf)] {
            painted += paint_world_disc(cam, img, zbuf, [at[0], at[1] + dy, at[2]], radius, colour);
        }
        painted
    }

    fn stroke_world(cam: &Cam, img: &mut [u8], zbuf: &mut [f32], a: [f64; 3], b: [f64; 3], radius: f64, rgb: [u8; 3]) -> u32 {
        let mut painted = 0u32;
        let span = ((b[0] - a[0]).powi(2) + (b[1] - a[1]).powi(2) + (b[2] - a[2]).powi(2)).sqrt();
        let steps = (span / 0.08).ceil().clamp(8.0, 240.0) as i32;
        for step in 0..=steps {
            let t = step as f64 / steps as f64;
            let p = [
                a[0] + (b[0] - a[0]) * t,
                a[1] + (b[1] - a[1]) * t,
                a[2] + (b[2] - a[2]) * t,
            ];
            painted += paint_world_disc(cam, img, zbuf, p, radius, rgb);
        }
        painted
    }

    fn paint_world_disc(cam: &Cam, img: &mut [u8], zbuf: &mut [f32], p: [f64; 3], radius_m: f64, rgb: [u8; 3]) -> u32 {
        let Some(s) = project(cam, p) else {
            return 0;
        };
        let focal = (H as f64 * 0.5) / (cam.fov_y * 0.5).tan();
        let radius = (radius_m * focal / s[2]).clamp(0.8, 90.0);
        paint_disc(img, zbuf, s[0], s[1], s[2] as f32, radius, rgb)
    }

    fn paint_disc(img: &mut [u8], zbuf: &mut [f32], x: f64, y: f64, z: f32, radius: f64, rgb: [u8; 3]) -> u32 {
        let r = radius.ceil() as i32;
        let x0 = (x as i32 - r).max(0);
        let x1 = (x as i32 + r).min(W - 1);
        let y0 = (y as i32 - r).max(0);
        let y1 = (y as i32 + r).min(H - 1);
        let mut painted = 0u32;
        for py in y0..=y1 {
            for px in x0..=x1 {
                let dx = px as f64 + 0.5 - x;
                let dy = py as f64 + 0.5 - y;
                if dx * dx + dy * dy > radius * radius {
                    continue;
                }
                let index = (py * W + px) as usize;
                if z >= zbuf[index] {
                    continue;
                }
                zbuf[index] = z;
                put(img, px, py, rgb);
                painted += 1;
            }
        }
        painted
    }

    fn stamp_seg(img: &mut [u8], x0: f64, y0: f64, x1: f64, y1: f64, radius: f64, rgb: [u8; 3]) {
        let steps = ((x1 - x0).abs() + (y1 - y0).abs()).ceil().max(1.0) as i32;
        for step in 0..=steps {
            let t = step as f64 / steps as f64;
            stamp_disc(img, x0 + (x1 - x0) * t, y0 + (y1 - y0) * t, radius, rgb);
        }
    }

    fn stamp_disc(img: &mut [u8], x: f64, y: f64, radius: f64, rgb: [u8; 3]) {
        let r = radius.ceil() as i32;
        for py in (y as i32 - r)..=(y as i32 + r) {
            for px in (x as i32 - r)..=(x as i32 + r) {
                let dx = px as f64 + 0.5 - x;
                let dy = py as f64 + 0.5 - y;
                if dx * dx + dy * dy <= radius * radius {
                    put(img, px, py, rgb);
                }
            }
        }
    }

    fn raster(cam: &Cam) -> (Vec<u8>, [u32; 8], u32, u32, Vec<f32>) {
        let mut rgb = vec![0u8; (W * H * 3) as usize];
        let mut zbuf = vec![1.0e9f32; (W * H) as usize];
        let mut counts = [0u32; 8];
        let mut water = 0u32;
        let mut sky_px = 0u32;
        for y in 0..H {
            for x in 0..W {
                let dir = ray(cam, x, y);
                if let Some(hit) = march(cam, dir) {
                    let (colour, material) = shade(hit);
                    put(&mut rgb, x, y, colour);
                    zbuf[(y * W + x) as usize] = hit.t as f32;
                    if hit.water {
                        water += 1;
                    } else {
                        counts[material as u8 as usize] += 1;
                    }
                } else {
                    put(&mut rgb, x, y, sky(dir));
                    sky_px += 1;
                }
            }
        }
        (rgb, counts, water, sky_px, zbuf)
    }

    fn cam_look(eye: [f64; 3], at: [f64; 3], fov_y: f64) -> Cam {
        let forward = norm(sub(at, eye));
        let right = norm(cross(forward, [0.0, 1.0, 0.0]));
        let up = norm(cross(right, forward));
        Cam {
            eye,
            forward,
            right,
            up,
            fov_y,
        }
    }

    fn project(cam: &Cam, p: [f64; 3]) -> Option<[f64; 3]> {
        let rel = sub(p, cam.eye);
        let x = dot(rel, cam.right);
        let y = dot(rel, cam.up);
        let z = dot(rel, cam.forward);
        if z < 0.2 {
            return None;
        }
        let tan = (cam.fov_y * 0.5).tan();
        let ndc_x = x / (z * tan * (W as f64 / H as f64));
        let ndc_y = y / (z * tan);
        Some([
            (ndc_x * 0.5 + 0.5) * W as f64,
            (1.0 - (ndc_y * 0.5 + 0.5)) * H as f64,
            z,
        ])
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

    fn march(cam: &Cam, dir: [f64; 3]) -> Option<Hit> {
        let mut t = 0.35;
        let mut prev = t;
        while t < 220.0 {
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
                            return Some(Hit {
                                t: tw,
                                p: [s[0], 0.24, s[2]],
                                water: true,
                            });
                        }
                    }
                }
                return Some(Hit {
                    t: hi,
                    p: [q[0], bed, q[2]],
                    water: false,
                });
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

    fn shade(hit: Hit) -> ([u8; 3], GroundMaterial) {
        if hit.water {
            let ripple = (hit.p[0] * 0.35 + hit.p[2] * 0.22).sin();
            return (
                fog(
                    [
                        (28.0 + ripple * 6.0) as u8,
                        (90.0 + ripple * 8.0) as u8,
                        (104.0 + ripple * 6.0) as u8,
                    ],
                    hit.t,
                ),
                GroundMaterial::WetBank,
            );
        }
        let material = ground::material_at(hit.p[0], hit.p[2]);
        let base = ground::colour_at(hit.p[0], hit.p[2]);
        let grain = hash01(hit.p[0], hit.p[2]);
        let rgb = match material {
            GroundMaterial::Pebble => {
                let lift = if grain > 0.72 {
                    26.0
                } else if grain < 0.38 {
                    -24.0
                } else {
                    (grain - 0.5) * 10.0
                };
                [base[0] + lift, base[1] + lift, base[2] + lift * 0.8]
            }
            GroundMaterial::Sandstone => [
                base[0] + (grain - 0.5) * 16.0,
                base[1] + (grain - 0.5) * 12.0,
                base[2] + (grain - 0.5) * 8.0,
            ],
            GroundMaterial::Granite => {
                let lift = if grain > 0.8 { 18.0 } else { (grain - 0.5) * 12.0 };
                [base[0] + lift, base[1] + lift, base[2] + lift]
            }
            _ => [
                base[0] + (grain - 0.5) * 8.0,
                base[1] + (grain - 0.5) * 7.0,
                base[2] + (grain - 0.5) * 5.0,
            ],
        };
        let n = ground::normal_at(hit.p[0], hit.p[2]);
        let sun = norm([0.42, 0.84, 0.34]);
        let light = 0.46 + 0.68 * dot(n, sun).max(0.0);
        (
            fog(
                [
                    (rgb[0] * light).clamp(0.0, 255.0) as u8,
                    (rgb[1] * light).clamp(0.0, 255.0) as u8,
                    (rgb[2] * light).clamp(0.0, 255.0) as u8,
                ],
                hit.t,
            ),
            material,
        )
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
        if x < 0 || y < 0 || x >= W || y >= H {
            return;
        }
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
    fn dot(a: [f64; 3], b: [f64; 3]) -> f64 {
        a[0] * b[0] + a[1] * b[1] + a[2] * b[2]
    }
    fn cross(a: [f64; 3], b: [f64; 3]) -> [f64; 3] {
        [
            a[1] * b[2] - a[2] * b[1],
            a[2] * b[0] - a[0] * b[2],
            a[0] * b[1] - a[1] * b[0],
        ]
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
            out.extend_from_slice(&(!n as u16).to_le_bytes());
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
}
