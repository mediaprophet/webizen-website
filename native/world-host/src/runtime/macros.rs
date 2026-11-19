//! Generic player for QualiaDB logic-engine macros stored in Q42 volumes.
//!
//! Apps, scenes, environments, and animations are authored in HMC containers,
//! Q42 SuperBlocks, and Vibe overlay functions. This host compiles those facts
//! with `pack_preset_command_quin` / `compile_norm_quin` and evaluates them with
//! `evaluate_predicate_triggers_zero_heap` and `evaluate_deontic_contract`.
//! Changing a sequence, including the woodland camp, must not require a new
//! per-scene Rust file or a host rebuild of the sequence.

use super::err;
use super::instruments::species::{
    evaluate_fauna_behavior, evaluate_flora_wind_sway, FaunaBehaviorState, LocomotionClass,
};
use super::knowledge::QuinIndex;
use qualia_core_db::modalities::holographic_scene::{
    evaluate_predicate_triggers_zero_heap, pack_preset_command_quin, AssetStateNode,
    PredicateTriggerRule, ScheduledPresetCommand,
};
use qualia_core_db::modalities::logic::{
    compile_norm_quin, evaluate_deontic_contract, DeonticStatus, DeonticVerdict, OP_FORBID,
    OP_PERMIT,
};
use qualia_core_db::modalities::manifold::ManifoldCoordinate10D;
use qualia_core_db::modalities::spatio_temporal::TemporalOp;
use qualia_core_db::{q_hash, NQuin};
use serde_json::{json, Value};
use vibe::animation::{evaluate_preset, AnimationFamily};
use wasm_bindgen::JsValue;

const LOGIC_SCHEMA: &str = "webizen.logic-sequence/v1";
const CAMP_SCHEMA: &str = "webizen.camp-sequence/v1";
const MAX_GROUPS: usize = 16;
const MAX_STAGES: usize = 32;
const MAX_NORMS: usize = 16;
const MAX_ACTIONS: usize = 8;
/// Authored hitch proximity default when Q42 omits hitchRadiusM.
pub const DEFAULT_HITCH_RADIUS_M: f32 = 4.0;
/// Continuous unhitched steer default when Q42 omits steerSpeedMps.
pub const DEFAULT_STEER_MPS: f32 = 8.0;
const KEY_N: u8 = 1;
const KEY_S: u8 = 2;
const KEY_W: u8 = 4;
const KEY_E: u8 = 8;

#[derive(Clone, Copy)]
struct Group {
    id_hash: u64,
    delta: [f32; 3],
    origin: [f32; 3],
    /// Target scale at fold pose 1. [1,1,1] leaves the mesh unchanged.
    scale: [f32; 3],
    travels_hitched: bool,
    travels_unhitched: bool,
}

#[derive(Clone, Copy)]
struct Stage {
    trigger: u64,
    group: usize,
    follow: u64,
    duration_ms: u16,
    goal: f32,
    /// q_hash of authored clipId; 0 means translation-only.
    clip: u64,
    /// Fold channel drives scale (panel suitcase / side pop-in). Stow
    /// channel (false) drives translation.
    fold: bool,
}

#[derive(Clone, Copy)]
struct Norm {
    quin: NQuin,
    gate_defeater: bool,
    unhitch_defeater: bool,
    near_defeater: bool,
}

#[derive(Clone)]
pub struct Action {
    pub id: String,
    pub label: String,
    uri: String,
    trigger: String,
    message: String,
    /// Group ids this action is about (rel:aboutGroup last path segment).
    pub about: Vec<String>,
}

#[derive(Clone)]
pub struct Live {
    groups: [Group; MAX_GROUPS],
    group_ids: [String; MAX_GROUPS],
    prefixes: [String; MAX_GROUPS],
    group_labels: [String; MAX_GROUPS],
    group_n: usize,
    stages: [Stage; MAX_STAGES],
    stage_n: usize,
    norms: [Norm; MAX_NORMS],
    norm_n: usize,
    actions: Vec<Action>,
    poses: [f32; MAX_GROUPS],
    goals: [f32; MAX_GROUPS],
    clocks: [f32; MAX_GROUPS],
    folds: [f32; MAX_GROUPS],
    fold_goals: [f32; MAX_GROUPS],
    fold_clocks: [f32; MAX_GROUPS],
    travel_from: [f32; 3],
    travel_to: [f32; 3],
    parked: [[f32; 3]; MAX_GROUPS],
    pose_travel: f32,
    clock_travel: f32,
    origin: [f32; 3],
    family: AnimationFamily,
    preset: String,
    party: u64,
    contract: u64,
    seq_hash: u64,
    topic: u64,
    travel_action: u64,
    gate_group: usize,
    hitch_anchor: usize,
    follow_event: u64,
    panel_label: String,
    forbid_message: String,
    travel_message: String,
    travel_message_hitched: String,
    travel_message_unhitched: String,
    hitch_radius_m: f32,
    steer_mps: f32,
    keys: u8,
    pending_clips: [u64; MAX_STAGES],
    pending_joints: [u64; MAX_STAGES],
    pending_clip_n: usize,
    biosphere_clock_s: f64,
    dirty: bool,
}

impl Live {
    pub fn from_world(
        host: &mut qualia_core_db::vibe_host::PoetSnapshot,
        source: &str,
    ) -> Result<Self, String> {
        let index = QuinIndex::from_host(host)?;
        let seq = find_sequence(&index)?;
        let schema = index.text(&seq, "schema")?;
        if schema != LOGIC_SCHEMA && schema != CAMP_SCHEMA {
            return Err("Unsupported logic sequence schema".into());
        }
        let family_name = index
            .text(&seq, "presetFamily")
            .unwrap_or_else(|_| "physical_dynamics".into());
        let family = AnimationFamily::from_name(&family_name)
            .ok_or("Unknown Vibe animation family in the logic sequence")?;
        let preset = index
            .text(&seq, "preset")
            .unwrap_or_else(|_| "spring_settle".into());
        let origin = [
            index.number(&seq, "originX").unwrap_or(0.) as f32,
            index.number(&seq, "originY").unwrap_or(0.) as f32,
            index.number(&seq, "originZ").unwrap_or(0.) as f32,
        ];
        let rig_fn = index.text(&seq, "rigFunction").unwrap_or_default();
        let rig = eval_rig(host, source, &rig_fn)?;
        let panel_label = index
            .text(&seq, "label")
            .or_else(|_| index.text(&seq, "id"))
            .unwrap_or_else(|_| "Sequence".into());
        let forbid_message = index.text(&seq, "forbidMessage").unwrap_or_else(|_| {
            "That action is not permitted until the sequence gate is complete.".into()
        });
        let travel_message = index
            .text(&seq, "travelMessage")
            .unwrap_or_else(|_| "Travelling to this place.".into());
        let travel_message_hitched = index
            .text(&seq, "travelMessageHitched")
            .unwrap_or_else(|_| travel_message.clone());
        let travel_message_unhitched = index
            .text(&seq, "travelMessageUnhitched")
            .unwrap_or_else(|_| travel_message.clone());
        let travel_action = q_hash(
            &index
                .text(&seq, "travelAction")
                .unwrap_or_else(|_| "urn:webizen:sequence:travel".into()),
        );
        let hitch_radius_m = index
            .number(&seq, "hitchRadiusM")
            .ok()
            .map(|n| n as f32)
            .filter(|n| n.is_finite() && *n > 0.)
            .unwrap_or(DEFAULT_HITCH_RADIUS_M);
        let steer_mps = index
            .number(&seq, "steerSpeedMps")
            .ok()
            .map(|n| n as f32)
            .filter(|n| n.is_finite() && *n > 0.)
            .unwrap_or(DEFAULT_STEER_MPS);
        let mut live = Self {
            groups: [Group {
                id_hash: 0,
                delta: [0.; 3],
                origin: [0.; 3],
                scale: [1.; 3],
                travels_hitched: true,
                travels_unhitched: false,
            }; MAX_GROUPS],
            group_ids: std::array::from_fn(|_| String::new()),
            prefixes: std::array::from_fn(|_| String::new()),
            group_labels: std::array::from_fn(|_| String::new()),
            group_n: 0,
            stages: [Stage {
                trigger: 0,
                group: 0,
                follow: 0,
                duration_ms: 1,
                goal: 1.,
                clip: 0,
                fold: false,
            }; MAX_STAGES],
            stage_n: 0,
            norms: [Norm {
                quin: compile_norm_quin(0, OP_PERMIT, 0, 0, 0, 0, false),
                gate_defeater: false,
                unhitch_defeater: false,
                near_defeater: false,
            }; MAX_NORMS],
            norm_n: 0,
            actions: Vec::new(),
            poses: [0.; MAX_GROUPS],
            goals: [0.; MAX_GROUPS],
            clocks: [0.; MAX_GROUPS],
            folds: [0.; MAX_GROUPS],
            fold_goals: [0.; MAX_GROUPS],
            fold_clocks: [0.; MAX_GROUPS],
            travel_from: [0.; 3],
            travel_to: [0.; 3],
            parked: [[0.; 3]; MAX_GROUPS],
            pose_travel: 1.,
            clock_travel: 0.,
            origin,
            family,
            preset,
            party: 0,
            contract: 0,
            seq_hash: q_hash(&seq),
            topic: q_hash(&seq),
            travel_action,
            gate_group: usize::MAX,
            hitch_anchor: usize::MAX,
            follow_event: 0,
            panel_label,
            forbid_message,
            travel_message,
            travel_message_hitched,
            travel_message_unhitched,
            hitch_radius_m,
            steer_mps,
            keys: 0,
            pending_clips: [0; MAX_STAGES],
            pending_joints: [0; MAX_STAGES],
            pending_clip_n: 0,
            biosphere_clock_s: 0.0,
            dirty: true,
        };
        if let Some(o) = vec3(rig.get("origin")) {
            live.origin = o;
        }
        if let Some(r) = rig.get("hitch_radius_m").and_then(Value::as_f64) {
            if r.is_finite() && r > 0. {
                live.hitch_radius_m = r as f32;
            }
        }
        if let Some(s) = rig.get("steer_speed_mps").and_then(Value::as_f64) {
            if s.is_finite() && s > 0. {
                live.steer_mps = s as f32;
            }
        }
        live.load_groups(&index, &seq, &rig)?;
        live.load_actions(&index, &seq, &rig)?;
        live.load_stages(&index, &seq)?;
        live.load_norms(&index, &seq)?;
        let gate_id = index.text(&seq, "gateGroup").unwrap_or_default();
        if !gate_id.is_empty() {
            live.gate_group = live
                .group_ids
                .iter()
                .take(live.group_n)
                .position(|id| id == &gate_id)
                .ok_or_else(|| format!("Logic sequence names unknown gate group {gate_id}"))?;
        }
        let anchor_id = index.text(&seq, "hitchAnchorGroup").unwrap_or_default();
        if !anchor_id.is_empty() {
            live.hitch_anchor = live
                .group_ids
                .iter()
                .take(live.group_n)
                .position(|id| id == &anchor_id)
                .ok_or_else(|| {
                    format!("Logic sequence names unknown hitch anchor group {anchor_id}")
                })?;
        }
        Ok(live)
    }

    fn load_groups(&mut self, index: &QuinIndex, seq: &str, rig: &Value) -> Result<(), String> {
        let mut rows: Vec<(u64, String)> = Vec::new();
        for uri in index.rel(seq, "hasGroup") {
            rows.push((index.u64(&uri, "index").unwrap_or(0), uri));
        }
        rows.sort_by_key(|(n, _)| *n);
        for (_, uri) in rows {
            if self.group_n >= MAX_GROUPS {
                return Err("Logic sequence exceeds the host group budget".into());
            }
            let id = index.text(&uri, "id")?;
            let prefixes = index.text(&uri, "prefixes")?;
            let n = self.group_n;
            let travels = index.boolean(&uri, "travels").unwrap_or(true);
            self.groups[n] = Group {
                id_hash: q_hash(&uri),
                delta: [
                    index.number(&uri, "deltaX").unwrap_or(0.) as f32,
                    index.number(&uri, "deltaY").unwrap_or(0.) as f32,
                    index.number(&uri, "deltaZ").unwrap_or(0.) as f32,
                ],
                origin: [
                    index.number(&uri, "originX").unwrap_or(0.) as f32,
                    index.number(&uri, "originY").unwrap_or(0.) as f32,
                    index.number(&uri, "originZ").unwrap_or(0.) as f32,
                ],
                scale: [
                    index.number(&uri, "scaleX").unwrap_or(1.) as f32,
                    index.number(&uri, "scaleY").unwrap_or(1.) as f32,
                    index.number(&uri, "scaleZ").unwrap_or(1.) as f32,
                ],
                travels_hitched: index.boolean(&uri, "travelsHitched").unwrap_or(travels),
                travels_unhitched: index.boolean(&uri, "travelsUnhitched").unwrap_or(false),
            };
            if let Some(g) = rig_group(rig, &id) {
                if let Some(d) = vec3(g.get("delta")) {
                    self.groups[n].delta = d;
                }
                if let Some(o) = vec3(g.get("origin")) {
                    self.groups[n].origin = o;
                }
                if let Some(s) = vec3(g.get("scale")) {
                    self.groups[n].scale = s;
                }
                if let Some(h) = g.get("travels_hitched").and_then(Value::as_bool) {
                    self.groups[n].travels_hitched = h;
                } else if let Some(t) = g.get("travels").and_then(Value::as_bool) {
                    self.groups[n].travels_hitched = t;
                }
                if let Some(u) = g.get("travels_unhitched").and_then(Value::as_bool) {
                    self.groups[n].travels_unhitched = u;
                }
            }
            self.group_ids[n] = id;
            self.group_labels[n] = index
                .text(&uri, "label")
                .unwrap_or_else(|_| self.group_ids[n].clone());
            if let Some(l) = rig_group(rig, &self.group_ids[n])
                .and_then(|g| g.get("label"))
                .and_then(Value::as_str)
            {
                if !l.is_empty() {
                    self.group_labels[n] = l.to_string();
                }
            }
            self.prefixes[n] = match rig_group(rig, &self.group_ids[n])
                .and_then(|g| g.get("prefixes"))
                .and_then(Value::as_str)
            {
                Some(p) if !p.is_empty() => p.to_string(),
                _ => prefixes,
            };
            self.group_n += 1;
        }
        if self.group_n == 0 {
            return Err("Logic sequence has no groups".into());
        }
        Ok(())
    }

    fn load_actions(&mut self, index: &QuinIndex, seq: &str, rig: &Value) -> Result<(), String> {
        let mut rows: Vec<(u64, String)> = Vec::new();
        for uri in index.rel(seq, "hasAction") {
            rows.push((index.u64(&uri, "index").unwrap_or(0), uri));
        }
        rows.sort_by_key(|(n, _)| *n);
        for (_, uri) in rows {
            if self.actions.len() >= MAX_ACTIONS {
                break;
            }
            let id = index.text(&uri, "id")?;
            let mut label = index.text(&uri, "label")?;
            if let Some(a) = rig.get("actions").and_then(Value::as_array) {
                if let Some(found) = a.iter().find(|v| v["id"].as_str() == Some(id.as_str())) {
                    if let Some(l) = found["label"].as_str() {
                        label = l.to_string();
                    }
                }
            }
            let action_uri = index
                .text(&uri, "actionUri")
                .unwrap_or_else(|_| format!("urn:webizen:sequence:{id}"));
            let trigger = index
                .text(&uri, "triggerEvent")
                .unwrap_or_else(|_| format!("event:{id}"));
            let message = index
                .text(&uri, "statusMessage")
                .unwrap_or_else(|_| label.clone());
            let about: Vec<String> = index
                .rel(&uri, "aboutGroup")
                .iter()
                .filter_map(|iri| iri.rsplit('/').next())
                .filter(|s| !s.is_empty())
                .map(|s| s.to_string())
                .collect();
            self.actions.push(Action {
                id,
                label,
                uri: action_uri,
                trigger,
                message,
                about,
            });
        }
        Ok(())
    }

    fn load_stages(&mut self, index: &QuinIndex, seq: &str) -> Result<(), String> {
        let mut rows: Vec<(u64, String)> = Vec::new();
        for uri in index.rel(seq, "hasStage") {
            rows.push((index.u64(&uri, "index").unwrap_or(0), uri));
        }
        rows.sort_by_key(|(n, _)| *n);
        for (_, uri) in rows {
            if self.stage_n >= MAX_STAGES {
                return Err("Logic sequence exceeds the host stage budget".into());
            }
            let group_id = index.text(&uri, "targetGroup")?;
            let group = self
                .group_ids
                .iter()
                .take(self.group_n)
                .position(|id| id == &group_id)
                .ok_or_else(|| format!("Logic stage names unknown group {group_id}"))?;
            let follow = index.text(&uri, "followUpEvent").unwrap_or_default();
            let clip = index.text(&uri, "clipId").unwrap_or_default();
            let channel = index.text(&uri, "channel").unwrap_or_default();
            self.stages[self.stage_n] = Stage {
                trigger: q_hash(&index.text(&uri, "triggerEvent")?),
                group,
                follow: if follow.is_empty() { 0 } else { q_hash(&follow) },
                duration_ms: index.u64(&uri, "durationMs").unwrap_or(1600) as u16,
                goal: index.number(&uri, "goal").unwrap_or(1.) as f32,
                clip: if clip.is_empty() { 0 } else { q_hash(&clip) },
                fold: channel == "fold",
            };
            self.stage_n += 1;
        }
        Ok(())
    }

    fn load_norms(&mut self, index: &QuinIndex, seq: &str) -> Result<(), String> {
        for uri in index.rel(seq, "hasNorm") {
            if self.norm_n >= MAX_NORMS {
                return Err("Logic sequence exceeds the host norm budget".into());
            }
            let opcode = match index.text(&uri, "opcode")?.as_str() {
                "0x11" | "PERMIT" => OP_PERMIT,
                "0x12" | "FORBID" => OP_FORBID,
                _ => OP_PERMIT,
            };
            let action = q_hash(&index.text(&uri, "action")?);
            let party = q_hash(
                &index
                    .text(&uri, "party")
                    .unwrap_or_else(|_| "urn:webizen:sequence:party".into()),
            );
            let contract = q_hash(
                &index
                    .text(&uri, "contract")
                    .unwrap_or_else(|_| "urn:webizen:sequence:contract".into()),
            );
            self.party = party;
            self.contract = contract;
            let gated = index.boolean(&uri, "requiresGate").unwrap_or(false)
                || index.boolean(&uri, "requiresHitch").unwrap_or(false);
            let unhitched = index.boolean(&uri, "requiresUnhitch").unwrap_or(false);
            let near = index.boolean(&uri, "requiresHitchNear").unwrap_or(false)
                || index.boolean(&uri, "requiresProximity").unwrap_or(false);
            self.norms[self.norm_n] = Norm {
                quin: compile_norm_quin(
                    party,
                    opcode,
                    action,
                    action,
                    contract,
                    0,
                    gated || unhitched || near,
                ),
                gate_defeater: gated,
                unhitch_defeater: unhitched,
                near_defeater: near,
            };
            self.norm_n += 1;
        }
        Ok(())
    }

    pub fn actions(&self) -> &[Action] {
        &self.actions
    }

    pub fn panel_label(&self) -> &str {
        &self.panel_label
    }

    pub fn hitch_radius_m(&self) -> f32 {
        self.hitch_radius_m
    }

    /// Q42/Vibe travel copy for the live hitch state. Player never hardcodes it.
    pub fn travel_status(&self) -> &str {
        let msg = if self.unhitch_ready() && !self.gate_ready() {
            self.travel_message_unhitched.as_str()
        } else {
            self.travel_message_hitched.as_str()
        };
        if msg.is_empty() {
            self.travel_message.as_str()
        } else {
            msg
        }
    }

    /// Planar XZ proximity of the gate group's parked freeze to a standing
    /// (travelsHitched, not travelsUnhitched) hitch-anchor parked freeze.
    pub fn hitch_near(&self) -> bool {
        parked_within_radius(self.gate_parked(), self.anchor_parked(), self.hitch_radius_m)
    }

    /// Integrator/ui: true when the key is consumed as unhitched steer.
    /// Observer path keeps unhitched-only steer; play mode also steers a
    /// hitched travelling party.
    pub fn steer_keydown(&mut self, key: &str) -> bool {
        self.steer_keydown_mode(key, false)
    }

    pub fn steer_keydown_mode(&mut self, key: &str, play: bool) -> bool {
        let bit = steer_bit(key);
        if bit == 0 {
            return false;
        }
        self.keys |= bit;
        self.steer_allowed_mode(play)
    }

    pub fn steer_keyup(&mut self, key: &str) -> bool {
        self.steer_keyup_mode(key, false)
    }

    pub fn steer_keyup_mode(&mut self, key: &str, play: bool) -> bool {
        let bit = steer_bit(key);
        if bit == 0 {
            return false;
        }
        self.keys &= !bit;
        self.steer_allowed_mode(play)
    }

    /// Accumulate WASD/Arrow travel offset for unhitched groups only.
    pub fn tick_steer(&mut self, dt_ms: f32, reduced: bool) -> bool {
        self.tick_steer_mode(dt_ms, reduced, false)
    }

    pub fn tick_steer_mode(&mut self, dt_ms: f32, _reduced: bool, play: bool) -> bool {
        if !self.steer_allowed_mode(play) {
            return false;
        }
        let (dx, dz) = steer_axes(self.keys);
        if dx == 0. && dz == 0. {
            return false;
        }
        let dt_s = (dt_ms / 1000.).max(0.);
        if dt_s == 0. {
            return false;
        }
        let mut len = (dx * dx + dz * dz).sqrt();
        if len < 1e-6 {
            return false;
        }
        len = self.steer_mps * dt_s / len;
        self.sync_parked();
        let mut offset = self.travel_offset();
        offset[0] += dx * len;
        offset[2] += dz * len;
        self.travel_from = offset;
        self.travel_to = offset;
        self.pose_travel = 1.;
        self.clock_travel = 0.;
        self.sync_parked();
        self.dirty = true;
        true
    }

    /// Drain walk-stage clip hashes for portal SKL1. Missing clips are ignored.
    pub fn take_pending_clips(&mut self, out: &mut [(u64, u64)]) -> usize {
        let n = self.pending_clip_n.min(out.len()).min(MAX_STAGES);
        for i in 0..n {
            out[i] = (self.pending_joints[i], self.pending_clips[i]);
        }
        self.pending_clip_n = 0;
        n
    }

    pub fn overlay(&self, asset_id: &str, has_placement: bool) -> [f64; 3] {
        let Some(g) = self.group_for(asset_id) else {
            return [0.; 3];
        };
        let pose = self.poses[g];
        let group = self.groups[g];
        let mut out = [0f64; 3];
        if !has_placement {
            for i in 0..3 {
                out[i] += group.origin[i] as f64;
            }
        }
        let travel = if self.group_travels(g) {
            self.travel_offset()
        } else {
            self.parked[g]
        };
        for i in 0..3 {
            out[i] += (group.delta[i] * pose) as f64;
            out[i] += travel[i] as f64;
        }
        out
    }

    /// Instance scale for the fold channel. Identity when the group does not fold.
    pub fn overlay_scale(&self, asset_id: &str) -> [f64; 3] {
        let Some(g) = self.group_for(asset_id) else {
            return [1.; 3];
        };
        let t = self.folds[g];
        let scale = self.groups[g].scale;
        [
            (1. + (scale[0] - 1.) * t) as f64,
            (1. + (scale[1] - 1.) * t) as f64,
            (1. + (scale[2] - 1.) * t) as f64,
        ]
    }

    pub fn fire(&mut self, action_id: &str, reduced: bool) -> Result<String, JsValue> {
        let found = self
            .actions
            .iter()
            .find(|a| a.id == action_id)
            .cloned()
            .ok_or_else(|| err("Unknown sequence action"))?;
        if !self.permitted(q_hash(&found.uri)) {
            return Err(err(&self.forbid_message));
        }
        let trigger = q_hash(&found.trigger);
        if self.trigger_hitch_pose(trigger) && !self.hitch_near() {
            return Err(err(&self.forbid_message));
        }
        self.sync_parked();
        self.play_trigger(trigger, reduced)?;
        Ok(found.message)
    }

    pub fn travel_to(&mut self, x: f32, z: f32, reduced: bool) -> Result<bool, JsValue> {
        if !self.permitted(self.travel_action) {
            return Ok(false);
        }
        self.sync_parked();
        let dest = [x - self.origin[0], 0., z - self.origin[2]];
        for i in 0..3 {
            let a = self.travel_from[i];
            let b = self.travel_to[i];
            self.travel_from[i] = a + (b - a) * self.pose_travel;
        }
        self.travel_to = dest;
        self.pose_travel = if reduced { 1. } else { 0. };
        self.clock_travel = 0.;
        self.dirty = true;
        let mover = (0..self.group_n)
            .find(|&i| self.group_travels(i))
            .map(|i| self.groups[i].id_hash)
            .unwrap_or(self.seq_hash);
        let _ = pack_preset_command_quin(
            mover,
            q_hash("subpart:travel"),
            q_hash(&self.preset),
            TemporalOp::Meets,
            2200,
            80,
            255,
            self.seq_hash,
            1,
        );
        Ok(true)
    }

    pub fn tick(&mut self, dt_ms: f32, reduced: bool) -> bool {
        let mut changed = false;
        let dt_s = (dt_ms as f64 / 1000.).max(0.);
        if !reduced {
            self.biosphere_clock_s += dt_s;
        }
        for i in 0..self.group_n {
            if advance_channel(
                &mut self.poses[i],
                self.goals[i],
                &mut self.clocks[i],
                dt_s as f32,
                self.family,
                &self.preset,
                reduced,
            ) {
                changed = true;
            }
            if advance_channel(
                &mut self.folds[i],
                self.fold_goals[i],
                &mut self.fold_clocks[i],
                dt_s as f32,
                self.family,
                &self.preset,
                reduced,
            ) {
                changed = true;
            }
        }
        if self.pose_travel < 0.9995 {
            self.clock_travel += dt_s as f32;
            let sample = evaluate_preset(self.family, &self.preset, self.clock_travel as f64);
            self.pose_travel = if reduced || sample.settled {
                1.
            } else {
                sample.scalar.clamp(0., 1.) as f32
            };
            changed = true;
        } else {
            self.pose_travel = 1.;
        }
        self.sync_parked();
        // walk-to-hitch (and other already-at-goal stages) settle without
        // interpolating. Follow-up hitch still has to fire in that frame.
        if self.follow_event != 0 && self.stage_settled() {
            let next = self.follow_event;
            self.follow_event = 0;
            let _ = self.play_trigger(next, reduced);
            changed = true;
        }
        if changed {
            self.dirty = true;
        }
        let dirty = self.dirty;
        self.dirty = false;
        dirty
    }

    pub fn receipt(&self) -> Value {
        json!({
            "engine": "QualiaDB holographic_scene + deontic",
            "schema": LOGIC_SCHEMA,
            "preset": self.preset,
            "family": self.family.name(),
            "groups": self.group_n,
            "stages": self.stage_n,
            "norms": self.norm_n,
            "gate": self.gate_ready(),
            "unhitched": self.unhitch_ready(),
            "hitchNear": self.hitch_near(),
            "hitchRadiusM": self.hitch_radius_m,
            "travel": self.pose_travel,
            "biosphereClockS": self.biosphere_clock_s,
            "party": self.party,
            "contract": self.contract
        })
    }

    /// Evaluate the generic fauna behavioral loop at `self.biosphere_clock_s`.
    #[allow(dead_code)]
    pub fn step_biosphere_fauna(
        &self,
        actors: &[FaunaLoopActor],
        reduced: bool,
    ) -> Vec<FaunaLoopStep> {
        step_fauna_behavioral_loop(actors, self.biosphere_clock_s, reduced)
    }

    /// Sample deterministic flora wind sway at `self.biosphere_clock_s`.
    #[allow(dead_code)]
    pub fn sample_flora_wind(
        &self,
        seed: u64,
        world_xz: [f32; 2],
        cfg: &FloraWindConfig,
        reduced: bool,
    ) -> [f64; 3] {
        flora_wind_sway_offset(self.biosphere_clock_s, seed, world_xz, cfg, reduced)
    }

    fn gate_ready(&self) -> bool {
        self.gate_group == usize::MAX
            || (self.gate_group < self.group_n && self.poses[self.gate_group] >= 0.99)
    }

    fn unhitch_ready(&self) -> bool {
        self.gate_group == usize::MAX
            || (self.gate_group < self.group_n && self.poses[self.gate_group] <= 0.01)
    }

    fn steer_allowed(&self) -> bool {
        self.steer_allowed_mode(false)
    }

    fn steer_allowed_mode(&self, play: bool) -> bool {
        if self.pose_travel < 0.9995 {
            return false;
        }
        if play {
            if self.gate_group < self.group_n
                && (self.poses[self.gate_group] - self.goals[self.gate_group]).abs() > 0.0005
            {
                return false;
            }
            return (0..self.group_n).any(|i| self.group_travels(i));
        }
        if !self.unhitch_ready() {
            return false;
        }
        if self.gate_group < self.group_n && self.goals[self.gate_group] > 0.01 {
            return false;
        }
        (0..self.group_n).any(|i| self.groups[i].travels_unhitched)
    }

    /// Travelling gate group, else the first group that currently travels.
    pub fn focus_group_index(&self) -> Option<usize> {
        if self.gate_group < self.group_n && self.group_travels(self.gate_group) {
            Some(self.gate_group)
        } else {
            (0..self.group_n).find(|&i| self.group_travels(i))
        }
    }

    /// Sequence-space position of the travelling party (origin + travel offset).
    pub fn focus_world(&self) -> [f32; 3] {
        let travel = match self.focus_group_index() {
            Some(i) if self.group_travels(i) => self.travel_offset(),
            Some(i) => self.parked[i],
            None => self.travel_offset(),
        };
        [
            self.origin[0] + travel[0],
            self.origin[1] + travel[1],
            self.origin[2] + travel[2],
        ]
    }

    pub fn group_for(&self, asset_id: &str) -> Option<usize> {
        let mut best = None;
        let mut best_len = 0usize;
        for i in 0..self.group_n {
            for prefix in self.prefixes[i].split(',') {
                let prefix = prefix.trim();
                if prefix.is_empty() || !asset_id.starts_with(prefix) {
                    continue;
                }
                if prefix.len() > best_len {
                    best_len = prefix.len();
                    best = Some(i);
                }
            }
        }
        best
    }

    pub fn group_id_for_asset(&self, asset_id: &str) -> Option<&str> {
        self.group_for(asset_id)
            .map(|i| self.group_ids[i].as_str())
    }

    pub fn group_label(&self, i: usize) -> &str {
        if i >= self.group_n {
            return "";
        }
        if self.group_labels[i].is_empty() {
            self.group_ids[i].as_str()
        } else {
            self.group_labels[i].as_str()
        }
    }

    pub fn group_label_for(&self, group_id: &str) -> Option<&str> {
        self.group_ids
            .iter()
            .take(self.group_n)
            .position(|id| id == group_id)
            .map(|i| self.group_label(i))
    }

    /// Permitted sequence actions authored about this group.
    pub fn permitted_actions_about(&self, group_id: &str) -> Vec<&Action> {
        self.actions
            .iter()
            .filter(|a| {
                a.about.iter().any(|g| g == group_id) && self.permitted(q_hash(&a.uri))
            })
            .collect()
    }

    fn gate_parked(&self) -> Option<[f32; 3]> {
        (self.gate_group < self.group_n).then(|| self.parked[self.gate_group])
    }

    fn anchor_parked(&self) -> Option<[f32; 3]> {
        let i = self.resolve_hitch_anchor()?;
        Some(self.parked[i])
    }

    fn resolve_hitch_anchor(&self) -> Option<usize> {
        if self.hitch_anchor < self.group_n && self.hitch_anchor != self.gate_group {
            return Some(self.hitch_anchor);
        }
        (0..self.group_n).find(|&i| {
            i != self.gate_group
                && self.groups[i].travels_hitched
                && !self.groups[i].travels_unhitched
        })
    }

    fn trigger_hitch_pose(&self, trigger: u64) -> bool {
        let mut seen = [0u64; MAX_STAGES];
        let mut n = 0usize;
        let mut cur = trigger;
        while cur != 0 && n < MAX_STAGES && !seen[..n].contains(&cur) {
            seen[n] = cur;
            n += 1;
            let mut next = 0u64;
            for i in 0..self.stage_n {
                if self.stages[i].trigger != cur {
                    continue;
                }
                if self.stages[i].group == self.gate_group && self.stages[i].goal >= 0.99 {
                    return true;
                }
                if self.stages[i].follow != 0 {
                    next = self.stages[i].follow;
                }
            }
            cur = next;
        }
        false
    }

    fn queue_clip(&mut self, joint: u64, clip: u64) {
        if clip == 0 || self.pending_clip_n >= MAX_STAGES {
            return;
        }
        self.pending_joints[self.pending_clip_n] = joint;
        self.pending_clips[self.pending_clip_n] = clip;
        self.pending_clip_n += 1;
    }

    fn group_travels(&self, i: usize) -> bool {
        if i >= self.group_n {
            return false;
        }
        if self.gate_ready() {
            self.groups[i].travels_hitched
        } else {
            self.groups[i].travels_unhitched
        }
    }

    fn travel_offset(&self) -> [f32; 3] {
        let mut out = [0.; 3];
        for i in 0..3 {
            let a = self.travel_from[i];
            let b = self.travel_to[i];
            out[i] = a + (b - a) * self.pose_travel;
        }
        out
    }

    fn sync_parked(&mut self) {
        let offset = self.travel_offset();
        for i in 0..self.group_n {
            if self.group_travels(i) {
                self.parked[i] = offset;
            }
        }
    }

    fn stage_settled(&self) -> bool {
        (0..self.group_n).all(|i| {
            (self.poses[i] - self.goals[i]).abs() < 0.0005
                && (self.folds[i] - self.fold_goals[i]).abs() < 0.0005
        })
    }

    fn permitted(&self, action: u64) -> bool {
        if self.norm_n == 0 {
            return true;
        }
        let gate = self.gate_ready();
        let unhitched = self.unhitch_ready();
        let mut quins = [compile_norm_quin(0, OP_PERMIT, 0, 0, 0, 0, false); MAX_NORMS];
        let mut n = 0usize;
        for i in 0..self.norm_n {
            if self.norms[i].gate_defeater && !gate {
                continue;
            }
            if self.norms[i].unhitch_defeater && !unhitched {
                continue;
            }
            if self.norms[i].near_defeater && !self.hitch_near() {
                continue;
            }
            quins[n] = self.norms[i].quin;
            n += 1;
        }
        let mut out = [DeonticVerdict::default(); MAX_NORMS];
        let count = evaluate_deontic_contract(&quins[..n], 0, &mut out).unwrap_or(0);
        let mut forbid = false;
        let mut permit = false;
        for v in out[..count].iter() {
            if v.status != DeonticStatus::Active {
                continue;
            }
            if v.norm.object != action {
                continue;
            }
            if v.opcode == OP_FORBID {
                forbid = true;
            }
            if v.opcode == OP_PERMIT {
                permit = true;
            }
        }
        // Defeaters are q42:unless nodes, not Active PERMIT verdicts. Hitch/unhitch
        // travel permits defeat forbid-travel; the travel action is then allowed.
        if forbid {
            false
        } else {
            permit || action == self.travel_action
        }
    }

    fn play_trigger(&mut self, trigger: u64, reduced: bool) -> Result<(), JsValue> {
        let mut assets = [AssetStateNode {
            entity_did: 0,
            class_hash: 0,
            topic_hash: 0,
            position: [0.; 3],
            manifold: ManifoldCoordinate10D::default(),
            active_preset_hash: 0,
            active_subpart_hash: 0,
            remaining_duration_ms: 0,
        }; MAX_GROUPS];
        for i in 0..self.group_n {
            assets[i] = AssetStateNode {
                entity_did: self.groups[i].id_hash,
                class_hash: self.groups[i].id_hash,
                topic_hash: self.topic,
                position: [i as f32 * 4., 0., 0.],
                manifold: ManifoldCoordinate10D::default(),
                active_preset_hash: q_hash(&self.preset),
                active_subpart_hash: 0,
                remaining_duration_ms: 0,
            };
        }
        let mut rules = [PredicateTriggerRule {
            trigger_event_hash: 0,
            target_class_hash: 0,
            target_topic_hash: 0,
            target_subpart_hash: 0,
            preset_hash: 0,
            follow_up_preset_hash: 0,
            temporal_op: TemporalOp::Meets,
            max_radius_m: 80.,
            propagation_speed_mps: 0.,
            duration_ms: 1600,
            blend_in_ms: 80,
            amplitude_u8: 255,
        }; MAX_STAGES];
        let mut rule_n = 0usize;
        for i in 0..self.stage_n {
            if self.stages[i].trigger != trigger {
                continue;
            }
            rules[rule_n] = PredicateTriggerRule {
                trigger_event_hash: trigger,
                target_class_hash: self.groups[self.stages[i].group].id_hash,
                target_topic_hash: self.topic,
                target_subpart_hash: self.groups[self.stages[i].group].id_hash,
                preset_hash: q_hash(&self.preset),
                follow_up_preset_hash: self.stages[i].follow,
                temporal_op: TemporalOp::Meets,
                max_radius_m: 80.,
                propagation_speed_mps: 0.,
                duration_ms: self.stages[i].duration_ms,
                blend_in_ms: 80,
                amplitude_u8: 255,
            };
            rule_n += 1;
        }
        if rule_n == 0 {
            return Ok(());
        }
        let mut commands = [ScheduledPresetCommand {
            entity_did: 0,
            subpart_hash: 0,
            preset_hash: 0,
            follow_up_preset_hash: 0,
            temporal_op: TemporalOp::Meets,
            delay_ms: 0,
            duration_ms: 0,
            blend_in_ms: 0,
            amplitude_u8: 0,
            command_quin: compile_norm_quin(0, OP_PERMIT, 0, 0, 0, 0, false),
        }; MAX_STAGES];
        let fired = evaluate_predicate_triggers_zero_heap(
            &assets[..self.group_n],
            &rules[..rule_n],
            &[],
            trigger,
            [0.; 3],
            self.seq_hash,
            1,
            &mut commands,
        )
        .map_err(|_| err("Logic sequence trigger buffer is full"))?;
        self.follow_event = 0;
        for cmd in commands[..fired].iter() {
            if let Some(g) = (0..self.group_n).find(|&i| self.groups[i].id_hash == cmd.entity_did) {
                let stage = self
                    .stages
                    .iter()
                    .take(self.stage_n)
                    .find(|s| s.trigger == trigger && s.group == g);
                let goal = stage.map(|s| s.goal).unwrap_or(1.);
                let hitch_pose = g == self.gate_group && goal >= 0.99;
                if hitch_pose && !self.hitch_near() {
                    continue;
                }
                if stage.map(|s| s.fold).unwrap_or(false) {
                    self.fold_goals[g] = goal;
                    self.fold_clocks[g] = 0.;
                    if reduced {
                        self.folds[g] = goal;
                    }
                } else {
                    self.goals[g] = goal;
                    self.clocks[g] = 0.;
                    if reduced {
                        self.poses[g] = goal;
                    }
                }
                if let Some(clip) = stage.map(|s| s.clip) {
                    self.queue_clip(self.groups[g].id_hash, clip);
                }
                if cmd.follow_up_preset_hash != 0 {
                    self.follow_event = cmd.follow_up_preset_hash;
                }
            }
        }
        self.dirty = true;
        Ok(())
    }
}

fn advance_channel(
    pose: &mut f32,
    goal: f32,
    clock: &mut f32,
    dt_s: f32,
    family: AnimationFamily,
    preset: &str,
    reduced: bool,
) -> bool {
    if (*pose - goal).abs() < 0.0005 {
        *pose = goal;
        return false;
    }
    *clock += dt_s;
    let sample = evaluate_preset(family, preset, *clock as f64);
    let t = if reduced || sample.settled {
        1.
    } else {
        sample.scalar.clamp(0., 1.) as f32
    };
    let start = if goal > 0.5 { 0. } else { 1. };
    *pose = start + (goal - start) * t;
    if sample.settled || reduced {
        *pose = goal;
    }
    true
}

fn find_sequence(index: &QuinIndex) -> Result<String, String> {
    if let Ok(seq) = index.subject_of_schema(LOGIC_SCHEMA) {
        return Ok(seq);
    }
    index.subject_of_schema(CAMP_SCHEMA)
}

fn eval_rig(
    host: &mut qualia_core_db::vibe_host::PoetSnapshot,
    source: &str,
    name: &str,
) -> Result<Value, String> {
    if name.is_empty() {
        return Ok(json!({}));
    }
    crate::policy::evaluate(host, source, name, &[]).or_else(|_| Ok(json!({})))
}

fn rig_group<'a>(rig: &'a Value, id: &str) -> Option<&'a Value> {
    rig.get("groups")
        .and_then(Value::as_array)
        .and_then(|g| g.iter().find(|v| v["id"].as_str() == Some(id)))
}

fn parked_within_radius(gate: Option<[f32; 3]>, anchor: Option<[f32; 3]>, radius_m: f32) -> bool {
    let Some(gate) = gate else {
        return true;
    };
    let Some(anchor) = anchor else {
        return true;
    };
    let radius = if radius_m.is_finite() && radius_m > 0. {
        radius_m
    } else {
        DEFAULT_HITCH_RADIUS_M
    };
    let dx = gate[0] - anchor[0];
    let dz = gate[2] - anchor[2];
    (dx * dx + dz * dz).sqrt() <= radius
}

fn steer_bit(key: &str) -> u8 {
    match key {
        "ArrowUp" | "w" | "W" => KEY_N,
        "ArrowDown" | "s" | "S" => KEY_S,
        "ArrowLeft" | "a" | "A" => KEY_W,
        "ArrowRight" | "d" | "D" => KEY_E,
        _ => 0,
    }
}

fn steer_axes(keys: u8) -> (f32, f32) {
    let mut dx = 0f32;
    let mut dz = 0f32;
    if keys & KEY_E != 0 {
        dx += 1.;
    }
    if keys & KEY_W != 0 {
        dx -= 1.;
    }
    if keys & KEY_S != 0 {
        dz += 1.;
    }
    if keys & KEY_N != 0 {
        dz -= 1.;
    }
    (dx, dz)
}

fn vec3(v: Option<&Value>) -> Option<[f32; 3]> {
    let a = v?.as_array()?;
    if a.len() != 3 {
        return None;
    }
    Some([
        a[0].as_f64()? as f32,
        a[1].as_f64()? as f32,
        a[2].as_f64()? as f32,
    ])
}

pub fn fire(w: &mut super::World, action_id: &str) -> Result<(), JsValue> {
    let reduced = w.reduced;
    let message = {
        let live = w
            .macros
            .as_mut()
            .ok_or(err("Logic macros are not admitted"))?;
        live.fire(action_id, reduced)?
    };
    rebind(w)?;
    super::text("native-world-status", &message);
    w.receipt["camp"] = w
        .macros
        .as_ref()
        .map(Live::receipt)
        .unwrap_or(Value::Null);
    Ok(())
}

pub fn travel_with_place(w: &mut super::World, x: f32, z: f32) -> Result<(), JsValue> {
    let reduced = w.reduced;
    let moved = match w.macros.as_mut() {
        Some(live) => live.travel_to(x, z, reduced)?,
        None => false,
    };
    if moved {
        rebind(w)?;
        if let Some(live) = w.macros.as_ref() {
            super::text("native-world-status", live.travel_status());
            w.receipt["camp"] = live.receipt();
        }
    }
    Ok(())
}

pub fn tick(w: &mut super::World, dt_ms: f32) -> Result<(), JsValue> {
    let reduced = w.reduced;
    let dirty = w
        .macros
        .as_mut()
        .map(|live| live.tick(dt_ms, reduced))
        .unwrap_or(false);
    if dirty {
        rebind(w)?;
        if let Some(live) = w.macros.as_ref() {
            w.receipt["camp"] = live.receipt();
        }
    }
    Ok(())
}

#[allow(dead_code)]
#[derive(Clone, Copy, Debug, PartialEq)]
pub struct FloraWindConfig {
    pub natural_freq_hz: f32,
    pub crown_height_m: f32,
    pub wind_stress: f32,
}

#[allow(dead_code)]
#[derive(Clone, Debug, PartialEq)]
pub struct FaunaLoopActor {
    pub id: String,
    pub locomotion_class: LocomotionClass,
    pub seed: u64,
    pub site_vigor: f32,
    pub base_translation: [f64; 3],
}

#[allow(dead_code)]
#[derive(Clone, Debug, PartialEq)]
pub struct FaunaLoopStep {
    pub id: String,
    pub state: FaunaBehaviorState,
    pub translation: [f64; 3],
    pub yaw_offset_rad: f64,
    pub pitch_offset_rad: f64,
    pub should_vocalize: bool,
}

/// Generic flora wind-sway offset helper driven by seed, world position, and natural sway frequency.
#[allow(dead_code)]
pub fn flora_wind_sway_offset(
    time_s: f64,
    seed: u64,
    world_xz: [f32; 2],
    cfg: &FloraWindConfig,
    reduced_motion: bool,
) -> [f64; 3] {
    evaluate_flora_wind_sway(
        time_s,
        seed,
        world_xz,
        cfg.natural_freq_hz,
        cfg.crown_height_m,
        cfg.wind_stress,
        reduced_motion,
    )
}

/// Generic data-driven fauna behavioral loop over placed actors (zero per-species branching).
#[allow(dead_code)]
pub fn step_fauna_behavioral_loop(
    actors: &[FaunaLoopActor],
    time_s: f64,
    reduced_motion: bool,
) -> Vec<FaunaLoopStep> {
    actors
        .iter()
        .map(|actor| {
            let sample = evaluate_fauna_behavior(
                actor.locomotion_class,
                actor.seed,
                time_s,
                actor.site_vigor,
                reduced_motion,
            );
            FaunaLoopStep {
                id: actor.id.clone(),
                state: sample.state,
                translation: [
                    actor.base_translation[0] + sample.translation_offset[0],
                    actor.base_translation[1] + sample.translation_offset[1],
                    actor.base_translation[2] + sample.translation_offset[2],
                ],
                yaw_offset_rad: sample.yaw_offset_rad,
                pitch_offset_rad: sample.pitch_offset_rad,
                should_vocalize: sample.should_vocalize,
            }
        })
        .collect()
}

fn rebind(w: &mut super::World) -> Result<(), JsValue> {
    let upload = w.upload.clone();
    if upload.is_null() {
        return Ok(());
    }
    let scene = w.scene.clone();
    let overlay = w.macros.clone();
    let receipt = super::instances::bind_resident(&mut w.portal, &scene, &upload, overlay.as_ref())?;
    w.receipt["instances"] = receipt;
    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;

    fn test_live() -> Live {
        let family = AnimationFamily::from_name("physical_dynamics").expect("family");
        let mut live = Live {
            groups: [Group {
                id_hash: 0,
                delta: [0.; 3],
                origin: [0.; 3],
                scale: [1.; 3],
                travels_hitched: true,
                travels_unhitched: false,
            }; MAX_GROUPS],
            group_ids: std::array::from_fn(|_| String::new()),
            prefixes: std::array::from_fn(|_| String::new()),
            group_labels: std::array::from_fn(|_| String::new()),
            group_n: 3,
            stages: [Stage {
                trigger: 0,
                group: 0,
                follow: 0,
                duration_ms: 1,
                goal: 1.,
                clip: 0,
                fold: false,
            }; MAX_STAGES],
            stage_n: 3,
            norms: [Norm {
                quin: compile_norm_quin(0, OP_PERMIT, 0, 0, 0, 0, false),
                gate_defeater: false,
                unhitch_defeater: false,
                near_defeater: false,
            }; MAX_NORMS],
            norm_n: 0,
            actions: vec![
                Action {
                    id: "hitch".into(),
                    label: "Hitch".into(),
                    uri: "urn:webizen:sequence:hitch".into(),
                    trigger: "event:hitch".into(),
                    message: "Hitching.".into(),
                    about: vec!["gate".into(), "standing".into()],
                },
                Action {
                    id: "unhitch".into(),
                    label: "Unhitch".into(),
                    uri: "urn:webizen:sequence:unhitch".into(),
                    trigger: "event:unhitch".into(),
                    message: "Unhitching.".into(),
                    about: vec!["gate".into(), "walker".into()],
                },
                Action {
                    id: "walk".into(),
                    label: "Walk".into(),
                    uri: "urn:webizen:sequence:walk".into(),
                    trigger: "event:walk".into(),
                    message: "Walking.".into(),
                    about: vec!["walker".into()],
                },
            ],
            poses: [0.; MAX_GROUPS],
            goals: [0.; MAX_GROUPS],
            clocks: [0.; MAX_GROUPS],
            folds: [0.; MAX_GROUPS],
            fold_goals: [0.; MAX_GROUPS],
            fold_clocks: [0.; MAX_GROUPS],
            travel_from: [0.; 3],
            travel_to: [0.; 3],
            parked: [[0.; 3]; MAX_GROUPS],
            pose_travel: 1.,
            clock_travel: 0.,
            origin: [0.; 3],
            family,
            preset: "spring_settle".into(),
            party: 0,
            contract: 0,
            seq_hash: 1,
            topic: 1,
            travel_action: q_hash("urn:webizen:sequence:travel"),
            gate_group: 1,
            hitch_anchor: 0,
            follow_event: 0,
            panel_label: "Sequence".into(),
            forbid_message: "Not permitted.".into(),
            travel_message: "Travelling.".into(),
            travel_message_hitched: "Hitched travel.".into(),
            travel_message_unhitched: "Unhitched travel.".into(),
            hitch_radius_m: DEFAULT_HITCH_RADIUS_M,
            steer_mps: DEFAULT_STEER_MPS,
            keys: 0,
            pending_clips: [0; MAX_STAGES],
            pending_joints: [0; MAX_STAGES],
            pending_clip_n: 0,
            biosphere_clock_s: 0.0,
            dirty: true,
        };
        live.groups[0] = Group {
            id_hash: q_hash("group:standing"),
            delta: [0.; 3],
            origin: [0.; 3],
            scale: [1.; 3],
            travels_hitched: true,
            travels_unhitched: false,
        };
        live.groups[1] = Group {
            id_hash: q_hash("group:gate"),
            delta: [2., 0., 0.],
            origin: [0.; 3],
            scale: [1.; 3],
            travels_hitched: true,
            travels_unhitched: true,
        };
        live.groups[2] = Group {
            id_hash: q_hash("group:walker"),
            delta: [0.; 3],
            origin: [0.; 3],
            scale: [1.; 3],
            travels_hitched: true,
            travels_unhitched: true,
        };
        live.group_ids[0] = "standing".into();
        live.group_ids[1] = "gate".into();
        live.group_ids[2] = "walker".into();
        live.group_labels[0] = "Standing camp".into();
        live.group_labels[1] = "Gate".into();
        live.group_labels[2] = "Walker".into();
        live.prefixes[0] = "standing-".into();
        live.prefixes[1] = "gate-".into();
        live.prefixes[2] = "walk-".into();
        live.stages[0] = Stage {
            trigger: q_hash("event:hitch"),
            group: 1,
            follow: 0,
            duration_ms: 1,
            goal: 1.,
            clip: 0,
            fold: false,
        };
        live.stages[1] = Stage {
            trigger: q_hash("event:unhitch"),
            group: 1,
            follow: 0,
            duration_ms: 1,
            goal: 0.,
            clip: 0,
            fold: false,
        };
        live.stages[2] = Stage {
            trigger: q_hash("event:walk"),
            group: 2,
            follow: 0,
            duration_ms: 1,
            goal: 0.,
            clip: q_hash("walk_to_panels"),
            fold: false,
        };
        live
    }

    #[test]
    fn hitch_radius_default_is_four_metres() {
        assert!((DEFAULT_HITCH_RADIUS_M - 4.0).abs() < f32::EPSILON);
        assert!(parked_within_radius(
            Some([0., 0., 0.]),
            Some([3., 0., 0.]),
            DEFAULT_HITCH_RADIUS_M
        ));
        assert!(!parked_within_radius(
            Some([0., 0., 0.]),
            Some([5., 0., 0.]),
            DEFAULT_HITCH_RADIUS_M
        ));
    }

    #[test]
    fn unhitched_steer_skips_standing_groups() {
        let mut live = test_live();
        assert!(live.fire("unhitch", true).is_ok());
        let standing_before = live.overlay("standing-body", true);
        let gate_before = live.overlay("gate-body", true);
        assert!(live.steer_keydown("ArrowRight"));
        assert!(live.tick_steer(1000., true));
        live.tick(0., true);
        let standing_after = live.overlay("standing-body", true);
        let gate_after = live.overlay("gate-body", true);
        assert_eq!(standing_before, standing_after);
        assert!((gate_after[0] - gate_before[0]).abs() > 1.0);
        assert!(!live.hitch_near());
    }

    #[test]
    fn hitched_named_place_moves_travels_hitched_groups() {
        let mut live = test_live();
        assert!(live.fire("hitch", true).is_ok());
        assert!(!live.steer_keydown("ArrowRight"));
        assert!(!live.tick_steer(1000., true));
        assert!(live.travel_to(20., 8., true).ok() == Some(true));
        live.tick(0., true);
        let standing = live.overlay("standing-body", true);
        let gate = live.overlay("gate-body", true);
        assert!((standing[0] - 20.).abs() < 0.01);
        assert!((standing[2] - 8.).abs() < 0.01);
        assert!((gate[0] - 22.).abs() < 0.01);
        assert_eq!(live.travel_status(), "Hitched travel.");
    }

    #[test]
    fn hitch_when_far_is_refused_without_special_case_ids() {
        let mut live = test_live();
        assert!(live.fire("unhitch", true).is_ok());
        live.steer_keydown("d");
        live.tick_steer(4000., true);
        live.tick(0., true);
        assert!(!live.hitch_near());
        assert!(live.fire("hitch", true).is_err());
        assert!(live.poses[1] <= 0.01);
        live.keys = 0;
        live.steer_keydown("a");
        live.tick_steer(4000., true);
        live.tick(0., true);
        assert!(live.hitch_near());
        assert!(live.fire("hitch", true).is_ok());
        assert!(live.poses[1] >= 0.99);
    }

    #[test]
    fn walk_stage_queues_clip_and_still_plays_translation() {
        let mut live = test_live();
        assert!(live.fire("walk", true).is_ok());
        let mut clips = [(0u64, 0u64); MAX_STAGES];
        let n = live.take_pending_clips(&mut clips);
        assert_eq!(n, 1);
        assert_eq!(clips[0].0, q_hash("group:walker"));
        assert_eq!(clips[0].1, q_hash("walk_to_panels"));
        assert!((live.goals[2] - 0.).abs() < f32::EPSILON);
        let n2 = live.take_pending_clips(&mut clips);
        assert_eq!(n2, 0);
    }

    #[test]
    fn travel_copy_is_authored_not_hardcoded() {
        let mut live = test_live();
        assert_eq!(live.travel_status(), "Unhitched travel.");
        assert!(live.fire("hitch", true).is_ok());
        assert_eq!(live.travel_status(), "Hitched travel.");
    }

    #[test]
    fn source_has_no_special_case_car_id() {
        let src = include_str!("macros.rs");
        let car = format!("if id == \"{}\"", "car");
        let chassis = format!("if id == \"{}\"", "chassis");
        assert!(!src.contains(&car));
        assert!(!src.contains(&chassis));
    }

    #[test]
    fn actions_about_group_lists_authored_affordances() {
        let live = test_live();
        let gate: Vec<&str> = live
            .permitted_actions_about("gate")
            .iter()
            .map(|a| a.id.as_str())
            .collect();
        assert!(gate.contains(&"hitch"));
        assert!(gate.contains(&"unhitch"));
        assert!(!gate.contains(&"walk"));
        let walker: Vec<&str> = live
            .permitted_actions_about("walker")
            .iter()
            .map(|a| a.id.as_str())
            .collect();
        assert!(walker.contains(&"walk"));
        assert!(walker.contains(&"unhitch"));
        assert!(!walker.contains(&"hitch"));
        assert_eq!(live.group_id_for_asset("gate-body"), Some("gate"));
        assert_eq!(live.group_label_for("gate"), Some("Gate"));
    }

    #[test]
    fn hitched_play_steer_moves_travelling_groups() {
        let mut live = test_live();
        assert!(live.fire("hitch", true).is_ok());
        assert!(!live.steer_keydown("ArrowRight"));
        live.keys = 0;
        let standing_before = live.overlay("standing-body", true);
        let gate_before = live.overlay("gate-body", true);
        assert!(live.steer_keydown_mode("d", true));
        assert!(live.tick_steer_mode(1000., true, true));
        live.tick(0., true);
        let standing_after = live.overlay("standing-body", true);
        let gate_after = live.overlay("gate-body", true);
        assert!((standing_after[0] - standing_before[0]).abs() > 1.0);
        assert!((gate_after[0] - gate_before[0]).abs() > 1.0);
        let focus = live.focus_world();
        assert!((focus[0] - standing_after[0]).abs() < 0.01);
    }

    #[test]
    fn already_settled_follow_still_plays_hitch() {
        let mut live = test_live();
        live.stages[2].follow = q_hash("event:hitch");
        assert!(live.fire("walk", true).is_ok());
        assert!(live.poses[1] <= 0.01);
        live.tick(16., true);
        assert!(
            live.poses[1] >= 0.99,
            "walk-to-hitch at goal 0 must still follow into hitch"
        );
        assert!(live.gate_ready());
    }

    #[test]
    fn fold_channel_scales_before_stow_translation() {
        let mut live = test_live();
        live.groups[0].scale = [0.4, 1., 1.];
        live.groups[0].delta = [0., 0.4, -4.];
        live.prefixes[0] = "solar-array-three".into();
        live.stages[0] = Stage {
            trigger: q_hash("event:fold_panels"),
            group: 0,
            follow: q_hash("event:stow_panels"),
            duration_ms: 1,
            goal: 1.,
            clip: 0,
            fold: true,
        };
        live.stages[1] = Stage {
            trigger: q_hash("event:stow_panels"),
            group: 0,
            follow: 0,
            duration_ms: 1,
            goal: 1.,
            clip: 0,
            fold: false,
        };
        live.stage_n = 2;
        assert!(live.play_trigger(q_hash("event:fold_panels"), true).is_ok());
        let folded = live.overlay_scale("solar-array-three--panel");
        let stow = live.overlay("solar-array-three--panel", true);
        assert!((folded[0] - 0.4).abs() < 0.01);
        assert!((folded[1] - 1.).abs() < 0.01);
        assert!(stow[2].abs() < 0.01, "fold must not translate yet");
        live.tick(16., true);
        let packed = live.overlay("solar-array-three--panel", true);
        assert!((packed[2] + 4.).abs() < 0.01);
        assert!((live.overlay_scale("solar-array-three--panel")[0] - 0.4).abs() < 0.01);
    }

    #[test]
    fn generic_fauna_loop_and_flora_wind_sway_advance_deterministically() {
        let mut live = test_live();
        live.tick(500., false);
        assert!(live.biosphere_clock_s > 0.49);
        let actors = vec![
            FaunaLoopActor {
                id: "inst-1".into(),
                locomotion_class: LocomotionClass::Saltatorial,
                seed: 101,
                site_vigor: 1.0,
                base_translation: [10.0, 4.0, -12.0],
            },
            FaunaLoopActor {
                id: "inst-2".into(),
                locomotion_class: LocomotionClass::Avian,
                seed: 202,
                site_vigor: 1.1,
                base_translation: [18.0, 9.5, -6.0],
            },
        ];
        let steps = live.step_biosphere_fauna(&actors, false);
        assert_eq!(steps.len(), 2);
        let wind = live.sample_flora_wind(
            42,
            [12.0, -8.0],
            &FloraWindConfig {
                natural_freq_hz: 0.66,
                crown_height_m: 20.0,
                wind_stress: 0.25,
            },
            false,
        );
        assert!(wind.iter().all(|v| v.is_finite()));
        let reduced_wind = live.sample_flora_wind(
            42,
            [12.0, -8.0],
            &FloraWindConfig {
                natural_freq_hz: 0.66,
                crown_height_m: 20.0,
                wind_stress: 0.25,
            },
            true,
        );
        assert_eq!(reduced_wind, [0.0, 0.0, 0.0]);
    }
}
