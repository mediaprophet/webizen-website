#[cfg(not(test))]
use super::*;

pub const MAGIC_10D_BYTES: [u8; 4] = *b"10d\0";
pub const MAGIC_Q42_PREFIX: [u8; 3] = *b"Q42";
pub const MAGIC_EMF_BYTES: [u8; 8] = *b"Q42EMF64";
pub const MAGIC_M64_BYTES: [u8; 4] = *b"m64\0";
pub const MAGIC_E64_BYTES: [u8; 4] = *b"e64\0";
pub const MAGIC_QAC_BYTES: [u8; 8] = *b"qac\0\0\0\0\0";

/// Trusted multi-source resolution prefixes mirroring `data/ewm/origins-allowlist.json`.
/// Tier 1 (zero-CORS same-origin) precedes Tier 2 (CORS-enabled HTTPS mirrors).
pub const DEFAULT_ALLOWED_ORIGINS: &[&str] = &[
    "/production/",
    "/packages/",
    "/data/ewm/",
    "/intenteco/",
    "https://raw.githubusercontent.com/intenteco/",
    "https://assets.webizen.au/",
    "https://r2.intenteco.org/",
];

const RAW_INTENTECO_PREFIX: &str = "https://raw.githubusercontent.com/intenteco/";

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum AssetFormat {
    TenD,
    Q42,
    Emf,
    M64,
    Qac,
}

pub fn default_allowlist() -> Vec<String> {
    DEFAULT_ALLOWED_ORIGINS
        .iter()
        .map(|s| (*s).to_string())
        .collect()
}

pub fn is_valid_sha256_hex(sha256: &str) -> bool {
    sha256.len() == 64 && sha256.bytes().all(|b| b.is_ascii_hexdigit())
}

fn has_forbidden_url_pattern(url: &str) -> bool {
    if url.is_empty()
        || url.bytes().any(|b| b.is_ascii_control() || b.is_ascii_whitespace())
        || url.starts_with("//")
        || url.contains("..")
        || url.contains('\\')
    {
        return true;
    }
    let lower = url.to_ascii_lowercase();
    if lower.starts_with("http://")
        || lower.contains("%2e%2e")
        || lower.contains("%5c")
        || lower.contains("github.com/")
        || lower.contains("/releases/download/")
    {
        return true;
    }
    if let Some(after_scheme) = lower.strip_prefix("https://") {
        let authority = after_scheme.split('/').next().unwrap_or("");
        if authority.is_empty() || authority.contains('@') || after_scheme.contains("//") {
            return true;
        }
    } else if url.contains("://") || url.contains("//") {
        return true;
    }
    false
}

fn is_safe_allowlist_prefix(prefix: &str) -> bool {
    if prefix.is_empty() || has_forbidden_url_pattern(prefix) {
        return false;
    }
    if prefix.starts_with('/') {
        prefix.ends_with('/')
    } else if let Some(rest) = prefix.strip_prefix("https://") {
        // Must include host and trailing slash so prefix matching cannot match lookalike domains.
        let mut parts = rest.splitn(2, '/');
        let host = parts.next().unwrap_or("");
        !host.is_empty() && prefix.ends_with('/')
    } else {
        false
    }
}

/// Returns `true` iff `url` is a safe, non-traversal, non-cleartext, non-GitHub-release URL
/// matching at least one safe prefix in `allowlist`.
pub fn is_allowed_asset_url(url: &str, allowlist: &[String]) -> bool {
    if has_forbidden_url_pattern(url) {
        return false;
    }
    allowlist
        .iter()
        .filter(|prefix| is_safe_allowlist_prefix(prefix))
        .any(|prefix| url.starts_with(prefix.as_str()) && url.len() > prefix.len())
}

fn push_unique_allowed(out: &mut Vec<String>, candidate: String, allowlist: &[String]) {
    if is_allowed_asset_url(&candidate, allowlist) && !out.iter().any(|u| u == &candidate) {
        out.push(candidate);
    }
}

/// Resolve an ordered, deduplicated list of allowlisted candidate URLs for a content-addressed
/// asset. Tier 1 same-origin paths (including `/intenteco/`) are always tried before Tier 2
/// CORS-enabled remote mirrors (`raw.githubusercontent.com/intenteco/`, Cloudflare R2, Webizen
/// Hosts). Requires a valid 64-character SHA-256 hex digest.
pub fn resolve_asset_candidates(
    logical_url: &str,
    sha256: &str,
    allowlist: &[String],
) -> Result<Vec<String>, String> {
    if !is_valid_sha256_hex(sha256) {
        return Err("Asset resolution requires a valid 64-character SHA-256 digest".into());
    }
    if has_forbidden_url_pattern(logical_url) {
        return Err(format!("Rejected forbidden asset URL: {logical_url}"));
    }

    let mut candidates = Vec::new();

    if let Some(rest) = logical_url.strip_prefix("/intenteco/") {
        let mut parts = rest.splitn(2, '/');
        let slug = parts.next().unwrap_or("");
        let sub_path = parts.next().unwrap_or("");
        if slug.is_empty() || sub_path.is_empty() {
            return Err(format!("Incomplete /intenteco/ asset path: {logical_url}"));
        }
        // Tier 1: same-origin local zero-copy mount or same-origin Pages mount.
        push_unique_allowed(&mut candidates, logical_url.to_string(), allowlist);
        // Tier 2: CORS-enabled remote mirrors.
        for prefix in allowlist {
            if !is_safe_allowlist_prefix(prefix) || !prefix.starts_with("https://") {
                continue;
            }
            if prefix == RAW_INTENTECO_PREFIX {
                let raw_url = format!("{RAW_INTENTECO_PREFIX}{slug}/main/{sub_path}");
                push_unique_allowed(&mut candidates, raw_url, allowlist);
            } else {
                let mirror_url = format!("{prefix}intenteco/{slug}/{sub_path}");
                push_unique_allowed(&mut candidates, mirror_url, allowlist);
            }
        }
    } else if let Some(rest) = logical_url.strip_prefix(RAW_INTENTECO_PREFIX) {
        let mut parts = rest.splitn(3, '/');
        let slug = parts.next().unwrap_or("");
        let branch = parts.next().unwrap_or("");
        let sub_path = parts.next().unwrap_or("");
        if !slug.is_empty() && !branch.is_empty() && !sub_path.is_empty() {
            // Prefer Tier 1 local zero-copy /intenteco/ mount before remote mirror.
            let local_url = format!("/intenteco/{slug}/{sub_path}");
            push_unique_allowed(&mut candidates, local_url, allowlist);
        }
        push_unique_allowed(&mut candidates, logical_url.to_string(), allowlist);
        if !slug.is_empty() && !sub_path.is_empty() {
            for prefix in allowlist {
                if !is_safe_allowlist_prefix(prefix)
                    || !prefix.starts_with("https://")
                    || prefix == RAW_INTENTECO_PREFIX
                {
                    continue;
                }
                let mirror_url = format!("{prefix}intenteco/{slug}/{sub_path}");
                push_unique_allowed(&mut candidates, mirror_url, allowlist);
            }
        }
    } else if logical_url.starts_with('/') {
        // Tier 1 same-origin (/production/, /packages/, /data/ewm/).
        push_unique_allowed(&mut candidates, logical_url.to_string(), allowlist);
        let rel = logical_url.trim_start_matches('/');
        for prefix in allowlist {
            if !is_safe_allowlist_prefix(prefix)
                || !prefix.starts_with("https://")
                || prefix == RAW_INTENTECO_PREFIX
            {
                continue;
            }
            let mirror_url = format!("{prefix}{rel}");
            push_unique_allowed(&mut candidates, mirror_url, allowlist);
        }
    } else if logical_url.starts_with("https://") {
        push_unique_allowed(&mut candidates, logical_url.to_string(), allowlist);
    }

    if candidates.is_empty() {
        return Err(format!(
            "No allowlisted asset candidates resolved for {logical_url}"
        ));
    }
    Ok(candidates)
}

pub fn format_from_path(url_or_path: &str) -> Option<AssetFormat> {
    let clean = url_or_path
        .split('?')
        .next()
        .unwrap_or(url_or_path)
        .split('#')
        .next()
        .unwrap_or(url_or_path);
    let lower = clean.to_ascii_lowercase();
    if lower.ends_with(".10d") {
        Some(AssetFormat::TenD)
    } else if lower.ends_with(".q42") {
        Some(AssetFormat::Q42)
    } else if lower.ends_with(".emf") {
        Some(AssetFormat::Emf)
    } else if lower.ends_with(".m64") {
        Some(AssetFormat::M64)
    } else if lower.ends_with(".qac") {
        Some(AssetFormat::Qac)
    } else {
        None
    }
}

pub fn check_magic(format: AssetFormat, bytes: &[u8]) -> Result<(), String> {
    match format {
        AssetFormat::TenD => {
            if bytes.len() >= 4 && bytes[..4] == MAGIC_10D_BYTES {
                Ok(())
            } else {
                Err("Magic check failed: expected .10d (10d\\0)".into())
            }
        }
        AssetFormat::Q42 => {
            if bytes.len() >= 8 && bytes[..3] == MAGIC_Q42_PREFIX {
                Ok(())
            } else {
                Err("Magic check failed: expected .q42 (Q42)".into())
            }
        }
        AssetFormat::Emf => {
            if bytes.len() >= 8 && bytes[..8] == MAGIC_EMF_BYTES {
                Ok(())
            } else {
                Err("Magic check failed: expected .emf (Q42EMF64)".into())
            }
        }
        AssetFormat::M64 => {
            if bytes.len() >= 4
                && (bytes[..4] == MAGIC_M64_BYTES || bytes[..4] == MAGIC_E64_BYTES)
            {
                Ok(())
            } else {
                Err("Magic check failed: expected .m64 (m64\\0 or e64\\0)".into())
            }
        }
        AssetFormat::Qac => {
            if bytes.len() >= 8 && bytes[..8] == MAGIC_QAC_BYTES {
                Ok(())
            } else {
                Err("Magic check failed: expected .qac (qac\\0\\0\\0\\0\\0)".into())
            }
        }
    }
}

#[cfg(not(test))]
pub fn sha256_hex(bytes: &[u8]) -> String {
    format!("{:x}", Sha256::digest(bytes))
}

#[cfg(test)]
pub fn sha256_hex(data: &[u8]) -> String {
    const K: [u32; 64] = [
        0x428a2f98, 0x71374491, 0xb5c0fbcf, 0xe9b5dba5, 0x3956c25b, 0x59f111f1, 0x923f82a4,
        0xab1c5ed5, 0xd807aa98, 0x12835b01, 0x243185be, 0x550c7dc3, 0x72be5d74, 0x80deb1fe,
        0x9bdc06a7, 0xc19bf174, 0xe49b69c1, 0xefbe4786, 0x0fc19dc6, 0x240ca1cc, 0x2de92c6f,
        0x4a7484aa, 0x5cb0a9dc, 0x76f988da, 0x983e5152, 0xa831c66d, 0xb00327c8, 0xbf597fc7,
        0xc6e00bf3, 0xd5a79147, 0x06ca6351, 0x14292967, 0x27b70a85, 0x2e1b2138, 0x4d2c6dfc,
        0x53380d13, 0x650a7354, 0x766a0abb, 0x81c2c92e, 0x92722c85, 0xa2bfe8a1, 0xa81a664b,
        0xc24b8b70, 0xc76c51a3, 0xd192e819, 0xd6990624, 0xf40e3585, 0x106aa070, 0x19a4c116,
        0x1e376c08, 0x2748774c, 0x34b0bcb5, 0x391c0cb3, 0x4ed8aa4a, 0x5b9cca4f, 0x682e6ff3,
        0x748f82ee, 0x78a5636f, 0x84c87814, 0x8cc70208, 0x90befffa, 0xa4506ceb, 0xbef9a3f7,
        0xc67178f2,
    ];
    let mut h: [u32; 8] = [
        0x6a09e667, 0xbb67ae85, 0x3c6ef372, 0xa54ff53a, 0x510e527f, 0x9b05688c, 0x1f83d9ab,
        0x5be0cd19,
    ];
    let bit_len = (data.len() as u64) * 8;
    let mut msg = data.to_vec();
    msg.push(0x80);
    while (msg.len() % 64) != 56 {
        msg.push(0);
    }
    msg.extend_from_slice(&bit_len.to_be_bytes());
    for chunk in msg.chunks_exact(64) {
        let mut w = [0u32; 64];
        for i in 0..16 {
            w[i] = u32::from_be_bytes([
                chunk[i * 4],
                chunk[i * 4 + 1],
                chunk[i * 4 + 2],
                chunk[i * 4 + 3],
            ]);
        }
        for i in 16..64 {
            let s0 = w[i - 15].rotate_right(7) ^ w[i - 15].rotate_right(18) ^ (w[i - 15] >> 3);
            let s1 = w[i - 2].rotate_right(17) ^ w[i - 2].rotate_right(19) ^ (w[i - 2] >> 10);
            w[i] = w[i - 16]
                .wrapping_add(s0)
                .wrapping_add(w[i - 7])
                .wrapping_add(s1);
        }
        let [mut a, mut b, mut c, mut d, mut e, mut f, mut g, mut hh] = h;
        for i in 0..64 {
            let s1 = e.rotate_right(6) ^ e.rotate_right(11) ^ e.rotate_right(25);
            let ch = (e & f) ^ ((!e) & g);
            let t1 = hh
                .wrapping_add(s1)
                .wrapping_add(ch)
                .wrapping_add(K[i])
                .wrapping_add(w[i]);
            let s0 = a.rotate_right(2) ^ a.rotate_right(13) ^ a.rotate_right(22);
            let maj = (a & b) ^ (a & c) ^ (b & c);
            let t2 = s0.wrapping_add(maj);
            hh = g;
            g = f;
            f = e;
            e = d.wrapping_add(t1);
            d = c;
            c = b;
            b = a;
            a = t1.wrapping_add(t2);
        }
        for (slot, delta) in h.iter_mut().zip([a, b, c, d, e, f, g, hh]) {
            *slot = slot.wrapping_add(delta);
        }
    }
    let mut out = String::with_capacity(64);
    for word in h {
        out.push_str(&format!("{word:08x}"));
    }
    out
}

/// Mandatory SHA-256 verification and binary container magic check before admitting any
/// `.10d`, `.q42`, `.emf`, `.m64`, or `.qac` payload to the portal or host graph.
pub fn verify_sha256_and_magic(
    id: &str,
    url_or_path: &str,
    bytes: &[u8],
    expected_sha256: &str,
) -> Result<AssetFormat, String> {
    if !is_valid_sha256_hex(expected_sha256) {
        return Err(format!(
            "Asset {id}: invalid expected SHA-256 digest"
        ));
    }
    let actual = sha256_hex(bytes);
    if !actual.eq_ignore_ascii_case(expected_sha256) {
        return Err(format!(
            "Asset {id}: SHA-256 digest mismatch (expected {expected_sha256}, got {actual})"
        ));
    }
    let format = format_from_path(url_or_path).ok_or_else(|| {
        format!("Asset {id}: unsupported container extension for {url_or_path}")
    })?;
    check_magic(format, bytes).map_err(|e| format!("Asset {id}: {e}"))?;
    Ok(format)
}

#[cfg(not(test))]
#[allow(dead_code)]
pub fn parse_allowlist_json(raw: &str) -> Result<Vec<String>, String> {
    let val: Value = serde_json::from_str(raw).map_err(|e| e.to_string())?;
    let mut out = Vec::new();
    if let Some(arr) = val.get("allowed_prefixes").and_then(|v| v.as_array()) {
        for item in arr {
            if let Some(s) = item.as_str() {
                if is_safe_allowlist_prefix(s) && !out.iter().any(|p| p == s) {
                    out.push(s.to_string());
                }
            }
        }
    }
    if out.is_empty() {
        return Err("origins-allowlist.json contains no valid prefixes".into());
    }
    Ok(out)
}

#[cfg(not(test))]
/// Full admission gate: verifies SHA-256 + binary magic for `.10d`, `.q42`, `.emf`, `.m64`, `.qac`,
/// and runs the native `.10d` header/CRC32C/provenance gate for `.10d` payloads.
pub(super) fn admit_verified_bytes(
    id: &str,
    url_or_path: &str,
    bytes: &[u8],
    expected_sha256: &str,
) -> Result<AssetFormat, JsValue> {
    let format = verify_sha256_and_magic(id, url_or_path, bytes, expected_sha256).map_err(err)?;
    if format == AssetFormat::TenD {
        check(id, bytes)?;
    }
    Ok(format)
}

#[cfg(not(test))]
/// Diagnose the existing native admission gate; never alter its flags or payload.
pub(super) fn check(id: &str, bytes: &[u8]) -> Result<(), JsValue> {
    use qualia_core_db::container_10d as codec;
    let header =
        codec::Container10dHeader::parse(bytes).map_err(|e| err(format!("Asset {id}: {e}")))?;
    codec::integrity::check_whole_file_crc32c(bytes)
        .map_err(|e| err(format!("Asset {id}: {e}")))?;
    let sections =
        codec::parse_section_table(bytes, &header).map_err(|e| err(format!("Asset {id}: {e}")))?;
    if header.flags & codec::FLAG_DEFAULT_DISPOSITION_REFUSE == 0 {
        return Ok(());
    }
    let descriptor = sections
        .iter()
        .find(|d| d.typ() == Some(codec::SectionType::ProvenanceSidecar))
        .ok_or_else(|| {
            err(format!(
                "Asset {id} requires embedded source provenance for native scene admission"
            ))
        })?;
    let begin = descriptor.byte_offset as usize;
    let end = begin
        .checked_add(descriptor.byte_length as usize)
        .filter(|end| *end <= bytes.len())
        .ok_or(err("Invalid provenance section bounds"))?;
    let provenance = codec::decode_provenance_section(&bytes[begin..end])
        .map_err(|e| err(format!("Asset {id} provenance: {e}")))?;
    codec::validate_provenance(&provenance)
        .map_err(|e| err(format!("Asset {id} provenance: {e}")))?;
    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;

    const SAMPLE_SHA: &str =
        "cc3a64a0353469776de571c27994425226ed92f7507b1b5c1de3bfd473777340";

    #[test]
    fn allows_tier1_same_origin_and_tier2_cors_mirrors() {
        let allow = default_allowlist();
        assert!(is_allowed_asset_url(
            "/production/lush-woodland/scene.q42",
            &allow
        ));
        assert!(is_allowed_asset_url(
            "/packages/woodland/organ.10d",
            &allow
        ));
        assert!(is_allowed_asset_url("/data/ewm/species-catalog.q42", &allow));
        assert!(is_allowed_asset_url(
            "/intenteco/eucalyptus-camaldulensis/dist/eucalyptus-camaldulensis.10d",
            &allow
        ));
        assert!(is_allowed_asset_url(
            "https://raw.githubusercontent.com/intenteco/eucalyptus-camaldulensis/main/dist/lods/eucalyptus-camaldulensis-lod1.10d",
            &allow
        ));
        assert!(is_allowed_asset_url(
            "https://assets.webizen.au/intenteco/eucalyptus-camaldulensis/dist/taxon.q42",
            &allow
        ));
        assert!(is_allowed_asset_url(
            "https://r2.intenteco.org/intenteco/eucalyptus-camaldulensis/dist/field.emf",
            &allow
        ));
    }

    #[test]
    fn rejects_github_releases_cleartext_traversal_and_unknown_hosts() {
        let allow = default_allowlist();
        assert!(!is_allowed_asset_url(
            "https://github.com/intenteco/eucalyptus-camaldulensis/releases/download/v1/tree.10d",
            &allow
        ));
        assert!(!is_allowed_asset_url(
            "http://raw.githubusercontent.com/intenteco/tree/main/dist/tree.10d",
            &allow
        ));
        assert!(!is_allowed_asset_url(
            "/intenteco/../etc/passwd",
            &allow
        ));
        assert!(!is_allowed_asset_url(
            "/intenteco/eucalyptus-camaldulensis/dist\\tree.10d",
            &allow
        ));
        assert!(!is_allowed_asset_url(
            "//raw.githubusercontent.com/intenteco/tree/main/dist/tree.10d",
            &allow
        ));
        assert!(!is_allowed_asset_url(
            "https://evil.example.com/intenteco/tree.10d",
            &allow
        ));
        assert!(!is_allowed_asset_url(
            "https://assets.webizen.au.evil.com/tree.10d",
            &allow
        ));
        assert!(!is_allowed_asset_url(
            "https://raw.githubusercontent.com@evil.com/intenteco/tree.10d",
            &allow
        ));
    }

    #[test]
    fn resolves_intenteco_tier1_before_tier2_mirrors() {
        let allow = default_allowlist();
        let candidates = resolve_asset_candidates(
            "/intenteco/eucalyptus-camaldulensis/dist/lods/eucalyptus-camaldulensis-lod1.10d",
            SAMPLE_SHA,
            &allow,
        )
        .expect("should resolve");
        assert_eq!(
            candidates[0],
            "/intenteco/eucalyptus-camaldulensis/dist/lods/eucalyptus-camaldulensis-lod1.10d"
        );
        assert_eq!(
            candidates[1],
            "https://raw.githubusercontent.com/intenteco/eucalyptus-camaldulensis/main/dist/lods/eucalyptus-camaldulensis-lod1.10d"
        );
        assert_eq!(
            candidates[2],
            "https://assets.webizen.au/intenteco/eucalyptus-camaldulensis/dist/lods/eucalyptus-camaldulensis-lod1.10d"
        );
        assert_eq!(
            candidates[3],
            "https://r2.intenteco.org/intenteco/eucalyptus-camaldulensis/dist/lods/eucalyptus-camaldulensis-lod1.10d"
        );
    }

    #[test]
    fn resolves_remote_raw_url_with_local_tier1_first() {
        let allow = default_allowlist();
        let candidates = resolve_asset_candidates(
            "https://raw.githubusercontent.com/intenteco/banksia-integrifolia/main/dist/taxon.q42",
            SAMPLE_SHA,
            &allow,
        )
        .expect("should resolve");
        assert_eq!(
            candidates[0],
            "/intenteco/banksia-integrifolia/dist/taxon.q42"
        );
        assert_eq!(
            candidates[1],
            "https://raw.githubusercontent.com/intenteco/banksia-integrifolia/main/dist/taxon.q42"
        );
    }

    #[test]
    fn rejects_resolution_without_valid_sha256_or_with_forbidden_url() {
        let allow = default_allowlist();
        assert!(resolve_asset_candidates("/intenteco/a/dist/a.10d", "not-a-sha", &allow).is_err());
        assert!(resolve_asset_candidates(
            "https://github.com/intenteco/a/releases/download/v1/a.10d",
            SAMPLE_SHA,
            &allow
        )
        .is_err());
        assert!(resolve_asset_candidates("/production/../secret.q42", SAMPLE_SHA, &allow).is_err());
    }

    #[test]
    fn verifies_sha256_and_magic_for_all_five_container_formats() {
        let cases: &[(&str, &[u8], AssetFormat)] = &[
            ("tree.10d", b"10d\0payload-10d", AssetFormat::TenD),
            ("catalog.q42", b"Q42\0\x04\x00\x1b\x00payload", AssetFormat::Q42),
            ("field.emf", b"Q42EMF64payload", AssetFormat::Emf),
            ("substrate.m64", b"m64\0payload", AssetFormat::M64),
            ("call.qac", b"qac\0\0\0\0\0payload", AssetFormat::Qac),
        ];
        for (path, payload, expected_fmt) in cases {
            let digest = sha256_hex(payload);
            let got = verify_sha256_and_magic("test", path, payload, &digest)
                .expect("valid payload must pass");
            assert_eq!(got, *expected_fmt);

            // Corrupt digest fails closed.
            let bad_digest = "0".repeat(64);
            assert!(verify_sha256_and_magic("test", path, payload, &bad_digest).is_err());

            // Corrupt magic fails closed even with matching digest.
            let bad_bytes = b"BAD!magic1234567";
            let bad_magic_sha = sha256_hex(bad_bytes);
            assert!(verify_sha256_and_magic("test", path, bad_bytes, &bad_magic_sha).is_err());
        }
    }
}
