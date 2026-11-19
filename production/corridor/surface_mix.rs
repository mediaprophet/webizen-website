//! Volume-weighted index of a queried mixture, then the Fresnel factor of that index.
//!
//! The parts, the pore fluid, and the saturation come from the surface ontology.
//! This module does not name a colour.

#[derive(Clone, Copy, Debug, PartialEq)]
pub struct MixPart {
    pub fraction: f64,
    pub refractive_index: f64,
    pub porosity: f64,
}

#[derive(Clone, Copy, Debug, PartialEq)]
pub struct Mixture {
    pub n_grain: f64,
    pub porosity: f64,
    pub n_pore: f64,
    pub n_eff: f64,
    pub fresnel_r: f64,
}

pub fn derive_mixture(
    parts: &[MixPart],
    pore_refractive_index: f64,
    saturation: f64,
    air_refractive_index: f64,
) -> Option<Mixture> {
    if parts.is_empty() || !(0.0..=1.0).contains(&saturation) {
        return None;
    }
    if !refractive_ok(pore_refractive_index) || !refractive_ok(air_refractive_index) {
        return None;
    }
    let mut fraction = 0.0;
    let mut n_grain = 0.0;
    let mut porosity = 0.0;
    for part in parts {
        if !(part.fraction > 0.0) || !part.fraction.is_finite() {
            return None;
        }
        if !refractive_ok(part.refractive_index) || !(0.0..=1.0).contains(&part.porosity) {
            return None;
        }
        fraction += part.fraction;
        n_grain += part.fraction * part.refractive_index;
        porosity += part.fraction * part.porosity;
    }
    if (fraction - 1.0).abs() > 1.0e-4 {
        return None;
    }
    n_grain /= fraction;
    porosity /= fraction;
    let n_pore = saturation * pore_refractive_index + (1.0 - saturation) * air_refractive_index;
    let n_eff = (1.0 - porosity) * n_grain + porosity * n_pore;
    if !n_eff.is_finite() || n_eff < 1.0 {
        return None;
    }
    let boundary = (n_eff - 1.0) / (n_eff + 1.0);
    let fresnel_r = boundary * boundary;
    if !(0.0..=1.0).contains(&fresnel_r) {
        return None;
    }
    Some(Mixture {
        n_grain,
        porosity,
        n_pore,
        n_eff,
        fresnel_r,
    })
}

fn refractive_ok(n: f64) -> bool {
    n.is_finite() && n >= 1.0
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn dirt_gravel_and_sealed_follow_the_queried_fractions() {
        let dirt = derive_mixture(
            &[
                MixPart { fraction: 0.5, refractive_index: 1.45, porosity: 0.55 },
                MixPart { fraction: 0.3, refractive_index: 1.56, porosity: 0.42 },
                MixPart { fraction: 0.2, refractive_index: 1.54, porosity: 0.32 },
            ],
            1.333,
            0.35,
            1.0,
        )
        .unwrap();
        let gravel = derive_mixture(
            &[MixPart { fraction: 1.0, refractive_index: 1.54, porosity: 0.32 }],
            1.0,
            0.0,
            1.0,
        )
        .unwrap();
        let sealed = derive_mixture(
            &[
                MixPart { fraction: 0.7, refractive_index: 1.54, porosity: 0.32 },
                MixPart { fraction: 0.3, refractive_index: 1.56, porosity: 0.42 },
            ],
            1.0,
            0.0,
            1.0,
        )
        .unwrap();
        assert!((dirt.n_eff - 1.3222).abs() < 1.0e-3, "{}", dirt.n_eff);
        assert!((gravel.n_eff - 1.3672).abs() < 1.0e-3, "{}", gravel.n_eff);
        assert!((sealed.n_eff - 1.3549).abs() < 1.0e-3, "{}", sealed.n_eff);
        assert!(gravel.fresnel_r > dirt.fresnel_r);
        assert!(gravel.fresnel_r > sealed.fresnel_r);
        assert!(dirt.fresnel_r < 0.03 && gravel.fresnel_r < 0.03);
    }

    #[test]
    fn a_fraction_that_does_not_sum_to_one_is_refused() {
        assert!(derive_mixture(
            &[MixPart { fraction: 0.4, refractive_index: 1.54, porosity: 0.32 }],
            1.0,
            0.0,
            1.0,
        )
        .is_none());
    }
}
