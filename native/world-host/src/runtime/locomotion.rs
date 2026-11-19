//! Playable locomotion, deterministic 600 m catchment terrain physics,
//! catchment-to-orbit flight, and mobile virtual joystick normalization.
//! Generic player only: no per-scene or per-species hardcoding.

use super::macros::{self, DEFAULT_HITCH_RADIUS_M};
use super::ocs_frames::{select_regime_with_hysteresis, ScaleRegime};
use qualia_core_db::render::camera::{orbit_view_projection_with_near, CAMERA_NEAR_PLANE};
use serde_json::json;
use std::cell::RefCell;
use wasm_bindgen::JsValue;

/// Clip fade into SKL1 when the section is present. Missing clips are ignored.
const CLIP_FADE_S: f64 = 0.15;

pub const EYE_HEIGHT_M: f64 = 1.65;
pub const WALK_BASE_MPS: f64 = 4.6;
pub const SPRINT_MULT: f64 = 1.8;
pub const CREEK_WATER_LEVEL_M: f64 = 0.24;
pub const CREEK_WADE_FACTOR: f64 = 0.58;
pub const SLOPE_CLAMP_DEG: f64 = 32.0;
pub const SLOPE_STEEP_DEG: f64 = 58.0;
pub const SLOPE_MIN_FACTOR: f64 = 0.22;
pub const JUMP_IMPULSE_MPS: f64 = 4.8;
pub const GRAVITY_MPS2: f64 = 12.0;
pub const CATCHMENT_HALF_SPAN_M: f64 = 298.0;
pub const JOYSTICK_MAX_RADIUS_PX: f64 = 44.0;
pub const JOYSTICK_DEADZONE: f64 = 0.12;
const PITCH_LIMIT_WALK: f32 = 1.35;
const LOOK_SENSITIVITY: f32 = 0.0052;
const KEY_LOOK_RAD_PER_S: f32 = 1.65;
const PORTRAIT_OBSERVE_YAW_OFFSET_RAD: f64 = 0.52;
const PORTRAIT_OBSERVE_TARGET_X_OFFSET_M: f64 = 0.9;

#[allow(unused_imports)]
pub use macros::DEFAULT_HITCH_RADIUS_M as HITCH_RADIUS_M;
#[allow(unused_imports)]
pub use macros::DEFAULT_STEER_MPS as STEER_MPS;

/// Seamless exploration stances (including vehicle camera handoff).
#[derive(Clone, Copy, Debug, PartialEq, Eq, Default)]
pub enum MovementMode {
    /// Guided chapters, accessible waypoints, orbit/pan/zoom (calm default on boot).
    #[default]
    ObserveOrbit,
    /// Terrain-conforming 1st/3rd-person ground locomotion on the 600 m catchment.
    WalkExplore,
    /// Continuous catchment-to-orbit flight across OCS LocalEnu / Planetary / Heliocentric regimes.
    FlyGlide,
    /// Seated vehicle driving; locomotion hands off camera and movement input to the vehicle controller.
    DriveVehicle,
}

impl MovementMode {
    pub const fn as_str(self) -> &'static str {
        match self {
            MovementMode::ObserveOrbit => "observe",
            MovementMode::WalkExplore => "walk",
            MovementMode::FlyGlide => "fly",
            MovementMode::DriveVehicle => "drive",
        }
    }

    #[allow(dead_code)]
    pub const fn label(self) -> &'static str {
        match self {
            MovementMode::ObserveOrbit => "Observe",
            MovementMode::WalkExplore => "Walk",
            MovementMode::FlyGlide => "Fly",
            MovementMode::DriveVehicle => "Drive",
        }
    }

    #[allow(dead_code)]
    pub fn from_str(s: &str) -> Option<Self> {
        match s.trim().to_ascii_lowercase().as_str() {
            "1" | "observe" | "observe-orbit" | "observeorbit" => Some(MovementMode::ObserveOrbit),
            "2" | "walk" | "walk-explore" | "walkexplore" => Some(MovementMode::WalkExplore),
            "3" | "fly" | "fly-glide" | "flyglide" => Some(MovementMode::FlyGlide),
            "4" | "drive" | "drive-vehicle" | "drivevehicle" => Some(MovementMode::DriveVehicle),
            _ => None,
        }
    }

    #[allow(dead_code)]
    pub const fn next(self) -> Self {
        match self {
            MovementMode::ObserveOrbit => MovementMode::WalkExplore,
            MovementMode::WalkExplore => MovementMode::FlyGlide,
            MovementMode::FlyGlide => MovementMode::ObserveOrbit,
            MovementMode::DriveVehicle => MovementMode::WalkExplore,
        }
    }
}

#[inline]
fn smoothstep(edge0: f64, edge1: f64, x: f64) -> f64 {
    let t = ((x - edge0) / (edge1 - edge0)).clamp(0.0, 1.0);
    t * t * (3.0 - 2.0 * t)
}

/// Deterministic river centreline `x(z)` matching `site/world/living-world.vibe` and `terrain.js`.
pub fn catchment_river_x(z: f64) -> f64 {
    let oz = smoothstep(65.0, 125.0, z.abs());
    (z * 0.044).sin() * 10.0
        + (z * 0.095).sin() * 3.0
        + oz * ((z * 0.0175).sin() * 26.0 + (z * 0.0085 + 0.7).sin() * 14.0)
}

#[allow(dead_code)]
fn core_noise(x: f64, z: f64) -> f64 {
    (x * 0.11 + z * 0.07).sin() * 0.5
        + (x * 0.28 - z * 0.16).sin() * 0.22
        + (z * 0.39 + x * 0.2).cos() * 0.12
        + (x * 0.73 + z * 0.61).sin() * 0.05
}

#[allow(dead_code)]
fn core_height(x: f64, z: f64) -> f64 {
    let d = (x - catchment_river_x(z)).abs();
    let bank = 1.0 - (-(d / 9.0).powi(2)).exp();
    let bank_ch = 1.0 - (-(d / 6.5).powi(2)).exp();
    let valley = 0.35 + bank * (2.4 + (x.abs() - 15.0).max(0.0) * 0.22);
    let mountain = ((-z - 25.0) / 65.0).max(0.0).powf(1.4)
        * (7.0 + 12.0 * (x * 0.055 + 1.2).sin().powi(2));
    let channel_gap = 1.0 - smoothstep(-34.0, -48.0, z) * (1.0 - bank_ch);
    let m_masked = mountain * channel_gap;
    valley + m_masked + core_noise(x, z) * bank * (1.4 + m_masked * 0.17)
        - (-d * d / 16.0).exp() * 1.4
}

#[allow(dead_code)]
fn outer_height(x: f64, z: f64) -> f64 {
    let rx = catchment_river_x(z);
    let dx = x - rx;
    let d = dx.abs();
    let bend = -(z * 0.044).sin() * 0.45 - (z * 0.0175).sin() * 0.55;
    let bend_mag = bend.abs().clamp(0.0, 1.0);
    let w_ch = 4.8 + 2.0 * (z * 0.023 + 0.4).sin().powi(2);
    let cut_side = (dx * (-bend)) / (dx.abs() + 3.0);
    let bank_scale = w_ch + 3.2 - 1.6 * cut_side;
    let channel_t = smoothstep(2.0, bank_scale + 4.8, d);
    let bed_y = -0.50 - 0.95 * bend_mag + 0.12 * (z * 0.19).sin();
    let point_bar =
        (-cut_side).max(0.0) * (-((d - (w_ch + 1.8)) / 3.4).powi(2)).exp() * 0.95;

    let t1 = 1.95 + 0.55 * smoothstep(w_ch + 2.0, w_ch + 9.0, d);
    let t2 = t1 + 2.15 * smoothstep(24.0, 35.0, d);
    let flood_micro = 0.20 * (x * 0.07 + z * 0.05).sin() + 0.10 * (x * 0.15 - z * 0.11).cos();
    let floodplain_y = t2 + flood_micro;

    let slope_d = ((d - 46.0) / 220.0).clamp(0.0, 1.25);
    let woodland_base = 21.0 * slope_d.powf(1.15);
    let woodland_undulation = (2.5 * (x * 0.021 + 0.4).sin() * (z * 0.019 - 0.3).cos()
        + 1.3 * (x * 0.047 - z * 0.039).sin()
        + 0.55 * core_noise(x * 0.55, z * 0.55))
        * smoothstep(32.0, 75.0, d);

    let n_rise = smoothstep(-95.0, -280.0, z);
    let n_ridge_profile = 40.0
        * n_rise.powf(1.22)
        * (0.78 + 0.22 * (x * 0.018 + 1.1).sin() * (x * 0.009).cos());
    let n_terrace = n_ridge_profile
        + 1.9 * (n_ridge_profile * 0.55).sin() * n_rise * smoothstep(45.0, 90.0, d);

    let e_headland_x = smoothstep(115.0, 275.0, x);
    let e_headland_z = smoothstep(-140.0, 40.0, z) * (1.0 - 0.35 * smoothstep(200.0, 300.0, z));
    let e_headland = 30.0 * e_headland_x * e_headland_z * (0.82 + 0.18 * (z * 0.025 - 0.5).cos());
    let e_bench = e_headland + 1.5 * (e_headland * 0.6).sin() * e_headland_x;

    let g_z_line = -55.0 + (x + 75.0) * 0.72;
    let g_dist = (z - g_z_line).abs();
    let g_along = smoothstep(-68.0, -120.0, x) * (1.0 - smoothstep(-245.0, -295.0, x));
    let g_carve = 10.5 * g_along * (-(g_dist / 26.0).powi(2)).exp();
    let g_flank = 5.0 * g_along * (-((g_dist - 42.0) / 18.0).powi(2)).exp();

    let slope_y = woodland_base + woodland_undulation + n_terrace + e_bench - g_carve + g_flank;
    let land_y = floodplain_y + slope_y.max(0.0) * smoothstep(38.0, 72.0, d);
    bed_y + point_bar + (land_y - bed_y - point_bar) * channel_t
}

#[allow(dead_code)]
fn natural_height(x: f64, z: f64) -> f64 {
    let r = (x * x + z * z).sqrt();
    let s_outer = smoothstep(68.0, 112.0, r);
    let c = core_height(x, z);
    if s_outer <= 0.0 {
        return c;
    }
    let o = outer_height(x, z);
    c + (o - c) * s_outer
}

/// Soil height under `(x, z)`, sampled from the catchment mesh.
pub fn catchment_height_at(x: f64, z: f64) -> f64 {
    super::ground::height_at(x, z)
}

/// Ground datum under `(x, z)`. Same mesh sample as [`catchment_height_at`].
pub fn terrain_datum_at(x: f64, z: f64) -> f64 {
    super::ground::height_at(x, z)
}

/// Directional uphill slope angle in degrees between two planar samples.
pub fn catchment_slope_deg(x0: f64, z0: f64, x1: f64, z1: f64) -> f64 {
    let ds = ((x1 - x0) * (x1 - x0) + (z1 - z0) * (z1 - z0)).sqrt();
    if ds < 1e-6 || !ds.is_finite() {
        return 0.0;
    }
    let dh = (catchment_height_at(x1, z1) - catchment_height_at(x0, z0)).max(0.0);
    (dh / ds).atan().to_degrees()
}

/// Walk speed multiplier for slopes `> 32 deg`, slowing gently without hard-stopping.
pub fn slope_speed_factor(slope_deg: f64) -> f64 {
    if !slope_deg.is_finite() || slope_deg <= SLOPE_CLAMP_DEG {
        return 1.0;
    }
    let t = smoothstep(SLOPE_CLAMP_DEG, SLOPE_STEEP_DEG, slope_deg);
    1.0 - t * (1.0 - SLOPE_MIN_FACTOR)
}

/// Gentle creek-wading resistance when terrain elevation `y <= 0.24 m`.
pub fn creek_wade_factor(ground_y: f64) -> f64 {
    if ground_y.is_finite() && ground_y <= CREEK_WATER_LEVEL_M {
        CREEK_WADE_FACTOR
    } else {
        1.0
    }
}

/// Altitude-scaled flight speed accelerating smoothly from `10 m` to `10,000 m+` into orbit.
pub fn fly_speed_mps(altitude_m: f64, sprint: bool) -> f64 {
    let alt = if altitude_m.is_finite() {
        altitude_m.max(0.0)
    } else {
        0.0
    };
    let base = 12.0 + 0.45 * alt + 0.08 * alt * (1.0 + alt / 500.0).ln();
    if sprint {
        base * SPRINT_MULT
    } else {
        base
    }
}

/// Normalize virtual thumbstick offset `(dx, dy)` in CSS pixels into `[strafe_right, forward]`
/// within the unit circle, applying a smooth deadzone so small jitter stays zero.
/// Screen-up (`dy < 0`) maps to positive `forward`.
pub fn normalize_joystick(dx: f64, dy: f64, max_radius: f64, deadzone: f64) -> [f64; 2] {
    if !dx.is_finite()
        || !dy.is_finite()
        || !max_radius.is_finite()
        || max_radius <= 1e-6
        || !deadzone.is_finite()
    {
        return [0.0, 0.0];
    }
    let dz = deadzone.clamp(0.0, 0.95);
    let len = (dx * dx + dy * dy).sqrt();
    if len < 1e-9 {
        return [0.0, 0.0];
    }
    let mag = (len / max_radius).clamp(0.0, 1.0);
    if mag <= dz {
        return [0.0, 0.0];
    }
    let scaled = (mag - dz) / (1.0 - dz);
    let ux = dx / len;
    let uy = -dy / len;
    [ux * scaled, uy * scaled]
}

/// Aspect-ratio-aware initial `ObserveOrbit` framing offset `(yaw_rad, target_x_m)` for
/// portrait viewports (`viewport_width < viewport_height`). Leaves landscape / desktop
/// (`viewport_width >= viewport_height`) framing untouched `(0.0, 0.0)`.
pub fn portrait_observe_framing_offset(viewport_width: f64, viewport_height: f64) -> (f32, f32) {
    if !viewport_width.is_finite()
        || !viewport_height.is_finite()
        || viewport_width <= 0.0
        || viewport_height <= 0.0
        || viewport_width >= viewport_height
    {
        return (0.0, 0.0);
    }
    let aspect = (viewport_width / viewport_height).clamp(0.25, 1.0);
    let t = ((1.0 - aspect) / 0.45).clamp(0.0, 1.0);
    let smooth = t * t * (3.0 - 2.0 * t);
    (
        (PORTRAIT_OBSERVE_YAW_OFFSET_RAD * smooth) as f32,
        (PORTRAIT_OBSERVE_TARGET_X_OFFSET_M * smooth) as f32,
    )
}

/// Apply the portrait observe offset to `(yaw, target)` when `viewport_width < viewport_height`.
pub fn apply_portrait_observe_offset(
    yaw: f32,
    target: [f32; 3],
    viewport_width: f64,
    viewport_height: f64,
) -> (f32, [f32; 3]) {
    let (dyaw, dtx) = portrait_observe_framing_offset(viewport_width, viewport_height);
    (yaw + dyaw, [target[0] + dtx, target[1], target[2]])
}

/// Pure-Rust kinematic state for WalkExplore and FlyGlide modes.
#[derive(Clone, Debug)]
pub struct LocomotionCore {
    pub mode: MovementMode,
    pub pos: [f64; 3],
    pub vel_y: f64,
    pub grounded: bool,
    pub eye_y: f64,
    pub yaw: f32,
    pub pitch: f32,
    pub walk_zoom: f32,
    pub fly_zoom: f32,
    pub keys_forward: bool,
    pub keys_back: bool,
    pub keys_left: bool,
    pub keys_right: bool,
    pub keys_up: bool,
    pub keys_down: bool,
    pub keys_sprint: bool,
    pub look_left: bool,
    pub look_right: bool,
    pub look_up: bool,
    pub look_down: bool,
    pub joystick: [f64; 2],
    pub walk_goal: Option<[f64; 2]>,
    pub last_slope_deg: f64,
    pub regime: ScaleRegime,
    pub rhizosphere_xray: bool,
    pub anatomy_xray: bool,
    pub third_person: bool,
    pub observer_saved: Option<[f32; 6]>,
    pub last_camera_target: [f32; 3],
    pub portrait_framing_applied: bool,
}

impl Default for LocomotionCore {
    fn default() -> Self {
        Self::new()
    }
}

impl LocomotionCore {
    pub fn new() -> Self {
        let ground = catchment_height_at(-22.0, 28.0);
        Self {
            mode: MovementMode::ObserveOrbit,
            pos: [-22.0, ground, 28.0],
            vel_y: 0.0,
            grounded: true,
            eye_y: ground + EYE_HEIGHT_M,
            yaw: 0.63275,
            pitch: 0.17555,
            walk_zoom: 0.15,
            fly_zoom: 0.25,
            keys_forward: false,
            keys_back: false,
            keys_left: false,
            keys_right: false,
            keys_up: false,
            keys_down: false,
            keys_sprint: false,
            look_left: false,
            look_right: false,
            look_up: false,
            look_down: false,
            joystick: [0.0, 0.0],
            walk_goal: None,
            last_slope_deg: 0.0,
            regime: ScaleRegime::LocalEnu,
            rhizosphere_xray: false,
            anatomy_xray: false,
            third_person: false,
            observer_saved: None,
            last_camera_target: [-22.0, (ground + EYE_HEIGHT_M) as f32, 28.0],
            portrait_framing_applied: false,
        }
    }

    /// Current camera perspective tag (`"first-person"` or `"third-person-chase"`).
    pub const fn perspective_str(&self) -> &'static str {
        if self.third_person {
            "third-person-chase"
        } else {
            "first-person"
        }
    }

    /// Toggle `FirstPerson` <-> `ThirdPersonChase` perspective, updating walk chase zoom.
    pub fn toggle_perspective(&mut self) -> bool {
        self.set_third_person(!self.third_person)
    }

    /// Explicitly set `ThirdPersonChase` (`true`) or `FirstPerson` (`false`).
    pub fn set_third_person(&mut self, third_person: bool) -> bool {
        self.third_person = third_person;
        self.walk_zoom = if third_person { 3.4 } else { 0.15 };
        self.third_person
    }

    /// Toggle human anatomy X-Ray layer (`Integument` <-> `AnatomyXRay`).
    pub fn toggle_anatomy_xray(&mut self) -> bool {
        self.anatomy_xray = !self.anatomy_xray;
        self.anatomy_xray
    }

    /// Explicitly set human anatomy X-Ray active state.
    pub fn set_anatomy_xray(&mut self, active: bool) -> bool {
        self.anatomy_xray = active;
        self.anatomy_xray
    }

    pub fn clear_inputs(&mut self) {
        self.keys_forward = false;
        self.keys_back = false;
        self.keys_left = false;
        self.keys_right = false;
        self.keys_up = false;
        self.keys_down = false;
        self.keys_sprint = false;
        self.look_left = false;
        self.look_right = false;
        self.look_up = false;
        self.look_down = false;
        self.joystick = [0.0, 0.0];
        self.walk_goal = None;
    }

    pub fn set_mode(
        &mut self,
        next: MovementMode,
        cam_yaw: f32,
        cam_pitch: f32,
        cam_zoom: f32,
        cam_target: [f32; 3],
    ) -> bool {
        if self.mode == next {
            return false;
        }
        let prev = self.mode;
        if prev == MovementMode::ObserveOrbit {
            self.observer_saved = Some([
                cam_yaw,
                cam_pitch,
                cam_zoom,
                cam_target[0],
                cam_target[1],
                cam_target[2],
            ]);
            self.yaw = cam_yaw;
            self.pitch = cam_pitch.clamp(-PITCH_LIMIT_WALK, PITCH_LIMIT_WALK);
            let x = (cam_target[0] as f64).clamp(-CATCHMENT_HALF_SPAN_M, CATCHMENT_HALF_SPAN_M);
            let z = (cam_target[2] as f64).clamp(-CATCHMENT_HALF_SPAN_M, CATCHMENT_HALF_SPAN_M);
            let ground = catchment_height_at(x, z);
            if next == MovementMode::FlyGlide {
                let start_y = (cam_target[1] as f64).max(ground + 4.0);
                self.pos = [x, start_y, z];
                self.eye_y = start_y;
            } else {
                self.pos = [x, ground, z];
                self.eye_y = ground + EYE_HEIGHT_M;
            }
        } else if next == MovementMode::WalkExplore {
            let x = self.pos[0].clamp(-CATCHMENT_HALF_SPAN_M, CATCHMENT_HALF_SPAN_M);
            let z = self.pos[2].clamp(-CATCHMENT_HALF_SPAN_M, CATCHMENT_HALF_SPAN_M);
            let ground = catchment_height_at(x, z);
            self.pos = [x, ground, z];
            self.eye_y = ground + EYE_HEIGHT_M;
        } else if next == MovementMode::FlyGlide {
            self.pos[1] = self.eye_y.max(terrain_datum_at(self.pos[0], self.pos[2]) + EYE_HEIGHT_M);
            self.eye_y = self.pos[1];
        }
        self.vel_y = 0.0;
        self.grounded = true;
        self.clear_inputs();
        self.mode = next;
        self.regime = select_regime_with_hysteresis(self.regime, self.altitude_m());
        true
    }

    pub fn trigger_hop(&mut self) {
        if self.mode == MovementMode::WalkExplore && self.grounded {
            self.vel_y = JUMP_IMPULSE_MPS;
            self.grounded = false;
        }
    }

    pub fn apply_look_delta(&mut self, dx: f32, dy: f32) {
        if !dx.is_finite() || !dy.is_finite() {
            return;
        }
        self.yaw += dx * LOOK_SENSITIVITY;
        self.pitch = (self.pitch + dy * LOOK_SENSITIVITY).clamp(-PITCH_LIMIT_WALK, PITCH_LIMIT_WALK);
    }

    pub fn apply_zoom_delta(&mut self, dy: f32) {
        if !dy.is_finite() {
            return;
        }
        let scale = (dy * 0.001).exp();
        match self.mode {
            MovementMode::WalkExplore => {
                self.walk_zoom = (self.walk_zoom * scale).clamp(0.10, 14.0);
            }
            MovementMode::FlyGlide => {
                self.fly_zoom = (self.fly_zoom * scale).clamp(0.15, 24.0);
            }
            MovementMode::ObserveOrbit | MovementMode::DriveVehicle => {}
        }
    }

    pub fn active_zoom(&self) -> f32 {
        match self.mode {
            MovementMode::WalkExplore => self.walk_zoom,
            MovementMode::FlyGlide => self.fly_zoom,
            MovementMode::ObserveOrbit => 16.0,
            MovementMode::DriveVehicle => 6.5,
        }
    }

    pub fn eye_target(&self) -> [f32; 3] {
        [self.pos[0] as f32, self.eye_y as f32, self.pos[2] as f32]
    }

    pub fn ground_y(&self) -> f64 {
        terrain_datum_at(self.pos[0], self.pos[2])
    }

    pub fn altitude_m(&self) -> f64 {
        (self.eye_y - self.ground_y()).max(0.0)
    }

    pub fn is_wading(&self) -> bool {
        self.mode == MovementMode::WalkExplore && self.pos[1] <= CREEK_WATER_LEVEL_M
    }

    pub fn keydown(&mut self, key: &str, shift: bool, ctrl: bool) -> bool {
        if shift {
            self.keys_sprint = true;
        }
        if ctrl {
            self.keys_down = true;
        }
        if matches!(key, "v" | "V") && self.mode != MovementMode::ObserveOrbit {
            self.toggle_perspective();
            return true;
        }
        if matches!(self.mode, MovementMode::ObserveOrbit | MovementMode::DriveVehicle) {
            return false;
        }
        match key {
            "w" | "W" | "ArrowUp" => {
                self.keys_forward = true;
                self.walk_goal = None;
                true
            }
            "s" | "S" | "ArrowDown" => {
                self.keys_back = true;
                self.walk_goal = None;
                true
            }
            "a" | "A" | "ArrowLeft" => {
                self.keys_left = true;
                self.walk_goal = None;
                true
            }
            "d" | "D" | "ArrowRight" => {
                self.keys_right = true;
                self.walk_goal = None;
                true
            }
            " " | "Spacebar" => {
                self.keys_up = true;
                if self.mode == MovementMode::WalkExplore {
                    self.trigger_hop();
                }
                true
            }
            "c" | "C" | "Control" => {
                if self.mode == MovementMode::FlyGlide {
                    self.keys_down = true;
                    true
                } else {
                    false
                }
            }
            "Shift" => {
                self.keys_sprint = true;
                true
            }
            "j" | "J" => {
                self.look_left = true;
                true
            }
            "l" | "L" => {
                self.look_right = true;
                true
            }
            "i" | "I" => {
                self.look_up = true;
                true
            }
            "k" | "K" => {
                self.look_down = true;
                true
            }
            _ => false,
        }
    }

    pub fn keyup(&mut self, key: &str) -> bool {
        let mut used = false;
        match key {
            "w" | "W" | "ArrowUp" => {
                self.keys_forward = false;
                used = true;
            }
            "s" | "S" | "ArrowDown" => {
                self.keys_back = false;
                used = true;
            }
            "a" | "A" | "ArrowLeft" => {
                self.keys_left = false;
                used = true;
            }
            "d" | "D" | "ArrowRight" => {
                self.keys_right = false;
                used = true;
            }
            " " | "Spacebar" => {
                self.keys_up = false;
                used = true;
            }
            "c" | "C" | "Control" => {
                self.keys_down = false;
                used = self.mode == MovementMode::FlyGlide;
            }
            "Shift" => {
                self.keys_sprint = false;
                used = true;
            }
            "j" | "J" => {
                self.look_left = false;
                used = true;
            }
            "l" | "L" => {
                self.look_right = false;
                used = true;
            }
            "i" | "I" => {
                self.look_up = false;
                used = true;
            }
            "k" | "K" => {
                self.look_down = false;
                used = true;
            }
            _ => {}
        }
        used && !matches!(self.mode, MovementMode::ObserveOrbit | MovementMode::DriveVehicle)
    }

    pub fn step(&mut self, dt_s: f64, reduced: bool) {
        let dt = if dt_s.is_finite() {
            dt_s.clamp(0.0, 0.05)
        } else {
            return;
        };
        if dt <= 0.0
            || matches!(self.mode, MovementMode::ObserveOrbit | MovementMode::DriveVehicle)
        {
            return;
        }
        let yaw_dt = if self.look_right { 1.0 } else { 0.0 } - if self.look_left { 1.0 } else { 0.0 };
        let pitch_dt = if self.look_down { 1.0 } else { 0.0 } - if self.look_up { 1.0 } else { 0.0 };
        if yaw_dt != 0.0 || pitch_dt != 0.0 {
            self.yaw += (yaw_dt as f32) * KEY_LOOK_RAD_PER_S * (dt as f32);
            self.pitch = (self.pitch + (pitch_dt as f32) * KEY_LOOK_RAD_PER_S * (dt as f32))
                .clamp(-PITCH_LIMIT_WALK, PITCH_LIMIT_WALK);
        }

        match self.mode {
            MovementMode::WalkExplore => self.step_walk(dt, reduced),
            MovementMode::FlyGlide => self.step_fly(dt),
            MovementMode::ObserveOrbit | MovementMode::DriveVehicle => {}
        }
        self.regime = select_regime_with_hysteresis(self.regime, self.altitude_m());
    }

    fn planar_input(&mut self) -> (f64, f64) {
        let key_fwd =
            if self.keys_forward { 1.0 } else { 0.0 } - if self.keys_back { 1.0 } else { 0.0 };
        let key_strafe =
            if self.keys_right { 1.0 } else { 0.0 } - if self.keys_left { 1.0 } else { 0.0 };
        let mut strafe = key_strafe + self.joystick[0];
        let mut fwd = key_fwd + self.joystick[1];
        let manual = strafe.abs() > 1e-4 || fwd.abs() > 1e-4;
        if manual {
            self.walk_goal = None;
        }
        let mag = (strafe * strafe + fwd * fwd).sqrt();
        if mag > 1.0 {
            strafe /= mag;
            fwd /= mag;
        }
        (strafe, fwd)
    }

    fn step_walk(&mut self, dt: f64, reduced: bool) {
        let (strafe, fwd) = self.planar_input();
        let (dir_x, dir_z) = if strafe.abs() > 1e-4 || fwd.abs() > 1e-4 {
            let sin_y = (self.yaw as f64).sin();
            let cos_y = (self.yaw as f64).cos();
            // Camera forward XZ = [-sin(yaw), -cos(yaw)], right XZ = [cos(yaw), -sin(yaw)].
            let dx = cos_y * strafe - sin_y * fwd;
            let dz = -sin_y * strafe - cos_y * fwd;
            (dx, dz)
        } else if let Some([gx, gz]) = self.walk_goal {
            let rx = gx - self.pos[0];
            let rz = gz - self.pos[2];
            let dist = (rx * rx + rz * rz).sqrt();
            if dist <= 0.65 {
                self.walk_goal = None;
                (0.0, 0.0)
            } else {
                self.yaw = (-rx).atan2(-rz) as f32;
                (rx / dist, rz / dist)
            }
        } else {
            (0.0, 0.0)
        };

        let speed = WALK_BASE_MPS * if self.keys_sprint { SPRINT_MULT } else { 1.0 };
        let h0 = catchment_height_at(self.pos[0], self.pos[2]);
        let probe_x = (self.pos[0] + dir_x * speed * dt)
            .clamp(-CATCHMENT_HALF_SPAN_M, CATCHMENT_HALF_SPAN_M);
        let probe_z = (self.pos[2] + dir_z * speed * dt)
            .clamp(-CATCHMENT_HALF_SPAN_M, CATCHMENT_HALF_SPAN_M);
        let slope_deg = catchment_slope_deg(self.pos[0], self.pos[2], probe_x, probe_z);
        self.last_slope_deg = slope_deg;
        let factor = slope_speed_factor(slope_deg) * creek_wade_factor(h0);

        self.pos[0] = (self.pos[0] + dir_x * speed * factor * dt)
            .clamp(-CATCHMENT_HALF_SPAN_M, CATCHMENT_HALF_SPAN_M);
        self.pos[2] = (self.pos[2] + dir_z * speed * factor * dt)
            .clamp(-CATCHMENT_HALF_SPAN_M, CATCHMENT_HALF_SPAN_M);

        let ground_y = catchment_height_at(self.pos[0], self.pos[2]);
        if !self.grounded {
            self.vel_y -= GRAVITY_MPS2 * dt;
            self.pos[1] += self.vel_y * dt;
            if self.pos[1] <= ground_y {
                self.pos[1] = ground_y;
                self.vel_y = 0.0;
                self.grounded = true;
            }
        } else {
            self.pos[1] = ground_y;
        }

        let target_eye = self.pos[1] + EYE_HEIGHT_M;
        if reduced || (target_eye - self.eye_y).abs() > 25.0 {
            self.eye_y = target_eye;
        } else {
            let damp = 1.0 - (-dt * 14.0).exp();
            self.eye_y += (target_eye - self.eye_y) * damp;
        }
    }

    fn step_fly(&mut self, dt: f64) {
        let (strafe, fwd) = self.planar_input();
        let sin_y = (self.yaw as f64).sin();
        let cos_y = (self.yaw as f64).cos();
        let dir_x = cos_y * strafe - sin_y * fwd;
        let dir_z = -sin_y * strafe - cos_y * fwd;
        let key_vert =
            if self.keys_up { 1.0 } else { 0.0 } - if self.keys_down { 1.0 } else { 0.0 };
        let pitch_glide = -(self.pitch as f64).sin() * fwd * 0.65;
        let mut dir_y = key_vert + pitch_glide;

        let len3 = (dir_x * dir_x + dir_y * dir_y + dir_z * dir_z).sqrt();
        let (vx, vy, vz) = if len3 > 1.0 {
            (dir_x / len3, dir_y / len3, dir_z / len3)
        } else {
            (dir_x, dir_y, dir_z)
        };
        let _ = &mut dir_y;

        let ground = terrain_datum_at(self.pos[0], self.pos[2]);
        let alt = (self.pos[1] - ground).max(0.0);
        let speed = fly_speed_mps(alt, self.keys_sprint);

        self.pos[0] += vx * speed * dt;
        self.pos[1] += vy * speed * dt;
        self.pos[2] += vz * speed * dt;

        let next_ground = terrain_datum_at(self.pos[0], self.pos[2]);
        if self.pos[1] < next_ground + EYE_HEIGHT_M {
            self.pos[1] = next_ground + EYE_HEIGHT_M;
        }
        self.eye_y = self.pos[1];
        self.last_slope_deg = 0.0;
    }
}

thread_local! {
    static ENGINE: RefCell<LocomotionCore> = RefCell::new(LocomotionCore::new());
}

pub fn current_mode() -> MovementMode {
    ENGINE.with(|e| e.borrow().mode)
}

pub fn rhizosphere_xray_active() -> bool {
    ENGINE.with(|e| e.borrow().rhizosphere_xray)
}

pub fn anatomy_xray_active() -> bool {
    ENGINE.with(|e| e.borrow().anatomy_xray)
}

pub fn third_person_active() -> bool {
    ENGINE.with(|e| e.borrow().third_person)
}

pub fn perspective_str() -> &'static str {
    ENGINE.with(|e| e.borrow().perspective_str())
}

pub fn set_mode(w: &mut super::World, next: MovementMode) -> Result<bool, JsValue> {
    let changed = ENGINE.with(|e| {
        let mut core = e.borrow_mut();
        let changed = core.set_mode(
            next,
            w.camera.yaw,
            w.camera.pitch,
            w.camera.zoom,
            w.camera.target,
        );
        if changed {
            if next == MovementMode::ObserveOrbit {
                if let Some([yaw, pitch, zoom, tx, ty, tz]) = core.observer_saved.take() {
                    w.goal.yaw = yaw;
                    w.goal.pitch = pitch;
                    w.goal.zoom = zoom;
                    w.goal.target = [tx, ty, tz];
                    if w.reduced {
                        w.camera = w.goal.clone();
                    }
                }
            } else if next != MovementMode::DriveVehicle {
                w.camera.yaw = core.yaw;
                w.camera.pitch = core.pitch;
                w.camera.zoom = core.active_zoom();
                w.camera.target = core.eye_target();
                w.goal = w.camera.clone();
                core.last_camera_target = w.camera.target;
            }
        }
        changed
    });
    if changed {
        if matches!(next, MovementMode::WalkExplore | MovementMode::FlyGlide) {
            super::set_camera(w)?;
        } else if next == MovementMode::ObserveOrbit && w.reduced {
            super::set_camera(w)?;
        }
        sync_locomotion_receipt(w);
        super::ui::sync_mode_ui(next, rhizosphere_xray_active());
    }
    Ok(changed)
}

pub fn toggle_perspective(w: &mut super::World) -> bool {
    let third = !third_person_active();
    set_perspective(w, third)
}

pub fn set_perspective(w: &mut super::World, third_person: bool) -> bool {
    let (active, zoom, mode) = ENGINE.with(|e| {
        let mut core = e.borrow_mut();
        let active = core.set_third_person(third_person);
        (active, core.active_zoom(), core.mode)
    });
    if mode == MovementMode::WalkExplore {
        w.camera.zoom = zoom;
        w.goal.zoom = zoom;
        let _ = super::set_camera(w);
    }
    sync_locomotion_receipt(w);
    let msg = if active {
        "Camera perspective: 3rd-person chase view."
    } else {
        "Camera perspective: 1st-person explorer view."
    };
    super::text("native-world-status", msg);
    active
}

pub fn toggle_anatomy_xray(w: &mut super::World) -> bool {
    let active = !anatomy_xray_active();
    set_anatomy_xray(w, active)
}

pub fn set_anatomy_xray(w: &mut super::World, active: bool) -> bool {
    ENGINE.with(|e| {
        e.borrow_mut().set_anatomy_xray(active);
    });
    sync_locomotion_receipt(w);
    let msg = if active {
        "Human Anatomy X-Ray active: osteology, myology, cardiac & pulmonary layers revealed."
    } else {
        "Human Anatomy X-Ray off: explorer integument restored."
    };
    super::text("native-world-status", msg);
    active
}

pub fn toggle_rhizosphere_xray(w: &mut super::World) -> bool {
    let active = !rhizosphere_xray_active();
    set_rhizosphere_xray(w, active)
}

pub fn set_rhizosphere_xray(w: &mut super::World, active: bool) -> bool {
    ENGINE.with(|e| {
        e.borrow_mut().rhizosphere_xray = active;
    });
    sync_locomotion_receipt(w);
    super::ui::sync_mode_ui(current_mode(), active);
    let mut wood = w.wood.take();
    if let Some(wood) = wood.as_mut() {
        wood.set_lens(&mut w.portal, active);
    }
    w.wood = wood;
    let msg = if active {
        "Rhizosphere X-Ray Lens active: subterranean root systems & bio-electric fields revealed."
    } else {
        "Rhizosphere X-Ray Lens off: surface landscape view restored."
    };
    super::text("native-world-status", msg);
    active
}

pub fn set_joystick_input(dx: f64, dy: f64, max_radius: f64) {
    let norm = normalize_joystick(dx, dy, max_radius, JOYSTICK_DEADZONE);
    ENGINE.with(|e| {
        e.borrow_mut().joystick = norm;
    });
}

pub fn clear_joystick_input() {
    ENGINE.with(|e| {
        e.borrow_mut().joystick = [0.0, 0.0];
    });
}

pub fn clear_all_inputs() {
    ENGINE.with(|e| {
        e.borrow_mut().clear_inputs();
    });
}

pub fn set_vertical_input(up: Option<bool>, down: Option<bool>) {
    ENGINE.with(|e| {
        let mut core = e.borrow_mut();
        if let Some(v) = up {
            core.keys_up = v;
        }
        if let Some(v) = down {
            core.keys_down = v;
        }
    });
}

pub fn apply_look_input(w: &mut super::World, dx: f32, dy: f32) -> Result<bool, JsValue> {
    let mode = current_mode();
    if matches!(mode, MovementMode::ObserveOrbit | MovementMode::DriveVehicle) {
        return Ok(false);
    }
    ENGINE.with(|e| {
        let mut core = e.borrow_mut();
        core.apply_look_delta(dx, dy);
        w.camera.yaw = core.yaw;
        w.camera.pitch = core.pitch;
        w.goal.yaw = core.yaw;
        w.goal.pitch = core.pitch;
    });
    super::set_camera(w)?;
    Ok(true)
}

pub fn apply_zoom_input(w: &mut super::World, dy: f32) -> Result<bool, JsValue> {
    let mode = current_mode();
    if matches!(mode, MovementMode::ObserveOrbit | MovementMode::DriveVehicle) {
        return Ok(false);
    }
    ENGINE.with(|e| {
        let mut core = e.borrow_mut();
        core.apply_zoom_delta(dy);
        let z = core.active_zoom();
        w.camera.zoom = z;
        w.goal.zoom = z;
    });
    super::set_camera(w)?;
    Ok(true)
}

/// Ray-march screen coordinates `(x, y)` onto `catchment_height_at` and set a tap-to-walk target.
pub fn tap_terrain_at(
    w: &mut super::World,
    x: f32,
    y: f32,
    width: u32,
    height: u32,
) -> bool {
    if current_mode() != MovementMode::WalkExplore || width == 0 || height == 0 {
        return false;
    }
    let aspect = width as f32 / height as f32;
    let vp = orbit_view_projection_with_near(
        w.camera.yaw,
        w.camera.pitch,
        w.camera.zoom,
        w.camera.target,
        aspect,
        CAMERA_NEAR_PLANE,
    );
    let Some(inv) = invert_mat4(vp) else {
        return false;
    };
    let ndc_x = (x / width as f32) * 2.0 - 1.0;
    let ndc_y = 1.0 - (y / height as f32) * 2.0;
    let near_p = unproject(&inv, ndc_x, ndc_y, 0.0);
    let far_p = unproject(&inv, ndc_x, ndc_y, 1.0);
    let (Some(p0), Some(p1)) = (near_p, far_p) else {
        return false;
    };
    let dir = [p1[0] - p0[0], p1[1] - p0[1], p1[2] - p0[2]];
    let len = (dir[0] * dir[0] + dir[1] * dir[1] + dir[2] * dir[2]).sqrt();
    if len < 1e-6 || !len.is_finite() {
        return false;
    }
    let dir = [dir[0] / len, dir[1] / len, dir[2] / len];
    let mut prev_t = 1.0_f64;
    let mut prev_diff =
        (p0[1] + dir[1] * prev_t) - catchment_height_at(p0[0] + dir[0] * prev_t, p0[2] + dir[2] * prev_t);
    for step in 1..=60 {
        let t = 1.0 + (step as f64) * 1.4;
        let wx = p0[0] + dir[0] * t;
        let wy = p0[1] + dir[1] * t;
        let wz = p0[2] + dir[2] * t;
        if wx.abs() > CATCHMENT_HALF_SPAN_M || wz.abs() > CATCHMENT_HALF_SPAN_M {
            break;
        }
        let h = catchment_height_at(wx, wz);
        let diff = wy - h;
        if diff <= 0.0 && prev_diff > 0.0 {
            let frac = prev_diff / (prev_diff - diff).max(1e-6);
            let ht = prev_t + (t - prev_t) * frac;
            let gx = (p0[0] + dir[0] * ht).clamp(-CATCHMENT_HALF_SPAN_M, CATCHMENT_HALF_SPAN_M);
            let gz = (p0[2] + dir[2] * ht).clamp(-CATCHMENT_HALF_SPAN_M, CATCHMENT_HALF_SPAN_M);
            ENGINE.with(|e| {
                e.borrow_mut().walk_goal = Some([gx, gz]);
            });
            return true;
        }
        prev_t = t;
        prev_diff = diff;
    }
    false
}

fn unproject(inv: &[[f64; 4]; 4], ndc_x: f32, ndc_y: f32, ndc_z: f32) -> Option<[f64; 3]> {
    let x = ndc_x as f64;
    let y = ndc_y as f64;
    let z = ndc_z as f64;
    let cx = inv[0][0] * x + inv[1][0] * y + inv[2][0] * z + inv[3][0];
    let cy = inv[0][1] * x + inv[1][1] * y + inv[2][1] * z + inv[3][1];
    let cz = inv[0][2] * x + inv[1][2] * y + inv[2][2] * z + inv[3][2];
    let cw = inv[0][3] * x + inv[1][3] * y + inv[2][3] * z + inv[3][3];
    if cw.abs() < 1e-9 || !cw.is_finite() {
        return None;
    }
    Some([cx / cw, cy / cw, cz / cw])
}

fn invert_mat4(m: [[f32; 4]; 4]) -> Option<[[f64; 4]; 4]> {
    let mut a = [[0.0_f64; 8]; 4];
    for r in 0..4 {
        for c in 0..4 {
            a[r][c] = m[c][r] as f64;
        }
        a[r][r + 4] = 1.0;
    }
    for i in 0..4 {
        let mut pivot = i;
        let mut best = a[i][i].abs();
        for r in (i + 1)..4 {
            if a[r][i].abs() > best {
                best = a[r][i].abs();
                pivot = r;
            }
        }
        if best < 1e-12 {
            return None;
        }
        a.swap(i, pivot);
        let div = a[i][i];
        for c in 0..8 {
            a[i][c] /= div;
        }
        for r in 0..4 {
            if r == i {
                continue;
            }
            let factor = a[r][i];
            for c in 0..8 {
                a[r][c] -= factor * a[i][c];
            }
        }
    }
    let mut out = [[0.0_f64; 4]; 4];
    for r in 0..4 {
        for c in 0..4 {
            out[c][r] = a[r][c + 4];
        }
    }
    Some(out)
}

/// Integrator/ui: consume movement keys when in Walk/Fly or unhitched/play-mode Observe steer.
#[allow(dead_code)]
pub fn keydown(w: &mut super::World, key: &str) -> bool {
    keydown_mods(w, key, false, false)
}

pub fn keydown_mods(w: &mut super::World, key: &str, shift: bool, ctrl: bool) -> bool {
    if matches!(key, "v" | "V") && current_mode() != MovementMode::ObserveOrbit {
        toggle_perspective(w);
        return true;
    }
    if ENGINE.with(|e| e.borrow_mut().keydown(key, shift, ctrl)) {
        return true;
    }
    let play = w.play_mode;
    w.macros
        .as_mut()
        .map(|live| live.steer_keydown_mode(key, play))
        .unwrap_or(false)
}

pub fn keyup(w: &mut super::World, key: &str) -> bool {
    let consumed_core = ENGINE.with(|e| e.borrow_mut().keyup(key));
    let play = w.play_mode;
    let consumed_macros = w
        .macros
        .as_mut()
        .map(|live| live.steer_keyup_mode(key, play))
        .unwrap_or(false);
    consumed_core || consumed_macros
}

/// Apply held locomotion (Observe steer or WalkExplore / FlyGlide kinematics) and clip forwarding.
pub fn tick(w: &mut super::World, dt_ms: f32) -> Result<(), JsValue> {
    let reduced = w.reduced;
    let mode = current_mode();
    let needs_portrait_check = ENGINE.with(|e| {
        let mut core = e.borrow_mut();
        if core.portrait_framing_applied {
            false
        } else {
            core.portrait_framing_applied = true;
            true
        }
    });
    if needs_portrait_check && mode == MovementMode::ObserveOrbit && !w.play_mode && w.last_input == 0.0 {
        let rect = w.canvas.get_bounding_client_rect();
        let vw = if rect.width() > 0.0 {
            rect.width()
        } else {
            w.canvas.width() as f64
        };
        let vh = if rect.height() > 0.0 {
            rect.height()
        } else {
            w.canvas.height() as f64
        };
        let (new_yaw, new_target) =
            apply_portrait_observe_offset(w.goal.yaw, w.goal.target, vw, vh);
        if new_yaw != w.goal.yaw || new_target != w.goal.target {
            w.goal.yaw = new_yaw;
            w.goal.target = new_target;
            w.camera.yaw = w.goal.yaw;
            w.camera.target = w.goal.target;
            super::set_camera(w)?;
        }
    }
    if matches!(mode, MovementMode::WalkExplore | MovementMode::FlyGlide) {
        let dt_s = (dt_ms as f64 / 1000.0).clamp(0.0, 0.05);
        ENGINE.with(|e| {
            let mut core = e.borrow_mut();
            // If an external place selection moved w.goal.target, adopt it cleanly.
            if (w.goal.target[0] - core.last_camera_target[0]).abs()
                + (w.goal.target[1] - core.last_camera_target[1]).abs()
                + (w.goal.target[2] - core.last_camera_target[2]).abs()
                > 0.05
            {
                let x = (w.goal.target[0] as f64).clamp(-CATCHMENT_HALF_SPAN_M, CATCHMENT_HALF_SPAN_M);
                let z = (w.goal.target[2] as f64).clamp(-CATCHMENT_HALF_SPAN_M, CATCHMENT_HALF_SPAN_M);
                let ground = catchment_height_at(x, z);
                core.pos[0] = x;
                core.pos[2] = z;
                core.pos[1] = if core.mode == MovementMode::FlyGlide {
                    (w.goal.target[1] as f64).max(ground + 4.0)
                } else {
                    ground
                };
                core.eye_y = if core.mode == MovementMode::FlyGlide {
                    core.pos[1]
                } else {
                    ground + EYE_HEIGHT_M
                };
                core.yaw = w.goal.yaw;
                core.pitch = w.goal.pitch.clamp(-PITCH_LIMIT_WALK, PITCH_LIMIT_WALK);
                core.walk_goal = None;
            }
            core.step(dt_s, reduced);
            w.camera.yaw = core.yaw;
            w.camera.pitch = core.pitch;
            w.camera.zoom = core.active_zoom();
            w.camera.target = core.eye_target();
            w.goal = w.camera.clone();
            core.last_camera_target = w.camera.target;
        });
        super::set_camera(w)?;
        sync_locomotion_receipt(w);
    }

    let mut clips = [(0u64, 0u64); 8];
    let play = w.play_mode;
    let n = match w.macros.as_mut() {
        Some(live) => {
            if mode == MovementMode::ObserveOrbit {
                let _ = live.tick_steer_mode(dt_ms, reduced, play);
            }
            live.take_pending_clips(&mut clips)
        }
        None => 0,
    };
    if reduced {
        return Ok(());
    }
    for &(joint, clip) in clips[..n].iter() {
        if clip == 0 {
            continue;
        }
        let _ = w.portal.select_scene_animation_clip(joint, clip, CLIP_FADE_S);
    }
    Ok(())
}

fn sync_locomotion_receipt(w: &mut super::World) {
    ENGINE.with(|e| {
        let core = e.borrow();
        let ground = core.ground_y();
        let alt = core.altitude_m();
        w.receipt["movement_mode"] = json!(core.mode.as_str());
        w.receipt["rhizosphere_xray"] = json!(core.rhizosphere_xray);
        w.receipt["anatomy_xray"] = json!(core.anatomy_xray);
        w.receipt["camera_perspective"] = json!(core.perspective_str());
        w.receipt["ocs_regime"] = json!(core.regime.as_str());
        w.receipt["locomotion"] = json!({
            "mode": core.mode.as_str(),
            "altitude_m": alt,
            "ground_y_m": ground,
            "wading": core.is_wading(),
            "slope_deg": core.last_slope_deg,
            "ocs_regime": core.regime.as_str(),
            "pos": [core.pos[0], core.eye_y, core.pos[2]],
            "rhizosphere_xray": core.rhizosphere_xray,
            "anatomy_xray": core.anatomy_xray,
            "camera_perspective": core.perspective_str()
        });
    });
}

#[allow(dead_code)]
pub fn hitch_radius_m(w: &super::World) -> f32 {
    w.macros
        .as_ref()
        .map(macros::Live::hitch_radius_m)
        .unwrap_or(DEFAULT_HITCH_RADIUS_M)
}

#[cfg(test)]
mod tests {
    use super::*;
    use qualia_core_db::q_hash;

    #[test]
    fn hitch_radius_api_defaults_to_four_metres() {
        assert!((HITCH_RADIUS_M - 4.0).abs() < f32::EPSILON);
        assert!((STEER_MPS - 8.0).abs() < f32::EPSILON);
        assert!((DEFAULT_HITCH_RADIUS_M - 4.0).abs() < f32::EPSILON);
    }

    #[test]
    fn clip_id_hashes_are_stable_q_hash_names() {
        assert_eq!(q_hash("walk_to_panels"), q_hash("walk_to_panels"));
        assert_ne!(q_hash("walk_to_panels"), q_hash("walk_to_hitch"));
        assert_ne!(q_hash("walk_to_panels"), 0);
    }

    #[test]
    fn module_is_generic_player_only() {
        let src = include_str!("locomotion.rs");
        let car = format!("if id == \"{}\"", "car");
        let camp = format!("{} camp", "mod");
        assert!(!src.contains(&car));
        assert!(!src.contains(&camp));
    }

    #[test]
    fn movement_mode_transitions_and_parsing() {
        let mut core = LocomotionCore::new();
        assert_eq!(core.mode, MovementMode::ObserveOrbit);
        assert_eq!(MovementMode::from_str("2"), Some(MovementMode::WalkExplore));
        assert_eq!(MovementMode::from_str("fly"), Some(MovementMode::FlyGlide));
        assert_eq!(MovementMode::from_str("observe"), Some(MovementMode::ObserveOrbit));

        assert!(core.set_mode(MovementMode::WalkExplore, 0.5, 0.2, 18.0, [-22.0, 5.49, 28.0]));
        assert_eq!(core.mode, MovementMode::WalkExplore);
        // Re-setting same mode is a no-op
        assert!(!core.set_mode(MovementMode::WalkExplore, 0.5, 0.2, 18.0, [-22.0, 5.49, 28.0]));

        assert!(core.set_mode(MovementMode::FlyGlide, 0.5, 0.2, 0.15, [-22.0, 6.14, 28.0]));
        assert_eq!(core.mode, MovementMode::FlyGlide);

        assert!(core.set_mode(MovementMode::ObserveOrbit, 0.5, 0.2, 0.25, [-22.0, 20.0, 28.0]));
        assert_eq!(core.mode, MovementMode::ObserveOrbit);
        assert_eq!(MovementMode::ObserveOrbit.next(), MovementMode::WalkExplore);
        assert_eq!(MovementMode::WalkExplore.next(), MovementMode::FlyGlide);
        assert_eq!(MovementMode::FlyGlide.next(), MovementMode::ObserveOrbit);
    }

    #[test]
    fn catchment_height_parity_at_campsite_and_creek_bed() {
        // Matches site/world/living-world.vibe camp_rig origin Y = 4.4909216892549235
        let camp_y = catchment_height_at(-22.0, 28.0);
        assert!(
            (camp_y - 4.4909216892549235).abs() < 1e-6,
            "campsite (-22, 28) expected ~4.4909 m, got {camp_y}"
        );
        // Flat campsite clearing within 8 m radius
        assert!((catchment_height_at(-20.0, 28.0) - camp_y).abs() < 1e-9);

        // Creek bed centreline at z = 0 and z = 16 lies submerged below 0.24 m water surface
        let rx0 = catchment_river_x(0.0);
        assert!(rx0.abs() < 1e-9);
        let bed0 = catchment_height_at(rx0, 0.0);
        assert!(bed0 < CREEK_WATER_LEVEL_M, "creek bed at z=0 expected < 0.24 m, got {bed0}");

        let rx16 = catchment_river_x(16.0);
        let bed16 = catchment_height_at(rx16, 16.0);
        assert!(bed16 < CREEK_WATER_LEVEL_M, "creek bed at z=16 expected < 0.24 m, got {bed16}");
        assert!(creek_wade_factor(bed16) < 1.0);
        assert!((creek_wade_factor(camp_y) - 1.0).abs() < 1e-9);
    }

    #[test]
    fn slope_clamping_slows_steep_climbs_above_32_degrees() {
        assert!((slope_speed_factor(0.0) - 1.0).abs() < 1e-9);
        assert!((slope_speed_factor(32.0) - 1.0).abs() < 1e-9);
        let mid = slope_speed_factor(45.0);
        assert!(mid < 1.0 && mid > SLOPE_MIN_FACTOR, "mid slope factor={mid}");
        let steep = slope_speed_factor(60.0);
        assert!((steep - SLOPE_MIN_FACTOR).abs() < 1e-9, "steep slope factor={steep}");
        assert!(slope_speed_factor(f64::NAN) == 1.0);
    }

    #[test]
    fn fly_speed_scales_smoothly_from_catchment_to_orbit() {
        let s_ground = fly_speed_mps(0.0, false);
        let s_10 = fly_speed_mps(10.0, false);
        let s_1k = fly_speed_mps(1_000.0, false);
        let s_10k = fly_speed_mps(10_000.0, false);
        let s_10m = fly_speed_mps(10_000_000.0, false);
        assert!(s_ground >= 10.0);
        assert!(s_10 > s_ground);
        assert!(s_1k > s_10);
        assert!(s_10k > s_1k);
        assert!(s_10m > s_10k);
        assert!((fly_speed_mps(100.0, true) - fly_speed_mps(100.0, false) * SPRINT_MULT).abs() < 1e-9);

        // Verify WalkExplore -> FlyGlide climb crosses into PlanetaryWgs84 (10 km+) then HeliocentricIcrf (10,000 km+)
        let mut core = LocomotionCore::new();
        core.set_mode(MovementMode::FlyGlide, 0.0, 0.0, 0.25, [-22.0, 10.0, 28.0]);
        core.keys_up = true;
        let mut saw_planetary = false;
        let mut saw_heliocentric = false;
        for _ in 0..500 {
            core.step(0.05, false);
            if core.regime == ScaleRegime::PlanetaryWgs84 {
                saw_planetary = true;
            }
            if core.regime == ScaleRegime::HeliocentricIcrf {
                saw_heliocentric = true;
                break;
            }
        }
        assert!(saw_planetary, "expected climb through PlanetaryWgs84 regime");
        assert!(saw_heliocentric, "expected climb into HeliocentricIcrf regime");
    }

    #[test]
    fn creek_wading_and_sprint_scale_walk_velocity() {
        let mut dry = LocomotionCore::new();
        dry.set_mode(MovementMode::WalkExplore, 0.0, 0.0, 0.15, [-22.0, 5.49, 28.0]);
        dry.keys_forward = true;
        let start_z = dry.pos[2];
        dry.step(0.05, false);
        let walk_dz = (start_z - dry.pos[2]).abs();

        let mut sprint = LocomotionCore::new();
        sprint.set_mode(MovementMode::WalkExplore, 0.0, 0.0, 0.15, [-22.0, 5.49, 28.0]);
        sprint.keys_forward = true;
        sprint.keys_sprint = true;
        sprint.step(0.05, false);
        let sprint_dz = (start_z - sprint.pos[2]).abs();
        assert!((sprint_dz / walk_dz - SPRINT_MULT).abs() < 1e-3);

        // Inside the creek channel (z = 16, x = river_x(16)), ground <= 0.24 m applies wading resistance
        let rx = catchment_river_x(16.0);
        let mut wade = LocomotionCore::new();
        wade.set_mode(MovementMode::WalkExplore, 0.0, 0.0, 0.15, [rx as f32, 0.0, 16.0]);
        wade.keys_forward = true;
        assert!(wade.is_wading());
        let wade_start_z = wade.pos[2];
        wade.step(0.05, false);
        let wade_dz = (wade_start_z - wade.pos[2]).abs();
        assert!(wade_dz < walk_dz * 0.75, "creek wading should slow walk ({wade_dz} vs {walk_dz})");
    }

    #[test]
    fn virtual_joystick_normalizes_deadzone_and_unit_circle() {
        assert_eq!(normalize_joystick(0.0, 0.0, 44.0, 0.12), [0.0, 0.0]);
        assert_eq!(normalize_joystick(2.0, -2.0, 44.0, 0.12), [0.0, 0.0]);
        assert_eq!(normalize_joystick(f64::NAN, 10.0, 44.0, 0.12), [0.0, 0.0]);

        // Full up push (dy = -100, beyond max_radius 44) clamps to [0.0, 1.0] forward
        let up = normalize_joystick(0.0, -100.0, 44.0, 0.12);
        assert!(up[0].abs() < 1e-9);
        assert!((up[1] - 1.0).abs() < 1e-9);

        // Diagonal overflow stays inside unit circle
        let diag = normalize_joystick(80.0, -80.0, 44.0, 0.12);
        let mag = (diag[0] * diag[0] + diag[1] * diag[1]).sqrt();
        assert!((mag - 1.0).abs() < 1e-9);
        assert!(diag[0] > 0.0 && diag[1] > 0.0);
    }

    #[test]
    fn walk_explore_conforms_to_terrain_and_supports_hop() {
        let mut core = LocomotionCore::new();
        core.set_mode(MovementMode::WalkExplore, 0.0, 0.0, 0.15, [-22.0, 5.49, 28.0]);
        let start_eye = core.eye_y;
        assert!((start_eye - (4.4909216892549235 + EYE_HEIGHT_M)).abs() < 1e-5);
        core.trigger_hop();
        assert!(!core.grounded);
        core.step(0.05, false);
        assert!(core.pos[1] > 4.4909, "hop should lift feet off ground");
        for _ in 0..40 {
            core.step(0.05, false);
        }
        assert!(core.grounded, "player should land back on catchment");
    }

    #[test]
    fn portrait_framing_leaves_desktop_landscape_untouched() {
        assert_eq!(portrait_observe_framing_offset(1280.0, 720.0), (0.0, 0.0));
        assert_eq!(portrait_observe_framing_offset(600.0, 600.0), (0.0, 0.0));
        assert_eq!(portrait_observe_framing_offset(0.0, 844.0), (0.0, 0.0));
        let (yaw, target) = apply_portrait_observe_offset(-0.5586, [-22.0, 5.49, 28.0], 1280.0, 720.0);
        assert_eq!(yaw, -0.5586);
        assert_eq!(target, [-22.0, 5.49, 28.0]);
    }

    #[test]
    fn portrait_framing_offsets_narrow_mobile_viewport() {
        let (dyaw, dtx) = portrait_observe_framing_offset(390.0, 844.0);
        assert!((dyaw - 0.52).abs() < 1e-4, "expected ~0.52 rad portrait yaw offset, got {dyaw}");
        assert!((dtx - 0.9).abs() < 1e-4, "expected ~0.9 m portrait lateral offset, got {dtx}");
        let (yaw, target) = apply_portrait_observe_offset(-0.5586, [-22.0, 5.49, 28.0], 390.0, 844.0);
        assert!((yaw - (-0.0386)).abs() < 1e-4);
        assert!((target[0] - (-21.1)).abs() < 1e-4);
        assert_eq!(target[1], 5.49);
        assert_eq!(target[2], 28.0);
    }

    #[test]
    fn drive_vehicle_mode_hands_off_camera_and_inputs() {
        assert_eq!(MovementMode::from_str("4"), Some(MovementMode::DriveVehicle));
        assert_eq!(MovementMode::from_str("drive"), Some(MovementMode::DriveVehicle));
        assert_eq!(MovementMode::DriveVehicle.as_str(), "drive");
        assert_eq!(MovementMode::DriveVehicle.label(), "Drive");
        assert_eq!(MovementMode::DriveVehicle.next(), MovementMode::WalkExplore);

        let mut core = LocomotionCore::new();
        assert!(core.set_mode(MovementMode::DriveVehicle, 0.5, 0.2, 18.0, [-22.0, 5.49, 28.0]));
        assert_eq!(core.mode, MovementMode::DriveVehicle);
        // Movement keys must NOT be consumed in DriveVehicle so the vehicle controller owns them.
        assert!(!core.keydown("w", false, false));
        assert!(!core.keydown("a", false, false));
        assert!(!core.keyup("w"));
        // Step is a no-op while driving so locomotion does not overwrite vehicle position.
        let pos_before = core.pos;
        core.keys_forward = true;
        core.step(0.05, false);
        assert_eq!(core.pos, pos_before);
        // Exiting DriveVehicle back to WalkExplore snaps cleanly to terrain.
        assert!(core.set_mode(MovementMode::WalkExplore, 0.5, 0.2, 6.5, [-22.0, 5.49, 28.0]));
        assert_eq!(core.mode, MovementMode::WalkExplore);
    }

    #[test]
    fn perspective_toggle_v_key_and_anatomy_xray_state() {
        let mut core = LocomotionCore::new();
        assert!(!core.third_person);
        assert_eq!(core.perspective_str(), "first-person");
        assert!(!core.anatomy_xray);

        core.set_mode(MovementMode::WalkExplore, 0.0, 0.0, 0.15, [-22.0, 5.49, 28.0]);
        assert!(core.keydown("v", false, false));
        assert!(core.third_person);
        assert_eq!(core.perspective_str(), "third-person-chase");
        assert!((core.walk_zoom - 3.4).abs() < 1e-4);

        assert!(core.keydown("V", false, false));
        assert!(!core.third_person);
        assert_eq!(core.perspective_str(), "first-person");
        assert!((core.walk_zoom - 0.15).abs() < 1e-4);

        // V key also toggles perspective while in DriveVehicle mode.
        core.set_mode(MovementMode::DriveVehicle, 0.0, 0.0, 6.5, [-22.0, 5.49, 28.0]);
        assert!(core.keydown("v", false, false));
        assert!(core.third_person);

        assert!(core.toggle_anatomy_xray());
        assert!(core.anatomy_xray);
        assert!(!core.toggle_anatomy_xray());
        assert!(!core.anatomy_xray);
    }
}
