//! EWM-07: auditable `ewm_static` prefixes. Same-origin `/packages/` and
//! `/production/` already bypass inside FetchRangeService. Live GIS stays
//! default-deny until opt-in.
#![allow(dead_code)]

use qualia_core_db::net::disclosure::NetworkDisclosureRegistry;
use qualia_core_db::net::fetch_range_wasm::{authorize_url, EWM_RANGE_ADAPTER, STATIC_PREFIXES};

pub fn empty_registry() -> NetworkDisclosureRegistry {
    NetworkDisclosureRegistry::new()
}

pub fn ewm_static_prefixes() -> &'static [&'static str] {
    STATIC_PREFIXES
}

pub fn live_gis_denied(url: &str) -> bool {
    authorize_url(&empty_registry(), EWM_RANGE_ADAPTER, url).is_err()
}