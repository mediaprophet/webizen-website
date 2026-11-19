//! Living Biosphere Species Instrument:
//! - Generic, data-driven fauna behavioral state machine (`Grazing`, `AlertTurn`,
//!   `Foraging`, `Perching`, `Vocalizing`) driven by deterministic time/seed cycles
//!   and locomotion class (`saltatorial`, `arboreal`, `fossorial`, `semi_aquatic`, `avian`).
//! - Generic flora wind sway kinematics driven by seed, world position, and natural sway frequency.
//! - Opt-in spatial `.qac` vocalization trigger with inverse-distance attenuation
//!   and strict user-gesture opt-in compliance (no forced audio).
//! - Rhizosphere & Mycelium X-Ray Lens controller (`RhizosphereLensState`) activating
//!   subterranean `.10d` layers (`rhizosphere` + `aboveground_lod1`) and formatting `.emf`
//!   bio-electric / chemical exudate signaling readouts (`0.1-50 Hz`, hydraulic lift, glomalin/C/P/N).
//! - Q42 SPARQL Species Inspection Card builder (`render_species_card_html`).

#![allow(dead_code)]

use std::fmt::Write as _;
use std::sync::atomic::{AtomicBool, Ordering};

#[cfg(target_arch = "wasm32")]
use super::*;
#[cfg(target_arch = "wasm32")]
use super::super::knowledge::{self, QuinIndex};
#[cfg(target_arch = "wasm32")]
use super::super::instances::InstanceState64;

/// 64-byte deterministic phenotype descriptor when running pure-Rust host unit tests.
#[cfg(not(target_arch = "wasm32"))]
#[repr(C)]
#[derive(Clone, Copy, Debug, PartialEq)]
pub struct InstanceState64 {
    pub seed: u64,
    pub ontogenetic_age_norm: f32,
    pub height_scale: f32,
    pub site_vigor: f32,
    pub environmental_stress: [f32; 4],
    pub _reserved: [u32; 7],
}

#[cfg(not(target_arch = "wasm32"))]
const _: () = assert!(std::mem::size_of::<InstanceState64>() == 64);

#[cfg(not(target_arch = "wasm32"))]
impl InstanceState64 {
    pub const fn new(
        seed: u64,
        ontogenetic_age_norm: f32,
        height_scale: f32,
        site_vigor: f32,
        environmental_stress: [f32; 4],
    ) -> Self {
        Self {
            seed,
            ontogenetic_age_norm,
            height_scale,
            site_vigor,
            environmental_stress,
            _reserved: [0u32; 7],
        }
    }
}

/// Global user-gesture audio opt-in gate (default `false` — never autoplays audio).
static AUDIO_OPT_IN_GRANTED: AtomicBool = AtomicBool::new(false);

pub fn set_audio_opt_in(granted: bool) {
    AUDIO_OPT_IN_GRANTED.store(granted, Ordering::SeqCst);
}

pub fn is_audio_opt_in_granted() -> bool {
    AUDIO_OPT_IN_GRANTED.load(Ordering::SeqCst)
}

/// Generic species catalog record hydrated from admitted `.q42` volumes (`species-catalog.q42`
/// and `se-australia-catchment.q42`).
#[derive(Clone, Debug, PartialEq)]
pub struct SpeciesCatalogEntry {
    pub taxon_iri: String,
    pub slug: String,
    pub scientific_name: String,
    pub vernacular_name: String,
    pub kingdom: String,
    pub ecological_zone: String,
    pub catchment_zone: String,
    pub locomotion_class: String,
    pub custodial_country: String,
    pub mycorrhizal_type: String,
    pub rhizosphere_signaling: String,
    pub natural_sway_freq_hz: f32,
    pub url: String,
    pub remote_url: String,
    pub sha256: String,
    pub taxon_q42_url: String,
    pub field_emf_url: String,
    pub substrate_m64_url: Option<String>,
    pub call_qac_url: Option<String>,
    pub call_qac_remote_url: Option<String>,
    pub call_name: String,
    pub call_fundamental_hz: f32,
    pub aboveground_lod1_url: Option<String>,
    pub rhizosphere_url: Option<String>,
    pub lod0_url: Option<String>,
    pub intenteco_repo_url: String,
    pub hmc_package_url: String,
}

#[cfg(target_arch = "wasm32")]
impl SpeciesCatalogEntry {
    /// Enrich an admitted `catalog::SpeciesCatalogEntry` with Q42 ecological traits.
    pub fn from_catalog_entry(
        entry: &super::super::catalog::SpeciesCatalogEntry,
        catchment_zone: &str,
        locomotion_class: &str,
        custodial_country: &str,
        call_qac_url: Option<&str>,
    ) -> Self {
        Self {
            taxon_iri: entry.taxon_iri.clone(),
            slug: entry.slug.clone(),
            scientific_name: entry.scientific_name.clone(),
            vernacular_name: entry.vernacular_name.clone(),
            kingdom: entry.kingdom.clone(),
            ecological_zone: entry.ecological_zone.clone(),
            catchment_zone: catchment_zone.to_string(),
            locomotion_class: locomotion_class.to_string(),
            custodial_country: custodial_country.to_string(),
            mycorrhizal_type: String::new(),
            rhizosphere_signaling: String::new(),
            natural_sway_freq_hz: 0.65,
            url: entry.url.clone(),
            remote_url: entry.remote_url.clone(),
            sha256: entry.sha256.clone(),
            taxon_q42_url: entry
                .sidecar_taxon_q42
                .as_ref()
                .map(|(u, _)| u.clone())
                .unwrap_or_else(|| format!("/intenteco/{}/dist/taxon.q42", entry.slug)),
            field_emf_url: entry
                .sidecar_field_emf
                .as_ref()
                .map(|(u, _)| u.clone())
                .unwrap_or_else(|| format!("/intenteco/{}/dist/{}-field.emf", entry.slug, entry.slug)),
            substrate_m64_url: entry.sidecar_substrate_m64.as_ref().map(|(u, _)| u.clone()),
            call_qac_url: call_qac_url.map(str::to_string),
            call_qac_remote_url: call_qac_url.map(|_| {
                format!(
                    "https://raw.githubusercontent.com/intenteco/{}/main/dist/{}-call.qac",
                    entry.slug, entry.slug
                )
            }),
            call_name: String::new(),
            call_fundamental_hz: 0.0,
            aboveground_lod1_url: entry.aboveground_lod1.as_ref().map(|l| l.url.clone()),
            rhizosphere_url: entry.rhizosphere.as_ref().map(|l| l.url.clone()),
            lod0_url: entry.lod0.as_ref().map(|l| l.url.clone()),
            intenteco_repo_url: format!("https://github.com/intenteco/{}", entry.slug),
            hmc_package_url: format!("https://github.com/intenteco/{}/tree/main/dist", entry.slug),
        }
    }

    /// Hydrate a `SpeciesCatalogEntry` directly from the admitted Q42 `QuinIndex`.
    pub fn from_quin_index(index: &QuinIndex, taxon_iri: &str) -> Option<Self> {
        let slug = index.text(taxon_iri, "slug").ok()?;
        let scientific_name = index
            .many(taxon_iri, "http://rs.tdwg.org/dwc/terms/scientificName")
            .into_iter()
            .next()
            .map(|v| strip_literal(&v))
            .unwrap_or_else(|| slug.clone());
        let vernacular_name = index
            .many(taxon_iri, "http://rs.tdwg.org/dwc/terms/vernacularName")
            .into_iter()
            .next()
            .map(|v| strip_literal(&v))
            .unwrap_or_else(|| slug.clone());
        let custodial_country = index
            .text(taxon_iri, "custodialCountry")
            .ok()
            .or_else(|| {
                index
                    .many(taxon_iri, "https://localcontexts.org/ontology/tk-bc.ttl#custodial_country")
                    .into_iter()
                    .next()
                    .map(|v| strip_literal(&v))
            })
            .unwrap_or_else(|| "SE Australian Country (custodial attribution in taxon.q42)".into());
        let intenteco_repo_url = index
            .text(taxon_iri, "intentecoRepo")
            .unwrap_or_else(|_| format!("https://github.com/intenteco/{slug}"));
        Some(Self {
            taxon_iri: taxon_iri.to_string(),
            slug: slug.clone(),
            scientific_name,
            vernacular_name,
            kingdom: index.text(taxon_iri, "kingdom").unwrap_or_default(),
            ecological_zone: index.text(taxon_iri, "ecologicalZone").unwrap_or_default(),
            catchment_zone: index.text(taxon_iri, "catchmentZone").unwrap_or_default(),
            locomotion_class: index
                .text(taxon_iri, "locomotionClass")
                .unwrap_or_else(|_| "sessile_flora".into()),
            custodial_country,
            mycorrhizal_type: index.text(taxon_iri, "mycorrhizalType").unwrap_or_default(),
            rhizosphere_signaling: index
                .text(taxon_iri, "rhizosphereSignaling")
                .unwrap_or_default(),
            natural_sway_freq_hz: index.number(taxon_iri, "naturalSwayFreqHz").unwrap_or(0.65) as f32,
            url: index
                .text(taxon_iri, "canonicalUrl")
                .unwrap_or_else(|_| format!("/intenteco/{slug}/dist/{slug}.10d")),
            remote_url: index
                .text(taxon_iri, "canonicalRemoteUrl")
                .unwrap_or_else(|_| {
                    format!("https://raw.githubusercontent.com/intenteco/{slug}/main/dist/{slug}.10d")
                }),
            sha256: index.text(taxon_iri, "canonicalSha256").unwrap_or_default(),
            taxon_q42_url: index
                .text(taxon_iri, "sidecarTaxonQ42")
                .unwrap_or_else(|_| format!("/intenteco/{slug}/dist/taxon.q42")),
            field_emf_url: index
                .text(taxon_iri, "sidecarFieldEmf")
                .unwrap_or_else(|_| format!("/intenteco/{slug}/dist/{slug}-field.emf")),
            substrate_m64_url: index.text(taxon_iri, "sidecarSubstrateM64").ok(),
            call_qac_url: index.text(taxon_iri, "sidecarCallQac").ok(),
            call_qac_remote_url: index.text(taxon_iri, "sidecarCallQacRemote").ok(),
            call_name: index.text(taxon_iri, "callName").unwrap_or_default(),
            call_fundamental_hz: index.number(taxon_iri, "callFundamentalHz").unwrap_or(0.0) as f32,
            aboveground_lod1_url: index.text(taxon_iri, "abovegroundLod1Url").ok(),
            rhizosphere_url: index.text(taxon_iri, "rhizosphereUrl").ok(),
            lod0_url: index.text(taxon_iri, "lod0Url").ok(),
            hmc_package_url: format!("{intenteco_repo_url}/tree/main/dist"),
            intenteco_repo_url,
        })
    }
}

#[cfg(target_arch = "wasm32")]
fn strip_literal(raw: &str) -> String {
    let t = raw.trim();
    let without_lang = if let Some(idx) = t.rfind("\"@") {
        &t[..idx + 1]
    } else if let Some(idx) = t.rfind("\"^^") {
        &t[..idx + 1]
    } else {
        t
    };
    if without_lang.starts_with('"') {
        if let Ok(s) = serde_json::from_str::<String>(without_lang) {
            return s;
        }
        if without_lang.len() >= 2 && without_lang.ends_with('"') {
            return without_lang[1..without_lang.len() - 1].to_string();
        }
    }
    without_lang.to_string()
}

// ---------------------------------------------------------------------------
// 1. Generic, Data-Driven Fauna Behavioral State Machine & Flora Wind Sway
// ---------------------------------------------------------------------------

#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash)]
pub enum FaunaBehaviorState {
    Grazing,
    AlertTurn,
    Foraging,
    Perching,
    Vocalizing,
}

impl FaunaBehaviorState {
    pub const fn as_str(self) -> &'static str {
        match self {
            Self::Grazing => "Grazing",
            Self::AlertTurn => "AlertTurn",
            Self::Foraging => "Foraging",
            Self::Perching => "Perching",
            Self::Vocalizing => "Vocalizing",
        }
    }
}

#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash)]
pub enum LocomotionClass {
    Saltatorial,
    Arboreal,
    Fossorial,
    SemiAquatic,
    Avian,
}

impl LocomotionClass {
    pub fn from_str(raw: &str) -> Option<Self> {
        match raw.trim().to_ascii_lowercase().as_str() {
            "saltatorial" => Some(Self::Saltatorial),
            "arboreal" => Some(Self::Arboreal),
            "fossorial" => Some(Self::Fossorial),
            "semi_aquatic" | "semi-aquatic" | "semiaquatic" => Some(Self::SemiAquatic),
            "avian" => Some(Self::Avian),
            _ => None,
        }
    }

    pub const fn as_str(self) -> &'static str {
        match self {
            Self::Saltatorial => "saltatorial",
            Self::Arboreal => "arboreal",
            Self::Fossorial => "fossorial",
            Self::SemiAquatic => "semi_aquatic",
            Self::Avian => "avian",
        }
    }

    pub const fn label(self) -> &'static str {
        match self {
            Self::Saltatorial => "Saltatorial (macropod hop / grazing stride)",
            Self::Arboreal => "Arboreal (canopy fork cling / leaf browse)",
            Self::Fossorial => "Fossorial (burrow apron / low-centre forage)",
            Self::SemiAquatic => "Semi-aquatic (riparian bank / electro-forage swim)",
            Self::Avian => "Avian (woodland limb perch / territorial chorus)",
        }
    }
}

#[derive(Clone, Copy, Debug)]
struct BehaviorPhaseSpec {
    state: FaunaBehaviorState,
    duration_s: f64,
    stride_amp_m: f64,
    bob_amp_m: f64,
    yaw_sweep_rad: f64,
    pitch_amp_rad: f64,
    cadence_hz: f64,
}

const SALTATORIAL_CYCLE: &[BehaviorPhaseSpec] = &[
    BehaviorPhaseSpec {
        state: FaunaBehaviorState::Grazing,
        duration_s: 7.0,
        stride_amp_m: 0.25,
        bob_amp_m: 0.04,
        yaw_sweep_rad: 0.18,
        pitch_amp_rad: 0.22,
        cadence_hz: 0.45,
    },
    BehaviorPhaseSpec {
        state: FaunaBehaviorState::AlertTurn,
        duration_s: 3.0,
        stride_amp_m: 0.0,
        bob_amp_m: 0.06,
        yaw_sweep_rad: 0.85,
        pitch_amp_rad: -0.14,
        cadence_hz: 0.55,
    },
    BehaviorPhaseSpec {
        state: FaunaBehaviorState::Foraging,
        duration_s: 5.5,
        stride_amp_m: 1.40,
        bob_amp_m: 0.28,
        yaw_sweep_rad: 0.24,
        pitch_amp_rad: 0.10,
        cadence_hz: 1.35,
    },
    BehaviorPhaseSpec {
        state: FaunaBehaviorState::Vocalizing,
        duration_s: 2.5,
        stride_amp_m: 0.05,
        bob_amp_m: 0.03,
        yaw_sweep_rad: 0.30,
        pitch_amp_rad: -0.08,
        cadence_hz: 0.80,
    },
];

const ARBOREAL_CYCLE: &[BehaviorPhaseSpec] = &[
    BehaviorPhaseSpec {
        state: FaunaBehaviorState::Perching,
        duration_s: 8.0,
        stride_amp_m: 0.02,
        bob_amp_m: 0.02,
        yaw_sweep_rad: 0.08,
        pitch_amp_rad: 0.04,
        cadence_hz: 0.20,
    },
    BehaviorPhaseSpec {
        state: FaunaBehaviorState::Foraging,
        duration_s: 5.0,
        stride_amp_m: 0.16,
        bob_amp_m: 0.10,
        yaw_sweep_rad: 0.28,
        pitch_amp_rad: 0.12,
        cadence_hz: 0.40,
    },
    BehaviorPhaseSpec {
        state: FaunaBehaviorState::AlertTurn,
        duration_s: 3.0,
        stride_amp_m: 0.0,
        bob_amp_m: 0.02,
        yaw_sweep_rad: 0.65,
        pitch_amp_rad: -0.08,
        cadence_hz: 0.45,
    },
    BehaviorPhaseSpec {
        state: FaunaBehaviorState::Vocalizing,
        duration_s: 4.0,
        stride_amp_m: 0.0,
        bob_amp_m: 0.04,
        yaw_sweep_rad: 0.18,
        pitch_amp_rad: -0.18,
        cadence_hz: 0.90,
    },
];

const FOSSORIAL_CYCLE: &[BehaviorPhaseSpec] = &[
    BehaviorPhaseSpec {
        state: FaunaBehaviorState::Foraging,
        duration_s: 7.5,
        stride_amp_m: 0.65,
        bob_amp_m: 0.03,
        yaw_sweep_rad: 0.32,
        pitch_amp_rad: 0.14,
        cadence_hz: 0.85,
    },
    BehaviorPhaseSpec {
        state: FaunaBehaviorState::Grazing,
        duration_s: 5.5,
        stride_amp_m: 0.15,
        bob_amp_m: 0.02,
        yaw_sweep_rad: 0.16,
        pitch_amp_rad: 0.16,
        cadence_hz: 0.50,
    },
    BehaviorPhaseSpec {
        state: FaunaBehaviorState::AlertTurn,
        duration_s: 2.5,
        stride_amp_m: 0.0,
        bob_amp_m: 0.03,
        yaw_sweep_rad: 0.72,
        pitch_amp_rad: -0.06,
        cadence_hz: 0.60,
    },
    BehaviorPhaseSpec {
        state: FaunaBehaviorState::Vocalizing,
        duration_s: 2.5,
        stride_amp_m: 0.04,
        bob_amp_m: 0.02,
        yaw_sweep_rad: 0.22,
        pitch_amp_rad: 0.04,
        cadence_hz: 1.10,
    },
];

const SEMI_AQUATIC_CYCLE: &[BehaviorPhaseSpec] = &[
    BehaviorPhaseSpec {
        state: FaunaBehaviorState::Foraging,
        duration_s: 8.0,
        stride_amp_m: 1.15,
        bob_amp_m: 0.05,
        yaw_sweep_rad: 0.48,
        pitch_amp_rad: 0.08,
        cadence_hz: 1.60,
    },
    BehaviorPhaseSpec {
        state: FaunaBehaviorState::AlertTurn,
        duration_s: 2.5,
        stride_amp_m: 0.10,
        bob_amp_m: 0.03,
        yaw_sweep_rad: 0.78,
        pitch_amp_rad: -0.06,
        cadence_hz: 0.70,
    },
    BehaviorPhaseSpec {
        state: FaunaBehaviorState::Perching,
        duration_s: 5.0,
        stride_amp_m: 0.05,
        bob_amp_m: 0.01,
        yaw_sweep_rad: 0.15,
        pitch_amp_rad: 0.03,
        cadence_hz: 0.30,
    },
    BehaviorPhaseSpec {
        state: FaunaBehaviorState::Vocalizing,
        duration_s: 2.0,
        stride_amp_m: 0.0,
        bob_amp_m: 0.02,
        yaw_sweep_rad: 0.20,
        pitch_amp_rad: -0.05,
        cadence_hz: 0.90,
    },
];

const AVIAN_CYCLE: &[BehaviorPhaseSpec] = &[
    BehaviorPhaseSpec {
        state: FaunaBehaviorState::Perching,
        duration_s: 6.0,
        stride_amp_m: 0.0,
        bob_amp_m: 0.02,
        yaw_sweep_rad: 0.25,
        pitch_amp_rad: 0.06,
        cadence_hz: 0.35,
    },
    BehaviorPhaseSpec {
        state: FaunaBehaviorState::AlertTurn,
        duration_s: 3.5,
        stride_amp_m: 0.0,
        bob_amp_m: 0.03,
        yaw_sweep_rad: 1.05,
        pitch_amp_rad: -0.10,
        cadence_hz: 0.95,
    },
    BehaviorPhaseSpec {
        state: FaunaBehaviorState::Vocalizing,
        duration_s: 4.0,
        stride_amp_m: 0.0,
        bob_amp_m: 0.05,
        yaw_sweep_rad: 0.32,
        pitch_amp_rad: -0.24,
        cadence_hz: 2.40,
    },
    BehaviorPhaseSpec {
        state: FaunaBehaviorState::Foraging,
        duration_s: 4.5,
        stride_amp_m: 0.85,
        bob_amp_m: 0.22,
        yaw_sweep_rad: 0.42,
        pitch_amp_rad: 0.16,
        cadence_hz: 1.85,
    },
];

fn cycle_for_locomotion(locomotion: LocomotionClass) -> &'static [BehaviorPhaseSpec] {
    match locomotion {
        LocomotionClass::Saltatorial => SALTATORIAL_CYCLE,
        LocomotionClass::Arboreal => ARBOREAL_CYCLE,
        LocomotionClass::Fossorial => FOSSORIAL_CYCLE,
        LocomotionClass::SemiAquatic => SEMI_AQUATIC_CYCLE,
        LocomotionClass::Avian => AVIAN_CYCLE,
    }
}

#[inline]
fn splitmix64_hash(mut x: u64) -> u64 {
    x = x.wrapping_add(0x9e37_79b9_7f4a_7c15);
    x = (x ^ (x >> 30)).wrapping_mul(0xbf58_476d_1ce4_e5b9);
    x = (x ^ (x >> 27)).wrapping_mul(0x94d0_49bb_1331_11eb);
    x ^ (x >> 31)
}

#[inline]
fn seed_unit_f64(seed: u64) -> f64 {
    ((splitmix64_hash(seed) >> 11) as f64) * (1.0 / ((1u64 << 53) as f64))
}

#[derive(Clone, Copy, Debug, PartialEq)]
pub struct FaunaBehaviorSample {
    pub state: FaunaBehaviorState,
    pub state_progress: f32,
    pub translation_offset: [f64; 3],
    pub yaw_offset_rad: f64,
    pub pitch_offset_rad: f64,
    pub should_vocalize: bool,
}

/// Evaluate a deterministic fauna behavioral frame purely from locomotion class,
/// `seed`, `time_s`, `site_vigor`, and `reduced_motion` — zero per-species branching.
pub fn evaluate_fauna_behavior(
    locomotion: LocomotionClass,
    seed: u64,
    time_s: f64,
    site_vigor: f32,
    reduced_motion: bool,
) -> FaunaBehaviorSample {
    let cycle = cycle_for_locomotion(locomotion);
    let total_duration: f64 = cycle.iter().map(|p| p.duration_s).sum();
    let t_safe = if time_s.is_finite() { time_s.max(0.0) } else { 0.0 };
    let vigor = if site_vigor.is_finite() {
        (site_vigor as f64).clamp(0.4, 1.8)
    } else {
        1.0
    };
    let phase_offset_s = seed_unit_f64(seed) * total_duration;
    let mut cursor = (t_safe + phase_offset_s) % total_duration;

    let mut active = &cycle[0];
    for phase in cycle {
        if cursor < phase.duration_s {
            active = phase;
            break;
        }
        cursor -= phase.duration_s;
    }

    let progress = (cursor / active.duration_s).clamp(0.0, 1.0) as f32;
    let should_vocalize = active.state == FaunaBehaviorState::Vocalizing;

    if reduced_motion {
        return FaunaBehaviorSample {
            state: active.state,
            state_progress: progress,
            translation_offset: [0.0; 3],
            yaw_offset_rad: 0.0,
            pitch_offset_rad: 0.0,
            should_vocalize,
        };
    }

    let heading = seed_unit_f64(seed ^ 0xa5a5_5a5a_1234_5678) * std::f64::consts::TAU;
    let omega = active.cadence_hz * std::f64::consts::TAU;
    let wave = (cursor * omega).sin();
    let bob_wave = (cursor * omega).sin().abs();

    let stride = active.stride_amp_m * vigor * (cursor / active.duration_s * std::f64::consts::PI).sin();
    let dx = heading.cos() * stride;
    let dz = heading.sin() * stride;
    let dy = active.bob_amp_m * vigor * bob_wave;

    FaunaBehaviorSample {
        state: active.state,
        state_progress: progress,
        translation_offset: [dx, dy, dz],
        yaw_offset_rad: active.yaw_sweep_rad * wave,
        pitch_offset_rad: active.pitch_amp_rad * (cursor * omega * 0.5).sin(),
        should_vocalize,
    }
}

/// Generic flora wind-sway kinematics driven by seed, world XZ position, natural
/// sway frequency (`natural_freq_hz`), crown height (`crown_height_m`), and wind stress.
pub fn evaluate_flora_wind_sway(
    time_s: f64,
    seed: u64,
    world_xz: [f32; 2],
    natural_freq_hz: f32,
    crown_height_m: f32,
    wind_stress: f32,
    reduced_motion: bool,
) -> [f64; 3] {
    if reduced_motion {
        return [0.0; 3];
    }
    let t = if time_s.is_finite() { time_s.max(0.0) } else { 0.0 };
    let freq = if natural_freq_hz.is_finite() && natural_freq_hz > 0.05 {
        (natural_freq_hz as f64).clamp(0.15, 2.5)
    } else {
        0.65
    };
    let h_factor = if crown_height_m.is_finite() {
        (crown_height_m as f64 / 18.0).clamp(0.15, 2.0)
    } else {
        0.5
    };
    let stress = if wind_stress.is_finite() {
        (wind_stress as f64).clamp(0.0, 1.0)
    } else {
        0.15
    };
    let wx = if world_xz[0].is_finite() { world_xz[0] as f64 } else { 0.0 };
    let wz = if world_xz[1].is_finite() { world_xz[1] as f64 } else { 0.0 };

    let phase = seed_unit_f64(seed) * std::f64::consts::TAU + wx * 0.045 + wz * 0.032;
    let gust = 0.65 + 0.35 * (t * 0.22 + wx * 0.015 - wz * 0.012).sin();
    let amp = (0.04 + 0.14 * stress) * h_factor * gust;
    let primary = (t * freq * std::f64::consts::TAU + phase).sin();
    let secondary = (t * freq * 1.618 * std::f64::consts::TAU + phase * 0.7).cos();

    [
        amp * (0.75 * primary + 0.25 * secondary),
        0.0,
        amp * (0.55 * secondary - 0.30 * primary),
    ]
}

// ---------------------------------------------------------------------------
// 2. Opt-In Spatial .qac Vocalization with Distance Attenuation
// ---------------------------------------------------------------------------

const QAC_REF_DISTANCE_M: f32 = 3.0;
const QAC_MAX_DISTANCE_M: f32 = 120.0;

#[derive(Clone, Debug, PartialEq)]
pub struct SpatialAudioDispatch {
    pub qac_url: String,
    pub distance_m: f32,
    pub gain: f32,
    pub attenuation_db: f32,
    pub lowpass_hz: f32,
    pub audible: bool,
    pub user_gesture_required: bool,
    pub opt_in_granted: bool,
    pub status: String,
}

/// Compute distance attenuation (`gain`, `attenuation_db`, atmospheric `lowpass_hz`)
/// using inverse-distance rolloff with smooth taper to zero at 120 m.
pub fn compute_spatial_attenuation(distance_m: f32) -> (f32, f32, f32) {
    if !distance_m.is_finite() {
        return (0.0, -96.0, 5_000.0);
    }
    let d = distance_m.max(0.0);
    if d <= QAC_REF_DISTANCE_M {
        return (1.0, 0.0, 18_000.0);
    }
    if d >= QAC_MAX_DISTANCE_M {
        return (0.0, -96.0, 5_040.0);
    }
    let inv = QAC_REF_DISTANCE_M / d;
    let span_frac = ((d - QAC_REF_DISTANCE_M) / (QAC_MAX_DISTANCE_M - QAC_REF_DISTANCE_M)).clamp(0.0, 1.0);
    let taper = 1.0 - span_frac * span_frac;
    let gain = (inv * taper).clamp(0.0, 1.0);
    let attenuation_db = if gain > 1e-5 {
        20.0 * gain.log10()
    } else {
        -96.0
    };
    let lowpass_hz = 18_000.0 * (1.0 - 0.72 * (d / QAC_MAX_DISTANCE_M).clamp(0.0, 1.0));
    (gain, attenuation_db, lowpass_hz)
}

fn is_valid_qac_url(qac_url: &str) -> bool {
    let u = qac_url.trim();
    if u.is_empty() || u.contains("..") || u.contains('\\') || !u.ends_with(".qac") {
        return false;
    }
    u.starts_with("/intenteco/")
        || u.starts_with("https://raw.githubusercontent.com/intenteco/")
}

/// Plan an opt-in spatial `.qac` vocalization with explicit opt-in status.
/// Never permits forced/autoplay audio when `opt_in_granted` is `false`.
pub fn vocalize_species_call_with_opt_in(
    qac_url: &str,
    distance_m: f32,
    opt_in_granted: bool,
) -> SpatialAudioDispatch {
    let d = if distance_m.is_finite() {
        distance_m.max(0.0)
    } else {
        QAC_MAX_DISTANCE_M
    };
    if !is_valid_qac_url(qac_url) {
        return SpatialAudioDispatch {
            qac_url: qac_url.to_string(),
            distance_m: d,
            gain: 0.0,
            attenuation_db: -96.0,
            lowpass_hz: 5_040.0,
            audible: false,
            user_gesture_required: true,
            opt_in_granted,
            status: "Refused invalid .qac sidecar URL".into(),
        };
    }
    let (gain, attenuation_db, lowpass_hz) = compute_spatial_attenuation(d);
    if !opt_in_granted {
        return SpatialAudioDispatch {
            qac_url: qac_url.to_string(),
            distance_m: d,
            gain: 0.0,
            attenuation_db,
            lowpass_hz,
            audible: false,
            user_gesture_required: true,
            opt_in_granted: false,
            status: "User-gesture audio opt-in required before playing .qac call".into(),
        };
    }
    let audible = gain > 0.001;
    let status = if audible {
        format!(
            "Opt-in spatial .qac call ready at {d:.1} m (gain {gain:.3}, {attenuation_db:.1} dB, LPF {lowpass_hz:.0} Hz)"
        )
    } else {
        format!("Caller beyond hearing horizon ({d:.1} m >= {QAC_MAX_DISTANCE_M:.0} m)")
    };
    SpatialAudioDispatch {
        qac_url: qac_url.to_string(),
        distance_m: d,
        gain,
        attenuation_db,
        lowpass_hz,
        audible,
        user_gesture_required: true,
        opt_in_granted: true,
        status,
    }
}

/// Opt-in spatial `.qac` vocalization trigger helper (`vocalize_species_call(qac_url, distance_m)`).
/// Enforces user-gesture opt-in (`is_audio_opt_in_granted()`) and applies distance attenuation.
pub fn vocalize_species_call(qac_url: &str, distance_m: f32) -> SpatialAudioDispatch {
    let dispatch = vocalize_species_call_with_opt_in(qac_url, distance_m, is_audio_opt_in_granted());
    #[cfg(target_arch = "wasm32")]
    if dispatch.audible {
        let url = dispatch.qac_url.clone();
        let gain = dispatch.gain;
        let lowpass_hz = dispatch.lowpass_hz;
        wasm_bindgen_futures::spawn_local(async move {
            if let Err(e) = play_qac_spatial_wasm(&url, gain, lowpass_hz).await {
                web_sys::console::warn_1(&e);
            }
        });
    }
    dispatch
}

#[cfg(target_arch = "wasm32")]
async fn play_qac_spatial_wasm(qac_url: &str, gain: f32, lowpass_hz: f32) -> Result<(), JsValue> {
    let raw = super::super::bytes(qac_url, 16 * 1024 * 1024).await?;
    let mut reader = qualia_core_db::QualiaPortal::new_qac_reader();
    let meta_js = reader.load_qac(&raw, 0)?;
    let meta_str = js_sys::JSON::stringify(&meta_js)?
        .as_string()
        .ok_or_else(|| err("Invalid QAC metadata"))?;
    let meta: serde_json::Value = serde_json::from_str(&meta_str).map_err(err)?;
    let sample_rate = meta["sample_rate_hz"].as_f64().unwrap_or(44_100.0).clamp(8_000.0, 192_000.0);
    let record_count = meta["record_count"].as_u64().unwrap_or(0).min(48_000 * 12) as u32;
    if record_count == 0 {
        return Err(err("QAC call contains zero audio records"));
    }
    let win_js = reader.qac_window("0", record_count, 0)?;
    let win_str = js_sys::JSON::stringify(&win_js)?
        .as_string()
        .ok_or_else(|| err("Invalid QAC window"))?;
    let win: serde_json::Value = serde_json::from_str(&win_str).map_err(err)?;
    let samples_arr = win["samples"]
        .as_array()
        .ok_or_else(|| err("QAC window missing PCM samples"))?;
    let samples: Vec<f32> = samples_arr
        .iter()
        .filter_map(|v| v.as_f64().map(|f| f as f32))
        .collect();
    if samples.is_empty() {
        return Err(err("QAC window yielded empty PCM buffer"));
    }

    let ctx = acquire_shared_audio_context(true)?;

    let create_buf: js_sys::Function =
        js_sys::Reflect::get(&ctx, &"createBuffer".into())?.dyn_into()?;
    let buf = create_buf.call3(
        &ctx,
        &JsValue::from_f64(1.0),
        &JsValue::from_f64(samples.len() as f64),
        &JsValue::from_f64(sample_rate),
    )?;
    let get_channel: js_sys::Function =
        js_sys::Reflect::get(&buf, &"getChannelData".into())?.dyn_into()?;
    let ch_val = get_channel.call1(&buf, &JsValue::from_f64(0.0))?;
    let f32_arr: js_sys::Float32Array = ch_val.dyn_into()?;
    f32_arr.copy_from(&samples);

    let create_src: js_sys::Function =
        js_sys::Reflect::get(&ctx, &"createBufferSource".into())?.dyn_into()?;
    let src = create_src.call0(&ctx)?;
    js_sys::Reflect::set(&src, &"buffer".into(), &buf)?;

    let create_filter: js_sys::Function =
        js_sys::Reflect::get(&ctx, &"createBiquadFilter".into())?.dyn_into()?;
    let filter = create_filter.call0(&ctx)?;
    js_sys::Reflect::set(&filter, &"type".into(), &"lowpass".into())?;
    let freq_param = js_sys::Reflect::get(&filter, &"frequency".into())?;
    js_sys::Reflect::set(&freq_param, &"value".into(), &JsValue::from_f64(lowpass_hz as f64))?;

    let create_gain: js_sys::Function =
        js_sys::Reflect::get(&ctx, &"createGain".into())?.dyn_into()?;
    let gain_node = create_gain.call0(&ctx)?;
    let gain_param = js_sys::Reflect::get(&gain_node, &"gain".into())?;
    js_sys::Reflect::set(&gain_param, &"value".into(), &JsValue::from_f64(gain as f64))?;

    let dest = js_sys::Reflect::get(&ctx, &"destination".into())?;
    let connect_src: js_sys::Function =
        js_sys::Reflect::get(&src, &"connect".into())?.dyn_into()?;
    connect_src.call1(&src, &filter)?;
    let connect_flt: js_sys::Function =
        js_sys::Reflect::get(&filter, &"connect".into())?.dyn_into()?;
    connect_flt.call1(&filter, &gain_node)?;
    let connect_gain: js_sys::Function =
        js_sys::Reflect::get(&gain_node, &"connect".into())?.dyn_into()?;
    connect_gain.call1(&gain_node, &dest)?;

    let start_fn: js_sys::Function = js_sys::Reflect::get(&src, &"start".into())?.dyn_into()?;
    start_fn.call0(&src)?;
    Ok(())
}

// ---------------------------------------------------------------------------
// 3. Rhizosphere & Mycelium X-Ray Lens Controller
// ---------------------------------------------------------------------------

#[derive(Clone, Copy, Debug, PartialEq)]
pub struct RhizosphereLensState {
    pub active: bool,
    pub ground_opacity: f32,
    pub show_emf_overlay: bool,
}

impl Default for RhizosphereLensState {
    fn default() -> Self {
        Self {
            active: false,
            ground_opacity: 1.0,
            show_emf_overlay: false,
        }
    }
}

impl RhizosphereLensState {
    pub fn toggle(&mut self) -> &Self {
        self.set_active(!self.active, None);
        self
    }

    pub fn set_active(&mut self, active: bool, custom_opacity: Option<f32>) {
        self.active = active;
        if active {
            let op = custom_opacity.unwrap_or(0.28);
            self.ground_opacity = if op.is_finite() {
                op.clamp(0.08, 0.85)
            } else {
                0.28
            };
            self.show_emf_overlay = true;
        } else {
            self.ground_opacity = 1.0;
            self.show_emf_overlay = false;
        }
    }
}

#[derive(Clone, Debug, PartialEq)]
pub struct RhizosphereLayerPlan {
    pub active_tiers: Vec<&'static str>,
    pub terrain_cutaway: bool,
    pub ground_opacity: f32,
    pub aboveground_url: Option<String>,
    pub rhizosphere_url: Option<String>,
    pub field_emf_url: Option<String>,
}

/// Compute the active `.10d` layers (`rhizosphere` + `aboveground_lod1`) and terrain
/// cutaway opacity for a species under the current [`RhizosphereLensState`].
pub fn compute_rhizosphere_layers(
    lens: &RhizosphereLensState,
    entry: &SpeciesCatalogEntry,
) -> RhizosphereLayerPlan {
    let is_flora = entry.kingdom.eq_ignore_ascii_case("flora");
    if lens.active {
        let opacity = if lens.ground_opacity.is_finite() {
            lens.ground_opacity.clamp(0.08, 0.85)
        } else {
            0.28
        };
        if is_flora {
            RhizosphereLayerPlan {
                active_tiers: vec!["rhizosphere", "aboveground_lod1"],
                terrain_cutaway: true,
                ground_opacity: opacity,
                aboveground_url: entry.aboveground_lod1_url.clone(),
                rhizosphere_url: entry.rhizosphere_url.clone(),
                field_emf_url: if lens.show_emf_overlay {
                    Some(entry.field_emf_url.clone())
                } else {
                    None
                },
            }
        } else {
            RhizosphereLayerPlan {
                active_tiers: vec!["lod0", "lod1", "lod2"],
                terrain_cutaway: true,
                ground_opacity: opacity,
                aboveground_url: entry.lod0_url.clone(),
                rhizosphere_url: None,
                field_emf_url: if lens.show_emf_overlay {
                    Some(entry.field_emf_url.clone())
                } else {
                    None
                },
            }
        }
    } else {
        RhizosphereLayerPlan {
            active_tiers: vec!["lod0", "lod1", "lod2"],
            terrain_cutaway: false,
            ground_opacity: 1.0,
            aboveground_url: entry.lod0_url.clone(),
            rhizosphere_url: None,
            field_emf_url: None,
        }
    }
}

#[derive(Clone, Debug, PartialEq)]
pub struct RhizosphereEmfReadout {
    pub is_flora: bool,
    pub mycorrhizal_type: String,
    pub carrier_band_hz: String,
    pub dominant_hz: f32,
    pub action_potential_mv: f32,
    pub hydraulic_lift_l_per_day: f32,
    pub glomalin_exudate_mg_c_per_g_day: f32,
    pub phosphorus_flux_umol_per_m2_hr: f32,
    pub nitrogen_flux_umol_per_m2_hr: f32,
    pub summary_text: String,
}

/// Format the `.emf` bio-electric / chemical exudate signaling readout (`0.1-50 Hz`
/// mycelial action potentials, hydraulic lift, glomalin/C/P/N exudate fluxes) for
/// any selected flora species from its Q42 traits and `InstanceState64`.
pub fn format_rhizosphere_emf_readout(
    entry: &SpeciesCatalogEntry,
    lens: &RhizosphereLensState,
    state64: &InstanceState64,
) -> RhizosphereEmfReadout {
    let is_flora = entry.kingdom.eq_ignore_ascii_case("flora");
    let vigor = if state64.site_vigor.is_finite() {
        state64.site_vigor.clamp(0.25, 2.0)
    } else {
        1.0
    };
    let age = if state64.ontogenetic_age_norm.is_finite() {
        state64.ontogenetic_age_norm.clamp(0.05, 1.0)
    } else {
        0.7
    };
    let moisture_stress = if state64.environmental_stress[1].is_finite() {
        state64.environmental_stress[1].clamp(0.0, 1.0)
    } else {
        0.1
    };
    let soil_stress = if state64.environmental_stress[2].is_finite() {
        state64.environmental_stress[2].clamp(0.0, 1.0)
    } else {
        0.1
    };

    if !is_flora {
        return RhizosphereEmfReadout {
            is_flora: false,
            mycorrhizal_type: "N/A (fauna organism)".into(),
            carrier_band_hz: "0.1-50 Hz ambient EM".into(),
            dominant_hz: 0.0,
            action_potential_mv: 0.0,
            hydraulic_lift_l_per_day: 0.0,
            glomalin_exudate_mg_c_per_g_day: 0.0,
            phosphorus_flux_umol_per_m2_hr: 0.0,
            nitrogen_flux_umol_per_m2_hr: 0.0,
            summary_text: format!(
                "{} is fauna; switch to a flora taxon to inspect subterranean root+mycelium .10d and .emf exudate fluxes.",
                entry.vernacular_name
            ),
        };
    }

    let myco = if entry.mycorrhizal_type.is_empty() {
        "Mycorrhizal / rhizosphere waveguide".to_string()
    } else {
        entry.mycorrhizal_type.clone()
    };
    let dominant_hz = (0.4 + 14.2 * vigor * (1.0 - 0.35 * soil_stress)).clamp(0.1, 50.0);
    let action_potential_mv = (14.0 + 28.0 * vigor * (1.0 - 0.25 * moisture_stress)).clamp(5.0, 65.0);
    let hydraulic_lift_l_per_day = (6.0 + 54.0 * age * vigor * (0.4 + 0.6 * (1.0 - moisture_stress))).max(0.5);
    let glomalin_exudate_mg_c_per_g_day = (1.2 + 4.8 * vigor * (1.0 - 0.3 * soil_stress)).max(0.1);
    let phosphorus_flux_umol_per_m2_hr = (0.8 + 3.4 * vigor * (0.6 + 0.4 * soil_stress)).max(0.1);
    let nitrogen_flux_umol_per_m2_hr = (1.5 + 6.2 * vigor * (1.0 - 0.2 * moisture_stress)).max(0.1);
    let state_label = if lens.active {
        "X-Ray active (rhizosphere + aboveground_lod1)"
    } else {
        "X-Ray standby (surface LODs)"
    };
    let summary_text = format!(
        "{state_label} | {myco} | 0.1-50 Hz mycelial action potentials (peak {dominant_hz:.1} Hz, {action_potential_mv:.1} mV) | Hydraulic lift {hydraulic_lift_l_per_day:.1} L/day | Exudates: glomalin/C {glomalin_exudate_mg_c_per_g_day:.2} mg/g/d, P {phosphorus_flux_umol_per_m2_hr:.2} umol/m2/h, N {nitrogen_flux_umol_per_m2_hr:.2} umol/m2/h"
    );

    RhizosphereEmfReadout {
        is_flora: true,
        mycorrhizal_type: myco,
        carrier_band_hz: "0.1-50 Hz".into(),
        dominant_hz,
        action_potential_mv,
        hydraulic_lift_l_per_day,
        glomalin_exudate_mg_c_per_g_day,
        phosphorus_flux_umol_per_m2_hr,
        nitrogen_flux_umol_per_m2_hr,
        summary_text,
    }
}

// ---------------------------------------------------------------------------
// 4. Q42 SPARQL Species Inspection Card HTML Builder
// ---------------------------------------------------------------------------

fn escape_html(raw: &str) -> String {
    let mut out = String::with_capacity(raw.len());
    for ch in raw.chars() {
        match ch {
            '&' => out.push_str("&amp;"),
            '<' => out.push_str("&lt;"),
            '>' => out.push_str("&gt;"),
            '"' => out.push_str("&quot;"),
            '\'' => out.push_str("&#39;"),
            _ => out.push(ch),
        }
    }
    out
}

/// Build an accessible, HTML-escaped Q42 SPARQL Species Inspection Card for any
/// flora or fauna [`SpeciesCatalogEntry`] and [`InstanceState64`].
pub fn render_species_card_html(
    entry: &SpeciesCatalogEntry,
    state64: &InstanceState64,
) -> String {
    render_species_card_html_with_lens(entry, state64, &RhizosphereLensState::default(), 12.0)
}

/// Build a Species Inspection Card reflecting a live [`RhizosphereLensState`] and distance.
pub fn render_species_card_html_with_lens(
    entry: &SpeciesCatalogEntry,
    state64: &InstanceState64,
    lens: &RhizosphereLensState,
    distance_m: f32,
) -> String {
    let common = escape_html(&entry.vernacular_name);
    let sci = escape_html(&entry.scientific_name);
    let did = escape_html(&entry.taxon_iri);
    let country = if entry.custodial_country.trim().is_empty() {
        "SE Australian Country (custodial attribution in taxon.q42)".to_string()
    } else {
        escape_html(&entry.custodial_country)
    };
    let zone = escape_html(&entry.ecological_zone);
    let catchment_zone = escape_html(&entry.catchment_zone);
    let repo_url = escape_html(&entry.intenteco_repo_url);
    let q42_url = escape_html(&entry.taxon_q42_url);
    let hmc_url = escape_html(&entry.hmc_package_url);
    let canonical_url = escape_html(&entry.url);
    let emf_url = escape_html(&entry.field_emf_url);

    let mut html = String::with_capacity(2048);
    let _ = write!(
        html,
        "<article class=\"species-card\" data-species-slug=\"{}\" data-kingdom=\"{}\">",
        escape_html(&entry.slug),
        escape_html(&entry.kingdom)
    );
    let _ = write!(
        html,
        "<header class=\"species-card-head\"><h3>{common}</h3><p class=\"species-scientific\"><em>{sci}</em></p><p class=\"species-did\"><code>{did}</code></p></header>"
    );
    let _ = write!(
        html,
        "<dl class=\"species-meta\">\
         <dt>Indigenous Country Provenance</dt><dd>{country}</dd>\
         <dt>Ecological Zone</dt><dd>{zone} ({catchment_zone})</dd>\
         <dt>InstanceState64 Phenotype</dt><dd>seed <code>0x{seed:016x}</code> · age {age:.2} · height_scale {hs:.2}x · site_vigor {sv:.2} · stress [{s0:.2}, {s1:.2}, {s2:.2}, {s3:.2}]</dd>\
         </dl>",
        seed = state64.seed,
        age = state64.ontogenetic_age_norm,
        hs = state64.height_scale,
        sv = state64.site_vigor,
        s0 = state64.environmental_stress[0],
        s1 = state64.environmental_stress[1],
        s2 = state64.environmental_stress[2],
        s3 = state64.environmental_stress[3],
    );

    if entry.kingdom.eq_ignore_ascii_case("flora") {
        let layers = compute_rhizosphere_layers(lens, entry);
        let readout = format_rhizosphere_emf_readout(entry, lens, state64);
        let pressed = if lens.active { "true" } else { "false" };
        let label = if lens.active {
            "Disable Rhizosphere X-Ray Lens"
        } else {
            "Activate Rhizosphere & Mycelium X-Ray Lens"
        };
        let _ = write!(
            html,
            "<section class=\"species-rhizosphere\">\
             <button type=\"button\" data-action=\"toggle-rhizosphere-xray\" aria-pressed=\"{pressed}\">{label}</button>\
             <p class=\"species-layers\">Active .10d layers: <code>{}</code> · Terrain opacity: {:.2}</p>\
             <p class=\"species-emf-readout\">{}</p>\
             </section>",
            escape_html(&layers.active_tiers.join(" + ")),
            layers.ground_opacity,
            escape_html(&readout.summary_text),
        );
    } else if let Some(ref qac_url) = entry.call_qac_url {
        let (gain, att_db, _) = compute_spatial_attenuation(distance_m);
        let call_desc = if entry.call_name.is_empty() {
            "Vocalization call (.qac)".to_string()
        } else {
            escape_html(&entry.call_name)
        };
        let _ = write!(
            html,
            "<section class=\"species-audio\">\
             <p>{call_desc} (distance {distance_m:.1} m · gain {gain:.2} · {att_db:.1} dB)</p>\
             <button type=\"button\" data-action=\"vocalize-species-call\" data-qac-url=\"{}\">Listen to spatial .qac call (opt-in)</button>\
             </section>",
            escape_html(qac_url),
        );
    }

    let _ = write!(
        html,
        "<footer class=\"species-links\">\
         <a href=\"{repo_url}\" target=\"_blank\" rel=\"noopener\">intenteco repository</a> · \
         <a href=\"{q42_url}\">taxon.q42</a> · \
         <a href=\"{hmc_url}\" target=\"_blank\" rel=\"noopener\">.hmc / dist packages</a> · \
         <a href=\"{canonical_url}\">canonical .10d</a> · \
         <a href=\"{emf_url}\">field .emf</a>\
         </footer></article>"
    );
    html
}

// ---------------------------------------------------------------------------
// 4b. Guided Ecological Tour Stops (6 SE Australian Stops)
// ---------------------------------------------------------------------------

#[derive(Clone, Copy, Debug, PartialEq)]
pub struct EcologicalTourStop {
    pub id: &'static str,
    pub title: &'static str,
    pub zone_label: &'static str,
    pub target: [f32; 3],
    pub yaw: f32,
    pub pitch: f32,
    pub zoom: f32,
    pub movement_mode: &'static str,
    pub activate_rhizosphere: bool,
    pub ocs_regime: &'static str,
    pub altitude_m: Option<f64>,
    pub focus_species_slug: Option<&'static str>,
    pub summary_en_au: &'static str,
}

pub const ECOLOGICAL_TOUR_STOPS: &[EcologicalTourStop] = &[
    EcologicalTourStop {
        id: "campsite-clearing",
        title: "Campsite Clearing",
        zone_label: "(-22, 28) · Local ENU",
        target: [-22.0, 5.4909217, 28.0],
        yaw: 0.63275,
        pitch: 0.17555,
        zoom: 18.8915,
        movement_mode: "observe",
        activate_rhizosphere: false,
        ocs_regime: "LocalEnu",
        altitude_m: None,
        focus_species_slug: None,
        summary_en_au: "Campsite Clearing at (-22, 28) in the Local ENU frame — solar arrays, field desk, and overland party.",
    },
    EcologicalTourStop {
        id: "platypus-creek",
        title: "River Red Gum & Platypus Creek",
        zone_label: "Riparian zone",
        target: [16.8, 2.2, 18.3],
        yaw: 0.45,
        pitch: 0.24,
        zoom: 12.0,
        movement_mode: "walk",
        activate_rhizosphere: false,
        ocs_regime: "LocalEnu",
        altitude_m: None,
        focus_species_slug: Some("ornithorhynchus-anatinus"),
        summary_en_au: "Riparian corridor where River Red Gums anchor shaded pools for foraging Platypus.",
    },
    EcologicalTourStop {
        id: "mountain-ash-canopy",
        title: "Mountain Ash Canopy",
        zone_label: "Montane slope",
        target: [-67.4, 14.5, -60.1],
        yaw: 0.55,
        pitch: 0.22,
        zoom: 18.0,
        movement_mode: "walk",
        activate_rhizosphere: false,
        ocs_regime: "LocalEnu",
        altitude_m: None,
        focus_species_slug: Some("eucalyptus-regnans"),
        summary_en_au: "Cool temperate wet-sclerophyll gully rising toward tall Mountain Ash and Koala canopy habitat.",
    },
    EcologicalTourStop {
        id: "rhizosphere-xray",
        title: "Rhizosphere X-Ray",
        zone_label: "Subterranean .10d + .emf",
        target: [6.7, 2.5, -19.9],
        yaw: 0.35,
        pitch: 0.34,
        zoom: 14.0,
        movement_mode: "walk",
        activate_rhizosphere: true,
        ocs_regime: "LocalEnu",
        altitude_m: None,
        focus_species_slug: Some("eucalyptus-camaldulensis"),
        summary_en_au: "Rhizosphere & Mycelium X-Ray lens active: terrain cutaway reveals root architecture and 0.1-50 Hz bio-electric fluxes.",
    },
    EcologicalTourStop {
        id: "kangaroo-meadow",
        title: "Kangaroo Meadow",
        zone_label: "Woodland clearing",
        target: [86.1, 13.8, 30.7],
        yaw: 0.90,
        pitch: 0.20,
        zoom: 14.0,
        movement_mode: "walk",
        activate_rhizosphere: false,
        ocs_regime: "LocalEnu",
        altitude_m: None,
        focus_species_slug: Some("macropus-giganteus"),
        summary_en_au: "Grassy box-gum woodland clearing on the eastern slope where Eastern Grey Kangaroos graze.",
    },
    EcologicalTourStop {
        id: "earth-solar-orbit",
        title: "Earth & Solar Orbit",
        zone_label: "Fly · OcsFrameTree",
        target: [0.0, 12_500_000.0, 0.0],
        yaw: 0.65,
        pitch: 0.42,
        zoom: 18.0,
        movement_mode: "fly",
        activate_rhizosphere: false,
        ocs_regime: "HeliocentricIcrf",
        altitude_m: Some(12_500_000.0),
        focus_species_slug: None,
        summary_en_au: "Orbital altitude in OcsFrameTree (Earth WGS84 / JPL 399 to Sun ICRF J2000) with starfield cue.",
    },
];

pub fn find_tour_stop(id: &str) -> Option<&'static EcologicalTourStop> {
    let needle = id.trim();
    ECOLOGICAL_TOUR_STOPS.iter().find(|stop| stop.id == needle)
}

// ---------------------------------------------------------------------------
// 4c. Interactive Species Field Guide Discovery & Drawer HTML Builder
// ---------------------------------------------------------------------------

pub const TOTAL_INTENTECO_SPECIES: usize = 17;
pub const DEFAULT_FLORA_SPECIES: usize = 11;
pub const DEFAULT_FAUNA_SPECIES: usize = 6;

#[derive(Clone, Debug, PartialEq, Eq)]
pub struct FieldGuideDiscovery {
    pub discovered: std::collections::BTreeSet<String>,
    pub total_species: usize,
}

impl Default for FieldGuideDiscovery {
    fn default() -> Self {
        Self::new(TOTAL_INTENTECO_SPECIES)
    }
}

impl FieldGuideDiscovery {
    pub fn new(total_species: usize) -> Self {
        Self {
            discovered: std::collections::BTreeSet::new(),
            total_species: if total_species == 0 {
                TOTAL_INTENTECO_SPECIES
            } else {
                total_species
            },
        }
    }

    pub fn set_total_species(&mut self, total: usize) {
        if total > 0 {
            self.total_species = total;
        }
    }

    /// Record a discovered species slug; returns `true` if newly added.
    pub fn mark_discovered(&mut self, slug: &str) -> bool {
        let clean = slug.trim();
        if clean.is_empty() {
            return false;
        }
        self.discovered.insert(clean.to_string())
    }

    pub fn is_discovered(&self, slug: &str) -> bool {
        self.discovered.contains(slug.trim())
    }

    pub fn discovered_count(&self) -> usize {
        self.discovered.len().min(self.total_species)
    }

    pub fn badge_label(&self) -> String {
        format!(
            "Discovered {} / {} species",
            self.discovered_count(),
            self.total_species
        )
    }
}

fn is_safe_href(raw: &str) -> bool {
    let trimmed = raw.trim();
    if trimmed.is_empty() || trimmed.contains("..") || trimmed.contains('\\') {
        return false;
    }
    (trimmed.starts_with('/') && !trimmed.starts_with("//"))
        || trimmed.starts_with("https://")
}

/// Build an Australian English (`en-AU`) summary purely from admitted Q42 traits.
pub fn format_species_en_au_summary(entry: &SpeciesCatalogEntry) -> String {
    let zone = if entry.ecological_zone.trim().is_empty() {
        "SE Australian catchment habitat"
    } else {
        entry.ecological_zone.trim()
    };
    if entry.kingdom.eq_ignore_ascii_case("flora") {
        let myco = if entry.mycorrhizal_type.trim().is_empty() {
            "symbiotic soil mycorrhizae"
        } else {
            entry.mycorrhizal_type.trim()
        };
        format!(
            "Native SE Australian flora thriving in {zone}; root guild links via {myco}."
        )
    } else {
        let loco = LocomotionClass::from_str(&entry.locomotion_class)
            .map(|l| l.label())
            .unwrap_or("Native catchment locomotion");
        if entry.call_name.trim().is_empty() {
            format!("Native SE Australian fauna inhabiting {zone}. Locomotion: {loco}.")
        } else {
            format!(
                "Native SE Australian fauna inhabiting {zone}. Locomotion: {loco}. Call: {}.",
                entry.call_name.trim()
            )
        }
    }
}

/// Render an accessible, HTML-escaped Species Field Guide drawer body from admitted Q42 entries.
pub fn render_field_guide_drawer_html(
    entries: &[SpeciesCatalogEntry],
    discovery: &FieldGuideDiscovery,
    kingdom_filter: Option<&str>,
) -> String {
    let filter = kingdom_filter.map(|k| k.trim().to_ascii_lowercase());
    let mut visible: Vec<&SpeciesCatalogEntry> = entries
        .iter()
        .filter(|e| match filter.as_deref() {
            Some("flora") => e.kingdom.eq_ignore_ascii_case("flora"),
            Some("fauna") => e.kingdom.eq_ignore_ascii_case("fauna"),
            _ => true,
        })
        .collect();
    visible.sort_by(|a, b| {
        let da = discovery.is_discovered(&a.slug);
        let db = discovery.is_discovered(&b.slug);
        db.cmp(&da)
            .then_with(|| a.kingdom.cmp(&b.kingdom))
            .then_with(|| a.vernacular_name.cmp(&b.vernacular_name))
    });

    if visible.is_empty() {
        return "<p class=\"fg-empty\">Loading admitted SE Australian species catalog from Q42…</p>".into();
    }

    let mut html = String::with_capacity(visible.len() * 768);
    for entry in visible {
        let discovered = discovery.is_discovered(&entry.slug);
        let status_label = if discovered { "Discovered" } else { "Unvisited" };
        let status_class = if discovered { "is-discovered" } else { "is-unvisited" };
        let slug_esc = escape_html(&entry.slug);
        let common = escape_html(&entry.vernacular_name);
        let sci = escape_html(&entry.scientific_name);
        let kingdom = escape_html(&entry.kingdom);
        let did = escape_html(&entry.taxon_iri);
        let zone = escape_html(&entry.ecological_zone);
        let catchment = escape_html(&entry.catchment_zone);
        let country = if entry.custodial_country.trim().is_empty() {
            "SE Australian Country (custodial attribution in taxon.q42)".to_string()
        } else {
            escape_html(&entry.custodial_country)
        };
        let summary = escape_html(&format_species_en_au_summary(entry));
        let emf_note = if entry.kingdom.eq_ignore_ascii_case("flora") {
            let myco = if entry.mycorrhizal_type.trim().is_empty() {
                "Rhizosphere network"
            } else {
                entry.mycorrhizal_type.trim()
            };
            let sig = if entry.rhizosphere_signaling.trim().is_empty() {
                "0.1-50 Hz bio-electric & chemical exudate fluxes"
            } else {
                entry.rhizosphere_signaling.trim()
            };
            escape_html(&format!("{myco} · {sig}"))
        } else {
            let call = if entry.call_name.trim().is_empty() {
                "Bio-electric .emf surface & ecological field trace"
            } else {
                entry.call_name.trim()
            };
            escape_html(&format!("Bio-field .emf · {call}"))
        };

        let _ = write!(
            html,
            "<article class=\"fg-card {status_class}\" data-species-slug=\"{slug_esc}\" data-kingdom=\"{kingdom}\">\
             <div class=\"fg-card-head\">\
               <div><h4>{common}</h4><p class=\"fg-sci\"><em>{sci}</em> · <span class=\"fg-kingdom\">{kingdom}</span></p></div>\
               <span class=\"fg-badge {status_class}\">{status_label}</span>\
             </div>\
             <p class=\"fg-summary\">{summary}</p>\
             <dl class=\"fg-meta\">\
               <dt>Taxonomy ID</dt><dd><code>{did}</code></dd>\
               <dt>Ecological zone</dt><dd>{zone} (<code>{catchment}</code>)</dd>\
               <dt>Mycorrhizal / EMF</dt><dd>{emf_note}</dd>\
               <dt>Custodial Country</dt><dd>{country}</dd>\
             </dl>\
             <div class=\"fg-actions\">\
               <button type=\"button\" class=\"native-pill fg-btn\" data-focus-species=\"{slug_esc}\" aria-label=\"Visit {common} in the world\">Visit</button>"
        );

        if let Some(ref qac_url) = entry.call_qac_url {
            if is_safe_href(qac_url) {
                let qac_esc = escape_html(qac_url);
                let _ = write!(
                    html,
                    "<button type=\"button\" class=\"native-pill fg-btn\" data-vocalize-qac=\"{qac_esc}\" data-species-slug=\"{slug_esc}\" aria-label=\"Play spatial .qac call for {common}\">Listen (.qac)</button>"
                );
            }
        }

        html.push_str("</div><footer class=\"fg-links\">");
        if is_safe_href(&entry.taxon_q42_url) {
            let u = escape_html(&entry.taxon_q42_url);
            let _ = write!(html, "<a href=\"{u}\">taxon.q42</a>");
        }
        if is_safe_href(&entry.hmc_package_url) {
            let u = escape_html(&entry.hmc_package_url);
            let _ = write!(html, " · <a href=\"{u}\" target=\"_blank\" rel=\"noopener\">.hmc / dist</a>");
        }
        if is_safe_href(&entry.field_emf_url) {
            let u = escape_html(&entry.field_emf_url);
            let _ = write!(html, " · <a href=\"{u}\">field .emf</a>");
        }
        if is_safe_href(&entry.intenteco_repo_url) {
            let u = escape_html(&entry.intenteco_repo_url);
            let _ = write!(html, " · <a href=\"{u}\" target=\"_blank\" rel=\"noopener\">intenteco</a>");
        }
        html.push_str("</footer></article>");
    }
    html
}

// ---------------------------------------------------------------------------
// 4d. SE Australian Time-of-Day Lighting & Orbital Starfield Presets
// ---------------------------------------------------------------------------

pub const ORBITAL_STARFIELD_ALTITUDE_M: f64 = 10_000.0;

#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash)]
pub enum TimeOfDayPreset {
    Dawn,
    Midday,
    GoldenHour,
    Dusk,
}

#[derive(Clone, Copy, Debug, PartialEq)]
pub struct AtmosphereLightingSpec {
    pub id: &'static str,
    pub label: &'static str,
    pub sky_preset: u32,
    pub clear_color: [f64; 4],
    pub sun: [f32; 5],
    pub exposure_ev: f32,
    pub starfield_active: bool,
    pub description_en_au: &'static str,
}

impl TimeOfDayPreset {
    pub const fn all() -> &'static [TimeOfDayPreset] {
        &[Self::Dawn, Self::Midday, Self::GoldenHour, Self::Dusk]
    }

    pub const fn as_str(self) -> &'static str {
        match self {
            Self::Dawn => "dawn",
            Self::Midday => "midday",
            Self::GoldenHour => "golden-hour",
            Self::Dusk => "dusk",
        }
    }

    pub const fn label(self) -> &'static str {
        match self {
            Self::Dawn => "Dawn",
            Self::Midday => "Midday",
            Self::GoldenHour => "Golden Hour",
            Self::Dusk => "Dusk",
        }
    }

    pub fn from_str(raw: &str) -> Option<Self> {
        match raw.trim().to_ascii_lowercase().as_str() {
            "dawn" => Some(Self::Dawn),
            "midday" | "day" | "noon" => Some(Self::Midday),
            "golden-hour" | "golden_hour" | "golden" | "afternoon" => Some(Self::GoldenHour),
            "dusk" | "twilight" | "evening" => Some(Self::Dusk),
            _ => None,
        }
    }

    pub const fn spec(self) -> AtmosphereLightingSpec {
        match self {
            Self::Dawn => AtmosphereLightingSpec {
                id: "dawn",
                label: "Dawn",
                sky_preset: 2,
                clear_color: [0.68, 0.56, 0.58, 1.0],
                sun: [0.82, 0.24, 0.42, 1.35, 0.34],
                exposure_ev: -0.05,
                starfield_active: false,
                description_en_au: "Cool eucalyptus-mist SE Australian dawn with low eastern sunlight.",
            },
            Self::Midday => AtmosphereLightingSpec {
                id: "midday",
                label: "Midday",
                sky_preset: 1,
                clear_color: [0.36, 0.58, 0.86, 1.0],
                sun: [-0.45, 0.82, 0.34, 1.75, 0.30],
                exposure_ev: 0.0,
                starfield_active: false,
                description_en_au: "Clear southern tablelands midday light across creek and woodland canopy.",
            },
            Self::GoldenHour => AtmosphereLightingSpec {
                id: "golden-hour",
                label: "Golden Hour",
                sky_preset: 2,
                clear_color: [0.90, 0.58, 0.34, 1.0],
                sun: [-0.84, 0.22, -0.44, 1.45, 0.34],
                exposure_ev: 0.10,
                starfield_active: false,
                description_en_au: "Warm ochre-amber afternoon raking across River Red Gums and kangaroo grass.",
            },
            Self::Dusk => AtmosphereLightingSpec {
                id: "dusk",
                label: "Dusk",
                sky_preset: 3,
                clear_color: [0.09, 0.13, 0.24, 1.0],
                sun: [-0.30, 0.72, -0.58, 0.62, 0.18],
                exposure_ev: -0.15,
                starfield_active: false,
                description_en_au: "Deep southern twilight descending over the catchment.",
            },
        }
    }
}

/// Resolve sky/sun preset for the active time-of-day and OCS altitude, switching
/// to the orbital starfield cue above `10,000 m`.
pub fn resolve_atmosphere_for_altitude(
    preset: TimeOfDayPreset,
    altitude_m: f64,
) -> AtmosphereLightingSpec {
    if altitude_m.is_finite() && altitude_m >= ORBITAL_STARFIELD_ALTITUDE_M {
        AtmosphereLightingSpec {
            id: preset.as_str(),
            label: preset.label(),
            sky_preset: 0,
            clear_color: [0.02, 0.03, 0.07, 1.0],
            sun: [0.55, 0.42, 0.72, 1.95, 0.14],
            exposure_ev: 0.0,
            starfield_active: true,
            description_en_au: "Orbital starfield atmosphere active above 10 km in OcsFrameTree.",
        }
    } else {
        preset.spec()
    }
}

// ---------------------------------------------------------------------------
// 4e. Progressive Streaming Status Pill Formatter
// ---------------------------------------------------------------------------

pub fn format_streaming_status_pill(
    selected_backend: Option<&str>,
    device_tier: Option<&str>,
    is_mobile: bool,
    species_count: usize,
    ewm_phase: Option<&str>,
) -> String {
    let backend_label = match selected_backend.map(str::trim) {
        Some(b) if b.eq_ignore_ascii_case("webgpu") => "WebGPU",
        Some(b) if b.eq_ignore_ascii_case("webgl2") => "WebGL2",
        Some(b) if !b.is_empty() => b,
        _ => "Native GPU",
    };
    let tier_norm = device_tier.map(str::trim).map(str::to_ascii_lowercase);
    let tier_label = match tier_norm.as_deref() {
        Some("tier-a") => "Tier A",
        Some("tier-b") => "Tier B",
        Some("tier-c") => "Tier C",
        Some(other) if !other.is_empty() => other,
        _ => "Tier B",
    };
    let mut parts = vec![backend_label.to_string(), tier_label.to_string()];
    if is_mobile {
        parts.push("Mobile".to_string());
    }
    if let Some(phase) = ewm_phase.map(str::trim).filter(|p| !p.is_empty()) {
        parts.push(phase.to_string());
    }
    if species_count > 0 {
        parts.push(format!("{species_count} species"));
    }
    parts.join(" · ")
}

// ---------------------------------------------------------------------------
// 5. WASM Runtime Controls & Q42 SPARQL Wiring
// ---------------------------------------------------------------------------

#[cfg(target_arch = "wasm32")]
const CATCHMENT_PLACE_Q42: &str = "/data/ewm/places/se-australia-catchment.q42";

#[cfg(target_arch = "wasm32")]
struct SharedAudioState {
    ctx: Option<JsValue>,
    ambient_gain: Option<JsValue>,
    ambient_sources: Vec<JsValue>,
    lfo_osc: Option<JsValue>,
}

#[cfg(target_arch = "wasm32")]
impl Default for SharedAudioState {
    fn default() -> Self {
        Self {
            ctx: None,
            ambient_gain: None,
            ambient_sources: Vec::new(),
            lfo_osc: None,
        }
    }
}

#[cfg(target_arch = "wasm32")]
struct FieldGuideController {
    entries: Vec<SpeciesCatalogEntry>,
    placements: Vec<(String, String, InstanceState64, [f32; 3])>,
    discovery: FieldGuideDiscovery,
    kingdom_filter: Option<String>,
    time_of_day: TimeOfDayPreset,
    world: Option<Shared>,
    last_vocalize_ms: f64,
    last_inspected_slug: String,
    last_starfield: bool,
    poller_installed: bool,
}

#[cfg(target_arch = "wasm32")]
impl Default for FieldGuideController {
    fn default() -> Self {
        Self {
            entries: Vec::new(),
            placements: Vec::new(),
            discovery: FieldGuideDiscovery::default(),
            kingdom_filter: None,
            time_of_day: TimeOfDayPreset::Midday,
            world: None,
            last_vocalize_ms: 0.0,
            last_inspected_slug: String::new(),
            last_starfield: false,
            poller_installed: false,
        }
    }
}

#[cfg(target_arch = "wasm32")]
thread_local! {
    static SHARED_AUDIO: std::cell::RefCell<SharedAudioState> =
        std::cell::RefCell::new(SharedAudioState::default());
    static FIELD_GUIDE: std::cell::RefCell<FieldGuideController> =
        std::cell::RefCell::new(FieldGuideController::default());
}

#[cfg(target_arch = "wasm32")]
fn acquire_shared_audio_context(create_if_missing: bool) -> Result<JsValue, JsValue> {
    if let Some(existing) = SHARED_AUDIO.with(|a| a.borrow().ctx.clone()) {
        if let Ok(resume) = js_sys::Reflect::get(&existing, &"resume".into())
            .and_then(|v| v.dyn_into::<js_sys::Function>())
        {
            let _ = resume.call0(&existing);
        }
        return Ok(existing);
    }
    if !create_if_missing {
        return Err(err("AudioContext not started"));
    }
    let window = web_sys::window().ok_or_else(|| err("No window"))?;
    let audio_ctor = js_sys::Reflect::get(&window, &"AudioContext".into())
        .ok()
        .filter(|v| v.is_function())
        .or_else(|| {
            js_sys::Reflect::get(&window, &"webkitAudioContext".into())
                .ok()
                .filter(|v| v.is_function())
        })
        .ok_or_else(|| err("Web Audio API unavailable"))?;
    let ctor_fn: js_sys::Function = audio_ctor.dyn_into()?;
    let ctx = js_sys::Reflect::construct(&ctor_fn, &js_sys::Array::new())?;
    SHARED_AUDIO.with(|a| {
        a.borrow_mut().ctx = Some(ctx.clone());
    });
    Ok(ctx)
}

#[cfg(target_arch = "wasm32")]
fn stop_ambient_nodes() {
    SHARED_AUDIO.with(|a| {
        let mut st = a.borrow_mut();
        for src in st.ambient_sources.drain(..) {
            if let Ok(stop_fn) = js_sys::Reflect::get(&src, &"stop".into())
                .and_then(|v| v.dyn_into::<js_sys::Function>())
            {
                let _ = stop_fn.call0(&src);
            }
            if let Ok(disc_fn) = js_sys::Reflect::get(&src, &"disconnect".into())
                .and_then(|v| v.dyn_into::<js_sys::Function>())
            {
                let _ = disc_fn.call0(&src);
            }
        }
        if let Some(osc) = st.lfo_osc.take() {
            if let Ok(stop_fn) = js_sys::Reflect::get(&osc, &"stop".into())
                .and_then(|v| v.dyn_into::<js_sys::Function>())
            {
                let _ = stop_fn.call0(&osc);
            }
        }
        if let Some(gain) = st.ambient_gain.take() {
            if let Ok(disc_fn) = js_sys::Reflect::get(&gain, &"disconnect".into())
                .and_then(|v| v.dyn_into::<js_sys::Function>())
            {
                let _ = disc_fn.call0(&gain);
            }
        }
        if let Some(ref ctx) = st.ctx {
            if let Ok(suspend_fn) = js_sys::Reflect::get(ctx, &"suspend".into())
                .and_then(|v| v.dyn_into::<js_sys::Function>())
            {
                let _ = suspend_fn.call0(ctx);
            }
        }
    });
}

#[cfg(target_arch = "wasm32")]
fn start_procedural_bush_soundscape(reduced_motion: bool) -> Result<(), JsValue> {
    stop_ambient_nodes();
    let ctx = acquire_shared_audio_context(true)?;
    let sample_rate = js_sys::Reflect::get(&ctx, &"sampleRate".into())
        .ok()
        .and_then(|v| v.as_f64())
        .unwrap_or(44_100.0)
        .clamp(8_000.0, 96_000.0);
    let len = (sample_rate * 3.0) as usize;
    let create_buf: js_sys::Function =
        js_sys::Reflect::get(&ctx, &"createBuffer".into())?.dyn_into()?;
    let buf = create_buf.call3(
        &ctx,
        &JsValue::from_f64(1.0),
        &JsValue::from_f64(len as f64),
        &JsValue::from_f64(sample_rate),
    )?;
    let get_channel: js_sys::Function =
        js_sys::Reflect::get(&buf, &"getChannelData".into())?.dyn_into()?;
    let ch_val = get_channel.call1(&buf, &JsValue::from_f64(0.0))?;
    let f32_arr: js_sys::Float32Array = ch_val.dyn_into()?;

    // Deterministic gentle pink/brown noise for creek water & canopy breeze (seamless loop).
    let mut pcm = vec![0.0_f32; len];
    let mut rng = 0x51a7_9921_u64;
    let mut acc = 0.0_f64;
    for sample in pcm.iter_mut() {
        rng = splitmix64_hash(rng);
        let white = ((rng >> 11) as f64 / ((1u64 << 53) as f64)) * 2.0 - 1.0;
        acc = (acc * 0.96 + white * 0.04).clamp(-1.0, 1.0);
        *sample = acc as f32;
    }
    let fade = (sample_rate * 0.08) as usize;
    for i in 0..fade.min(len / 4) {
        let w = i as f32 / fade as f32;
        pcm[i] *= w;
        pcm[len - 1 - i] *= w;
    }
    f32_arr.copy_from(&pcm);

    let create_gain: js_sys::Function =
        js_sys::Reflect::get(&ctx, &"createGain".into())?.dyn_into()?;
    let master_gain = create_gain.call0(&ctx)?;
    let master_param = js_sys::Reflect::get(&master_gain, &"gain".into())?;
    let gain_val = if reduced_motion { 0.04 } else { 0.06 };
    js_sys::Reflect::set(&master_param, &"value".into(), &JsValue::from_f64(gain_val))?;
    let dest = js_sys::Reflect::get(&ctx, &"destination".into())?;
    let connect_master: js_sys::Function =
        js_sys::Reflect::get(&master_gain, &"connect".into())?.dyn_into()?;
    connect_master.call1(&master_gain, &dest)?;

    let create_src: js_sys::Function =
        js_sys::Reflect::get(&ctx, &"createBufferSource".into())?.dyn_into()?;
    let create_filter: js_sys::Function =
        js_sys::Reflect::get(&ctx, &"createBiquadFilter".into())?.dyn_into()?;

    // Layer 1: Gentle creek babble (lowpass ~420 Hz)
    let creek_src = create_src.call0(&ctx)?;
    js_sys::Reflect::set(&creek_src, &"buffer".into(), &buf)?;
    js_sys::Reflect::set(&creek_src, &"loop".into(), &JsValue::TRUE)?;
    let creek_flt = create_filter.call0(&ctx)?;
    js_sys::Reflect::set(&creek_flt, &"type".into(), &"lowpass".into())?;
    let creek_freq = js_sys::Reflect::get(&creek_flt, &"frequency".into())?;
    js_sys::Reflect::set(&creek_freq, &"value".into(), &JsValue::from_f64(420.0))?;
    js_sys::Reflect::get(&creek_src, &"connect".into())?
        .dyn_into::<js_sys::Function>()?
        .call1(&creek_src, &creek_flt)?;
    js_sys::Reflect::get(&creek_flt, &"connect".into())?
        .dyn_into::<js_sys::Function>()?
        .call1(&creek_flt, &master_gain)?;
    js_sys::Reflect::get(&creek_src, &"start".into())?
        .dyn_into::<js_sys::Function>()?
        .call0(&creek_src)?;

    // Layer 2: Soft canopy wind breeze (bandpass ~260 Hz, subtle LFO when motion is allowed)
    let wind_src = create_src.call0(&ctx)?;
    js_sys::Reflect::set(&wind_src, &"buffer".into(), &buf)?;
    js_sys::Reflect::set(&wind_src, &"loop".into(), &JsValue::TRUE)?;
    let wind_flt = create_filter.call0(&ctx)?;
    js_sys::Reflect::set(&wind_flt, &"type".into(), &"bandpass".into())?;
    let wind_freq = js_sys::Reflect::get(&wind_flt, &"frequency".into())?;
    js_sys::Reflect::set(&wind_freq, &"value".into(), &JsValue::from_f64(260.0))?;
    let wind_q = js_sys::Reflect::get(&wind_flt, &"Q".into())?;
    js_sys::Reflect::set(&wind_q, &"value".into(), &JsValue::from_f64(1.4))?;
    js_sys::Reflect::get(&wind_src, &"connect".into())?
        .dyn_into::<js_sys::Function>()?
        .call1(&wind_src, &wind_flt)?;
    js_sys::Reflect::get(&wind_flt, &"connect".into())?
        .dyn_into::<js_sys::Function>()?
        .call1(&wind_flt, &master_gain)?;
    js_sys::Reflect::get(&wind_src, &"start".into())?
        .dyn_into::<js_sys::Function>()?
        .call0(&wind_src)?;

    let mut lfo_osc_opt = None;
    if !reduced_motion {
        if let Ok(create_osc) = js_sys::Reflect::get(&ctx, &"createOscillator".into())
            .and_then(|v| v.dyn_into::<js_sys::Function>())
        {
            if let Ok(osc) = create_osc.call0(&ctx) {
                let osc_freq = js_sys::Reflect::get(&osc, &"frequency".into())?;
                let _ = js_sys::Reflect::set(&osc_freq, &"value".into(), &JsValue::from_f64(0.08));
                let lfo_gain = create_gain.call0(&ctx)?;
                let lfo_gparam = js_sys::Reflect::get(&lfo_gain, &"gain".into())?;
                let _ = js_sys::Reflect::set(&lfo_gparam, &"value".into(), &JsValue::from_f64(85.0));
                let _ = js_sys::Reflect::get(&osc, &"connect".into())?
                    .dyn_into::<js_sys::Function>()?
                    .call1(&osc, &lfo_gain);
                let _ = js_sys::Reflect::get(&lfo_gain, &"connect".into())?
                    .dyn_into::<js_sys::Function>()?
                    .call1(&lfo_gain, &wind_freq);
                let _ = js_sys::Reflect::get(&osc, &"start".into())?
                    .dyn_into::<js_sys::Function>()?
                    .call0(&osc);
                lfo_osc_opt = Some(osc);
            }
        }
    }

    SHARED_AUDIO.with(|a| {
        let mut st = a.borrow_mut();
        st.ambient_gain = Some(master_gain);
        st.ambient_sources = vec![creek_src, wind_src];
        st.lfo_osc = lfo_osc_opt;
    });
    Ok(())
}

/// Toggle opt-in bush soundscape (`Muted` by default; activates only on explicit user gesture).
#[cfg(target_arch = "wasm32")]
pub(crate) fn set_bush_soundscape(enabled: bool, reduced_motion: bool) -> bool {
    set_audio_opt_in(enabled);
    if enabled {
        if let Err(e) = start_procedural_bush_soundscape(reduced_motion) {
            web_sys::console::warn_1(&e);
            set_audio_opt_in(false);
            stop_ambient_nodes();
            return false;
        }
        true
    } else {
        stop_ambient_nodes();
        false
    }
}

#[cfg(target_arch = "wasm32")]
pub(crate) fn mark_species_discovered(slug: &str) -> bool {
    let newly = FIELD_GUIDE.with(|fg| fg.borrow_mut().discovery.mark_discovered(slug));
    if newly {
        sync_field_guide_ui();
        let (discovered, total) = FIELD_GUIDE.with(|fg| {
            let d = &fg.borrow().discovery;
            (d.discovered_count(), d.total_species)
        });
        super::super::ui::emit_custom_event(
            "webizen:species-discovered",
            &json!({
                "slug": slug,
                "discovered": discovered,
                "total": total,
            }),
        );
    }
    newly
}

#[cfg(target_arch = "wasm32")]
pub(crate) fn discovery_badge_text() -> String {
    FIELD_GUIDE.with(|fg| fg.borrow().discovery.badge_label())
}

#[cfg(target_arch = "wasm32")]
pub fn hydrate_entries(entries: Vec<SpeciesCatalogEntry>) {
    if entries.is_empty() {
        return;
    }
    FIELD_GUIDE.with(|fg| {
        let mut ctrl = fg.borrow_mut();
        ctrl.discovery.total_species = entries.len().max(17);
        ctrl.entries = entries;
    });
    sync_field_guide_ui();
}

#[cfg(not(target_arch = "wasm32"))]
pub fn hydrate_entries(_entries: Vec<SpeciesCatalogEntry>) {}

#[cfg(target_arch = "wasm32")]
pub(crate) fn species_count_loaded() -> usize {
    FIELD_GUIDE.with(|fg| fg.borrow().entries.len())
}

#[cfg(target_arch = "wasm32")]
pub(crate) fn nearest_placement_for_slug(slug: &str) -> Option<[f32; 3]> {
    FIELD_GUIDE.with(|fg| {
        fg.borrow()
            .placements
            .iter()
            .find(|(s, ..)| s == slug)
            .map(|(_, _, _, pos)| *pos)
    })
}

#[cfg(target_arch = "wasm32")]
pub(crate) fn current_time_of_day() -> TimeOfDayPreset {
    FIELD_GUIDE.with(|fg| fg.borrow().time_of_day)
}

#[cfg(target_arch = "wasm32")]
pub(crate) fn set_current_time_of_day(preset: TimeOfDayPreset) {
    FIELD_GUIDE.with(|fg| {
        fg.borrow_mut().time_of_day = preset;
    });
}

#[cfg(target_arch = "wasm32")]
pub(crate) fn set_field_guide_filter(kingdom: Option<&str>) {
    FIELD_GUIDE.with(|fg| {
        fg.borrow_mut().kingdom_filter = kingdom.map(str::to_string);
    });
    sync_field_guide_ui();
}

#[cfg(target_arch = "wasm32")]
pub(crate) fn sync_field_guide_ui() {
    let (badge, html, filter_val, total_count, flora_count, fauna_count) = FIELD_GUIDE.with(|fg| {
        let st = fg.borrow();
        let (total_c, flora_c, fauna_c) = if st.entries.is_empty() {
            (
                st.discovery.total_species,
                DEFAULT_FLORA_SPECIES,
                DEFAULT_FAUNA_SPECIES,
            )
        } else {
            let flora_c = st
                .entries
                .iter()
                .filter(|e| e.kingdom.eq_ignore_ascii_case("flora"))
                .count();
            let fauna_c = st
                .entries
                .iter()
                .filter(|e| e.kingdom.eq_ignore_ascii_case("fauna"))
                .count();
            (st.entries.len(), flora_c, fauna_c)
        };
        (
            st.discovery.badge_label(),
            render_field_guide_drawer_html(
                &st.entries,
                &st.discovery,
                st.kingdom_filter.as_deref(),
            ),
            st.kingdom_filter.clone().unwrap_or_default(),
            total_c,
            flora_c,
            fauna_c,
        )
    });
    if let Some(btn) = doc().get_element_by_id("native-field-guide-toggle") {
        btn.set_text_content(Some(&badge));
    }
    if let Some(count) = doc().get_element_by_id("native-field-guide-count") {
        count.set_text_content(Some(&badge));
    }
    let all_label = format!("All ({total_count})");
    let flora_label = format!("Flora ({flora_count})");
    let fauna_label = format!("Fauna ({fauna_count})");
    for (id, key, label) in [
        ("native-fg-filter-all", "", all_label.as_str()),
        ("native-fg-filter-flora", "flora", flora_label.as_str()),
        ("native-fg-filter-fauna", "fauna", fauna_label.as_str()),
    ] {
        if let Some(tab) = doc().get_element_by_id(id) {
            tab.set_text_content(Some(label));
            let active = filter_val.eq_ignore_ascii_case(key);
            let _ = tab.set_attribute("aria-pressed", if active { "true" } else { "false" });
        }
    }
    if let Some(list) = doc().get_element_by_id("native-field-guide-list") {
        list.set_inner_html(&html);
    }
}

#[cfg(target_arch = "wasm32")]
pub(crate) fn focus_species_in_world(world: &mut World, slug: &str) -> bool {
    let info = FIELD_GUIDE.with(|fg| {
        let st = fg.borrow();
        let entry = st.entries.iter().find(|e| e.slug == slug).cloned()?;
        let pos = st
            .placements
            .iter()
            .find(|(s, ..)| s == slug)
            .map(|(_, _, _, p)| *p)
            .unwrap_or([-22.0, 4.5, 28.0]);
        Some((entry, pos))
    });
    let Some((entry, pos)) = info else {
        return false;
    };
    mark_species_discovered(slug);
    let _ = super::super::locomotion::set_mode(
        world,
        super::super::locomotion::MovementMode::WalkExplore,
    );
    let ground = super::super::locomotion::catchment_height_at(pos[0] as f64, pos[2] as f64) as f32;
    let target_y = pos[1].max(ground + 1.2);
    world.goal.target = [pos[0], target_y, pos[2]];
    world.goal.zoom = 12.0;
    world.goal.pitch = 0.22;
    if world.reduced {
        world.camera = world.goal.clone();
        let _ = super::super::set_camera(world);
    }
    if entry.kingdom.eq_ignore_ascii_case("flora")
        && super::super::locomotion::rhizosphere_xray_active()
    {
        text(
            "native-world-status",
            &format!(
                "{} ({}) — Rhizosphere X-Ray active: inspecting roots & .emf exudates.",
                entry.vernacular_name, entry.scientific_name
            ),
        );
    } else {
        text(
            "native-world-status",
            &format!(
                "Visiting {} ({}) in {}.",
                entry.vernacular_name, entry.scientific_name, entry.ecological_zone
            ),
        );
    }
    if is_audio_opt_in_granted() {
        if let Some(ref qac_url) = entry.call_qac_url {
            let _ = vocalize_species_call(qac_url, 6.0);
        }
    }
    true
}

#[cfg(target_arch = "wasm32")]
fn trigger_nearest_fauna_vocalization(cam_target: [f32; 3], now_ms: f64) {
    if !is_audio_opt_in_granted() {
        return;
    }
    let candidate = FIELD_GUIDE.with(|fg| {
        let mut st = fg.borrow_mut();
        if now_ms - st.last_vocalize_ms < 9_500.0 {
            return None;
        }
        let mut best: Option<(f32, String, String)> = None;
        for (slug, _, _, pos) in &st.placements {
            let Some(entry) = st.entries.iter().find(|e| &e.slug == slug) else {
                continue;
            };
            let Some(ref qac) = entry.call_qac_url else {
                continue;
            };
            let dx = pos[0] - cam_target[0];
            let dy = pos[1] - cam_target[1];
            let dz = pos[2] - cam_target[2];
            let dist = (dx * dx + dy * dy + dz * dz).sqrt();
            if dist <= QAC_MAX_DISTANCE_M && best.as_ref().map(|(d, _, _)| dist < *d).unwrap_or(true) {
                best = Some((dist, qac.clone(), slug.clone()));
            }
        }
        if best.is_some() {
            st.last_vocalize_ms = now_ms;
        }
        best
    });
    if let Some((dist, qac_url, slug)) = candidate {
        let _ = vocalize_species_call(&qac_url, dist);
        mark_species_discovered(&slug);
    }
}

#[cfg(target_arch = "wasm32")]
fn install_biosphere_poller(world: Shared) {
    let already = FIELD_GUIDE.with(|fg| {
        let mut st = fg.borrow_mut();
        st.world = Some(world.clone());
        if st.poller_installed {
            true
        } else {
            st.poller_installed = true;
            false
        }
    });
    if already {
        return;
    }
    let Some(window) = web_sys::window() else {
        return;
    };
    let cb = Closure::<dyn FnMut()>::new(move || {
        let Some(world_rc) = FIELD_GUIDE.with(|fg| fg.borrow().world.clone()) else {
            return;
        };
        let Ok(mut w) = world_rc.try_borrow_mut() else {
            return;
        };
        let cam_target = w.camera.target;
        let alt_m = if super::super::locomotion::current_mode()
            == super::super::locomotion::MovementMode::ObserveOrbit
        {
            let ground = super::super::locomotion::terrain_datum_at(
                cam_target[0] as f64,
                cam_target[2] as f64,
            );
            (cam_target[1] as f64 - ground).max(0.0)
        } else {
            w.receipt
                .get("locomotion")
                .and_then(|l| l.get("altitude_m"))
                .and_then(|v| v.as_f64())
                .unwrap_or_else(|| (cam_target[1] as f64).max(0.0))
        };
        let inspected_slug = w
            .receipt
            .get("inspected_species")
            .and_then(|s| s.get("slug"))
            .and_then(|v| v.as_str())
            .unwrap_or("")
            .to_string();

        if !inspected_slug.is_empty() {
            let changed = FIELD_GUIDE.with(|fg| {
                let mut st = fg.borrow_mut();
                if st.last_inspected_slug != inspected_slug {
                    st.last_inspected_slug = inspected_slug.clone();
                    true
                } else {
                    false
                }
            });
            if changed {
                mark_species_discovered(&inspected_slug);
            }
        }

        // Proximity discovery when exploring on foot or low flight (< 45 m altitude)
        if alt_m < 45.0 {
            let nearby: Vec<String> = FIELD_GUIDE.with(|fg| {
                let st = fg.borrow();
                let mut found = Vec::new();
                for (slug, _, _, pos) in &st.placements {
                    if st.discovery.is_discovered(slug) {
                        continue;
                    }
                    let dx = pos[0] - cam_target[0];
                    let dz = pos[2] - cam_target[2];
                    if (dx * dx + dz * dz).sqrt() <= 14.0 {
                        found.push(slug.clone());
                    }
                }
                found
            });
            for slug in nearby {
                mark_species_discovered(&slug);
            }
        }

        // Automatic orbital starfield cue at >= 10 km altitude
        let preset = current_time_of_day();
        let resolved = resolve_atmosphere_for_altitude(preset, alt_m);
        let star_changed = FIELD_GUIDE.with(|fg| {
            let mut st = fg.borrow_mut();
            if st.last_starfield != resolved.starfield_active {
                st.last_starfield = resolved.starfield_active;
                true
            } else {
                false
            }
        });
        if star_changed {
            super::super::ui::apply_atmosphere_spec(&mut w, resolved);
        }

        super::super::ui::refresh_streaming_status_pill(&w);
        drop(w);
        trigger_nearest_fauna_vocalization(cam_target, super::super::now());
    });
    let _ = window.set_interval_with_callback_and_timeout_and_arguments_0(
        cb.as_ref().unchecked_ref(),
        1200,
    );
    cb.forget();
}

#[cfg(target_arch = "wasm32")]
struct SpeciesInstrumentState {
    entries: Vec<SpeciesCatalogEntry>,
    placements: Vec<(String, String, InstanceState64, [f32; 3])>,
    lens: RhizosphereLensState,
    selected_slug: String,
    selected_placement: usize,
}

#[cfg(target_arch = "wasm32")]
pub(super) fn controls(lab: &Lab) -> Result<(), JsValue> {
    let root = doc()
        .get_element_by_id("native-lab-species")
        .ok_or_else(|| err("Missing species biosphere panel"))?;
    add(&root, "h3", "", "SE Australia Living Biosphere (13 Species)")?;
    add(
        &root,
        "p",
        "",
        "Inspect Q42 species cards, toggle the Rhizosphere & Mycelium X-Ray lens for subterranean .10d/.emf signaling, or opt in to spatial .qac fauna vocalizations.",
    )?;
    let xray_btn = button(
        &root,
        "native-rhizosphere-xray-toggle",
        "Rhizosphere & Mycelium X-Ray Lens: Off",
    )?;
    xray_btn.set_attribute("aria-pressed", "false")?;

    let species_lbl = add(&root, "label", "", "Catchment species")?;
    species_lbl.set_attribute("for", "native-species-select")?;
    add(&root, "select", "native-species-select", "")?;

    let inst_lbl = add(&root, "label", "", "Placed individual (InstanceState64)")?;
    inst_lbl.set_attribute("for", "native-species-instance")?;
    add(&root, "select", "native-species-instance", "")?;

    add(&root, "div", "native-species-card", "")?;
    let _ = lab;
    Ok(())
}

#[cfg(target_arch = "wasm32")]
pub(super) async fn install(lab: &Lab) -> Result<(), JsValue> {
    let raw = bytes(CATCHMENT_PLACE_Q42, 4 * 1024 * 1024).await?;
    let (entries, placements) = {
        let world_rc = lab.borrow().world.clone();
        let mut world = world_rc.borrow_mut();
        knowledge::admit(&mut world.host, &raw, "catchment", "se-australia-catchment.q42")
            .map_err(err)?;
        let sparql_res = knowledge::sparql(
            &mut world.host,
            "SELECT ?taxon ?slug WHERE { ?taxon <https://webizen.au/relation/slug> ?slug }",
        )
        .unwrap_or(vibe::Value::Null);
        let index = QuinIndex::from_host(&world.host).map_err(err)?;
        let mut entries = Vec::new();
        for taxon_iri in knowledge::sparql_iris(&world.host, &sparql_res, 0) {
            if let Some(entry) = SpeciesCatalogEntry::from_quin_index(&index, &taxon_iri) {
                if !entries.iter().any(|e: &SpeciesCatalogEntry| e.slug == entry.slug) {
                    entries.push(entry);
                }
            }
        }
        entries.sort_by(|a, b| a.slug.cmp(&b.slug));

        let mut placements = Vec::new();
        for p_iri in index.rel("https://webizen.au/catalog/place/se-australia-catchment", "hasPlacement") {
            let Ok(slug) = index.text(&p_iri, "speciesSlug") else {
                continue;
            };
            let id = index.text(&p_iri, "id").unwrap_or_default();
            let seed = index.u64(&p_iri, "seed").unwrap_or(1);
            let age = index.number(&p_iri, "ontogeneticAgeNorm").unwrap_or(0.8) as f32;
            let hs = index.number(&p_iri, "heightScale").unwrap_or(1.0) as f32;
            let sv = index.number(&p_iri, "siteVigor").unwrap_or(1.0) as f32;
            let sw = index.number(&p_iri, "stressWind").unwrap_or(0.1) as f32;
            let pos = [
                index.number(&p_iri, "translationX").unwrap_or(0.0) as f32,
                index.number(&p_iri, "translationY").unwrap_or(0.0) as f32,
                index.number(&p_iri, "translationZ").unwrap_or(0.0) as f32,
            ];
            placements.push((
                slug,
                id,
                InstanceState64::new(seed, age, hs, sv, [sw, 0.08, 0.08, 0.08]),
                pos,
            ));
        }
        world.receipt["catchment_species_count"] = json!(entries.len());
        world.receipt["catchment_placement_count"] = json!(placements.len());
        (entries, placements)
    };

    if entries.is_empty() {
        return Ok(());
    }

    FIELD_GUIDE.with(|fg| {
        let mut st = fg.borrow_mut();
        st.discovery.set_total_species(entries.len());
        st.entries = entries.clone();
        st.placements = placements.clone();
    });
    sync_field_guide_ui();
    {
        let world_rc = lab.borrow().world.clone();
        let w = world_rc.borrow();
        super::super::ui::refresh_streaming_status_pill(&w);
        install_biosphere_poller(world_rc.clone());
    }

    let state = Rc::new(RefCell::new(SpeciesInstrumentState {
        selected_slug: entries[0].slug.clone(),
        entries,
        placements,
        lens: RhizosphereLensState::default(),
        selected_placement: 0,
    }));

    if let Some(sel) = doc().get_element_by_id("native-species-select") {
        sel.set_text_content(None);
        for e in &state.borrow().entries {
            option(&sel, &e.slug, &format!("{} ({})", e.vernacular_name, e.scientific_name))?;
        }
        let s = state.clone();
        let l = lab.clone();
        event(sel.unchecked_ref(), "change", move |_| {
            let slug = select_value("native-species-select");
            {
                let mut st = s.borrow_mut();
                st.selected_slug = slug;
                st.selected_placement = 0;
            }
            let _ = refresh_species_view(&s, &l);
        })?;
    }

    if let Some(inst_sel) = doc().get_element_by_id("native-species-instance") {
        let s = state.clone();
        let l = lab.clone();
        event(inst_sel.unchecked_ref(), "change", move |_| {
            let idx = select_value("native-species-instance")
                .parse::<usize>()
                .unwrap_or(0);
            s.borrow_mut().selected_placement = idx;
            let _ = refresh_species_view(&s, &l);
        })?;
    }

    if let Some(xray_btn) = doc().get_element_by_id("native-rhizosphere-xray-toggle") {
        let s = state.clone();
        let l = lab.clone();
        event(xray_btn.unchecked_ref(), "click", move |_| {
            s.borrow_mut().lens.toggle();
            let _ = refresh_species_view(&s, &l);
        })?;
    }

    refresh_species_view(&state, lab)
}

#[cfg(target_arch = "wasm32")]
fn refresh_species_view(state: &Rc<RefCell<SpeciesInstrumentState>>, lab: &Lab) -> Result<(), JsValue> {
    let (entry, state64, distance_m, lens, matching_ids) = {
        let st = state.borrow();
        let entry = st
            .entries
            .iter()
            .find(|e| e.slug == st.selected_slug)
            .cloned()
            .ok_or_else(|| err("Missing selected species"))?;
        let matching: Vec<(usize, String, InstanceState64, [f32; 3])> = st
            .placements
            .iter()
            .enumerate()
            .filter(|(_, (slug, _, _, _))| slug == &entry.slug)
            .map(|(i, (_, id, s64, pos))| (i, id.clone(), *s64, *pos))
            .collect();
        let chosen = matching
            .get(st.selected_placement)
            .or_else(|| matching.first())
            .cloned()
            .unwrap_or((0, "default".into(), InstanceState64::new(1, 0.85, 1.0, 1.0, [0.1; 4]), [0.0; 3]));
        let cam_target = lab.borrow().world.borrow().camera.target;
        let dx = chosen.3[0] - cam_target[0];
        let dy = chosen.3[1] - cam_target[1];
        let dz = chosen.3[2] - cam_target[2];
        let distance_m = (dx * dx + dy * dy + dz * dz).sqrt();
        (
            entry,
            chosen.2,
            distance_m,
            st.lens,
            matching
                .into_iter()
                .enumerate()
                .map(|(local_idx, (_, id, _, pos))| {
                    (
                        local_idx.to_string(),
                        format!("{id} @ ({:.1}, {:.1}, {:.1})", pos[0], pos[1], pos[2]),
                    )
                })
                .collect::<Vec<_>>(),
        )
    };

    if let Some(inst_sel) = doc().get_element_by_id("native-species-instance") {
        inst_sel.set_text_content(None);
        for (val, label) in matching_ids {
            option(&inst_sel, &val, &label)?;
        }
        if let Ok(sel) = inst_sel.dyn_into::<HtmlSelectElement>() {
            sel.set_value(&state.borrow().selected_placement.to_string());
        }
    }

    if let Some(btn) = doc().get_element_by_id("native-rhizosphere-xray-toggle") {
        btn.set_attribute("aria-pressed", if lens.active { "true" } else { "false" })?;
        btn.set_text_content(Some(if lens.active {
            "Rhizosphere & Mycelium X-Ray Lens: Active (Cutaway)"
        } else {
            "Rhizosphere & Mycelium X-Ray Lens: Off"
        }));
    }

    let layers = compute_rhizosphere_layers(&lens, &entry);
    {
        let world_rc = lab.borrow().world.clone();
        let mut world = world_rc.borrow_mut();
        world.receipt["rhizosphere_lens"] = json!({
            "active": lens.active,
            "ground_opacity": layers.ground_opacity,
            "terrain_cutaway": layers.terrain_cutaway,
            "show_emf_overlay": lens.show_emf_overlay,
            "active_tiers": layers.active_tiers,
            "aboveground_url": layers.aboveground_url,
            "rhizosphere_url": layers.rhizosphere_url,
            "field_emf_url": layers.field_emf_url,
        });
    }

    if let Some(card_host) = doc().get_element_by_id("native-species-card") {
        let html = render_species_card_html_with_lens(&entry, &state64, &lens, distance_m);
        card_host.set_inner_html(&html);

        if let Some(xray_card_btn) = card_host
            .query_selector("[data-action='toggle-rhizosphere-xray']")
            .ok()
            .flatten()
        {
            let s = state.clone();
            let l = lab.clone();
            event(xray_card_btn.unchecked_ref(), "click", move |_| {
                s.borrow_mut().lens.toggle();
                let _ = refresh_species_view(&s, &l);
            })?;
        }
        if let Some(call_btn) = card_host
            .query_selector("[data-action='vocalize-species-call']")
            .ok()
            .flatten()
        {
            let qac_url = call_btn
                .get_attribute("data-qac-url")
                .unwrap_or_default();
            let l = lab.clone();
            event(call_btn.unchecked_ref(), "click", move |_| {
                set_audio_opt_in(true);
                let dispatch = vocalize_species_call(&qac_url, distance_m);
                let world_rc = l.borrow().world.clone();
                world_rc.borrow_mut().receipt["spatial_audio_dispatch"] = json!({
                    "qac_url": dispatch.qac_url,
                    "distance_m": dispatch.distance_m,
                    "gain": dispatch.gain,
                    "attenuation_db": dispatch.attenuation_db,
                    "lowpass_hz": dispatch.lowpass_hz,
                    "audible": dispatch.audible,
                    "status": dispatch.status,
                });
            })?;
        }
    }
    Ok(())
}

// ---------------------------------------------------------------------------
// 6. Pure-Rust Unit Tests (runnable via `rustc --edition 2021 --test`)
// ---------------------------------------------------------------------------

#[cfg(test)]
mod tests {
    use super::*;

    fn sample_flora_entry() -> SpeciesCatalogEntry {
        SpeciesCatalogEntry {
            taxon_iri: "did:q42:taxon:eucalyptus-camaldulensis".into(),
            slug: "eucalyptus-camaldulensis".into(),
            scientific_name: "Eucalyptus camaldulensis Dehnh.".into(),
            vernacular_name: "River Red Gum".into(),
            kingdom: "flora".into(),
            ecological_zone: "Riverine floodplains & riparian corridors".into(),
            catchment_zone: "riparian-floodplain".into(),
            locomotion_class: "sessile_flora".into(),
            custodial_country: "Yorta Yorta Country; Wiradjuri Country; Wurundjeri Country".into(),
            mycorrhizal_type: "ECM (Common Mycorrhizal Network / Hartig net sheaths)".into(),
            rhizosphere_signaling: "0.1-50 Hz mycelial action potentials".into(),
            natural_sway_freq_hz: 0.66,
            url: "/intenteco/eucalyptus-camaldulensis/dist/eucalyptus-camaldulensis.10d".into(),
            remote_url: "https://raw.githubusercontent.com/intenteco/eucalyptus-camaldulensis/main/dist/eucalyptus-camaldulensis.10d".into(),
            sha256: "cc3a64a0353469776de571c27994425226ed92f7507b1b5c1de3bfd473777340".into(),
            taxon_q42_url: "/intenteco/eucalyptus-camaldulensis/dist/taxon.q42".into(),
            field_emf_url: "/intenteco/eucalyptus-camaldulensis/dist/eucalyptus-camaldulensis-field.emf".into(),
            substrate_m64_url: Some("/intenteco/eucalyptus-camaldulensis/dist/eucalyptus-camaldulensis-substrate.m64".into()),
            call_qac_url: None,
            call_qac_remote_url: None,
            call_name: String::new(),
            call_fundamental_hz: 0.0,
            aboveground_lod1_url: Some("/intenteco/eucalyptus-camaldulensis/dist/lods/eucalyptus-camaldulensis-aboveground-lod1.10d".into()),
            rhizosphere_url: Some("/intenteco/eucalyptus-camaldulensis/dist/lods/eucalyptus-camaldulensis-rhizosphere.10d".into()),
            lod0_url: Some("/intenteco/eucalyptus-camaldulensis/dist/lods/eucalyptus-camaldulensis-lod0.10d".into()),
            intenteco_repo_url: "https://github.com/intenteco/eucalyptus-camaldulensis".into(),
            hmc_package_url: "https://github.com/intenteco/eucalyptus-camaldulensis/tree/main/dist".into(),
        }
    }

    fn sample_fauna_entry() -> SpeciesCatalogEntry {
        SpeciesCatalogEntry {
            taxon_iri: "did:q42:taxon:dacelo-novaeguineae".into(),
            slug: "dacelo-novaeguineae".into(),
            scientific_name: "Dacelo novaeguineae (Hermann, 1783)".into(),
            vernacular_name: "Laughing Kookaburra".into(),
            kingdom: "fauna".into(),
            ecological_zone: "Open eucalypt woodlands & riparian perches".into(),
            catchment_zone: "box-gum-woodland".into(),
            locomotion_class: "avian".into(),
            custodial_country: "Wiradjuri Country, Ngunnawal Country, Yuin Country".into(),
            mycorrhizal_type: "n/a".into(),
            rhizosphere_signaling: String::new(),
            natural_sway_freq_hz: 0.0,
            url: "/intenteco/dacelo-novaeguineae/dist/dacelo-novaeguineae.10d".into(),
            remote_url: "https://raw.githubusercontent.com/intenteco/dacelo-novaeguineae/main/dist/dacelo-novaeguineae.10d".into(),
            sha256: "81eaf882df7cb1aeef8251bf5df1443ae3d402ca3e30473dbd7d7c7f350fc065".into(),
            taxon_q42_url: "/intenteco/dacelo-novaeguineae/dist/taxon.q42".into(),
            field_emf_url: "/intenteco/dacelo-novaeguineae/dist/dacelo-novaeguineae-field.emf".into(),
            substrate_m64_url: None,
            call_qac_url: Some("/intenteco/dacelo-novaeguineae/dist/dacelo-novaeguineae-call.qac".into()),
            call_qac_remote_url: Some("https://raw.githubusercontent.com/intenteco/dacelo-novaeguineae/main/dist/dacelo-novaeguineae-call.qac".into()),
            call_name: "Territorial dawn/dusk laughing chorus".into(),
            call_fundamental_hz: 720.0,
            aboveground_lod1_url: None,
            rhizosphere_url: None,
            lod0_url: Some("/intenteco/dacelo-novaeguineae/dist/lods/dacelo-novaeguineae-lod0.10d".into()),
            intenteco_repo_url: "https://github.com/intenteco/dacelo-novaeguineae".into(),
            hmc_package_url: "https://github.com/intenteco/dacelo-novaeguineae/tree/main/dist".into(),
        }
    }

    #[test]
    fn fauna_behavior_transitions_across_all_five_locomotion_classes() {
        for class_name in ["saltatorial", "arboreal", "fossorial", "semi_aquatic", "avian"] {
            let loco = LocomotionClass::from_str(class_name).expect("valid locomotion class");
            let mut seen_states = std::collections::BTreeSet::new();
            for step in 0..25 {
                let sample = evaluate_fauna_behavior(loco, 0x1234_5678, step as f64 * 1.0, 1.0, false);
                seen_states.insert(sample.state.as_str());
                assert!(sample.translation_offset.iter().all(|v| v.is_finite()));
                assert!(sample.yaw_offset_rad.is_finite());
                assert!(sample.pitch_offset_rad.is_finite());
            }
            assert!(
                seen_states.len() >= 4,
                "locomotion {class_name} should transition through 4+ behavioral states, got {seen_states:?}"
            );
        }
    }

    #[test]
    fn fauna_behavior_and_flora_sway_respect_reduced_motion() {
        let fauna = evaluate_fauna_behavior(LocomotionClass::Saltatorial, 99, 5.25, 1.2, true);
        assert_eq!(fauna.translation_offset, [0.0, 0.0, 0.0]);
        assert_eq!(fauna.yaw_offset_rad, 0.0);
        assert_eq!(fauna.pitch_offset_rad, 0.0);

        let sway_active = evaluate_flora_wind_sway(3.5, 42, [15.0, -25.0], 0.66, 20.0, 0.4, false);
        assert!(sway_active[0].abs() > 1e-4 || sway_active[2].abs() > 1e-4);

        let sway_reduced = evaluate_flora_wind_sway(3.5, 42, [15.0, -25.0], 0.66, 20.0, 0.4, true);
        assert_eq!(sway_reduced, [0.0, 0.0, 0.0]);
    }

    #[test]
    fn spatial_qac_audio_enforces_opt_in_and_distance_attenuation() {
        let qac = "/intenteco/dacelo-novaeguineae/dist/dacelo-novaeguineae-call.qac";
        // Without user opt-in gesture, audio is strictly blocked (audible == false, gain == 0.0)
        let blocked = vocalize_species_call_with_opt_in(qac, 5.0, false);
        assert!(!blocked.audible);
        assert!(blocked.user_gesture_required);
        assert_eq!(blocked.gain, 0.0);

        // With opt-in, attenuation decreases monotonically with distance and hits 0 beyond 120m
        let near = vocalize_species_call_with_opt_in(qac, 3.0, true);
        let mid = vocalize_species_call_with_opt_in(qac, 25.0, true);
        let far = vocalize_species_call_with_opt_in(qac, 80.0, true);
        let silent = vocalize_species_call_with_opt_in(qac, 140.0, true);

        assert!(near.audible && (near.gain - 1.0).abs() < 1e-5);
        assert!(mid.audible && mid.gain < near.gain && mid.gain > far.gain);
        assert!(far.audible && far.lowpass_hz < mid.lowpass_hz);
        assert!(!silent.audible && silent.gain == 0.0);
    }

    #[test]
    fn rhizosphere_xray_lens_switches_10d_layers_and_formats_emf_readout() {
        let flora = sample_flora_entry();
        let state64 = InstanceState64::new(0x00ab_cdef_1234_5678, 0.85, 1.1, 1.15, [0.1, 0.05, 0.08, 0.1]);

        let mut lens = RhizosphereLensState::default();
        assert!(!lens.active);
        let off_layers = compute_rhizosphere_layers(&lens, &flora);
        assert_eq!(off_layers.active_tiers, vec!["lod0", "lod1", "lod2"]);
        assert!(!off_layers.terrain_cutaway);
        assert_eq!(off_layers.ground_opacity, 1.0);
        assert!(off_layers.rhizosphere_url.is_none());

        lens.toggle();
        assert!(lens.active);
        assert!(lens.show_emf_overlay);
        assert!(lens.ground_opacity < 0.5);

        let on_layers = compute_rhizosphere_layers(&lens, &flora);
        assert_eq!(on_layers.active_tiers, vec!["rhizosphere", "aboveground_lod1"]);
        assert!(on_layers.terrain_cutaway);
        assert!(on_layers.aboveground_url.as_deref().unwrap().contains("aboveground-lod1.10d"));
        assert!(on_layers.rhizosphere_url.as_deref().unwrap().contains("rhizosphere.10d"));
        assert!(on_layers.field_emf_url.as_deref().unwrap().contains("-field.emf"));

        let readout = format_rhizosphere_emf_readout(&flora, &lens, &state64);
        assert!(readout.is_flora);
        assert!((0.1..=50.0).contains(&readout.dominant_hz));
        assert!(readout.hydraulic_lift_l_per_day > 0.0);
        assert!(readout.glomalin_exudate_mg_c_per_g_day > 0.0);
        assert!(readout.summary_text.contains("0.1-50 Hz"));
        assert!(readout.summary_text.contains("Hydraulic lift"));
    }

    #[test]
    fn species_card_html_includes_q42_provenance_state64_controls_and_escapes_html() {
        let flora = sample_flora_entry();
        let fauna = sample_fauna_entry();
        let state64 = InstanceState64::new(0xdead_beef_cafe_0001, 0.90, 1.05, 1.10, [0.12, 0.04, 0.08, 0.09]);

        let flora_html = render_species_card_html(&flora, &state64);
        assert!(flora_html.contains("River Red Gum"));
        assert!(flora_html.contains("Eucalyptus camaldulensis Dehnh."));
        assert!(flora_html.contains("did:q42:taxon:eucalyptus-camaldulensis"));
        assert!(flora_html.contains("Yorta Yorta Country"));
        assert!(flora_html.contains("0xdeadbeefcafe0001"));
        assert!(flora_html.contains("toggle-rhizosphere-xray"));
        assert!(flora_html.contains("https://github.com/intenteco/eucalyptus-camaldulensis"));
        assert!(flora_html.contains("taxon.q42"));

        let fauna_html = render_species_card_html(&fauna, &state64);
        assert!(fauna_html.contains("Laughing Kookaburra"));
        assert!(fauna_html.contains("vocalize-species-call"));
        assert!(fauna_html.contains("dacelo-novaeguineae-call.qac"));

        // Verify HTML escaping prevents XSS injection
        let mut xss = fauna.clone();
        xss.vernacular_name = "<script>alert(1)</script>".into();
        let escaped = render_species_card_html(&xss, &state64);
        assert!(!escaped.contains("<script>"));
        assert!(escaped.contains("&lt;script&gt;alert(1)&lt;/script&gt;"));
    }

    #[test]
    fn ecological_tour_stops_cover_all_six_stops_with_valid_coordinates_and_modes() {
        assert_eq!(ECOLOGICAL_TOUR_STOPS.len(), 6);
        let expected_ids = [
            "campsite-clearing",
            "platypus-creek",
            "mountain-ash-canopy",
            "rhizosphere-xray",
            "kangaroo-meadow",
            "earth-solar-orbit",
        ];
        for id in expected_ids {
            let stop = find_tour_stop(id).expect("expected tour stop");
            assert!(stop.target.iter().all(|v| v.is_finite()));
            assert!(stop.yaw.is_finite() && stop.pitch.is_finite());
            assert!(stop.zoom.is_finite() && stop.zoom > 0.0 && stop.zoom <= 10000.0);
            assert!(["observe", "walk", "fly"].contains(&stop.movement_mode));
            assert!(!stop.summary_en_au.is_empty());
        }
        let camp = find_tour_stop("campsite-clearing").unwrap();
        assert_eq!(camp.target[0], -22.0);
        assert_eq!(camp.target[2], 28.0);
        assert_eq!(camp.ocs_regime, "LocalEnu");

        let rhizo = find_tour_stop("rhizosphere-xray").unwrap();
        assert!(rhizo.activate_rhizosphere);

        let orbit = find_tour_stop("earth-solar-orbit").unwrap();
        assert_eq!(orbit.movement_mode, "fly");
        assert!(orbit.altitude_m.unwrap_or(0.0) >= ORBITAL_STARFIELD_ALTITUDE_M);
    }

    #[test]
    fn field_guide_discovery_tracks_17_species_and_escapes_drawer_html() {
        let mut discovery = FieldGuideDiscovery::default();
        assert_eq!(discovery.total_species, 17);
        assert_eq!(discovery.badge_label(), "Discovered 0 / 17 species");
        assert!(!discovery.mark_discovered("   "));
        assert!(discovery.mark_discovered("eucalyptus-camaldulensis"));
        assert!(!discovery.mark_discovered("eucalyptus-camaldulensis"));
        assert!(discovery.mark_discovered("dacelo-novaeguineae"));
        assert_eq!(discovery.discovered_count(), 2);
        assert_eq!(discovery.badge_label(), "Discovered 2 / 17 species");

        let flora = sample_flora_entry();
        let mut fauna = sample_fauna_entry();
        fauna.vernacular_name = "Kookaburra <img src=x onerror=alert(1)>".into();
        fauna.taxon_q42_url = "javascript:alert(1)".into();
        let entries = vec![flora.clone(), fauna];

        let all_html = render_field_guide_drawer_html(&entries, &discovery, None);
        assert!(all_html.contains("River Red Gum"));
        assert!(all_html.contains("Discovered"));
        assert!(all_html.contains("ECM (Common Mycorrhizal Network"));
        assert!(all_html.contains("https://github.com/intenteco/eucalyptus-camaldulensis/tree/main/dist"));
        assert!(!all_html.contains("<img"));
        assert!(all_html.contains("&lt;img src=x onerror=alert(1)&gt;"));
        assert!(!all_html.contains("javascript:alert(1)"));

        let flora_only = render_field_guide_drawer_html(&entries, &discovery, Some("flora"));
        assert!(flora_only.contains("River Red Gum"));
        assert!(!flora_only.contains("Kookaburra"));

        let fauna_only = render_field_guide_drawer_html(&entries, &discovery, Some("fauna"));
        assert!(!fauna_only.contains("River Red Gum"));
        assert!(fauna_only.contains("vocalize-qac"));
    }

    #[test]
    fn time_of_day_presets_and_orbital_starfield_altitude_cue() {
        let mut sky_ids = std::collections::BTreeSet::new();
        for preset in TimeOfDayPreset::all() {
            assert_eq!(TimeOfDayPreset::from_str(preset.as_str()), Some(*preset));
            let spec = preset.spec();
            assert!(!spec.starfield_active);
            assert!(spec.clear_color.iter().all(|v| v.is_finite() && (0.0..=1.0).contains(v)));
            assert!(spec.sun.iter().all(|v| v.is_finite()));
            sky_ids.insert(spec.id);

            let ground = resolve_atmosphere_for_altitude(*preset, 150.0);
            assert!(!ground.starfield_active);
            assert_eq!(ground.sky_preset, spec.sky_preset);

            let orbit = resolve_atmosphere_for_altitude(*preset, 12_500_000.0);
            assert!(orbit.starfield_active);
            assert_eq!(orbit.sky_preset, 0);
            assert!(orbit.clear_color[0] < 0.05 && orbit.clear_color[2] < 0.1);
        }
        assert_eq!(sky_ids.len(), 4);
    }

    #[test]
    fn streaming_status_pill_formats_backend_tier_mobile_and_species_count() {
        assert_eq!(
            format_streaming_status_pill(Some("webgpu"), Some("tier-a"), false, 13, None),
            "WebGPU · Tier A · 13 species"
        );
        assert_eq!(
            format_streaming_status_pill(Some("webgl2"), Some("tier-c"), true, 0, None),
            "WebGL2 · Tier C · Mobile"
        );
        assert_eq!(
            format_streaming_status_pill(Some("webgl2"), Some("tier-c"), true, 13, Some("Streaming")),
            "WebGL2 · Tier C · Mobile · Streaming · 13 species"
        );
    }
}
