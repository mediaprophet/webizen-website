//! Playable human explorer character state, deterministic 28-joint `SKL2` pose &
//! analytic two-bone leg/arm IK solver, 1st/3rd-person camera rig, and
//! `DeviceTier` / `AnatomyXRayMode` `.10d` asset selection.
//!
//! Pure-Rust (`std` only) so this module compiles identically on `wasm32`,
//! native `cargo test`, and standalone `rustc --edition 2021 --test character.rs`.
//! Anthropometry and joint topology match `eco-intent/homo-sapiens/ontology/biomechanics.json`
//! (`WEB-WP-16`) and `eco-intent/homo-sapiens/dist/lods/manifest.json` (`WEB-WP-17`).

use std::f32::consts::{PI, TAU};

/// Canonical `SKL2` joint count (`human_bipedal_skl2_28`).
pub const SKL2_JOINT_COUNT: usize = 28;

/// Adult standing stature in metres (`biomechanics.json`).
pub const ADULT_STATURE_M: f32 = 1.74;

/// First-person camera eye height above character ground origin (`locomotion.rs` parity).
pub const EYE_HEIGHT_M: f32 = 1.65;

/// Standing hip height in metres (`pelvis_root` rest Y).
pub const STANDING_HIP_HEIGHT_M: f32 = 0.92;

/// Standing ankle height above ground plane in metres (`ankle_l`/`ankle_r` rest Y).
pub const ANKLE_HEIGHT_M: f32 = 0.08;

/// Leg two-bone IK segment lengths (`hip -> knee -> ankle`, `biomechanics.json`).
pub const LEG_UPPER_LENGTH_M: f32 = 0.4301;
pub const LEG_LOWER_LENGTH_M: f32 = 0.4105;

/// Arm two-bone IK segment lengths (`shoulder -> elbow -> wrist`, `biomechanics.json`).
pub const ARM_UPPER_LENGTH_M: f32 = 0.265;
pub const ARM_LOWER_LENGTH_M: f32 = 0.225;

/// Seated driver ergonomics (`biomechanics.json` `postures.seated_driver`).
pub const SEATED_DRIVER_PELVIS_OFFSET_M: [f32; 3] = [0.000, 0.610, -0.050];
pub const SEATED_DRIVER_HIP_PITCH_DEG: f32 = 95.0;
pub const SEATED_DRIVER_KNEE_PITCH_DEG: f32 = 65.0;

/// Steering wheel geometry relative to seated driver pelvis (`biomechanics.json`).
pub const STEERING_WHEEL_CENTER_REL_PELVIS_M: [f32; 3] = [0.000, 0.280, 0.390];
pub const STEERING_WHEEL_RADIUS_M: f32 = 0.185;
pub const STEERING_WHEEL_TILT_DEG: f32 = 24.0;
pub const STEERING_WHEEL_TARGET_L_M: [f32; 3] = [0.175, 0.310, 0.375];
pub const STEERING_WHEEL_TARGET_R_M: [f32; 3] = [-0.175, 0.310, 0.375];

/// Specimen close-inspection reach targets relative to pelvis (`biomechanics.json`).
pub const INSPECT_REACH_TARGET_L_M: [f32; 3] = [0.220, 0.280, 0.420];
pub const INSPECT_REACH_TARGET_R_M: [f32; 3] = [-0.220, 0.280, 0.420];

/// Minimum vertical clearance of the 3rd-person chase camera above terrain.
pub const THIRD_PERSON_TERRAIN_CLEARANCE_M: f32 = 0.35;

/// Default 3rd-person chase distance behind the right shoulder in metres.
pub const DEFAULT_CHASE_DISTANCE_M: f32 = 3.4;

/// Resting respiratory frequency in Hz (`biomechanics.json` `bioelectric_emf.respiratory`).
pub const RESTING_RESPIRATION_HZ: f32 = 0.2333;

/// Same-origin asset URLs for `homo-sapiens` LODs and Anatomy X-Ray (`WEB-WP-17`).
pub const HOMO_SAPIENS_LOD0_URL: &str = "/intenteco/homo-sapiens/dist/lods/homo-sapiens-lod0.10d";
pub const HOMO_SAPIENS_LOD1_URL: &str = "/intenteco/homo-sapiens/dist/lods/homo-sapiens-lod1.10d";
pub const HOMO_SAPIENS_LOD2_URL: &str = "/intenteco/homo-sapiens/dist/lods/homo-sapiens-lod2.10d";
pub const HOMO_SAPIENS_ANATOMY_XRAY_URL: &str =
    "/intenteco/homo-sapiens/dist/lods/homo-sapiens-anatomy-xray.10d";

/// Verified `WEB-WP-17` SHA-256 digests, byte lengths, and triangle counts.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct CharacterAssetDescriptor {
    pub url: &'static str,
    pub sha256: &'static str,
    pub bytes: usize,
    pub triangle_count: u32,
}

pub const ASSET_LOD0: CharacterAssetDescriptor = CharacterAssetDescriptor {
    url: HOMO_SAPIENS_LOD0_URL,
    sha256: "955ba0ec440cefc809f60647f8a83d512df43d4d0637f6a93a6d296637fb04bc",
    bytes: 119_608,
    triangle_count: 7_432,
};

pub const ASSET_LOD1: CharacterAssetDescriptor = CharacterAssetDescriptor {
    url: HOMO_SAPIENS_LOD1_URL,
    sha256: "ccd2ab2eca7aa13a87682e42992e4fdb8f5d20c455865ac932e749cd1932db87",
    bytes: 69_992,
    triangle_count: 3_616,
};

pub const ASSET_LOD2: CharacterAssetDescriptor = CharacterAssetDescriptor {
    url: HOMO_SAPIENS_LOD2_URL,
    sha256: "eb4d9bae38d73ab275758cf1bcf97f3c402cbb6ba34e99c879cd28cd7cfa8e80",
    bytes: 35_824,
    triangle_count: 1_900,
};

pub const ASSET_ANATOMY_XRAY: CharacterAssetDescriptor = CharacterAssetDescriptor {
    url: HOMO_SAPIENS_ANATOMY_XRAY_URL,
    sha256: "fe3b32d4a9e9d695db5084d294b7ad813b3ff604e90bec16e71b9fe1446d0657",
    bytes: 101_512,
    triangle_count: 5_088,
};

/// Joint indices matching `eco-intent/homo-sapiens/ontology/biomechanics.json`.
pub mod joint {
    pub const PELVIS_ROOT: usize = 0;
    pub const SPINE_LUMBAR: usize = 1;
    pub const SPINE_THORACIC: usize = 2;
    pub const SPINE_CERVICAL: usize = 3;
    pub const NECK: usize = 4;
    pub const HEAD: usize = 5;
    pub const JAW: usize = 6;
    pub const RIBCAGE_STERNUM: usize = 7;
    pub const CLAVICLE_L: usize = 8;
    pub const SHOULDER_L: usize = 9;
    pub const ELBOW_L: usize = 10;
    pub const WRIST_L: usize = 11;
    pub const HAND_L: usize = 12;
    pub const THUMB_L: usize = 13;
    pub const CLAVICLE_R: usize = 14;
    pub const SHOULDER_R: usize = 15;
    pub const ELBOW_R: usize = 16;
    pub const WRIST_R: usize = 17;
    pub const HAND_R: usize = 18;
    pub const THUMB_R: usize = 19;
    pub const HIP_L: usize = 20;
    pub const KNEE_L: usize = 21;
    pub const ANKLE_L: usize = 22;
    pub const TOE_L: usize = 23;
    pub const HIP_R: usize = 24;
    pub const KNEE_R: usize = 25;
    pub const ANKLE_R: usize = 26;
    pub const TOE_R: usize = 27;
}

/// Static `SKL2` joint descriptor from `biomechanics.json`.
#[derive(Clone, Copy, Debug, PartialEq)]
pub struct Skl2JointSpec {
    pub index: usize,
    pub name: &'static str,
    pub parent_index: i8,
    pub rest_local_m: [f32; 3],
    pub rest_world_m: [f32; 3],
    pub bone_length_m: f32,
}

/// Complete 28-joint `SKL2` rest-pose specification (`0..27`).
pub const SKL2_REST_JOINTS: [Skl2JointSpec; SKL2_JOINT_COUNT] = [
    Skl2JointSpec { index: 0,  name: "pelvis_root",     parent_index: -1, rest_local_m: [ 0.000,  0.920,  0.000], rest_world_m: [ 0.000, 0.920,  0.000], bone_length_m: 0.130 },
    Skl2JointSpec { index: 1,  name: "spine_lumbar",    parent_index:  0, rest_local_m: [ 0.000,  0.130, -0.010], rest_world_m: [ 0.000, 1.050, -0.010], bone_length_m: 0.220 },
    Skl2JointSpec { index: 2,  name: "spine_thoracic",  parent_index:  1, rest_local_m: [ 0.000,  0.220, -0.010], rest_world_m: [ 0.000, 1.270, -0.020], bone_length_m: 0.170 },
    Skl2JointSpec { index: 3,  name: "spine_cervical",  parent_index:  2, rest_local_m: [ 0.000,  0.170,  0.005], rest_world_m: [ 0.000, 1.440, -0.015], bone_length_m: 0.075 },
    Skl2JointSpec { index: 4,  name: "neck",            parent_index:  3, rest_local_m: [ 0.000,  0.075,  0.010], rest_world_m: [ 0.000, 1.515, -0.005], bone_length_m: 0.075 },
    Skl2JointSpec { index: 5,  name: "head",            parent_index:  4, rest_local_m: [ 0.000,  0.075,  0.010], rest_world_m: [ 0.000, 1.590,  0.005], bone_length_m: 0.220 },
    Skl2JointSpec { index: 6,  name: "jaw",             parent_index:  5, rest_local_m: [ 0.000, -0.025,  0.030], rest_world_m: [ 0.000, 1.565,  0.035], bone_length_m: 0.085 },
    Skl2JointSpec { index: 7,  name: "ribcage_sternum", parent_index:  2, rest_local_m: [ 0.000,  0.050,  0.105], rest_world_m: [ 0.000, 1.320,  0.085], bone_length_m: 0.200 },
    Skl2JointSpec { index: 8,  name: "clavicle_l",      parent_index:  2, rest_local_m: [ 0.045,  0.160,  0.040], rest_world_m: [ 0.045, 1.430,  0.020], bone_length_m: 0.176 },
    Skl2JointSpec { index: 9,  name: "shoulder_l",      parent_index:  8, rest_local_m: [ 0.175,  0.000, -0.020], rest_world_m: [ 0.220, 1.430,  0.000], bone_length_m: 0.265 },
    Skl2JointSpec { index: 10, name: "elbow_l",         parent_index:  9, rest_local_m: [ 0.265,  0.000,  0.000], rest_world_m: [ 0.485, 1.430,  0.000], bone_length_m: 0.225 },
    Skl2JointSpec { index: 11, name: "wrist_l",         parent_index: 10, rest_local_m: [ 0.225,  0.000,  0.000], rest_world_m: [ 0.710, 1.430,  0.000], bone_length_m: 0.075 },
    Skl2JointSpec { index: 12, name: "hand_l",          parent_index: 11, rest_local_m: [ 0.075,  0.000,  0.000], rest_world_m: [ 0.785, 1.430,  0.000], bone_length_m: 0.085 },
    Skl2JointSpec { index: 13, name: "thumb_l",         parent_index: 12, rest_local_m: [-0.030, -0.020,  0.035], rest_world_m: [ 0.755, 1.410,  0.035], bone_length_m: 0.055 },
    Skl2JointSpec { index: 14, name: "clavicle_r",      parent_index:  2, rest_local_m: [-0.045,  0.160,  0.040], rest_world_m: [-0.045, 1.430,  0.020], bone_length_m: 0.176 },
    Skl2JointSpec { index: 15, name: "shoulder_r",      parent_index: 14, rest_local_m: [-0.175,  0.000, -0.020], rest_world_m: [-0.220, 1.430,  0.000], bone_length_m: 0.265 },
    Skl2JointSpec { index: 16, name: "elbow_r",         parent_index: 15, rest_local_m: [-0.265,  0.000,  0.000], rest_world_m: [-0.485, 1.430,  0.000], bone_length_m: 0.225 },
    Skl2JointSpec { index: 17, name: "wrist_r",         parent_index: 16, rest_local_m: [-0.225,  0.000,  0.000], rest_world_m: [-0.710, 1.430,  0.000], bone_length_m: 0.075 },
    Skl2JointSpec { index: 18, name: "hand_r",          parent_index: 17, rest_local_m: [-0.075,  0.000,  0.000], rest_world_m: [-0.785, 1.430,  0.000], bone_length_m: 0.085 },
    Skl2JointSpec { index: 19, name: "thumb_r",         parent_index: 18, rest_local_m: [ 0.030, -0.020,  0.035], rest_world_m: [-0.755, 1.410,  0.035], bone_length_m: 0.055 },
    Skl2JointSpec { index: 20, name: "hip_l",           parent_index:  0, rest_local_m: [ 0.100,  0.000,  0.000], rest_world_m: [ 0.100, 0.920,  0.000], bone_length_m: 0.430 },
    Skl2JointSpec { index: 21, name: "knee_l",          parent_index: 20, rest_local_m: [ 0.000, -0.430,  0.010], rest_world_m: [ 0.100, 0.490,  0.010], bone_length_m: 0.410 },
    Skl2JointSpec { index: 22, name: "ankle_l",         parent_index: 21, rest_local_m: [ 0.000, -0.410, -0.020], rest_world_m: [ 0.100, 0.080, -0.010], bone_length_m: 0.150 },
    Skl2JointSpec { index: 23, name: "toe_l",           parent_index: 22, rest_local_m: [ 0.000, -0.055,  0.140], rest_world_m: [ 0.100, 0.025,  0.130], bone_length_m: 0.065 },
    Skl2JointSpec { index: 24, name: "hip_r",           parent_index:  0, rest_local_m: [-0.100,  0.000,  0.000], rest_world_m: [-0.100, 0.920,  0.000], bone_length_m: 0.430 },
    Skl2JointSpec { index: 25, name: "knee_r",          parent_index: 24, rest_local_m: [ 0.000, -0.430,  0.010], rest_world_m: [-0.100, 0.490,  0.010], bone_length_m: 0.410 },
    Skl2JointSpec { index: 26, name: "ankle_r",         parent_index: 25, rest_local_m: [ 0.000, -0.410, -0.020], rest_world_m: [-0.100, 0.080, -0.010], bone_length_m: 0.150 },
    Skl2JointSpec { index: 27, name: "toe_r",           parent_index: 26, rest_local_m: [ 0.000, -0.055,  0.140], rest_world_m: [-0.100, 0.025,  0.130], bone_length_m: 0.065 },
];

/// Camera perspective for the playable human explorer.
#[derive(Clone, Copy, Debug, PartialEq, Eq, Default)]
pub enum CameraPerspective {
    FirstPerson,
    #[default]
    ThirdPersonChase,
}

impl CameraPerspective {
    pub const fn toggle(self) -> Self {
        match self {
            Self::FirstPerson => Self::ThirdPersonChase,
            Self::ThirdPersonChase => Self::FirstPerson,
        }
    }

    pub const fn as_str(self) -> &'static str {
        match self {
            Self::FirstPerson => "first-person",
            Self::ThirdPersonChase => "third-person-chase",
        }
    }

    pub const fn label(self) -> &'static str {
        match self {
            Self::FirstPerson => "1st Person",
            Self::ThirdPersonChase => "3rd Person",
        }
    }

    pub fn from_third_person_flag(third_person: bool) -> Self {
        if third_person {
            Self::ThirdPersonChase
        } else {
            Self::FirstPerson
        }
    }
}

/// High-level locomotion / activity posture state.
#[derive(Clone, Copy, Debug, PartialEq, Eq, Default)]
pub enum CharacterPoseState {
    #[default]
    IdleBreathing,
    Walking,
    Jogging,
    Sprinting,
    Airborne,
    Inspecting,
    SeatedDriver,
}

impl CharacterPoseState {
    pub const fn as_str(self) -> &'static str {
        match self {
            Self::IdleBreathing => "idle-breathing",
            Self::Walking => "walking",
            Self::Jogging => "jogging",
            Self::Sprinting => "sprinting",
            Self::Airborne => "airborne",
            Self::Inspecting => "inspecting",
            Self::SeatedDriver => "seated-driver",
        }
    }

    /// Classify pose state from kinematic speed and interaction flags.
    pub fn classify(speed_mps: f32, grounded: bool, inspecting: bool, seated: bool) -> Self {
        if seated {
            return Self::SeatedDriver;
        }
        if !grounded {
            return Self::Airborne;
        }
        if inspecting && speed_mps < 0.35 {
            return Self::Inspecting;
        }
        let s = if speed_mps.is_finite() {
            speed_mps.abs()
        } else {
            0.0
        };
        if s < 0.15 {
            Self::IdleBreathing
        } else if s < 3.2 {
            Self::Walking
        } else if s < 6.0 {
            Self::Jogging
        } else {
            Self::Sprinting
        }
    }
}

/// Human character visual surface mode (outer integument vs osteology/myology/viscera X-Ray).
#[derive(Clone, Copy, Debug, PartialEq, Eq, Default)]
pub enum AnatomyXRayMode {
    #[default]
    Integument,
    AnatomyXRay,
}

impl AnatomyXRayMode {
    pub const fn toggle(self) -> Self {
        match self {
            Self::Integument => Self::AnatomyXRay,
            Self::AnatomyXRay => Self::Integument,
        }
    }

    pub const fn is_xray(self) -> bool {
        matches!(self, Self::AnatomyXRay)
    }

    pub const fn as_str(self) -> &'static str {
        match self {
            Self::Integument => "integument",
            Self::AnatomyXRay => "anatomy-xray",
        }
    }

    pub const fn from_bool(active: bool) -> Self {
        if active {
            Self::AnatomyXRay
        } else {
            Self::Integument
        }
    }
}

/// Device capability tier mirroring `runtime::graphics::DeviceTier` without
/// coupling `character.rs` to browser/WASM modules.
#[allow(non_camel_case_types)]
#[derive(Clone, Copy, Debug, PartialEq, Eq, Default)]
pub enum DeviceTier {
    TierA_DesktopWebGPU,
    #[default]
    TierB_DesktopWebGL2_MobileWebGPU,
    TierC_MobileWebGL2,
}

impl DeviceTier {
    pub const fn as_str(self) -> &'static str {
        match self {
            Self::TierA_DesktopWebGPU => "tier-a",
            Self::TierB_DesktopWebGL2_MobileWebGPU => "tier-b",
            Self::TierC_MobileWebGL2 => "tier-c",
        }
    }

    /// Map `runtime::graphics::DeviceTier::as_str()` (`"tier-a"`, `"tier-b"`, `"tier-c"`).
    pub fn from_tier_id(tier_id: &str) -> Self {
        match tier_id.trim().to_ascii_lowercase().as_str() {
            "tier-a" | "a" | "lod0" => Self::TierA_DesktopWebGPU,
            "tier-c" | "c" | "lod2" => Self::TierC_MobileWebGL2,
            _ => Self::TierB_DesktopWebGL2_MobileWebGPU,
        }
    }
}

/// Select the verified `homo-sapiens` `.10d` asset descriptor for `(DeviceTier, AnatomyXRayMode)`.
pub const fn select_character_asset(
    tier: DeviceTier,
    mode: AnatomyXRayMode,
) -> CharacterAssetDescriptor {
    match mode {
        AnatomyXRayMode::AnatomyXRay => ASSET_ANATOMY_XRAY,
        AnatomyXRayMode::Integument => match tier {
            DeviceTier::TierA_DesktopWebGPU => ASSET_LOD0,
            DeviceTier::TierB_DesktopWebGL2_MobileWebGPU => ASSET_LOD1,
            DeviceTier::TierC_MobileWebGL2 => ASSET_LOD2,
        },
    }
}

/// Select the same-origin `/intenteco/homo-sapiens/dist/lods/...` URL.
pub const fn select_character_asset_url(
    tier: DeviceTier,
    mode: AnatomyXRayMode,
) -> &'static str {
    select_character_asset(tier, mode).url
}

/// Convenience helper for callers holding `runtime::graphics::DeviceTier::as_str()`.
pub fn select_character_asset_url_for_tier_id(
    tier_id: &str,
    mode: AnatomyXRayMode,
) -> &'static str {
    select_character_asset_url(DeviceTier::from_tier_id(tier_id), mode)
}

/// Result of an analytic two-bone IK solve in a 2D limb plane `(u, v)`.
#[derive(Clone, Copy, Debug, PartialEq)]
pub struct TwoBoneIk2dResult {
    /// Upper-segment angle in radians relative to straight-down `(0, -1)` (`+` toward `+u`).
    pub upper_angle_rad: f32,
    /// Mid-joint flexion angle in radians (`>= 0.0`, `0.0` = fully extended).
    pub mid_bend_rad: f32,
    /// Solved mid-joint `(u, v)` coordinates.
    pub mid_pos: [f32; 2],
    /// Solved end-joint `(u, v)` coordinates (clamped to reachable annulus).
    pub end_pos: [f32; 2],
    /// True when the requested target lies within the exact reachable envelope.
    pub reachable: bool,
}

/// Analytic 2D two-bone inverse kinematics solver (law of cosines).
///
/// In the sagittal `(z, y)` plane (`+z` forward, `+y` up):
/// - `bend_sign >= 0.0` bends the mid-joint toward `+z` (anterior knee flexion).
/// - `bend_sign < 0.0` bends the mid-joint toward `-z` (posterior elbow flexion).
pub fn solve_two_bone_ik_2d(
    root: [f32; 2],
    target: [f32; 2],
    upper_len: f32,
    lower_len: f32,
    bend_sign: f32,
) -> TwoBoneIk2dResult {
    let l1 = upper_len.max(1e-4);
    let l2 = lower_len.max(1e-4);
    let du = target[0] - root[0];
    let dv = target[1] - root[1];
    let raw_dist = (du * du + dv * dv).sqrt();

    let min_reach = (l1 - l2).abs() + 1e-4;
    let max_reach = (l1 + l2) - 1e-4;
    let reachable = raw_dist.is_finite() && raw_dist >= min_reach && raw_dist <= max_reach;
    let d = if raw_dist.is_finite() {
        raw_dist.clamp(min_reach, max_reach)
    } else {
        max_reach
    };

    let (ux, uy) = if raw_dist > 1e-6 && raw_dist.is_finite() {
        (du / raw_dist, dv / raw_dist)
    } else {
        (0.0, -1.0)
    };

    let cos_alpha = ((l1 * l1 + d * d - l2 * l2) / (2.0 * l1 * d)).clamp(-1.0, 1.0);
    let alpha = cos_alpha.acos();
    let cos_gamma = ((l1 * l1 + l2 * l2 - d * d) / (2.0 * l1 * l2)).clamp(-1.0, 1.0);
    let mid_bend_rad = (PI - cos_gamma.acos()).max(0.0);

    let sign = if bend_sign >= 0.0 { 1.0 } else { -1.0 };
    let sin_a = alpha.sin() * sign;
    let cos_a = alpha.cos();
    // Perpendicular to (ux, uy) is (-uy, ux): for (0, -1), (-uy, ux) = (+1, 0) = +Z anterior.
    let dir_u = ux * cos_a + (-uy) * sin_a;
    let dir_v = uy * cos_a + ux * sin_a;

    let mid_pos = [root[0] + dir_u * l1, root[1] + dir_v * l1];
    let end_pos = [root[0] + ux * d, root[1] + uy * d];
    let upper_angle_rad = dir_u.atan2(-dir_v);

    TwoBoneIk2dResult {
        upper_angle_rad,
        mid_bend_rad,
        mid_pos,
        end_pos,
        reachable,
    }
}

/// Solved pose for a single `SKL2` joint.
#[derive(Clone, Copy, Debug, PartialEq)]
pub struct JointPose {
    /// Local Euler angles `[pitch_rad, yaw_rad, roll_rad]`.
    pub rotation_rad: [f32; 3],
    /// Joint position in character-local metres (`y = 0` at character ground datum).
    pub local_pos_m: [f32; 3],
}

/// Complete 28-joint `SKL2` pose evaluation output.
#[derive(Clone, Debug, PartialEq)]
pub struct CharacterPose {
    pub state: CharacterPoseState,
    pub joints: [JointPose; SKL2_JOINT_COUNT],
    pub pelvis_offset_m: [f32; 3],
    pub left_foot_world_y_m: f32,
    pub right_foot_world_y_m: f32,
    pub gait_phase_rad: f32,
    pub breath_phase_rad: f32,
    pub chest_expansion_m: f32,
    pub slope_lean_rad: f32,
    pub steering_angle_rad: f32,
    pub left_hand_target_local_m: [f32; 3],
    pub right_hand_target_local_m: [f32; 3],
}

/// Build the canonical 28-joint `SKL2` rest pose (standing T-pose from `biomechanics.json`).
pub fn rest_pose_28() -> [JointPose; SKL2_JOINT_COUNT] {
    let mut joints = [JointPose {
        rotation_rad: [0.0; 3],
        local_pos_m: [0.0; 3],
    }; SKL2_JOINT_COUNT];
    for (i, spec) in SKL2_REST_JOINTS.iter().enumerate() {
        joints[i].local_pos_m = spec.rest_world_m;
    }
    joints
}

/// Compute rotated steering-wheel rim hand targets `(left, right)` relative to the seated pelvis.
pub fn steering_wheel_hand_targets_rel_pelvis(steering_angle_rad: f32) -> ([f32; 3], [f32; 3]) {
    let theta = if steering_angle_rad.is_finite() {
        steering_angle_rad.clamp(-1.35, 1.35)
    } else {
        0.0
    };
    let tilt = STEERING_WHEEL_TILT_DEG.to_radians();
    let cos_t = tilt.cos();
    let sin_t = tilt.sin();
    let c = STEERING_WHEEL_CENTER_REL_PELVIS_M;

    let rotate_point = |base: [f32; 3]| -> [f32; 3] {
        let rx = base[0] - c[0];
        let dy = base[1] - c[1];
        let dz = base[2] - c[2];
        // Decompose into tilted wheel plane (e_x, e_u) and column normal (e_n).
        let ru = dy * cos_t - dz * sin_t;
        let rn = dy * sin_t + dz * cos_t;

        let cos_a = theta.cos();
        let sin_a = theta.sin();
        let rx_rot = rx * cos_a - ru * sin_a;
        let ru_rot = rx * sin_a + ru * cos_a;

        [
            c[0] + rx_rot,
            c[1] + ru_rot * cos_t + rn * sin_t,
            c[2] - ru_rot * sin_t + rn * cos_t,
        ]
    };

    (
        rotate_point(STEERING_WHEEL_TARGET_L_M),
        rotate_point(STEERING_WHEEL_TARGET_R_M),
    )
}

#[inline]
fn deg(d: f32) -> f32 {
    d.to_radians()
}

/// Evaluate the deterministic 28-joint `SKL2` character pose with explicit starting phases.
#[allow(clippy::too_many_arguments)]
pub fn evaluate_character_pose_with_phases(
    state: CharacterPoseState,
    ground_left_y: f32,
    ground_right_y: f32,
    slope_rad: f32,
    steering_angle_rad: f32,
    dt_s: f32,
    reduced_motion: bool,
    gait_phase_in: f32,
    breath_phase_in: f32,
) -> CharacterPose {
    let dt = if dt_s.is_finite() {
        dt_s.clamp(0.0, 0.1)
    } else {
        0.0
    };
    let gl = if ground_left_y.is_finite() {
        ground_left_y
    } else {
        0.0
    };
    let gr = if ground_right_y.is_finite() {
        ground_right_y
    } else {
        0.0
    };
    let slope = if slope_rad.is_finite() {
        slope_rad.clamp(-0.75, 0.75)
    } else {
        0.0
    };
    let steer = if steering_angle_rad.is_finite() {
        steering_angle_rad.clamp(-1.35, 1.35)
    } else {
        0.0
    };

    let motion_scale = if reduced_motion { 0.0 } else { 1.0 };

    // Advance respiratory and gait phases deterministically.
    let breath_rate_hz = match state {
        CharacterPoseState::Sprinting => 0.55,
        CharacterPoseState::Jogging => 0.38,
        CharacterPoseState::Walking => 0.28,
        _ => RESTING_RESPIRATION_HZ,
    };
    let breath_phase = (breath_phase_in + dt * TAU * breath_rate_hz * motion_scale).rem_euclid(TAU);
    let chest_expansion_m = 0.012 * (0.5 + 0.5 * breath_phase.sin()) * motion_scale;

    let (gait_freq_hz, stride_half_m, lift_m, arm_swing_rad) = match state {
        CharacterPoseState::Walking => (1.75, 0.34, 0.09, 0.42),
        CharacterPoseState::Jogging => (2.55, 0.48, 0.16, 0.78),
        CharacterPoseState::Sprinting => (3.45, 0.62, 0.24, 1.15),
        _ => (0.0, 0.0, 0.0, 0.0),
    };
    let gait_phase = (gait_phase_in + dt * TAU * gait_freq_hz * motion_scale).rem_euclid(TAU);

    let mut joints = rest_pose_28();
    let ground_center_y = 0.5 * (gl + gr);
    let dh_l = (gl - ground_center_y).clamp(-0.32, 0.32);
    let dh_r = (gr - ground_center_y).clamp(-0.32, 0.32);

    // Apply respiratory ribcage expansion across all postures.
    joints[joint::RIBCAGE_STERNUM].rotation_rad[0] = deg(3.5) * (0.5 + 0.5 * breath_phase.sin()) * motion_scale;
    joints[joint::RIBCAGE_STERNUM].local_pos_m[2] += chest_expansion_m;

    // Seated driver branch: ergonomic posture + steering wheel two-bone arm IK.
    if state == CharacterPoseState::SeatedDriver {
        let pelvis = SEATED_DRIVER_PELVIS_OFFSET_M;
        let dy_pelvis = pelvis[1] - STANDING_HIP_HEIGHT_M;
        let dz_pelvis = pelvis[2];
        for j in &mut joints {
            j.local_pos_m[1] += dy_pelvis;
            j.local_pos_m[2] += dz_pelvis;
        }

        joints[joint::PELVIS_ROOT].rotation_rad = [deg(-12.0), 0.0, 0.0];
        joints[joint::SPINE_LUMBAR].rotation_rad = [deg(-8.0), 0.0, 0.0];
        joints[joint::SPINE_THORACIC].rotation_rad = [deg(6.0), 0.0, 0.0];
        joints[joint::SPINE_CERVICAL].rotation_rad = [deg(5.0), 0.0, 0.0];
        joints[joint::NECK].rotation_rad = [deg(4.0), 0.0, 0.0];
        joints[joint::HEAD].rotation_rad = [deg(5.0), steer * 0.18, 0.0];

        // Seated driver legs: hip +95 deg, knee +65 deg (biomechanics.json).
        let hip_pitch = deg(SEATED_DRIVER_HIP_PITCH_DEG);
        let knee_pitch = deg(SEATED_DRIVER_KNEE_PITCH_DEG);
        for (hip_i, knee_i, ankle_i, toe_i, x_sign) in [
            (joint::HIP_L, joint::KNEE_L, joint::ANKLE_L, joint::TOE_L, 1.0_f32),
            (joint::HIP_R, joint::KNEE_R, joint::ANKLE_R, joint::TOE_R, -1.0_f32),
        ] {
            joints[hip_i].rotation_rad = [hip_pitch, deg(4.0 * x_sign), deg(6.0 * x_sign)];
            joints[knee_i].rotation_rad = [knee_pitch, 0.0, 0.0];
            joints[ankle_i].rotation_rad = [deg(if x_sign > 0.0 { -10.0 } else { -14.0 }), 0.0, 0.0];
            joints[toe_i].rotation_rad = [deg(if x_sign > 0.0 { 8.0 } else { 10.0 }), 0.0, 0.0];

            let hip_pos = joints[hip_i].local_pos_m;
            // Forward thigh vector rotated +95 deg from straight down (0, -1).
            let thigh_z = hip_pitch.sin() * LEG_UPPER_LENGTH_M;
            let thigh_y = -hip_pitch.cos() * LEG_UPPER_LENGTH_M;
            let knee_pos = [hip_pos[0], hip_pos[1] + thigh_y, hip_pos[2] + thigh_z];
            let shin_angle = hip_pitch - knee_pitch;
            let shin_z = shin_angle.sin() * LEG_LOWER_LENGTH_M;
            let shin_y = -shin_angle.cos() * LEG_LOWER_LENGTH_M;
            let ankle_pos = [knee_pos[0], knee_pos[1] + shin_y, knee_pos[2] + shin_z];

            joints[knee_i].local_pos_m = knee_pos;
            joints[ankle_i].local_pos_m = ankle_pos;
            joints[toe_i].local_pos_m = [ankle_pos[0], ankle_pos[1] - 0.04, ankle_pos[2] + 0.14];
        }

        // Steering wheel hand targets rotated by steering_angle_rad, solved with two-bone arm IK.
        let (target_l_rel, target_r_rel) = steering_wheel_hand_targets_rel_pelvis(steer);
        let target_l = [
            pelvis[0] + target_l_rel[0],
            pelvis[1] + target_l_rel[1],
            pelvis[2] + target_l_rel[2],
        ];
        let target_r = [
            pelvis[0] + target_r_rel[0],
            pelvis[1] + target_r_rel[1],
            pelvis[2] + target_r_rel[2],
        ];

        solve_seated_arm_ik(
            &mut joints,
            joint::CLAVICLE_L,
            joint::SHOULDER_L,
            joint::ELBOW_L,
            joint::WRIST_L,
            joint::HAND_L,
            joint::THUMB_L,
            target_l,
            1.0,
        );
        solve_seated_arm_ik(
            &mut joints,
            joint::CLAVICLE_R,
            joint::SHOULDER_R,
            joint::ELBOW_R,
            joint::WRIST_R,
            joint::HAND_R,
            joint::THUMB_R,
            target_r,
            -1.0,
        );

        let left_foot_world_y_m = ground_center_y + joints[joint::ANKLE_L].local_pos_m[1] - ANKLE_HEIGHT_M;
        let right_foot_world_y_m = ground_center_y + joints[joint::ANKLE_R].local_pos_m[1] - ANKLE_HEIGHT_M;

        return CharacterPose {
            state,
            joints,
            pelvis_offset_m: pelvis,
            left_foot_world_y_m,
            right_foot_world_y_m,
            gait_phase_rad: gait_phase,
            breath_phase_rad: breath_phase,
            chest_expansion_m,
            slope_lean_rad: 0.0,
            steering_angle_rad: steer,
            left_hand_target_local_m: target_l,
            right_hand_target_local_m: target_r,
        };
    }

    // Standing / Walking / Jogging / Sprinting / Airborne / Inspecting branch.
    let slope_lean_rad = (slope * 0.45).clamp(-0.30, 0.30);
    let gait_bob_m = if stride_half_m > 0.0 {
        -0.025 * (2.0 * gait_phase).cos().abs() * motion_scale
    } else {
        0.004 * breath_phase.sin() * motion_scale
    };

    // Drop pelvis on slopes so the downhill leg remains inside the 2-bone IK reach envelope.
    let pelvis_drop_m = if state == CharacterPoseState::Airborne {
        0.06
    } else {
        dh_l.min(dh_r).min(0.0) - 0.015
    };
    let pelvis_offset_m = [0.0, STANDING_HIP_HEIGHT_M + pelvis_drop_m + gait_bob_m, 0.0];
    let dy_root = pelvis_offset_m[1] - STANDING_HIP_HEIGHT_M;

    for j in &mut joints {
        j.local_pos_m[1] += dy_root;
    }

    // Spine lean into uphill slope + counter-rotation during gait.
    let torso_twist = if stride_half_m > 0.0 {
        0.10 * gait_phase.sin() * motion_scale
    } else {
        0.0
    };
    let forward_lean = slope_lean_rad
        + match state {
            CharacterPoseState::Walking => 0.05,
            CharacterPoseState::Jogging => 0.12,
            CharacterPoseState::Sprinting => 0.22,
            CharacterPoseState::Inspecting => 0.16,
            _ => 0.0,
        };
    joints[joint::SPINE_LUMBAR].rotation_rad = [forward_lean * 0.45, torso_twist * 0.5, 0.0];
    joints[joint::SPINE_THORACIC].rotation_rad = [forward_lean * 0.55, torso_twist, 0.0];
    joints[joint::HEAD].rotation_rad = [-forward_lean * 0.35, 0.0, 0.0];

    // Arms: neutral relaxed arms-down, gait counter-swing, or inspecting reach.
    let mut left_hand_target_local_m = [0.22, pelvis_offset_m[1] - 0.12, 0.05];
    let mut right_hand_target_local_m = [-0.22, pelvis_offset_m[1] - 0.12, 0.05];

    if state == CharacterPoseState::Inspecting {
        left_hand_target_local_m = [
            pelvis_offset_m[0] + INSPECT_REACH_TARGET_L_M[0],
            pelvis_offset_m[1] + INSPECT_REACH_TARGET_L_M[1],
            pelvis_offset_m[2] + INSPECT_REACH_TARGET_L_M[2],
        ];
        right_hand_target_local_m = [
            pelvis_offset_m[0] + INSPECT_REACH_TARGET_R_M[0],
            pelvis_offset_m[1] + INSPECT_REACH_TARGET_R_M[1],
            pelvis_offset_m[2] + INSPECT_REACH_TARGET_R_M[2],
        ];
        solve_seated_arm_ik(
            &mut joints,
            joint::CLAVICLE_L,
            joint::SHOULDER_L,
            joint::ELBOW_L,
            joint::WRIST_L,
            joint::HAND_L,
            joint::THUMB_L,
            left_hand_target_local_m,
            1.0,
        );
        solve_seated_arm_ik(
            &mut joints,
            joint::CLAVICLE_R,
            joint::SHOULDER_R,
            joint::ELBOW_R,
            joint::WRIST_R,
            joint::HAND_R,
            joint::THUMB_R,
            right_hand_target_local_m,
            -1.0,
        );
    } else {
        let swing = arm_swing_rad * gait_phase.sin() * motion_scale;
        joints[joint::SHOULDER_L].rotation_rad = [-swing, 0.0, deg(-82.0)];
        joints[joint::SHOULDER_R].rotation_rad = [swing, 0.0, deg(82.0)];
        let elbow_bend = if stride_half_m > 0.0 {
            deg(22.0) + 0.35 * arm_swing_rad
        } else {
            deg(8.0)
        };
        joints[joint::ELBOW_L].rotation_rad = [elbow_bend, 0.0, 0.0];
        joints[joint::ELBOW_R].rotation_rad = [elbow_bend, 0.0, 0.0];
    }

    // Two-bone leg IK for ground-contacting feet (left phase = gait_phase, right = gait_phase + PI).
    let (foot_l_z, foot_l_lift) = if state == CharacterPoseState::Airborne {
        (0.10, 0.22)
    } else {
        let s = gait_phase.sin() * motion_scale;
        let lift = (gait_phase.cos().max(0.0)) * lift_m * motion_scale;
        (s * stride_half_m, lift)
    };
    let (foot_r_z, foot_r_lift) = if state == CharacterPoseState::Airborne {
        (-0.08, 0.16)
    } else {
        let phase_r = gait_phase + PI;
        let s = phase_r.sin() * motion_scale;
        let lift = (phase_r.cos().max(0.0)) * lift_m * motion_scale;
        (s * stride_half_m, lift)
    };

    let target_ankle_l = [foot_l_z, dh_l + ANKLE_HEIGHT_M + foot_l_lift];
    let target_ankle_r = [foot_r_z, dh_r + ANKLE_HEIGHT_M + foot_r_lift];

    let hip_l_pos = joints[joint::HIP_L].local_pos_m;
    let ik_l = solve_two_bone_ik_2d(
        [hip_l_pos[2], hip_l_pos[1]],
        target_ankle_l,
        LEG_UPPER_LENGTH_M,
        LEG_LOWER_LENGTH_M,
        1.0,
    );
    joints[joint::HIP_L].rotation_rad = [ik_l.upper_angle_rad, 0.0, 0.0];
    joints[joint::KNEE_L].rotation_rad = [ik_l.mid_bend_rad, 0.0, 0.0];
    joints[joint::ANKLE_L].rotation_rad = [slope - ik_l.upper_angle_rad + ik_l.mid_bend_rad * 0.5, 0.0, 0.0];
    joints[joint::KNEE_L].local_pos_m = [hip_l_pos[0], ik_l.mid_pos[1], ik_l.mid_pos[0]];
    joints[joint::ANKLE_L].local_pos_m = [hip_l_pos[0], ik_l.end_pos[1], ik_l.end_pos[0]];
    joints[joint::TOE_L].local_pos_m = [
        hip_l_pos[0],
        ik_l.end_pos[1] - 0.055,
        ik_l.end_pos[0] + 0.140,
    ];

    let hip_r_pos = joints[joint::HIP_R].local_pos_m;
    let ik_r = solve_two_bone_ik_2d(
        [hip_r_pos[2], hip_r_pos[1]],
        target_ankle_r,
        LEG_UPPER_LENGTH_M,
        LEG_LOWER_LENGTH_M,
        1.0,
    );
    joints[joint::HIP_R].rotation_rad = [ik_r.upper_angle_rad, 0.0, 0.0];
    joints[joint::KNEE_R].rotation_rad = [ik_r.mid_bend_rad, 0.0, 0.0];
    joints[joint::ANKLE_R].rotation_rad = [slope - ik_r.upper_angle_rad + ik_r.mid_bend_rad * 0.5, 0.0, 0.0];
    joints[joint::KNEE_R].local_pos_m = [hip_r_pos[0], ik_r.mid_pos[1], ik_r.mid_pos[0]];
    joints[joint::ANKLE_R].local_pos_m = [hip_r_pos[0], ik_r.end_pos[1], ik_r.end_pos[0]];
    joints[joint::TOE_R].local_pos_m = [
        hip_r_pos[0],
        ik_r.end_pos[1] - 0.055,
        ik_r.end_pos[0] + 0.140,
    ];

    let left_foot_world_y_m = ground_center_y + ik_l.end_pos[1] - ANKLE_HEIGHT_M;
    let right_foot_world_y_m = ground_center_y + ik_r.end_pos[1] - ANKLE_HEIGHT_M;

    CharacterPose {
        state,
        joints,
        pelvis_offset_m,
        left_foot_world_y_m,
        right_foot_world_y_m,
        gait_phase_rad: gait_phase,
        breath_phase_rad: breath_phase,
        chest_expansion_m,
        slope_lean_rad,
        steering_angle_rad: 0.0,
        left_hand_target_local_m,
        right_hand_target_local_m,
    }
}

#[allow(clippy::too_many_arguments)]
fn solve_seated_arm_ik(
    joints: &mut [JointPose; SKL2_JOINT_COUNT],
    clavicle_i: usize,
    shoulder_i: usize,
    elbow_i: usize,
    wrist_i: usize,
    hand_i: usize,
    thumb_i: usize,
    target: [f32; 3],
    x_sign: f32,
) {
    let sh = joints[shoulder_i].local_pos_m;
    let dx = target[0] - sh[0];
    let dy = target[1] - sh[1];
    let dz = target[2] - sh[2];
    let planar_z = (dx * dx + dz * dz).sqrt();
    let yaw = dx.atan2(dz.max(1e-4));

    // Bend elbow downward/posteriorly (-1.0) in the arm reach plane.
    let ik = solve_two_bone_ik_2d(
        [0.0, sh[1]],
        [planar_z, target[1]],
        ARM_UPPER_LENGTH_M,
        ARM_LOWER_LENGTH_M,
        -1.0,
    );

    let cos_y = yaw.cos();
    let sin_y = yaw.sin();
    let elbow_pos = [
        sh[0] + ik.mid_pos[0] * sin_y,
        ik.mid_pos[1],
        sh[2] + ik.mid_pos[0] * cos_y,
    ];
    let wrist_pos = [
        sh[0] + ik.end_pos[0] * sin_y,
        ik.end_pos[1],
        sh[2] + ik.end_pos[0] * cos_y,
    ];

    joints[clavicle_i].rotation_rad = [0.0, deg(12.0 * x_sign), deg(4.0 * x_sign)];
    joints[shoulder_i].rotation_rad = [ik.upper_angle_rad, yaw, deg(-68.0 * x_sign)];
    joints[elbow_i].rotation_rad = [ik.mid_bend_rad, 0.0, deg(40.0 * x_sign)];
    joints[wrist_i].rotation_rad = [deg(-8.0), deg(5.0 * x_sign), 0.0];
    joints[hand_i].rotation_rad = [deg(55.0), 0.0, 0.0];
    joints[thumb_i].rotation_rad = [deg(38.0), deg(32.0 * x_sign), 0.0];

    joints[elbow_i].local_pos_m = elbow_pos;
    joints[wrist_i].local_pos_m = wrist_pos;
    let reach_norm = (dx * dx + dy * dy + dz * dz).sqrt().max(1e-4);
    joints[hand_i].local_pos_m = [
        wrist_pos[0] + (dx / reach_norm) * 0.04,
        wrist_pos[1] + (dy / reach_norm) * 0.04,
        wrist_pos[2] + (dz / reach_norm) * 0.04,
    ];
    joints[thumb_i].local_pos_m = [
        joints[hand_i].local_pos_m[0] - 0.02 * x_sign,
        joints[hand_i].local_pos_m[1] + 0.02,
        joints[hand_i].local_pos_m[2] + 0.02,
    ];
}

/// Pure stateless entry point for evaluating the 28-joint `SKL2` pose from a zero phase baseline.
pub fn evaluate_character_pose(
    state: CharacterPoseState,
    ground_left_y: f32,
    ground_right_y: f32,
    slope_rad: f32,
    steering_angle_rad: f32,
    dt_s: f32,
    reduced_motion: bool,
) -> CharacterPose {
    evaluate_character_pose_with_phases(
        state,
        ground_left_y,
        ground_right_y,
        slope_rad,
        steering_angle_rad,
        dt_s,
        reduced_motion,
        0.0,
        0.0,
    )
}

/// Solved 1st-person or 3rd-person chase camera rig.
#[derive(Clone, Copy, Debug, PartialEq)]
pub struct CharacterCameraRig {
    pub perspective: CameraPerspective,
    pub eye_pos: [f32; 3],
    pub target_pos: [f32; 3],
    pub orbit_zoom: f32,
    pub terrain_clamped: bool,
}

/// Compute 1st-person eye position (`+1.65 m`) or 3rd-person chase camera
/// (behind & above right shoulder, clamped above `terrain_height(x, z) + 0.35 m`).
///
/// Uses `locomotion.rs` heading convention:
/// forward XZ = `[-sin(yaw), -cos(yaw)]`, right XZ = `[cos(yaw), -sin(yaw)]`.
pub fn compute_character_camera<F>(
    perspective: CameraPerspective,
    char_pos: [f32; 3],
    yaw: f32,
    pitch: f32,
    chase_distance_m: f32,
    terrain_height_fn: F,
) -> CharacterCameraRig
where
    F: Fn(f32, f32) -> f32,
{
    let sin_y = yaw.sin();
    let cos_y = yaw.cos();
    let fwd_x = -sin_y;
    let fwd_z = -cos_y;
    let right_x = cos_y;
    let right_z = -sin_y;

    match perspective {
        CameraPerspective::FirstPerson => {
            let eye_pos = [char_pos[0], char_pos[1] + EYE_HEIGHT_M, char_pos[2]];
            let cos_p = pitch.cos();
            let sin_p = pitch.sin();
            let target_pos = [
                eye_pos[0] + fwd_x * cos_p,
                eye_pos[1] - sin_p,
                eye_pos[2] + fwd_z * cos_p,
            ];
            CharacterCameraRig {
                perspective,
                eye_pos,
                target_pos,
                orbit_zoom: 0.15,
                terrain_clamped: false,
            }
        }
        CameraPerspective::ThirdPersonChase => {
            let dist = if chase_distance_m.is_finite() {
                chase_distance_m.clamp(1.6, 8.0)
            } else {
                DEFAULT_CHASE_DISTANCE_M
            };
            let shoulder_right_m = 0.42;
            let shoulder_up_m = 1.56;
            let target_pos = [
                char_pos[0] + right_x * shoulder_right_m,
                char_pos[1] + shoulder_up_m,
                char_pos[2] + right_z * shoulder_right_m,
            ];

            let horiz = dist * pitch.cos().max(0.25);
            let cam_x = target_pos[0] - fwd_x * horiz;
            let cam_z = target_pos[2] - fwd_z * horiz;
            let raw_y = target_pos[1] + 0.42 + dist * pitch.sin().clamp(-0.65, 0.85);
            let terrain_y = terrain_height_fn(cam_x, cam_z);
            let min_y = if terrain_y.is_finite() {
                terrain_y + THIRD_PERSON_TERRAIN_CLEARANCE_M
            } else {
                raw_y
            };
            let terrain_clamped = raw_y < min_y;
            let cam_y = raw_y.max(min_y);

            CharacterCameraRig {
                perspective,
                eye_pos: [cam_x, cam_y, cam_z],
                target_pos,
                orbit_zoom: dist,
                terrain_clamped,
            }
        }
    }
}

/// Stateful playable human explorer character combining locomotion posture,
/// camera perspective, `AnatomyXRayMode`, and `DeviceTier` LOD selection.
#[derive(Clone, Debug)]
pub struct ExplorerCharacter {
    pub pos: [f32; 3],
    pub yaw: f32,
    pub pitch: f32,
    pub speed_mps: f32,
    pub grounded: bool,
    pub pose_state: CharacterPoseState,
    pub perspective: CameraPerspective,
    pub anatomy_xray: AnatomyXRayMode,
    pub device_tier: DeviceTier,
    pub gait_phase_rad: f32,
    pub breath_phase_rad: f32,
    pub chase_distance_m: f32,
    pub last_pose: CharacterPose,
}

impl Default for ExplorerCharacter {
    fn default() -> Self {
        Self::new(DeviceTier::default())
    }
}

impl ExplorerCharacter {
    pub fn new(device_tier: DeviceTier) -> Self {
        let initial_pose = evaluate_character_pose(
            CharacterPoseState::IdleBreathing,
            0.0,
            0.0,
            0.0,
            0.0,
            0.0,
            false,
        );
        Self {
            pos: [-22.0, 4.4909215, 28.0],
            yaw: 0.63275,
            pitch: 0.17555,
            speed_mps: 0.0,
            grounded: true,
            pose_state: CharacterPoseState::IdleBreathing,
            perspective: CameraPerspective::ThirdPersonChase,
            anatomy_xray: AnatomyXRayMode::Integument,
            device_tier,
            gait_phase_rad: 0.0,
            breath_phase_rad: 0.0,
            chase_distance_m: DEFAULT_CHASE_DISTANCE_M,
            last_pose: initial_pose,
        }
    }

    pub fn toggle_perspective(&mut self) -> CameraPerspective {
        self.perspective = self.perspective.toggle();
        self.perspective
    }

    pub fn set_perspective(&mut self, perspective: CameraPerspective) {
        self.perspective = perspective;
    }

    pub fn toggle_anatomy_xray(&mut self) -> AnatomyXRayMode {
        self.anatomy_xray = self.anatomy_xray.toggle();
        self.anatomy_xray
    }

    pub fn set_anatomy_xray(&mut self, mode: AnatomyXRayMode) {
        self.anatomy_xray = mode;
    }

    pub fn active_asset(&self) -> CharacterAssetDescriptor {
        select_character_asset(self.device_tier, self.anatomy_xray)
    }

    pub fn active_asset_url(&self) -> &'static str {
        select_character_asset_url(self.device_tier, self.anatomy_xray)
    }

    #[allow(clippy::too_many_arguments)]
    pub fn update_pose(
        &mut self,
        state: CharacterPoseState,
        ground_left_y: f32,
        ground_right_y: f32,
        slope_rad: f32,
        steering_angle_rad: f32,
        dt_s: f32,
        reduced_motion: bool,
    ) -> &CharacterPose {
        self.pose_state = state;
        let pose = evaluate_character_pose_with_phases(
            state,
            ground_left_y,
            ground_right_y,
            slope_rad,
            steering_angle_rad,
            dt_s,
            reduced_motion,
            self.gait_phase_rad,
            self.breath_phase_rad,
        );
        self.gait_phase_rad = pose.gait_phase_rad;
        self.breath_phase_rad = pose.breath_phase_rad;
        self.last_pose = pose;
        &self.last_pose
    }

    pub fn camera_rig<F>(&self, terrain_height_fn: F) -> CharacterCameraRig
    where
        F: Fn(f32, f32) -> f32,
    {
        compute_character_camera(
            self.perspective,
            self.pos,
            self.yaw,
            self.pitch,
            self.chase_distance_m,
            terrain_height_fn,
        )
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn dist2(a: [f32; 2], b: [f32; 2]) -> f32 {
        ((a[0] - b[0]).powi(2) + (a[1] - b[1]).powi(2)).sqrt()
    }

    fn dist3(a: [f32; 3], b: [f32; 3]) -> f32 {
        ((a[0] - b[0]).powi(2) + (a[1] - b[1]).powi(2) + (a[2] - b[2]).powi(2)).sqrt()
    }

    #[test]
    fn skl2_rest_pose_has_28_joints_and_valid_hierarchy() {
        assert_eq!(SKL2_REST_JOINTS.len(), SKL2_JOINT_COUNT);
        assert_eq!(SKL2_JOINT_COUNT, 28);
        let rest = rest_pose_28();
        assert_eq!(rest.len(), 28);
        for (i, spec) in SKL2_REST_JOINTS.iter().enumerate() {
            assert_eq!(spec.index, i);
            if i == 0 {
                assert_eq!(spec.parent_index, -1);
                assert!((spec.rest_world_m[1] - STANDING_HIP_HEIGHT_M).abs() < 1e-4);
            } else {
                assert!(spec.parent_index >= 0 && (spec.parent_index as usize) < i);
            }
            assert_eq!(rest[i].local_pos_m, spec.rest_world_m);
        }
        // Cranial vertex: head joint y (1.59) + 0.15 m == 1.74 m stature.
        assert!((rest[joint::HEAD].local_pos_m[1] + 0.15 - ADULT_STATURE_M).abs() < 1e-4);
        assert!((rest[joint::ANKLE_L].local_pos_m[1] - ANKLE_HEIGHT_M).abs() < 1e-4);
        assert!((rest[joint::ANKLE_R].local_pos_m[1] - ANKLE_HEIGHT_M).abs() < 1e-4);
    }

    #[test]
    fn two_bone_foot_ik_grounds_both_feet_on_20_degree_slope() {
        let slope_rad = 20.0_f32.to_radians();
        // Lateral / stance elevation difference across ~0.55 m on a 20 deg slope (~0.10 m each side).
        let base_y = 4.50_f32;
        let dh = 0.28 * slope_rad.tan();
        let ground_left_y = base_y + dh;
        let ground_right_y = base_y - dh;

        let pose = evaluate_character_pose(
            CharacterPoseState::IdleBreathing,
            ground_left_y,
            ground_right_y,
            slope_rad,
            0.0,
            0.016,
            false,
        );

        assert!(
            (pose.left_foot_world_y_m - ground_left_y).abs() < 0.005,
            "left foot expected {ground_left_y:.4}, got {:.4}",
            pose.left_foot_world_y_m
        );
        assert!(
            (pose.right_foot_world_y_m - ground_right_y).abs() < 0.005,
            "right foot expected {ground_right_y:.4}, got {:.4}",
            pose.right_foot_world_y_m
        );
        // Uphill (left) knee must flex more than downhill (right) knee.
        assert!(
            pose.joints[joint::KNEE_L].rotation_rad[0]
                > pose.joints[joint::KNEE_R].rotation_rad[0],
            "uphill knee should flex more than downhill knee"
        );
        // Verify segment lengths are preserved by the analytic 2-bone IK solver.
        let hip_l = pose.joints[joint::HIP_L].local_pos_m;
        let knee_l = pose.joints[joint::KNEE_L].local_pos_m;
        let ankle_l = pose.joints[joint::ANKLE_L].local_pos_m;
        assert!(
            (dist2([hip_l[2], hip_l[1]], [knee_l[2], knee_l[1]]) - LEG_UPPER_LENGTH_M).abs() < 1e-3
        );
        assert!(
            (dist2([knee_l[2], knee_l[1]], [ankle_l[2], ankle_l[1]]) - LEG_LOWER_LENGTH_M).abs()
                < 1e-3
        );
        // Spine leans into uphill slope.
        assert!(pose.slope_lean_rad > 0.0);
    }

    #[test]
    fn seated_driver_pose_and_steering_wheel_hand_tracking() {
        let straight = evaluate_character_pose(
            CharacterPoseState::SeatedDriver,
            4.5,
            4.5,
            0.0,
            0.0,
            0.016,
            false,
        );
        assert!(
            (straight.joints[joint::HIP_L].rotation_rad[0] - SEATED_DRIVER_HIP_PITCH_DEG.to_radians())
                .abs()
                < 1e-4
        );
        assert!(
            (straight.joints[joint::KNEE_L].rotation_rad[0]
                - SEATED_DRIVER_KNEE_PITCH_DEG.to_radians())
                .abs()
                < 1e-4
        );

        let pelvis = SEATED_DRIVER_PELVIS_OFFSET_M;
        let expected_l = [
            pelvis[0] + STEERING_WHEEL_TARGET_L_M[0],
            pelvis[1] + STEERING_WHEEL_TARGET_L_M[1],
            pelvis[2] + STEERING_WHEEL_TARGET_L_M[2],
        ];
        let expected_r = [
            pelvis[0] + STEERING_WHEEL_TARGET_R_M[0],
            pelvis[1] + STEERING_WHEEL_TARGET_R_M[1],
            pelvis[2] + STEERING_WHEEL_TARGET_R_M[2],
        ];
        assert!(dist3(straight.left_hand_target_local_m, expected_l) < 1e-4);
        assert!(dist3(straight.right_hand_target_local_m, expected_r) < 1e-4);

        // Steering left (+0.6 rad) rotates rim targets symmetrically around wheel center.
        let steered = evaluate_character_pose(
            CharacterPoseState::SeatedDriver,
            4.5,
            4.5,
            0.0,
            0.6,
            0.016,
            false,
        );
        assert!(
            steered.left_hand_target_local_m[1] > straight.left_hand_target_local_m[1],
            "left hand should rise on +0.6 rad steering input"
        );
        assert!(
            steered.right_hand_target_local_m[1] < straight.right_hand_target_local_m[1],
            "right hand should lower on +0.6 rad steering input"
        );
        // Wheel radius from center stays invariant under rotation.
        let wheel_center = [
            pelvis[0] + STEERING_WHEEL_CENTER_REL_PELVIS_M[0],
            pelvis[1] + STEERING_WHEEL_CENTER_REL_PELVIS_M[1],
            pelvis[2] + STEERING_WHEEL_CENTER_REL_PELVIS_M[2],
        ];
        let r0 = dist3(straight.left_hand_target_local_m, wheel_center);
        let r1 = dist3(steered.left_hand_target_local_m, wheel_center);
        assert!((r0 - r1).abs() < 1e-4, "steering rim radius changed: {r0} vs {r1}");
        // Wrist IK tracks the rotated steering target.
        assert!(
            dist3(
                steered.joints[joint::WRIST_L].local_pos_m,
                steered.left_hand_target_local_m
            ) < 0.08
        );
    }

    #[test]
    fn first_and_third_person_camera_clamps_above_terrain() {
        let char_pos = [0.0, 5.0, 0.0];
        let fp = compute_character_camera(
            CameraPerspective::FirstPerson,
            char_pos,
            0.0,
            0.0,
            3.4,
            |_, _| 5.0,
        );
        assert!((fp.eye_pos[1] - (5.0 + EYE_HEIGHT_M)).abs() < 1e-4);
        assert!(!fp.terrain_clamped);

        // Steep hill rising directly behind the character (+z when yaw = 0, fwd = -z).
        let tp_clamped = compute_character_camera(
            CameraPerspective::ThirdPersonChase,
            char_pos,
            0.0,
            -0.35,
            3.4,
            |_x, z| 5.0 + z.max(0.0) * 1.5,
        );
        let terrain_under_cam = 5.0 + tp_clamped.eye_pos[2].max(0.0) * 1.5;
        assert!(tp_clamped.terrain_clamped);
        assert!(
            tp_clamped.eye_pos[1] >= terrain_under_cam + THIRD_PERSON_TERRAIN_CLEARANCE_M - 1e-4,
            "3rd-person camera {} fell below terrain+0.35 ({})",
            tp_clamped.eye_pos[1],
            terrain_under_cam + THIRD_PERSON_TERRAIN_CLEARANCE_M
        );
        // Right-shoulder framing offset (x > 0 when yaw = 0 -> right = [+1, 0, 0]).
        assert!(tp_clamped.target_pos[0] > char_pos[0]);
    }

    #[test]
    fn anatomy_xray_and_device_tier_select_verified_assets() {
        assert_eq!(
            select_character_asset_url(DeviceTier::TierA_DesktopWebGPU, AnatomyXRayMode::Integument),
            HOMO_SAPIENS_LOD0_URL
        );
        assert_eq!(
            select_character_asset_url(
                DeviceTier::TierB_DesktopWebGL2_MobileWebGPU,
                AnatomyXRayMode::Integument
            ),
            HOMO_SAPIENS_LOD1_URL
        );
        assert_eq!(
            select_character_asset_url(DeviceTier::TierC_MobileWebGL2, AnatomyXRayMode::Integument),
            HOMO_SAPIENS_LOD2_URL
        );
        for tier in [
            DeviceTier::TierA_DesktopWebGPU,
            DeviceTier::TierB_DesktopWebGL2_MobileWebGPU,
            DeviceTier::TierC_MobileWebGL2,
        ] {
            let desc = select_character_asset(tier, AnatomyXRayMode::AnatomyXRay);
            assert_eq!(desc.url, HOMO_SAPIENS_ANATOMY_XRAY_URL);
            assert_eq!(desc.triangle_count, 5_088);
            assert_eq!(desc.bytes, 101_512);
        }
        assert_eq!(
            select_character_asset_url_for_tier_id("tier-a", AnatomyXRayMode::Integument),
            HOMO_SAPIENS_LOD0_URL
        );
        assert_eq!(
            select_character_asset_url_for_tier_id("tier-c", AnatomyXRayMode::Integument),
            HOMO_SAPIENS_LOD2_URL
        );
    }

    #[test]
    fn explorer_character_respects_reduced_motion_and_toggles() {
        let mut ch = ExplorerCharacter::new(DeviceTier::TierA_DesktopWebGPU);
        assert_eq!(ch.perspective, CameraPerspective::ThirdPersonChase);
        assert_eq!(ch.toggle_perspective(), CameraPerspective::FirstPerson);
        assert_eq!(ch.toggle_perspective(), CameraPerspective::ThirdPersonChase);

        assert_eq!(ch.anatomy_xray, AnatomyXRayMode::Integument);
        assert_eq!(ch.active_asset_url(), HOMO_SAPIENS_LOD0_URL);
        assert_eq!(ch.toggle_anatomy_xray(), AnatomyXRayMode::AnatomyXRay);
        assert_eq!(ch.active_asset_url(), HOMO_SAPIENS_ANATOMY_XRAY_URL);

        let pose_reduced = ch.update_pose(
            CharacterPoseState::Walking,
            4.5,
            4.5,
            0.0,
            0.0,
            0.05,
            true,
        );
        assert_eq!(pose_reduced.chest_expansion_m, 0.0);
        assert_eq!(pose_reduced.gait_phase_rad, 0.0);
    }

    #[test]
    fn pose_classification_inspect_reach_and_gait_progression() {
        assert_eq!(
            CharacterPoseState::classify(0.0, true, false, false),
            CharacterPoseState::IdleBreathing
        );
        assert_eq!(
            CharacterPoseState::classify(1.8, true, false, false),
            CharacterPoseState::Walking
        );
        assert_eq!(
            CharacterPoseState::classify(4.5, true, false, false),
            CharacterPoseState::Jogging
        );
        assert_eq!(
            CharacterPoseState::classify(7.5, true, false, false),
            CharacterPoseState::Sprinting
        );
        assert_eq!(
            CharacterPoseState::classify(2.0, false, false, false),
            CharacterPoseState::Airborne
        );
        assert_eq!(
            CharacterPoseState::classify(0.0, true, true, false),
            CharacterPoseState::Inspecting
        );
        assert_eq!(
            CharacterPoseState::classify(12.0, true, false, true),
            CharacterPoseState::SeatedDriver
        );

        let inspect = evaluate_character_pose(
            CharacterPoseState::Inspecting,
            4.5,
            4.5,
            0.0,
            0.0,
            0.05,
            false,
        );
        assert!(inspect.joints[joint::WRIST_L].local_pos_m[2] > 0.25);
        assert!(inspect.joints[joint::WRIST_R].local_pos_m[2] > 0.25);

        let mut ch = ExplorerCharacter::new(DeviceTier::TierB_DesktopWebGL2_MobileWebGPU);
        ch.update_pose(CharacterPoseState::Jogging, 4.5, 4.5, 0.0, 0.0, 0.05, false);
        let phase_1 = ch.gait_phase_rad;
        ch.update_pose(CharacterPoseState::Jogging, 4.5, 4.5, 0.0, 0.0, 0.05, false);
        assert!(ch.gait_phase_rad > phase_1);
    }
}
