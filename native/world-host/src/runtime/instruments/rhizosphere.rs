//! Which fitted joints a view draws.
//!
//! Normal view is the collar and the wood above it. Rhizosphere view is the
//! fitted root skeleton. Above-collar buttress rings stay in the rhizosphere
//! set because they are root-role and frozen. The evidence painter omits
//! them so that frame does not show root wood on the surface.

include!("rhizosphere_joints.inc.rs");

/// What the host draws for one fitted gum.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum DrawView {
    /// Collar upward: bole, boughs, and sprigs.
    Normal,
    /// Fitted root parts, including anchors and the deeper orders.
    Rhizosphere,
}

/// Root wood is a root-role joint, a frozen anchor (`inv_mass` 0), or any
/// joint authored at or below the soil collar.
pub fn is_root_wood(role: u8, authored_y: f64, inv_mass: f64) -> bool {
    role == 0 || inv_mass == 0.0 || authored_y <= 0.0
}

/// Lens filter. The two views are disjoint for the joints this table stores.
pub fn visible(view: DrawView, role: u8, authored_y: f64, inv_mass: f64) -> bool {
    let root = is_root_wood(role, authored_y, inv_mass);
    match view {
        DrawView::Normal => !root,
        DrawView::Rhizosphere => root,
    }
}
