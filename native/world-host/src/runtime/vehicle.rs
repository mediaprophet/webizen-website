//! Drivable VW Passat B7 Wagon + Articulated 1980s Jayco Wind-Up Camper Trailer dynamics.
//!
//! Pure-Rust, deterministic, zero-external-crate physics module supporting:
//! - 4-wheel independent spring-damper suspension (`FL`, `FR`, `RL`, `RR`) sampling terrain
//!   `height_at(x, z)` independently for realistic heave, pitch (nose dive/squat), and roll.
//! - Slip-angle tyre friction across 5 ecological/road `SurfaceType` grip profiles.
//! - Ackermann steering geometry with speed-sensitive damping, 6-speed automatic gearbox
//!   (`P / R / N / D1..D6`), steep-slope (`>28°`) traction loss clamp, and circular obstacle
//!   collision response (trees, campsite dome, community-grounds perimeter).
//! - Articulated Jayco camper trailer ball-hitch coupling with reverse-steering articulation,
//!   jackknife angle clamping (`±72°`), 2-wheel terrain-conforming pitch/roll, and hitch/unhitch.

#![allow(dead_code)]

use std::f64::consts::{FRAC_PI_2, PI};

// ---------------------------------------------------------------------------
// VW Passat B7 Wagon Physical Constants
// ---------------------------------------------------------------------------
pub const PASSAT_WHEELBASE_M: f64 = 2.712;
pub const PASSAT_TRACK_WIDTH_M: f64 = 1.552;
pub const PASSAT_KERB_MASS_KG: f64 = 1_560.0;
pub const PASSAT_WHEEL_RADIUS_M: f64 = 0.318;
pub const PASSAT_MAX_STEER_RAD: f64 = 0.56;
pub const PASSAT_HITCH_OFFSET_Z_M: f64 = -2.35;
pub const PASSAT_COLLISION_RADIUS_M: f64 = 1.15;
pub const PASSAT_CG_HEIGHT_M: f64 = 0.54;
pub const PASSAT_AERO_CDA: f64 = 0.694; // Cd 0.31 * frontal area 2.24 m^2

// 4-Wheel Spring-Damper Suspension Defaults
pub const SUSPENSION_K_SPRING_NPM: f64 = 24_000.0;
pub const SUSPENSION_C_DAMPER_NSPM: f64 = 3_400.0;
pub const SUSPENSION_REST_LENGTH_M: f64 = 0.42;
pub const SUSPENSION_MAX_TRAVEL_M: f64 = 0.28;

// ---------------------------------------------------------------------------
// 1980s Jayco Wind-Up Camper Trailer Physical Constants
// ---------------------------------------------------------------------------
pub const JAYCO_DRAWBAR_LEN_M: f64 = 2.95;
pub const JAYCO_TRACK_WIDTH_M: f64 = 1.85;
pub const JAYCO_MASS_KG: f64 = 980.0;
pub const JAYCO_WHEEL_RADIUS_M: f64 = 0.295;
pub const JAYCO_MAX_JACKKNIFE_DEG: f64 = 72.0;
pub const JAYCO_HITCH_PROXIMITY_M: f64 = 8.0;

// Environment & Slope Constants
pub const GRAVITY_MPS2: f64 = 9.81;
pub const STEEP_SLOPE_CLAMP_DEG: f64 = 28.0;

// Campsite Spawn (Gold VW Passat B7 facing east toward Road Corridor & Community Grounds)
pub const CAMPSITE_SPAWN_X: f64 = -22.0;
pub const CAMPSITE_SPAWN_Z: f64 = 28.0;
pub const CAMPSITE_SPAWN_YAW_RAD: f64 = -FRAC_PI_2; // forward_xz = [+1, 0] (+X east)

// ---------------------------------------------------------------------------
// Surface Grip, Gearbox, Camera & Corridor Zone Enums
// ---------------------------------------------------------------------------

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum SurfaceType {
    BitumenRoad,
    GravelTrack,
    GrassClearing,
    AlluvialSilt,
    CreekBank,
}

impl SurfaceType {
    /// Peak tyre-surface friction coefficient (mu).
    pub const fn friction_mu(self) -> f64 {
        match self {
            SurfaceType::BitumenRoad => 0.95,
            SurfaceType::GravelTrack => 0.72,
            SurfaceType::GrassClearing => 0.65,
            SurfaceType::AlluvialSilt => 0.52,
            SurfaceType::CreekBank => 0.40,
        }
    }

    /// Rolling resistance coefficient (`C_rr`).
    pub const fn rolling_resistance(self) -> f64 {
        match self {
            SurfaceType::BitumenRoad => 0.014,
            SurfaceType::GravelTrack => 0.032,
            SurfaceType::GrassClearing => 0.045,
            SurfaceType::AlluvialSilt => 0.075,
            SurfaceType::CreekBank => 0.110,
        }
    }

    pub const fn as_str(self) -> &'static str {
        match self {
            SurfaceType::BitumenRoad => "BitumenRoad",
            SurfaceType::GravelTrack => "GravelTrack",
            SurfaceType::GrassClearing => "GrassClearing",
            SurfaceType::AlluvialSilt => "AlluvialSilt",
            SurfaceType::CreekBank => "CreekBank",
        }
    }
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum Gear {
    Park,
    Reverse,
    Neutral,
    D1,
    D2,
    D3,
    D4,
    D5,
    D6,
}

impl Gear {
    pub const fn as_str(self) -> &'static str {
        match self {
            Gear::Park => "P",
            Gear::Reverse => "R",
            Gear::Neutral => "N",
            Gear::D1 => "D1",
            Gear::D2 => "D2",
            Gear::D3 => "D3",
            Gear::D4 => "D4",
            Gear::D5 => "D5",
            Gear::D6 => "D6",
        }
    }

    /// Approximate VW 6-speed DSG gear ratio.
    pub const fn ratio(self) -> f64 {
        match self {
            Gear::Park | Gear::Neutral => 0.0,
            Gear::Reverse => 3.15,
            Gear::D1 => 3.46,
            Gear::D2 => 2.05,
            Gear::D3 => 1.30,
            Gear::D4 => 0.90,
            Gear::D5 => 0.71,
            Gear::D6 => 0.58,
        }
    }
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum DriveCameraMode {
    ChaseFollow,
    BonnetHood,
    CockpitDriver,
}

impl DriveCameraMode {
    pub const fn as_str(self) -> &'static str {
        match self {
            DriveCameraMode::ChaseFollow => "ChaseFollow",
            DriveCameraMode::BonnetHood => "BonnetHood",
            DriveCameraMode::CockpitDriver => "CockpitDriver",
        }
    }

    pub const fn next(self) -> Self {
        match self {
            DriveCameraMode::ChaseFollow => DriveCameraMode::BonnetHood,
            DriveCameraMode::BonnetHood => DriveCameraMode::CockpitDriver,
            DriveCameraMode::CockpitDriver => DriveCameraMode::ChaseFollow,
        }
    }
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum CorridorZone {
    ForestCampsite,
    RoadCorridor,
    TownOutskirtsCommunityGrounds,
}

impl CorridorZone {
    pub const fn as_str(self) -> &'static str {
        match self {
            CorridorZone::ForestCampsite => "ForestCampsite",
            CorridorZone::RoadCorridor => "RoadCorridor",
            CorridorZone::TownOutskirtsCommunityGrounds => "TownOutskirtsCommunityGrounds",
        }
    }
}

// ---------------------------------------------------------------------------
// Input, Config, Obstacle, Wheel, Trailer & Telemetry Structs
// ---------------------------------------------------------------------------

#[derive(Clone, Copy, Debug, Default)]
pub struct VehicleInput {
    /// Accelerator pedal `[0.0, 1.0]`.
    pub throttle: f64,
    /// Brake / reverse pedal `[0.0, 1.0]`.
    pub brake: f64,
    /// Steering input `[-1.0, +1.0]` (`-1.0` = left, `+1.0` = right).
    pub steer: f64,
    /// Parking / handbrake (`Space`).
    pub handbrake: bool,
}

#[derive(Clone, Copy, Debug)]
pub struct CircleObstacle {
    pub x: f64,
    pub z: f64,
    pub radius: f64,
    pub label: &'static str,
}

#[derive(Clone, Copy, Debug)]
pub struct SuspensionConfig {
    pub k_spring: f64,
    pub c_damper: f64,
    pub rest_length: f64,
    pub max_travel: f64,
}

impl Default for SuspensionConfig {
    fn default() -> Self {
        Self {
            k_spring: SUSPENSION_K_SPRING_NPM,
            c_damper: SUSPENSION_C_DAMPER_NSPM,
            rest_length: SUSPENSION_REST_LENGTH_M,
            max_travel: SUSPENSION_MAX_TRAVEL_M,
        }
    }
}

/// Per-wheel suspension & visual articulation state (`[FL, FR, RL, RR]`).
#[derive(Clone, Copy, Debug, Default)]
pub struct WheelState {
    pub compression: [f64; 4],
    pub contact_y: [f64; 4],
    pub normal_force_n: [f64; 4],
    pub steer_angle_fl: f64,
    pub steer_angle_fr: f64,
    pub wheel_spin_rad: f64,
}

/// 1980s Jayco Wind-Up Camper Trailer kinematic & terrain-conforming state.
#[derive(Clone, Copy, Debug)]
pub struct JaycoTrailer {
    pub x: f64,
    pub y: f64,
    pub z: f64,
    pub yaw: f64,
    pub yaw_rate: f64,
    pub pitch_rad: f64,
    pub roll_rad: f64,
    /// Signed articulation angle `wrap(trailer.yaw - car.yaw)` clamped to `±72°`.
    pub hitch_angle_rad: f64,
    pub speed_mps: f64,
    pub wheel_spin_rad: f64,
    pub hitch_attached: bool,
}

#[derive(Clone, Copy, Debug)]
pub struct DriveCameraPose {
    pub yaw: f32,
    pub pitch: f32,
    pub zoom: f32,
    pub target: [f32; 3],
}

#[derive(Clone, Debug)]
pub struct VehicleTelemetry {
    pub active: bool,
    pub speed_kmh: f64,
    pub gear: &'static str,
    pub pitch_deg: f64,
    pub roll_deg: f64,
    pub steer_deg: f64,
    pub surface: &'static str,
    pub traction: f64,
    pub hitch_attached: bool,
    pub trailer_yaw_deg: f64,
    pub headlights: bool,
    pub brake_lights: bool,
    pub reverse_lights: bool,
    pub camera_mode: &'static str,
    pub x: f64,
    pub y: f64,
    pub z: f64,
    pub corridor_zone: &'static str,
}

// ---------------------------------------------------------------------------
// Deterministic Corridor Geometry, Terrain & Obstacles
// ---------------------------------------------------------------------------

#[inline]
fn smoothstep(edge0: f64, edge1: f64, x: f64) -> f64 {
    let t = ((x - edge0) / (edge1 - edge0)).clamp(0.0, 1.0);
    t * t * (3.0 - 2.0 * t)
}

#[inline]
pub fn wrap_angle(mut a: f64) -> f64 {
    while a > PI {
        a -= 2.0 * PI;
    }
    while a < -PI {
        a += 2.0 * PI;
    }
    a
}

/// Forward XZ unit vector matching `world-host` orbit camera convention (`[-sin(yaw), -cos(yaw)]`).
#[inline]
pub fn forward_xz(yaw: f64) -> (f64, f64) {
    (-yaw.sin(), -yaw.cos())
}

/// Right XZ unit vector matching `world-host` orbit camera convention (`[cos(yaw), -sin(yaw)]`).
#[inline]
pub fn right_xz(yaw: f64) -> (f64, f64) {
    (yaw.cos(), -yaw.sin())
}

/// Smooth road/track centreline `z(x)` connecting Forest Campsite `(-22, 28)` ->
/// Road Corridor `(100..300, z=50)` -> Town-Outskirts Community Grounds `(300..380, z=56)`.
pub fn corridor_road_centreline_z(x: f64) -> f64 {
    if x < 100.0 {
        let t = smoothstep(-22.0, 100.0, x);
        28.0 + (50.0 - 28.0) * t
    } else if x <= 300.0 {
        50.0 + 2.5 * ((x - 100.0) * 0.015).sin()
    } else {
        let t = smoothstep(300.0, 340.0, x);
        50.0 + (56.0 - 50.0) * t
    }
}

/// Classify corridor zone along the Forest Campsite -> Country Road Corridor ->
/// Town-Outskirts Community Grounds transect (aligned with `gis_join.rs`).
pub fn corridor_zone_at(x: f64, _z: f64) -> CorridorZone {
    if x >= 300.0 {
        CorridorZone::TownOutskirtsCommunityGrounds
    } else if x >= 100.0 {
        CorridorZone::RoadCorridor
    } else {
        CorridorZone::ForestCampsite
    }
}

/// Deterministic river centreline `x(z)` matching `locomotion::catchment_river_x`.
pub fn corridor_river_x(z: f64) -> f64 {
    let oz = smoothstep(65.0, 125.0, z.abs());
    (z * 0.044).sin() * 10.0
        + (z * 0.095).sin() * 3.0
        + oz * ((z * 0.0175).sin() * 26.0 + (z * 0.0085 + 0.7).sin() * 14.0)
}

/// Deterministic corridor heightfield covering Forest Campsite `(-22, 28)` through
/// the graded Road Corridor `(100..300)` into Town-Outskirts Community Grounds `(300..400)`.
pub fn corridor_height_at(x: f64, z: f64) -> f64 {
    // Gentle valley + road grading so the vehicle can drive smoothly end-to-end
    // while off-road flanks retain natural undulation and creek depression.
    let rx = corridor_river_x(z);
    let d_river = (x - rx).abs();
    let creek_dip = -(-((d_river / 7.0).powi(2))).exp() * 1.45;
    let natural = 2.6
        + 0.45 * (x * 0.025 + z * 0.02).sin()
        + 0.22 * (x * 0.065 - z * 0.04).cos()
        + 0.08 * (x * 0.14 + z * 0.11).sin()
        + creek_dip * (1.0 - smoothstep(45.0, 85.0, x));

    // Flatten campsite clearing around (-22, 28).
    let d_camp = ((x + 22.0).powi(2) + (z - 28.0).powi(2)).sqrt();
    let camp_blend = smoothstep(8.0, 18.0, d_camp);
    let camp_level = 2.85;
    let h_camp = camp_level + (natural - camp_level) * camp_blend;

    // Grade the corridor carriageway gently down toward community grounds (~2.1 m).
    let road_z = corridor_road_centreline_z(x);
    let d_road = (z - road_z).abs();
    let road_grade = 2.85 - 0.75 * smoothstep(-20.0, 340.0, x)
        + 0.35 * ((x - 100.0) * 0.02).sin() * smoothstep(80.0, 140.0, x);
    let on_corridor_x = smoothstep(-25.0, -10.0, x) * (1.0 - smoothstep(375.0, 395.0, x));
    let road_mask = (1.0 - smoothstep(4.5, 11.0, d_road)) * on_corridor_x;

    // Flatten the Community Grounds cleared oval around (354, 56).
    let d_oval = ((x - 354.0).powi(2) + (z - 56.0).powi(2)).sqrt();
    let oval_mask = 1.0 - smoothstep(26.0, 36.0, d_oval);

    let h_road = h_camp + (road_grade - h_camp) * road_mask;
    h_road + (2.10 - h_road) * oval_mask
}

/// Classify driving surface type at `(x, z)` given ground elevation `ground_y`.
pub fn surface_type_at(x: f64, z: f64, ground_y: f64) -> SurfaceType {
    let road_z = corridor_road_centreline_z(x);
    let d_road = (z - road_z).abs();

    // Sealed bitumen carriageway along the Road Corridor (x in 100..315)
    if (100.0..=315.0).contains(&x) && d_road <= 6.5 {
        return SurfaceType::BitumenRoad;
    }

    // Community grounds gravel loop (26..32 m around [354, 56]) & entry apron
    let d_oval = ((x - 354.0).powi(2) + (z - 56.0).powi(2)).sqrt();
    if (26.0..=32.0).contains(&d_oval) || ((315.0..=332.0).contains(&x) && d_road <= 5.5) {
        return SurfaceType::GravelTrack;
    }
    if d_oval < 26.0 {
        return SurfaceType::GrassClearing;
    }

    // Forest bush access track from campsite (-22, 28) to bitumen entry (100, 50)
    if (-16.0..100.0).contains(&x) && d_road <= 4.5 {
        return SurfaceType::GravelTrack;
    }

    // Campsite clearing grass
    let d_camp = ((x + 22.0).powi(2) + (z - 28.0).powi(2)).sqrt();
    if d_camp <= 16.0 {
        return SurfaceType::GrassClearing;
    }

    // Creek bank & alluvial silt near the catchment river channel
    let d_river = (x - corridor_river_x(z)).abs();
    if x < 95.0 && (ground_y <= 0.65 || d_river <= 6.5) {
        return SurfaceType::CreekBank;
    }
    if x < 95.0 && (ground_y <= 1.75 || d_river <= 14.0) {
        return SurfaceType::AlluvialSilt;
    }

    SurfaceType::GrassClearing
}

/// Default circular obstacles along the Forest Campsite, Road Corridor verges,
/// and Town-Outskirts Community Grounds perimeter (kept clear of the carriageway).
pub fn default_corridor_obstacles() -> Vec<CircleObstacle> {
    vec![
        // Campsite geodesic dome & firepit boulder
        CircleObstacle { x: -24.0, z: 37.5, radius: 3.4, label: "campsite-dome" },
        CircleObstacle { x: -15.5, z: 34.0, radius: 1.4, label: "campsite-firepit" },
        // Old-growth Eucalyptus viminalis trunks flanking the forest clearing
        CircleObstacle { x: -32.0, z: 20.0, radius: 1.2, label: "tree-eucalyptus-1" },
        CircleObstacle { x: -10.0, z: 18.0, radius: 1.1, label: "tree-eucalyptus-2" },
        CircleObstacle { x: 18.0, z: 24.0, radius: 1.2, label: "tree-eucalyptus-3" },
        CircleObstacle { x: 52.0, z: 54.0, radius: 1.1, label: "tree-eucalyptus-4" },
        CircleObstacle { x: 85.0, z: 36.0, radius: 1.2, label: "tree-eucalyptus-5" },
        // Road corridor roadside gums (outside 6.5 m bitumen carriageway)
        CircleObstacle { x: 145.0, z: 62.0, radius: 1.2, label: "tree-corridor-north" },
        CircleObstacle { x: 215.0, z: 37.0, radius: 1.2, label: "tree-corridor-south" },
        CircleObstacle { x: 275.0, z: 63.0, radius: 1.1, label: "tree-corridor-east" },
        // Town-Outskirts Community Grounds shelter pavilion & perimeter fence posts
        CircleObstacle { x: 320.0, z: 68.0, radius: 3.6, label: "community-shelter-pad" },
        CircleObstacle { x: 354.0, z: 91.0, radius: 2.0, label: "community-fence-north" },
        CircleObstacle { x: 354.0, z: 21.0, radius: 2.0, label: "community-fence-south" },
        CircleObstacle { x: 390.0, z: 56.0, radius: 2.2, label: "community-fence-east" },
    ]
}

// ---------------------------------------------------------------------------
// Ackermann Steering & VehicleRig Implementation
// ---------------------------------------------------------------------------

/// Compute exact Ackermann front-left and front-right wheel steering angles (radians).
/// Positive `center_steer_rad` steers RIGHT (so FR is inner wheel with larger angle).
pub fn ackermann_wheel_angles(
    center_steer_rad: f64,
    wheelbase: f64,
    track_width: f64,
) -> (f64, f64) {
    let delta = center_steer_rad.clamp(-PASSAT_MAX_STEER_RAD, PASSAT_MAX_STEER_RAD);
    if delta.abs() < 1e-5 {
        return (0.0, 0.0);
    }
    let sign = delta.signum();
    let r = wheelbase / delta.abs().tan();
    let half_w = track_width * 0.5;
    let inner = (wheelbase / (r - half_w).max(0.45)).atan();
    let outer = (wheelbase / (r + half_w)).atan();
    if sign > 0.0 {
        // Steering RIGHT: FR is inner, FL is outer
        (outer, inner)
    } else {
        // Steering LEFT: FL is inner, FR is outer
        (-inner, -outer)
    }
}

/// Complete VW Passat B7 Wagon + Articulated 1980s Jayco Camper physics rig.
#[derive(Clone, Debug)]
pub struct VehicleRig {
    pub active: bool,
    pub x: f64,
    pub y: f64,
    pub z: f64,
    pub vx: f64,
    pub vy: f64,
    pub vz: f64,
    pub yaw: f64,
    pub yaw_rate: f64,
    pub pitch_rad: f64,
    pub pitch_rate: f64,
    pub roll_rad: f64,
    pub roll_rate: f64,
    pub steer_rad: f64,
    /// Signed longitudinal velocity along vehicle forward axis (m/s).
    pub speed_mps: f64,
    pub gear: Gear,
    pub surface: SurfaceType,
    /// Current effective tyre traction `[0.0, 1.0]`.
    pub traction: f64,
    /// Uphill terrain pitch angle in degrees (`>0` = climbing uphill).
    pub slope_deg: f64,
    pub suspension: SuspensionConfig,
    pub wheels: WheelState,
    pub trailer: JaycoTrailer,
    pub camera_mode: DriveCameraMode,
    pub headlights_on: bool,
    pub brake_lights_on: bool,
    pub reverse_lights_on: bool,
    pub corridor_zone: CorridorZone,
    pub last_collision: Option<&'static str>,
}

impl VehicleRig {
    /// Spawn the gold VW Passat B7 Wagon at the Forest Campsite (`x = -22.0, z = 28.0`)
    /// facing east along the corridor, with the 1980s Jayco Wind-Up Camper positioned
    /// directly behind the rear hitch ball ready to hitch and drive.
    pub fn spawn_campsite() -> Self {
        Self::spawn_at(
            CAMPSITE_SPAWN_X,
            CAMPSITE_SPAWN_Z,
            CAMPSITE_SPAWN_YAW_RAD,
            corridor_height_at,
        )
    }

    /// Spawn at an arbitrary `(x, z, yaw)` resting in static suspension equilibrium on `height_at`.
    pub fn spawn_at<F>(x: f64, z: f64, yaw: f64, height_at: F) -> Self
    where
        F: Fn(f64, f64) -> f64,
    {
        let susp = SuspensionConfig::default();
        let static_sag = (PASSAT_KERB_MASS_KG * GRAVITY_MPS2 * 0.25 / susp.k_spring)
            .clamp(0.0, susp.max_travel);
        let ride_height = susp.rest_length - static_sag;
        let ground_y = height_at(x, z);

        let (fx, fz) = forward_xz(yaw);
        let hx = x + fx * PASSAT_HITCH_OFFSET_Z_M;
        let hz = z + fz * PASSAT_HITCH_OFFSET_Z_M;
        let tx = hx - fx * JAYCO_DRAWBAR_LEN_M;
        let tz = hz - fz * JAYCO_DRAWBAR_LEN_M;
        let ty = height_at(tx, tz) + JAYCO_WHEEL_RADIUS_M;

        let surface = surface_type_at(x, z, ground_y);
        let zone = corridor_zone_at(x, z);

        let mut rig = Self {
            active: false,
            x,
            y: ground_y + ride_height,
            z,
            vx: 0.0,
            vy: 0.0,
            vz: 0.0,
            yaw: wrap_angle(yaw),
            yaw_rate: 0.0,
            pitch_rad: 0.0,
            pitch_rate: 0.0,
            roll_rad: 0.0,
            roll_rate: 0.0,
            steer_rad: 0.0,
            speed_mps: 0.0,
            gear: Gear::Park,
            surface,
            traction: surface.friction_mu(),
            slope_deg: 0.0,
            suspension: susp,
            wheels: WheelState {
                compression: [static_sag; 4],
                contact_y: [ground_y; 4],
                normal_force_n: [PASSAT_KERB_MASS_KG * GRAVITY_MPS2 * 0.25; 4],
                steer_angle_fl: 0.0,
                steer_angle_fr: 0.0,
                wheel_spin_rad: 0.0,
            },
            trailer: JaycoTrailer {
                x: tx,
                y: ty,
                z: tz,
                yaw: wrap_angle(yaw),
                yaw_rate: 0.0,
                pitch_rad: 0.0,
                roll_rad: 0.0,
                hitch_angle_rad: 0.0,
                speed_mps: 0.0,
                wheel_spin_rad: 0.0,
                hitch_attached: false,
            },
            camera_mode: DriveCameraMode::ChaseFollow,
            headlights_on: false,
            brake_lights_on: false,
            reverse_lights_on: false,
            corridor_zone: zone,
            last_collision: None,
        };
        rig.update_trailer_terrain(&height_at);
        rig
    }

    pub fn enter(&mut self) {
        self.active = true;
        if self.gear == Gear::Park && self.speed_mps.abs() < 0.05 {
            self.gear = Gear::Neutral;
        }
    }

    pub fn exit(&mut self) {
        self.active = false;
        self.brake_lights_on = false;
        self.reverse_lights_on = false;
        if self.speed_mps.abs() < 0.5 {
            self.vx = 0.0;
            self.vz = 0.0;
            self.speed_mps = 0.0;
            self.gear = Gear::Park;
        }
    }

    pub fn toggle_active(&mut self) -> bool {
        if self.active {
            self.exit();
        } else {
            self.enter();
        }
        self.active
    }

    /// Rear bumper ball-hitch world position `(hx, hz)`.
    pub fn hitch_ball_world_xz(&self) -> (f64, f64) {
        let (fx, fz) = forward_xz(self.yaw);
        (
            self.x + fx * PASSAT_HITCH_OFFSET_Z_M,
            self.z + fz * PASSAT_HITCH_OFFSET_Z_M,
        )
    }

    /// Distance in metres between the Passat rear hitch ball and the Jayco drawbar tongue tip.
    pub fn hitch_distance_m(&self) -> f64 {
        let (hx, hz) = self.hitch_ball_world_xz();
        let (tfx, tfz) = forward_xz(self.trailer.yaw);
        let tongue_x = self.trailer.x + tfx * JAYCO_DRAWBAR_LEN_M;
        let tongue_z = self.trailer.z + tfz * JAYCO_DRAWBAR_LEN_M;
        ((hx - tongue_x).powi(2) + (hz - tongue_z).powi(2)).sqrt()
    }

    /// Toggle Jayco camper hitch attachment. Attaches cleanly if the camper tongue is within
    /// `JAYCO_HITCH_PROXIMITY_M` (8 m) of the Passat rear ball hitch.
    pub fn toggle_hitch(&mut self) -> bool {
        if self.trailer.hitch_attached {
            self.trailer.hitch_attached = false;
            self.trailer.yaw_rate = 0.0;
            false
        } else if self.hitch_distance_m() <= JAYCO_HITCH_PROXIMITY_M {
            self.attach_trailer_now();
            true
        } else {
            false
        }
    }

    /// Explicitly attach or detach the Jayco camper (snapping tongue onto ball if attaching).
    pub fn set_hitch_attached(&mut self, attached: bool) -> bool {
        if attached {
            self.attach_trailer_now();
        } else {
            self.trailer.hitch_attached = false;
            self.trailer.yaw_rate = 0.0;
        }
        self.trailer.hitch_attached
    }

    fn attach_trailer_now(&mut self) {
        let (hx, hz) = self.hitch_ball_world_xz();
        let max_rad = JAYCO_MAX_JACKKNIFE_DEG.to_radians();
        let rel = wrap_angle(self.trailer.yaw - self.yaw).clamp(-max_rad, max_rad);
        self.trailer.yaw = wrap_angle(self.yaw + rel);
        self.trailer.hitch_angle_rad = rel;
        let (tfx, tfz) = forward_xz(self.trailer.yaw);
        self.trailer.x = hx - tfx * JAYCO_DRAWBAR_LEN_M;
        self.trailer.z = hz - tfz * JAYCO_DRAWBAR_LEN_M;
        self.trailer.hitch_attached = true;
    }

    pub fn toggle_headlights(&mut self) -> bool {
        self.headlights_on = !self.headlights_on;
        self.headlights_on
    }

    pub fn cycle_camera_mode(&mut self) -> DriveCameraMode {
        self.camera_mode = self.camera_mode.next();
        self.camera_mode
    }

    /// Reposition the vehicle (and attached trailer) to a named waypoint / tour stop.
    pub fn teleport_to<F>(&mut self, x: f64, z: f64, yaw: f64, height_at: F)
    where
        F: Fn(f64, f64) -> f64,
    {
        let was_active = self.active;
        let was_hitched = self.trailer.hitch_attached;
        let cam_mode = self.camera_mode;
        let lights = self.headlights_on;
        *self = Self::spawn_at(x, z, yaw, &height_at);
        self.active = was_active;
        self.camera_mode = cam_mode;
        self.headlights_on = lights;
        if was_hitched {
            self.attach_trailer_now();
            self.update_trailer_terrain(&height_at);
        }
    }

    /// Step physics on the default Forest-to-Community-Grounds corridor terrain & obstacles.
    pub fn step_corridor(&mut self, input: VehicleInput, dt_s: f64) {
        let obstacles = default_corridor_obstacles();
        self.step_with_terrain(input, dt_s, corridor_height_at, None, &obstacles);
    }

    /// Advance 4-wheel suspension, tyre friction, gearbox, obstacle collisions, and articulated
    /// Jayco camper trailer dynamics by `dt_s` seconds.
    pub fn step_with_terrain<F>(
        &mut self,
        input: VehicleInput,
        dt_s: f64,
        height_at: F,
        surface_override: Option<SurfaceType>,
        obstacles: &[CircleObstacle],
    ) where
        F: Fn(f64, f64) -> f64,
    {
        if !dt_s.is_finite() || dt_s <= 0.0 {
            return;
        }
        let total_dt = dt_s.clamp(0.0, 0.05);
        let substeps = ((total_dt / (1.0 / 120.0)).ceil() as usize).clamp(1, 8);
        let dt = total_dt / (substeps as f64);

        let throttle = if input.throttle.is_finite() {
            input.throttle.clamp(0.0, 1.0)
        } else {
            0.0
        };
        let brake = if input.brake.is_finite() {
            input.brake.clamp(0.0, 1.0)
        } else {
            0.0
        };
        let steer_cmd = if input.steer.is_finite() {
            input.steer.clamp(-1.0, 1.0)
        } else {
            0.0
        };

        self.last_collision = None;

        for _ in 0..substeps {
            self.step_substep(
                throttle,
                brake,
                steer_cmd,
                input.handbrake,
                dt,
                &height_at,
                surface_override,
                obstacles,
            );
        }
    }

    #[allow(clippy::too_many_arguments)]
    fn step_substep<F>(
        &mut self,
        throttle: f64,
        brake: f64,
        steer_cmd: f64,
        handbrake: bool,
        dt: f64,
        height_at: &F,
        surface_override: Option<SurfaceType>,
        obstacles: &[CircleObstacle],
    ) where
        F: Fn(f64, f64) -> f64,
    {
        let (fx, fz) = forward_xz(self.yaw);
        let (rx, rz) = right_xz(self.yaw);

        // 1. Sample 4 independent wheel contact patches [FL, FR, RL, RR]
        let half_l = PASSAT_WHEELBASE_M * 0.5;
        let half_w = PASSAT_TRACK_WIDTH_M * 0.5;
        let wheel_offsets: [(f64, f64); 4] = [
            (-half_w, half_l),  // FL
            (half_w, half_l),   // FR
            (-half_w, -half_l), // RL
            (half_w, -half_l),  // RR
        ];

        let mut ground_sum = 0.0;
        let mut total_fz = 0.0;
        let mut pitch_spring_torque = 0.0;
        let mut roll_spring_torque = 0.0;

        for (i, &(loc_r, loc_f)) in wheel_offsets.iter().enumerate() {
            let wx = self.x + rx * loc_r + fx * loc_f;
            let wz = self.z + rz * loc_r + fz * loc_f;
            let h_i = height_at(wx, wz);
            self.wheels.contact_y[i] = h_i;
            ground_sum += h_i;

            // Wheel mount height on sprung body:
            // pitch_rad > 0 => nose up; roll_rad > 0 => right side down.
            let mount_y = self.y + loc_f * self.pitch_rad.sin() - loc_r * self.roll_rad.sin();
            let raw_comp = self.suspension.rest_length - (mount_y - h_i);
            let comp = raw_comp.clamp(0.0, self.suspension.max_travel);
            self.wheels.compression[i] = comp;

            // Positive comp_vel when suspension is compressing
            let mount_vy = self.vy + loc_f * self.pitch_rate - loc_r * self.roll_rate;
            let comp_vel = -mount_vy;

            let f_z = if raw_comp > 0.0 {
                (self.suspension.k_spring * comp + self.suspension.c_damper * comp_vel).max(0.0)
            } else {
                0.0
            };
            self.wheels.normal_force_n[i] = f_z;
            total_fz += f_z;

            // Positive f_z on front wheels pushes nose UP (+pitch).
            pitch_spring_torque += f_z * loc_f;
            // Positive f_z on right wheels pushes right side UP (-roll).
            roll_spring_torque -= f_z * loc_r;
        }

        let avg_ground_y = ground_sum * 0.25;
        let front_ground = 0.5 * (self.wheels.contact_y[0] + self.wheels.contact_y[1]);
        let rear_ground = 0.5 * (self.wheels.contact_y[2] + self.wheels.contact_y[3]);
        let slope_rad = ((front_ground - rear_ground) / PASSAT_WHEELBASE_M).atan();
        self.slope_deg = slope_rad.to_degrees();

        // 2. Determine surface grip & corridor zone
        self.surface =
            surface_override.unwrap_or_else(|| surface_type_at(self.x, self.z, avg_ground_y));
        self.corridor_zone = corridor_zone_at(self.x, self.z);
        let mu = self.surface.friction_mu();
        let c_rr = self.surface.rolling_resistance();

        // 3. Decompose planar velocity into vehicle longitudinal & lateral components
        let mut v_long = self.vx * fx + self.vz * fz;
        let mut v_lat = self.vx * rx + self.vz * rz;

        // 4. Speed-sensitive steering smoothing + Ackermann front wheel angles
        let speed_damp = 1.0 / (1.0 + 0.016 * v_long * v_long);
        let target_steer = steer_cmd * PASSAT_MAX_STEER_RAD * speed_damp;
        let steer_rate_limit = 3.8; // rad/s rack response
        let max_delta = steer_rate_limit * dt;
        self.steer_rad += (target_steer - self.steer_rad).clamp(-max_delta, max_delta);
        let (fl_ang, fr_ang) =
            ackermann_wheel_angles(self.steer_rad, PASSAT_WHEELBASE_M, PASSAT_TRACK_WIDTH_M);
        self.wheels.steer_angle_fl = fl_ang;
        self.wheels.steer_angle_fr = fr_ang;

        // 5. Automatic gearbox selection (`P / R / N / D1..D6`)
        let speed_kmh = v_long.abs() * 3.6;
        let reversing_cmd = brake > 0.08 && throttle < 0.05 && v_long <= 0.35 && !handbrake;
        self.gear = if !self.active && v_long.abs() < 0.25 {
            Gear::Park
        } else if handbrake && v_long.abs() < 0.25 && throttle < 0.05 {
            Gear::Park
        } else if v_long < -0.15 || reversing_cmd {
            Gear::Reverse
        } else if v_long.abs() < 0.08 && throttle < 0.02 && brake < 0.02 {
            Gear::Neutral
        } else if speed_kmh < 22.0 {
            Gear::D1
        } else if speed_kmh < 42.0 {
            Gear::D2
        } else if speed_kmh < 66.0 {
            Gear::D3
        } else if speed_kmh < 92.0 {
            Gear::D4
        } else if speed_kmh < 125.0 {
            Gear::D5
        } else {
            Gear::D6
        };

        self.reverse_lights_on = self.gear == Gear::Reverse;
        self.brake_lights_on = (brake > 0.05 && !reversing_cmd) || handbrake;

        // 6. Longitudinal forces: engine, braking, gravity slope, rolling resistance, aero drag
        let total_mass = PASSAT_KERB_MASS_KG
            + if self.trailer.hitch_attached {
                JAYCO_MASS_KG
            } else {
                0.0
            };

        // Steep uphill slope traction clamp (>28 deg causes severe tyre slip / stall)
        let uphill_slope = if throttle > 0.0 {
            self.slope_deg.max(0.0)
        } else if reversing_cmd {
            (-self.slope_deg).max(0.0)
        } else {
            self.slope_deg.abs()
        };
        let slope_traction_factor = if uphill_slope > STEEP_SLOPE_CLAMP_DEG {
            (1.0 - (uphill_slope - STEEP_SLOPE_CLAMP_DEG) / 4.0).clamp(0.0, 0.25)
        } else {
            1.0
        };

        // Engine torque curve (VW Passat 2.0 TDI ~350 Nm through 6-speed DSG)
        let max_traction_force = mu * PASSAT_KERB_MASS_KG * GRAVITY_MPS2 * 0.85 * slope_traction_factor;
        let drive_force = if throttle > 0.0 && !handbrake && self.gear != Gear::Reverse {
            let gear_ratio = self.gear.ratio().max(0.58);
            let raw_drive = throttle * (3_450.0 * gear_ratio).min(8_200.0);
            // Soft speed governor near 145 km/h (40 m/s)
            let speed_fade = (1.0 - (v_long / 42.0).clamp(0.0, 1.0)).max(0.0);
            (raw_drive * (0.35 + 0.65 * speed_fade)).min(max_traction_force)
        } else if reversing_cmd {
            let rev_fade = (1.0 - ((-v_long) / 8.5).clamp(0.0, 1.0)).max(0.0);
            -(brake * 3_400.0 * rev_fade).min(max_traction_force)
        } else {
            0.0
        };

        // Service brake + handbrake opposing current longitudinal motion
        let mut brake_force = 0.0;
        if brake > 0.0 && !reversing_cmd {
            brake_force += brake * mu * total_mass * GRAVITY_MPS2 * 0.92;
        }
        if handbrake {
            brake_force += mu * total_mass * GRAVITY_MPS2 * 0.78;
        }
        if !self.active {
            // Gentle coasting / parking drag when unoccupied
            brake_force += total_mass * 3.5;
        }

        let f_grav = -total_mass * GRAVITY_MPS2 * slope_rad.sin();
        let f_aero = -0.5 * 1.225 * PASSAT_AERO_CDA * v_long * v_long.abs();
        let f_roll = if v_long.abs() > 0.02 {
            -c_rr * total_mass * GRAVITY_MPS2 * v_long.signum()
        } else {
            0.0
        };

        let mut f_long = drive_force + f_grav + f_aero + f_roll;
        if v_long.abs() > 0.02 {
            let max_stop = (v_long.abs() * total_mass) / dt;
            f_long -= brake_force.min(max_stop) * v_long.signum();
        } else if drive_force.abs() + f_grav.abs() <= brake_force + c_rr * total_mass * GRAVITY_MPS2 {
            // Static friction holds vehicle stationary
            v_long = 0.0;
            f_long = 0.0;
        }

        let a_long = f_long / total_mass;
        v_long += a_long * dt;
        if brake_force > 0.0 && v_long.abs() < 0.04 && drive_force.abs() < 50.0 {
            v_long = 0.0;
        }

        // 7. Lateral slip-angle tyre friction + bicycle/Ackermann yaw dynamics
        // Turning right (steer_rad > 0) with v_long > 0 rotates yaw negatively in [-sin, -cos] frame.
        let desired_yaw_rate = if v_long.abs() > 0.05 {
            -(v_long / PASSAT_WHEELBASE_M) * self.steer_rad.tan()
        } else {
            0.0
        };
        // Limit yaw rate by surface lateral friction mu * g / |v|
        let max_yaw_rate = if v_long.abs() > 1.0 {
            (mu * GRAVITY_MPS2 * 0.95) / v_long.abs()
        } else {
            3.5
        };
        let clamped_yaw_rate = desired_yaw_rate.clamp(-max_yaw_rate, max_yaw_rate);
        let yaw_response = (mu * 10.0 * dt).clamp(0.0, 1.0);
        self.yaw_rate += (clamped_yaw_rate - self.yaw_rate) * yaw_response;
        self.yaw = wrap_angle(self.yaw + self.yaw_rate * dt);

        // Slip-angle lateral restoring force (Pacejka-style saturation)
        let slip_angle = v_lat.atan2(v_long.abs().max(1.2));
        let pacejka_lat = -mu * total_mass * GRAVITY_MPS2 * (1.35 * (6.0 * slip_angle).atan()).sin();
        let a_lat_tyre = pacejka_lat / total_mass;
        let next_v_lat = v_lat + a_lat_tyre * dt;
        // Prevent sign overshoot on stiff low-speed damping
        v_lat = if v_lat * next_v_lat < 0.0 {
            0.0
        } else {
            next_v_lat
        };
        if v_long.abs() < 0.1 && v_lat.abs() < 0.1 {
            v_lat = 0.0;
        }

        // Centripetal lateral acceleration for body roll (positive right turn => body leans left)
        let a_lat_centripetal = -v_long * self.yaw_rate;

        // Effective traction readout `[0.0, 1.0]`
        let slip_penalty = (slip_angle.abs() * 0.85).clamp(0.0, 0.55);
        self.traction = (mu * slope_traction_factor * (1.0 - slip_penalty)).clamp(0.0, 1.0);

        // Reconstruct world-space planar velocity & integrate position
        let (nfx, nfz) = forward_xz(self.yaw);
        let (nrx, nrz) = right_xz(self.yaw);
        self.vx = nfx * v_long + nrx * v_lat;
        self.vz = nfz * v_long + nrz * v_lat;
        self.speed_mps = v_long;

        self.x += self.vx * dt;
        self.z += self.vz * dt;
        self.wheels.wheel_spin_rad += (v_long * dt) / PASSAT_WHEEL_RADIUS_M;

        // 8. Circular obstacle collision clamping (trees, campsite dome, community grounds fence)
        for obs in obstacles {
            let dx = self.x - obs.x;
            let dz = self.z - obs.z;
            let min_dist = PASSAT_COLLISION_RADIUS_M + obs.radius;
            let dist_sq = dx * dx + dz * dz;
            if dist_sq < min_dist * min_dist {
                let dist = dist_sq.sqrt();
                let (nx, nz) = if dist > 1e-6 {
                    (dx / dist, dz / dist)
                } else {
                    (1.0, 0.0)
                };
                self.x = obs.x + nx * min_dist;
                self.z = obs.z + nz * min_dist;
                let vn = self.vx * nx + self.vz * nz;
                if vn < 0.0 {
                    // Remove inward velocity component + gentle restitution
                    self.vx -= 1.15 * vn * nx;
                    self.vz -= 1.15 * vn * nz;
                    self.speed_mps = self.vx * nfx + self.vz * nfz;
                }
                self.last_collision = Some(obs.label);
            }
        }

        // 9. Integrate sprung body heave, pitch (nose dive/squat), and roll (cornering + uneven track)
        let a_y = (total_fz - PASSAT_KERB_MASS_KG * GRAVITY_MPS2) / PASSAT_KERB_MASS_KG;
        self.vy += a_y * dt;
        self.y += self.vy * dt;

        // Hard bump-stop floor so chassis never penetrates terrain
        let min_chassis_y = avg_ground_y + (self.suspension.rest_length - self.suspension.max_travel);
        if self.y < min_chassis_y {
            self.y = min_chassis_y;
            if self.vy < 0.0 {
                self.vy = 0.0;
            }
        }

        // Rotational inertias & load-transfer torques
        let i_pitch = 2_850.0;
        let i_roll = 1_250.0;
        let pitch_load_transfer = PASSAT_KERB_MASS_KG * a_long * PASSAT_CG_HEIGHT_M;
        let roll_load_transfer = -PASSAT_KERB_MASS_KG * a_lat_centripetal * PASSAT_CG_HEIGHT_M;

        let alpha_pitch = (pitch_spring_torque + pitch_load_transfer - 2_400.0 * self.pitch_rate) / i_pitch;
        let alpha_roll = (roll_spring_torque + roll_load_transfer - 1_600.0 * self.roll_rate) / i_roll;

        self.pitch_rate += alpha_pitch * dt;
        self.roll_rate += alpha_roll * dt;
        self.pitch_rad = (self.pitch_rad + self.pitch_rate * dt).clamp(-0.62, 0.62);
        self.roll_rad = (self.roll_rad + self.roll_rate * dt).clamp(-0.55, 0.55);

        // 10. Articulated 1980s Jayco Wind-Up Camper Trailer dynamics
        self.step_trailer(v_long, dt, height_at);
    }

    fn step_trailer<F>(&mut self, car_v_long: f64, dt: f64, height_at: &F)
    where
        F: Fn(f64, f64) -> f64,
    {
        if self.trailer.hitch_attached {
            let rel_angle = wrap_angle(self.yaw - self.trailer.yaw);
            // Kinematic + dynamic single-axle articulated trailer yaw rate:
            // Works seamlessly in forward AND reverse (reverse-steering articulation).
            let hitch_arm = PASSAT_HITCH_OFFSET_Z_M.abs();
            let trailer_yaw_rate = (car_v_long / JAYCO_DRAWBAR_LEN_M) * rel_angle.sin()
                - (hitch_arm / JAYCO_DRAWBAR_LEN_M) * self.yaw_rate * rel_angle.cos();

            self.trailer.yaw_rate = trailer_yaw_rate;
            let raw_yaw = wrap_angle(self.trailer.yaw + trailer_yaw_rate * dt);

            // Enforce strict ±72° jackknife angle clamp relative to Passat heading
            let max_jackknife_rad = JAYCO_MAX_JACKKNIFE_DEG.to_radians();
            let clamped_rel = wrap_angle(raw_yaw - self.yaw).clamp(-max_jackknife_rad, max_jackknife_rad);
            self.trailer.hitch_angle_rad = clamped_rel;
            self.trailer.yaw = wrap_angle(self.yaw + clamped_rel);

            let (hx, hz) = self.hitch_ball_world_xz();
            let (tfx, tfz) = forward_xz(self.trailer.yaw);
            self.trailer.x = hx - tfx * JAYCO_DRAWBAR_LEN_M;
            self.trailer.z = hz - tfz * JAYCO_DRAWBAR_LEN_M;
            self.trailer.speed_mps = car_v_long * clamped_rel.cos();
            self.trailer.wheel_spin_rad += (self.trailer.speed_mps * dt) / JAYCO_WHEEL_RADIUS_M;
        } else {
            // Decoupled: gently brake to a parked stop where unhitched
            let decel = 6.5 * dt;
            if self.trailer.speed_mps.abs() <= decel {
                self.trailer.speed_mps = 0.0;
            } else {
                self.trailer.speed_mps -= self.trailer.speed_mps.signum() * decel;
                let (tfx, tfz) = forward_xz(self.trailer.yaw);
                self.trailer.x += tfx * self.trailer.speed_mps * dt;
                self.trailer.z += tfz * self.trailer.speed_mps * dt;
                self.trailer.wheel_spin_rad += (self.trailer.speed_mps * dt) / JAYCO_WHEEL_RADIUS_M;
            }
            self.trailer.yaw_rate = 0.0;
            self.trailer.hitch_angle_rad = wrap_angle(self.trailer.yaw - self.yaw);
        }

        self.update_trailer_terrain(height_at);
    }

    /// Sample 2-wheel trailer terrain height + hitch tongue height for realistic roll & pitch.
    fn update_trailer_terrain<F>(&mut self, height_at: &F)
    where
        F: Fn(f64, f64) -> f64,
    {
        let (tfx, tfz) = forward_xz(self.trailer.yaw);
        let (trx, trz) = right_xz(self.trailer.yaw);
        let half_tw = JAYCO_TRACK_WIDTH_M * 0.5;

        let h_left = height_at(self.trailer.x - trx * half_tw, self.trailer.z - trz * half_tw);
        let h_right = height_at(self.trailer.x + trx * half_tw, self.trailer.z + trz * half_tw);
        let axle_y = 0.5 * (h_left + h_right);
        self.trailer.y = axle_y + JAYCO_WHEEL_RADIUS_M;

        // Trailer roll > 0 when right wheel is lower than left wheel
        self.trailer.roll_rad = ((h_left - h_right) / JAYCO_TRACK_WIDTH_M).atan().clamp(-0.55, 0.55);

        let tongue_y = if self.trailer.hitch_attached {
            self.y - 0.08
        } else {
            height_at(
                self.trailer.x + tfx * JAYCO_DRAWBAR_LEN_M,
                self.trailer.z + tfz * JAYCO_DRAWBAR_LEN_M,
            ) + 0.34
        };
        self.trailer.pitch_rad = ((tongue_y - self.trailer.y) / JAYCO_DRAWBAR_LEN_M)
            .atan()
            .clamp(-0.55, 0.55);
    }

    /// Compute orbit-compatible camera pose for `ChaseFollow`, `BonnetHood`, or `CockpitDriver`.
    pub fn compute_drive_camera(&self) -> DriveCameraPose {
        let (fx, fz) = forward_xz(self.yaw);
        let (rx, rz) = right_xz(self.yaw);
        match self.camera_mode {
            DriveCameraMode::ChaseFollow => DriveCameraPose {
                yaw: self.yaw as f32,
                pitch: (0.24 - self.pitch_rad * 0.35).clamp(0.08, 0.65) as f32,
                zoom: if self.trailer.hitch_attached { 11.5 } else { 7.8 },
                target: [
                    (self.x + fx * 0.6) as f32,
                    (self.y + 1.25) as f32,
                    (self.z + fz * 0.6) as f32,
                ],
            },
            DriveCameraMode::BonnetHood => DriveCameraPose {
                yaw: self.yaw as f32,
                pitch: (0.06 - self.pitch_rad * 0.85).clamp(-0.35, 0.45) as f32,
                zoom: 0.45,
                target: [
                    (self.x + fx * 1.15) as f32,
                    (self.y + 1.18) as f32,
                    (self.z + fz * 1.15) as f32,
                ],
            },
            DriveCameraMode::CockpitDriver => DriveCameraPose {
                yaw: self.yaw as f32,
                pitch: (0.04 - self.pitch_rad * 0.90).clamp(-0.35, 0.45) as f32,
                zoom: 0.25,
                // Right-hand drive Australian VW Passat B7 driver seat offset (+0.38 m right)
                target: [
                    (self.x + fx * 0.15 + rx * 0.38) as f32,
                    (self.y + 1.22) as f32,
                    (self.z + fz * 0.15 + rz * 0.38) as f32,
                ],
            },
        }
    }

    /// Export frame-rate vehicle telemetry snapshot for `window.__webizenVehicleState`.
    pub fn telemetry(&self) -> VehicleTelemetry {
        VehicleTelemetry {
            active: self.active,
            speed_kmh: self.speed_mps * 3.6,
            gear: self.gear.as_str(),
            pitch_deg: self.pitch_rad.to_degrees(),
            roll_deg: self.roll_rad.to_degrees(),
            steer_deg: self.steer_rad.to_degrees(),
            surface: self.surface.as_str(),
            traction: self.traction,
            hitch_attached: self.trailer.hitch_attached,
            trailer_yaw_deg: self.trailer.yaw.to_degrees(),
            headlights: self.headlights_on,
            brake_lights: self.brake_lights_on,
            reverse_lights: self.reverse_lights_on,
            camera_mode: self.camera_mode.as_str(),
            x: self.x,
            y: self.y,
            z: self.z,
            corridor_zone: self.corridor_zone.as_str(),
        }
    }
}

// ---------------------------------------------------------------------------
// Comprehensive Unit Tests (standalone `rustc --edition 2021 --test vehicle.rs`)
// ---------------------------------------------------------------------------

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn suspension_equilibrium_and_sloped_pitch_roll() {
        // 1. Flat ground static equilibrium
        let mut rig = VehicleRig::spawn_at(0.0, 0.0, -FRAC_PI_2, |_, _| 2.0);
        rig.enter();
        for _ in 0..120 {
            rig.step_with_terrain(
                VehicleInput { handbrake: true, ..Default::default() },
                1.0 / 60.0,
                |_, _| 2.0,
                Some(SurfaceType::BitumenRoad),
                &[],
            );
        }
        let expected_sag = PASSAT_KERB_MASS_KG * GRAVITY_MPS2 * 0.25 / SUSPENSION_K_SPRING_NPM;
        for c in rig.wheels.compression {
            assert!(
                (c - expected_sag).abs() < 0.015,
                "wheel compression {c:.4} should settle near static sag {expected_sag:.4}"
            );
        }
        assert!(rig.pitch_rad.abs() < 0.02, "flat pitch should be near 0");
        assert!(rig.roll_rad.abs() < 0.02, "flat roll should be near 0");

        // 2. Sloped terrain (10 deg uphill along +X and 7 deg right-side drop along +Z)
        let slope_fn = |x: f64, z: f64| 2.0 + x * 10.0_f64.to_radians().tan() - z * 7.0_f64.to_radians().tan();
        let mut sloped = VehicleRig::spawn_at(0.0, 0.0, -FRAC_PI_2, slope_fn);
        sloped.enter();
        for _ in 0..180 {
            sloped.step_with_terrain(
                VehicleInput { handbrake: true, ..Default::default() },
                1.0 / 60.0,
                slope_fn,
                Some(SurfaceType::GravelTrack),
                &[],
            );
        }
        assert!(
            sloped.pitch_rad.to_degrees() > 6.0,
            "uphill terrain should produce positive nose-up pitch, got {:.2} deg",
            sloped.pitch_rad.to_degrees()
        );
        assert!(
            sloped.roll_rad.to_degrees() > 3.5,
            "right-side-down terrain should produce positive roll, got {:.2} deg",
            sloped.roll_rad.to_degrees()
        );
    }

    #[test]
    fn acceleration_gear_shifts_ackermann_steering_and_braking_distance() {
        let mut rig = VehicleRig::spawn_at(0.0, 0.0, -FRAC_PI_2, |_, _| 0.0);
        rig.enter();
        let mut saw_d1 = false;
        let mut saw_d2 = false;
        let mut saw_d3 = false;

        // Accelerate full throttle for 6 seconds on flat bitumen
        for _ in 0..360 {
            rig.step_with_terrain(
                VehicleInput { throttle: 1.0, ..Default::default() },
                1.0 / 60.0,
                |_, _| 0.0,
                Some(SurfaceType::BitumenRoad),
                &[],
            );
            match rig.gear {
                Gear::D1 => saw_d1 = true,
                Gear::D2 => saw_d2 = true,
                Gear::D3 => saw_d3 = true,
                _ => {}
            }
        }
        assert!(saw_d1 && saw_d2 && saw_d3, "expected automatic shifts D1 -> D2 -> D3");
        let speed_before_brake = rig.speed_mps;
        assert!(speed_before_brake > 18.0, "expected >18 m/s after 6s throttle, got {speed_before_brake:.2}");

        // Hard braking: measure braking distance and verify nose-dive pitch (< 0)
        let x_start_brake = rig.x;
        let mut min_pitch_deg = 0.0_f64;
        for _ in 0..240 {
            rig.step_with_terrain(
                VehicleInput { brake: 1.0, ..Default::default() },
                1.0 / 60.0,
                |_, _| 0.0,
                Some(SurfaceType::BitumenRoad),
                &[],
            );
            min_pitch_deg = min_pitch_deg.min(rig.pitch_rad.to_degrees());
            if rig.speed_mps <= 0.05 {
                break;
            }
        }
        let brake_dist = rig.x - x_start_brake;
        assert!(
            brake_dist > 8.0 && brake_dist < 60.0,
            "realistic bitumen braking distance expected in (8..60) m, got {brake_dist:.2} m"
        );
        assert!(
            min_pitch_deg < -0.5,
            "hard braking should induce nose-dive pitch < -0.5 deg, got {min_pitch_deg:.2} deg"
        );

        // Ackermann geometry: right turn => inner wheel (FR) turns sharper than outer wheel (FL)
        let (fl, fr) = ackermann_wheel_angles(0.42, PASSAT_WHEELBASE_M, PASSAT_TRACK_WIDTH_M);
        assert!(fr > fl && fl > 0.0, "right turn Ackermann requires FR ({fr:.4}) > FL ({fl:.4}) > 0");
        let (fl_l, fr_l) = ackermann_wheel_angles(-0.42, PASSAT_WHEELBASE_M, PASSAT_TRACK_WIDTH_M);
        assert!(fl_l < fr_l && fr_l < 0.0, "left turn Ackermann requires |FL| > |FR|");
    }

    #[test]
    fn surface_friction_differences_and_steep_slope_stall() {
        let run_surface = |surf: SurfaceType| -> f64 {
            let mut rig = VehicleRig::spawn_at(0.0, 0.0, -FRAC_PI_2, |_, _| 0.0);
            rig.enter();
            for _ in 0..180 {
                rig.step_with_terrain(
                    VehicleInput { throttle: 1.0, ..Default::default() },
                    1.0 / 60.0,
                    |_, _| 0.0,
                    Some(surf),
                    &[],
                );
            }
            rig.speed_mps
        };
        let v_bitumen = run_surface(SurfaceType::BitumenRoad);
        let v_gravel = run_surface(SurfaceType::GravelTrack);
        let v_silt = run_surface(SurfaceType::AlluvialSilt);
        assert!(
            v_bitumen > v_gravel && v_gravel > v_silt,
            "expected Bitumen ({v_bitumen:.2}) > Gravel ({v_gravel:.2}) > AlluvialSilt ({v_silt:.2})"
        );

        // Steep >28 deg slope (35 deg ramp) must lose traction and stall
        let steep_ramp = |x: f64, _z: f64| x * 35.0_f64.to_radians().tan();
        let mut climber = VehicleRig::spawn_at(0.0, 0.0, -FRAC_PI_2, steep_ramp);
        climber.enter();
        for _ in 0..180 {
            climber.step_with_terrain(
                VehicleInput { throttle: 1.0, ..Default::default() },
                1.0 / 60.0,
                steep_ramp,
                Some(SurfaceType::GravelTrack),
                &[],
            );
        }
        assert!(
            climber.slope_deg > STEEP_SLOPE_CLAMP_DEG,
            "slope_deg should exceed 28 deg, got {:.2}",
            climber.slope_deg
        );
        assert!(
            climber.speed_mps <= 0.1,
            "vehicle must stall/slip on 35 deg ramp, got speed {:.2} m/s",
            climber.speed_mps
        );
        assert!(
            climber.traction < 0.15,
            "traction must collapse on >28 deg slope, got {:.3}",
            climber.traction
        );
    }

    #[test]
    fn articulated_jayco_trailer_towing_jackknife_clamp_and_unhitch() {
        let mut rig = VehicleRig::spawn_at(0.0, 0.0, -FRAC_PI_2, |_, _| 0.0);
        rig.enter();
        assert!(!rig.trailer.hitch_attached, "spawns ready to hitch");
        assert!(rig.toggle_hitch(), "should hitch cleanly when aligned behind Passat");
        assert!(rig.trailer.hitch_attached);

        let initial_tx = rig.trailer.x;
        // Tow forward while turning right
        for _ in 0..240 {
            rig.step_with_terrain(
                VehicleInput { throttle: 0.45, steer: 0.65, ..Default::default() },
                1.0 / 60.0,
                |_, _| 0.0,
                Some(SurfaceType::BitumenRoad),
                &[],
            );
            assert!(
                rig.trailer.hitch_angle_rad.to_degrees().abs() <= JAYCO_MAX_JACKKNIFE_DEG + 1e-6,
                "hitch angle exceeded 72 deg jackknife clamp: {:.2}",
                rig.trailer.hitch_angle_rad.to_degrees()
            );
        }
        assert!(
            (rig.trailer.x - initial_tx).abs() > 4.0,
            "hitched Jayco trailer should follow Passat"
        );
        assert!(
            (rig.trailer.yaw - (-FRAC_PI_2)).abs() > 0.15,
            "trailer yaw should articulate along turn"
        );

        // Aggressive reverse full-lock steering to stress jackknife clamp
        for _ in 0..360 {
            rig.step_with_terrain(
                VehicleInput { brake: 0.7, steer: 1.0, ..Default::default() },
                1.0 / 60.0,
                |_, _| 0.0,
                Some(SurfaceType::BitumenRoad),
                &[],
            );
            assert!(
                rig.trailer.hitch_angle_rad.to_degrees().abs() <= JAYCO_MAX_JACKKNIFE_DEG + 1e-6,
                "reverse articulation exceeded 72 deg jackknife clamp: {:.2}",
                rig.trailer.hitch_angle_rad.to_degrees()
            );
        }

        // Unhitch and drive away — trailer must remain parked!
        assert!(!rig.toggle_hitch(), "toggle_hitch should detach camper");
        let parked_tx = rig.trailer.x;
        let parked_tz = rig.trailer.z;
        for _ in 0..180 {
            rig.step_with_terrain(
                VehicleInput { throttle: 0.6, ..Default::default() },
                1.0 / 60.0,
                |_, _| 0.0,
                Some(SurfaceType::BitumenRoad),
                &[],
            );
        }
        let trailer_drift = ((rig.trailer.x - parked_tx).powi(2) + (rig.trailer.z - parked_tz).powi(2)).sqrt();
        assert!(
            trailer_drift < 1.5,
            "unhitched trailer must stay parked, drifted {trailer_drift:.2} m"
        );
        assert!(
            rig.hitch_distance_m() > 5.0,
            "Passat should have driven away from unhitched camper"
        );
    }

    #[test]
    fn tree_and_campsite_obstacle_collision_clamping() {
        let obstacles = [CircleObstacle {
            x: 10.0,
            z: 0.0,
            radius: 2.0,
            label: "test-eucalyptus-trunk",
        }];
        let mut rig = VehicleRig::spawn_at(0.0, 0.0, -FRAC_PI_2, |_, _| 0.0);
        rig.enter();
        let mut collided = false;
        for _ in 0..240 {
            rig.step_with_terrain(
                VehicleInput { throttle: 0.8, ..Default::default() },
                1.0 / 60.0,
                |_, _| 0.0,
                Some(SurfaceType::GravelTrack),
                &obstacles,
            );
            let dist = ((rig.x - 10.0).powi(2) + rig.z.powi(2)).sqrt();
            assert!(
                dist >= PASSAT_COLLISION_RADIUS_M + 2.0 - 1e-4,
                "vehicle penetrated obstacle: dist={dist:.4}"
            );
            if rig.last_collision == Some("test-eucalyptus-trunk") {
                collided = true;
            }
        }
        assert!(collided, "expected collision response with tree trunk obstacle");
    }

    #[test]
    fn full_corridor_drive_forest_campsite_to_community_grounds_zero_nan() {
        let mut rig = VehicleRig::spawn_campsite();
        rig.enter();
        assert!(rig.toggle_hitch(), "hitch Jayco camper at campsite");
        assert_eq!(rig.corridor_zone, CorridorZone::ForestCampsite);

        let obstacles = default_corridor_obstacles();
        let mut saw_forest = false;
        let mut saw_road = false;
        let mut saw_community = false;

        // Simple corridor-following steering controller from (-22, 28) -> (345, 56)
        for _ in 0..2400 {
            let lookahead_x = rig.x + 14.0;
            let target_z = corridor_road_centreline_z(lookahead_x);
            let dx = lookahead_x - rig.x;
            let dz = target_z - rig.z;
            let desired_yaw = (-dx).atan2(-dz);
            let yaw_err = wrap_angle(desired_yaw - rig.yaw);
            // Positive steer turns right (negative yaw delta)
            let steer = (-yaw_err * 1.4).clamp(-1.0, 1.0);
            let throttle = if rig.speed_mps < 14.0 { 0.65 } else { 0.15 };

            rig.step_with_terrain(
                VehicleInput { throttle, steer, ..Default::default() },
                1.0 / 60.0,
                corridor_height_at,
                None,
                &obstacles,
            );

            // Assert zero NaN / Inf across all state channels
            assert!(rig.x.is_finite() && rig.y.is_finite() && rig.z.is_finite());
            assert!(rig.vx.is_finite() && rig.vy.is_finite() && rig.vz.is_finite());
            assert!(rig.pitch_rad.is_finite() && rig.roll_rad.is_finite() && rig.yaw.is_finite());
            assert!(rig.trailer.x.is_finite() && rig.trailer.y.is_finite() && rig.trailer.z.is_finite());
            assert!(rig.trailer.pitch_rad.is_finite() && rig.trailer.roll_rad.is_finite());

            match rig.corridor_zone {
                CorridorZone::ForestCampsite => saw_forest = true,
                CorridorZone::RoadCorridor => saw_road = true,
                CorridorZone::TownOutskirtsCommunityGrounds => saw_community = true,
            }
            if rig.x >= 340.0 {
                break;
            }
        }

        assert!(saw_forest, "trajectory must start in ForestCampsite");
        assert!(saw_road, "trajectory must traverse RoadCorridor");
        assert!(
            saw_community && rig.x >= 300.0,
            "trajectory must reach TownOutskirtsCommunityGrounds, ended at x={:.1}",
            rig.x
        );
    }

    #[test]
    fn drive_camera_modes_lights_and_tour_stop_teleport() {
        let mut rig = VehicleRig::spawn_campsite();
        assert_eq!(rig.camera_mode, DriveCameraMode::ChaseFollow);
        assert_eq!(rig.cycle_camera_mode(), DriveCameraMode::BonnetHood);
        assert_eq!(rig.cycle_camera_mode(), DriveCameraMode::CockpitDriver);
        assert_eq!(rig.cycle_camera_mode(), DriveCameraMode::ChaseFollow);

        assert!(!rig.headlights_on);
        assert!(rig.toggle_headlights());
        assert!(rig.headlights_on);

        rig.enter();
        assert!(rig.toggle_hitch());
        let chase = rig.compute_drive_camera();
        assert!(chase.zoom >= 10.0, "hitched chase camera should zoom out to frame trailer");

        // Teleport to Town-Outskirts Community Grounds tour stop
        rig.teleport_to(330.0, 54.0, -FRAC_PI_2, corridor_height_at);
        let t = rig.telemetry();
        assert!(t.active);
        assert!(t.hitch_attached);
        assert_eq!(t.corridor_zone, "TownOutskirtsCommunityGrounds");
        assert!(t.headlights);
        assert!(t.x.is_finite() && t.y.is_finite() && t.z.is_finite());
    }

    #[test]
    fn enter_from_rest_shifts_to_d1_and_accelerates_above_10_kmh() {
        let mut rig = VehicleRig::spawn_campsite();
        assert_eq!(rig.gear, Gear::Park);
        rig.enter();
        assert!(rig.active);
        assert_eq!(rig.gear, Gear::Neutral);

        // Hold full throttle (KeyW) for 1.0s (60 frames)
        for _ in 0..60 {
            rig.step_corridor(
                VehicleInput {
                    throttle: 1.0,
                    steer: 0.25,
                    ..Default::default()
                },
                1.0 / 60.0,
            );
        }
        let t = rig.telemetry();
        assert_eq!(t.gear, "D1", "expected shift from Neutral to D1, got {}", t.gear);
        assert!(
            t.speed_kmh > 10.0,
            "expected speed_kmh > 10.0 after 1.0s throttle, got {:.2}",
            t.speed_kmh
        );
        assert!(
            t.x > CAMPSITE_SPAWN_X + 1.0,
            "expected Passat to move east from campsite, x={:.2}",
            t.x
        );
        assert!(
            t.steer_deg.abs() > 2.0,
            "expected steering angle to respond, got {:.2} deg",
            t.steer_deg
        );

        // From rest, holding brake engages Reverse
        let mut rev = VehicleRig::spawn_campsite();
        rev.enter();
        for _ in 0..30 {
            rev.step_corridor(VehicleInput { brake: 1.0, ..Default::default() }, 1.0 / 60.0);
        }
        assert_eq!(rev.gear, Gear::Reverse, "brake from rest must engage Reverse");
        assert!(rev.speed_mps < -0.5, "expected negative reverse velocity, got {:.2}", rev.speed_mps);
    }
}
