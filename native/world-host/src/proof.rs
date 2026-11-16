//! Bounded proof computations using the existing QualiaDB engines.
use qualia_core_db::{q42::q42_volume::WasmSliceQ42Reader, NQuin};
use serde_json::{json, Value};
use std::collections::BTreeMap;

pub struct Dataset {
    pub quins: Vec<NQuin>,
    pub names: BTreeMap<u64, String>,
}
impl Dataset {
    pub fn open(bytes: &[u8]) -> Result<Self, String> {
        let reader = WasmSliceQ42Reader::open(bytes).map_err(|e| e.to_string())?;
        if !reader.is_unified_volume() || reader.version() != 4 {
            return Err("Proof dataset must be native Q42VOL v4".into());
        }
        let quins = reader.collect_all_quins().map_err(|e| e.to_string())?;
        if quins.len() > 100_000 {
            return Err("Proof dataset exceeds its declared 100,000-row browser profile; no sampling applied".into());
        }
        // Existing native zero-copy lexical view: validate once, then binary
        // search borrowed pages. Avoid copying a large Q42LEX page per term.
        use qualia_core_db::q42::q42_volume::{OpfsSliceRangeSource, Q42RangeVolume};
        let volume =
            Q42RangeVolume::open(OpfsSliceRangeSource::new(bytes)).map_err(|e| e.to_string())?;
        let header = volume.header();
        let begin = header.lex_offset as usize;
        let end = begin
            .checked_add(header.lex_length as usize)
            .filter(|n| *n <= bytes.len())
            .ok_or("Invalid native lexicon range")?;
        let lex = qualia_core_db::q42_lex::Q42LexMmap::from_bytes(&bytes[begin..end])
            .map_err(|e| format!("Native lexical view: {e:?}"))?;
        let mut names = BTreeMap::new();
        for q in &quins {
            for h in [q.subject, q.predicate, q.object] {
                if names.contains_key(&h) {
                    continue;
                }
                if let Some(value) = lex.lookup_owned(h) {
                    names.insert(h, value);
                }
            }
        }
        Ok(Self { quins, names })
    }
    pub fn rows(&self, quins: &[NQuin]) -> Vec<[String; 3]> {
        quins
            .iter()
            .map(|q| {
                [q.subject, q.predicate, q.object].map(|h| {
                    self.names
                        .get(&h)
                        .cloned()
                        .unwrap_or_else(|| format!("hash:{h:016x}"))
                })
            })
            .collect()
    }
    pub fn verify_fixture(&self, fixture: &Value, rows: &[NQuin]) -> Result<(), String> {
        let expected = fixture["expected_count"]
            .as_u64()
            .ok_or("Query fixture lacks expected count")?;
        if rows.len() as u64 != expected {
            return Err(format!("Expected {expected} matches, found {}", rows.len()));
        }
        if let Some(values) = fixture.get("expected_rows") {
            let mut expected: Vec<[String; 3]> =
                serde_json::from_value(values.clone()).map_err(|e| e.to_string())?;
            let mut actual = self.rows(rows);
            expected.sort();
            actual.sort();
            if actual != expected {
                return Err("Decoded native query rows differ from published source oracle".into());
            }
        }
        Ok(())
    }
    pub fn query(&self, source: &str) -> Result<(Vec<NQuin>, Value), String> {
        if source.len() > 8192 {
            return Err("Query exceeds 8 KiB proof budget".into());
        }
        // The actual native mini-parser accepts a single whitespace-delimited
        // pattern, not arbitrary SPARQL. Refuse ambiguous UI input before it
        // could silently execute only its first three tokens.
        let tokens: Vec<&str> = source.split_whitespace().collect();
        let terms = if tokens.last() == Some(&".") {
            &tokens[..tokens.len() - 1]
        } else {
            &tokens[..]
        };
        if terms.len() != 3
            || terms.iter().any(|t| {
                !(t.starts_with('?')
                    || (t.starts_with('<') && t.ends_with('>'))
                    || t.starts_with('"'))
            })
        {
            return Err("Enter exactly three native pattern terms (URI in angle brackets or ?variable), optionally followed by a full stop; this instrument is not a SPARQL parser".into());
        }
        let mut program = [0u8; 1024];
        qualia_core_db::mini_parser::compile_ntriples_to_bytecode(source.as_bytes(), &mut program)
            .map_err(|e| format!("Query compilation failed: {e:?}"))?;
        let mut out = vec![NQuin::default(); self.quins.len().max(1)];
        let stats = qualia_core_db::webizen_bytecode::execute_program_with_stats(
            &program,
            &self.quins,
            &mut out,
            None,
        )
        .map_err(|e| format!("Native query failed: {e:?}"))?;
        out.truncate(stats.match_count);
        Ok((
            out,
            json!({"engine":"QualiaDB mini_parser + webizen_bytecode::execute_program_with_stats","vm_cycles":stats.vm_cycles,"direct_jump_ops":stats.direct_jump_ops,"lexicon_lookup_ops":stats.lexicon_lookup_ops}),
        ))
    }
}
pub fn rights(scope: &str, withdrawn: bool, now: u32, expiry: u32) -> Result<Value, String> {
    use qualia_core_db::modalities::logic::{
        compile_norm_quin, evaluate_deontic_contract, DeonticStatus, DeonticVerdict, OP_FORBID,
        OP_PERMIT,
    };
    use qualia_core_db::q_hash;
    if scope != "aggregate" && scope != "individual" {
        return Err("Unknown disclosure scope".into());
    }
    let party = q_hash("urn:webizen:proof:anonymous");
    let action = q_hash("urn:webizen:proof:disclose");
    let contract = q_hash("urn:webizen:proof:public-data-policy");
    let target = q_hash(if scope == "aggregate" {
        "urn:webizen:proof:aggregate"
    } else {
        "urn:webizen:proof:individual"
    });
    let opcode = if scope == "aggregate" {
        OP_PERMIT
    } else {
        OP_FORBID
    };
    let mut norms = vec![compile_norm_quin(
        party, opcode, action, target, contract, expiry, false,
    )];
    if withdrawn && scope == "aggregate" {
        norms.push(compile_norm_quin(
            party, OP_FORBID, action, target, contract, 0, true,
        ));
    }
    let mut verdicts = [DeonticVerdict::default(); 4];
    let n = evaluate_deontic_contract(&norms, now, &mut verdicts)
        .map_err(|e| format!("Native rights evaluation: {e:?}"))?;
    let permitted = verdicts[..n]
        .iter()
        .any(|v| v.opcode == OP_PERMIT && v.status == DeonticStatus::Active);
    Ok(
        json!({"engine":"QualiaDB evaluate_deontic_contract","kind":"authored public scenario, not a credential or legal compliance decision","scope":scope,"withdrawn":withdrawn,"now_unix":now,"expiry_unix":expiry,"permitted":permitted,"norm_count":norms.len(),"verdicts":verdicts[..n].iter().map(|v|json!({"opcode":v.opcode,"status":format!("{:?}",v.status),"defeat_kind":format!("{:?}",v.defeat_kind)})).collect::<Vec<_>>()}),
    )
}
pub fn rights_fixture(case: &Value) -> Result<Value, String> {
    use qualia_core_db::modalities::logic::{
        compile_norm_quin, evaluate_deontic_contract, DeonticVerdict,
    };
    use qualia_core_db::q_hash;
    let now = case["now"]
        .as_u64()
        .filter(|n| *n <= u32::MAX as u64)
        .ok_or("Invalid fixture evaluation time")? as u32;
    let inputs = case["norms"]
        .as_array()
        .filter(|a| !a.is_empty() && a.len() <= 32)
        .ok_or("A fixture requires1..32 norms")?;
    let mut norms = Vec::new();
    for n in inputs {
        let term = |key: &str| {
            n[key]
                .as_str()
                .filter(|s| !s.is_empty() && s.len() < 2048)
                .ok_or_else(|| format!("Invalid norm {key}"))
        };
        let op = n["opcode"]
            .as_u64()
            .filter(|n| (16..=18).contains(n))
            .ok_or("Unsupported proof norm opcode")? as u8;
        let expiry = n["expiry"]
            .as_u64()
            .filter(|n| *n <= u32::MAX as u64)
            .ok_or("Invalid norm expiry")? as u32;
        norms.push(compile_norm_quin(
            q_hash(term("party")?),
            op,
            q_hash(term("path")?),
            q_hash(term("object")?),
            q_hash(term("contract")?),
            expiry,
            n["defeater"].as_bool().ok_or("Invalid defeater flag")?,
        ));
    }
    let mut out = vec![DeonticVerdict::default(); norms.len()];
    let count = evaluate_deontic_contract(&norms, now, &mut out)
        .map_err(|e| format!("Native fixture evaluation:{e:?}"))?;
    let statuses: Vec<String> = out[..count]
        .iter()
        .map(|v| format!("{:?}", v.status))
        .collect();
    let expected: Vec<String> =
        serde_json::from_value(case["expected_statuses"].clone()).map_err(|e| e.to_string())?;
    Ok(
        json!({"schema":"webizen.native-rights-fixture-run/v1","engine":"QualiaDB compile_norm_quin + evaluate_deontic_contract","fixture":case,"actual_statuses":statuses,"expected_statuses":expected,"passed":statuses==expected,"assertions":expected.len(),"native_verdicts":out[..count].iter().map(|v|json!({"opcode":v.opcode,"status":format!("{:?}",v.status),"defeat_kind":format!("{:?}",v.defeat_kind)})).collect::<Vec<_>>()}),
    )
}
pub fn space(lat: f64, lon: f64, alt: f64, unix: f64) -> Result<Value, String> {
    use qualia_core_db::domains::geospatial::{geodesy::ephemeris_orbit as e, geodetic};
    if ![lat, lon, alt, unix].iter().all(|x| x.is_finite())
        || !(-90.0..=90.0).contains(&lat)
        || !(-180.0..=180.0).contains(&lon)
        || !(-1000.0..=10_000_000.0).contains(&alt)
        || !(0.0..=4_102_444_800.0).contains(&unix)
    {
        return Err("Use valid latitude/longitude, height -1,000 to 10,000,000 m, and a UTC date from 1970 to 2100".into());
    }
    let (x, y, z) = geodetic::lat_lon_alt_to_ecef(lat, lon, alt);
    let (a, b, c) = geodetic::ecef_to_lat_lon_alt(x, y, z);
    let sun = e::solar_ephemeris(unix, lat, lon);
    let moon = e::lunar_ephemeris(unix, lat, lon);
    Ok(
        json!({"engine":"QualiaDB geodetic + ephemeris_orbit","model":"WGS84 coordinates; analytical solar/lunar ephemeris; UTC used as UT1 approximation","input":{"latitude_deg":lat,"longitude_deg":lon,"altitude_m":alt,"unix_utc":unix},"ecef_m":[x,y,z],"roundtrip":{"latitude_deg":a,"longitude_deg":b,"altitude_m":c},"julian_date":e::unix_to_julian_date(unix),"earth_rotation_rad":e::earth_rotation_angle_rad(e::unix_to_julian_date(unix)),"sun":{"elevation_deg":sun.elevation_deg,"azimuth_deg":sun.azimuth_deg,"distance_au":sun.distance_au,"ecef_unit":sun.ecef_unit},"moon":{"elevation_deg":moon.elevation_deg,"azimuth_deg":moon.azimuth_deg,"illuminated_fraction":moon.illumination_fraction}}),
    )
}
pub fn checks() -> Value {
    let mut cases = Vec::new();
    let a = rights("aggregate", false, 100, 200).unwrap();
    cases.push(
        json!({"name":"aggregate permit active before expiry","passed":a["permitted"]==true}),
    );
    let a = rights("individual", false, 100, 200).unwrap();
    cases.push(json!({"name":"individual disclosure forbidden","passed":a["permitted"]==false && a["verdicts"][0]["status"]=="Active"}));
    let a = rights("aggregate", true, 100, 200).unwrap();
    cases.push(json!({"name":"withdrawal defeats aggregate permission","passed":a["permitted"]==false && a["verdicts"][0]["status"]=="Defeated"}));
    let a = rights("aggregate", false, 201, 200).unwrap();
    cases.push(json!({"name":"expired permission cannot permit disclosure","passed":a["permitted"]==false && a["verdicts"][0]["status"]=="Expired"}));
    let p = space(0., 0., 0., 946728000.).unwrap();
    cases.push(json!({"name":"WGS84 equator ECEF X is 6378137 m","passed":(p["ecef_m"][0].as_f64().unwrap()-6378137.).abs()<0.001}));
    cases.push(json!({"name":"J2000 noon is Julian 2451545","passed":(p["julian_date"].as_f64().unwrap()-2451545.).abs()<1e-9}));
    let p = space(-35., 149., 600., 1763337600.).unwrap();
    cases.push(json!({"name":"WGS84 coordinate round trip","passed":(p["roundtrip"]["latitude_deg"].as_f64().unwrap()+35.).abs()<1e-7 && (p["roundtrip"]["altitude_m"].as_f64().unwrap()-600.).abs()<0.01}));
    cases.push(json!({"name":"invalid latitude refused","passed":space(100.,0.,0.,0.).is_err()}));
    let passed = cases.iter().filter(|c| c["passed"] == true).count();
    json!({"schema":"webizen.native-proof-checks/v1","assertions":cases.len(),"passed":passed,"failed":cases.len()-passed,"cases":cases})
}
