//! Continuous unhitched steer, hitch proximity helpers, and walk-stage clip
//! forwarding. Generic player only: no per-scene ids, no camp.rs.
//!
//! Integrator routes (not owned here):
//! - `runtime.rs`: `mod locomotion;` and `locomotion::tick(w, dt)?;` before `macros::tick`
//! - `ui.rs`: canvas keydown/keyup after focus — `if locomotion::keydown(w, &key) { e.prevent_default(); }`
//! - `boot.rs`: no sequence hardcoding
//!
//! Hitched named-place travel stays the spring lerp in `macros`. Unhitched
//! groups accumulate WASD/Arrow travel offset. Hitch pose 1 is refused when
//! the gate group's parked freeze is outside hitchRadiusM of a standing
//! (travelsHitched, not travelsUnhitched) group's parked freeze.

use super::macros::{self, DEFAULT_HITCH_RADIUS_M};
use wasm_bindgen::JsValue;

/// Clip fade into SKL1 when the section is present. Missing clips are ignored.
const CLIP_FADE_S: f64 = 0.15;

pub use macros::DEFAULT_HITCH_RADIUS_M as HITCH_RADIUS_M;
pub use macros::DEFAULT_STEER_MPS as STEER_MPS;

/// Integrator/ui: consume Arrow/WASD as unhitched steer after canvas focus.
/// Returns true when the player used the key (hitched travel keeps camera
/// unless play mode is on, which steers the travelling party).
pub fn keydown(w: &mut super::World, key: &str) -> bool {
    let play = w.play_mode;
    w.macros
        .as_mut()
        .map(|live| live.steer_keydown_mode(key, play))
        .unwrap_or(false)
}

pub fn keyup(w: &mut super::World, key: &str) -> bool {
    let play = w.play_mode;
    w.macros
        .as_mut()
        .map(|live| live.steer_keyup_mode(key, play))
        .unwrap_or(false)
}

/// Apply held steer, then forward pending clipId hashes to portal SKL1.
/// Translation goals still play when the clip is missing or reduced-motion.
pub fn tick(w: &mut super::World, dt_ms: f32) -> Result<(), JsValue> {
    let reduced = w.reduced;
    let mut clips = [(0u64, 0u64); 8];
    let play = w.play_mode;
    let n = match w.macros.as_mut() {
        Some(live) => {
            let _ = live.tick_steer_mode(dt_ms, reduced, play);
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
        assert!(!src.contains(&car));
        assert!(!src.contains("mod camp"));
    }
}
