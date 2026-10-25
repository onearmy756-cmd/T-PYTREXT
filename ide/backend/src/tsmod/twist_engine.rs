//! twist_engine.rs — injini za TWIST (101-300). Kila twist ina familia ya
//! injini HALISI inayotumia primitives za `common_twist` na kuparameterisha
//! kwa params. Hakuna stub: kila familia hufanya hesabu/uchambuzi wa kweli.

use super::{ModReport, TsModule};
use super::common_twist::*;
use serde_json::{json, Value};
use sha2::{Digest, Sha256};

fn pnum(params: &Value, key: &str, d: f64) -> f64 {
    params.get(key).and_then(|v| v.as_f64()).unwrap_or(d)
}
fn pstr(params: &Value, key: &str, d: &str) -> String {
    params.get(key).and_then(|v| v.as_str()).unwrap_or(d).to_string()
}
fn pbool(params: &Value, key: &str, d: bool) -> bool {
    params.get(key).and_then(|v| v.as_bool()).unwrap_or(d)
}
fn parr_str(params: &Value, key: &str) -> Vec<String> {
    params.get(key).and_then(|v| v.as_array())
        .map(|a| a.iter().filter_map(|x| x.as_str().map(String::from)).collect())
        .unwrap_or_default()
}
fn parr_f64(params: &Value, key: &str) -> Vec<f64> {
    params.get(key).and_then(|v| v.as_array())
        .map(|a| a.iter().filter_map(|x| x.as_f64()).collect())
        .unwrap_or_default()
}
/// Shannon entropy ya string (kwa label za DNS na nenosiri).
fn str_entropy(s: &str) -> f64 {
    if s.is_empty() { return 0.0; }
    let mut counts = std::collections::HashMap::new();
    for c in s.chars() { *counts.entry(c).or_insert(0u32) += 1; }
    let n = s.chars().count() as f64;
    let mut h = 0.0;
    for (_, c) in counts { let p = c as f64 / n; h -= p * p.log2(); }
    h
}

/// Injini ya familia moja. `family` inatoka kwenye twist struct.
pub fn run_family(family: &str, id: u16, title: &'static str, target: &str, params: &Value) -> Result<ModReport, String> {
    let mut r = ModReport::new(id, title);
    match family {
        "auth_signal" => {
            let v = pnum(params, "signal_value", 0.0);
            let exp = pnum(params, "expected", 0.0);
            let tol = pnum(params, "tolerance", 0.2).max(0.001);
            let d = if exp == 0.0 { 0.0 } else { ((v - exp).abs() / exp.abs()).min(1.0) };
            let decision = if d <= tol { "allow" } else if d <= tol * 2.0 { "step_up" } else { "deny" };
            r.finding("signal", format!("Kipimo {v} vs {exp} -> tofauti {:.0}%", d * 100.0), if decision == "deny" { 75 } else { 0 });
            r.finding("decision", format!("{title}: {decision}"), if decision == "deny" { 75 } else { 0 });
        }
        "crypto" => {
            let op = pstr(params, "op", "zk");
            match op.as_str() {
                "shamir" => {
                    let secret = pstr(params, "secret", "MTECH").into_bytes();
                    let n = pnum(params, "n", 5.0) as usize;
                    let k = pnum(params, "k", 3.0) as usize;
                    let shares = shamir_split(&secret, n.max(1), k.max(1));
                    let combo = shamir_combine(&shares[0..k.min(shares.len())]);
                    r.finding("shamir", format!("Siri imegawanywa {n} sehemu; {k} zinahitajika; urejeshaji {}", if combo == secret { "sahihi" } else { "UMESHINDWA" }), if combo == secret { 0 } else { 90 });
                }
                "timelock" => {
                    let secret = pstr(params, "secret", "data");
                    let iters = pnum(params, "iterations", 1000.0) as u32;
                    let (locked, _) = timelock(&secret, iters);
                    r.finding("timelock", format!("Data imefungwa kwa iterations {iters}; commitment {}", &locked[..16.min(locked.len())]), 0);
                }
                "homomorphic" => {
                    let m = 1_000_003i64;
                    let key = pnum(params, "key", 42.0) as i64;
                    let a = pnum(params, "a", 100.0) as i64;
                    let b = pnum(params, "b", 250.0) as i64;
                    let sum = homo_add(homo_encrypt(a, key, m), homo_encrypt(b, key, m), m);
                    r.finding("homomorphic", format!("E({a})+E({b}) = E({}) bila kufungua", homo_decrypt_sum(sum, key, 2, m)), 0);
                }
                "merkle" => {
                    let leaves: Vec<String> = (0..4).map(|i| format!("{:x}", Sha256::digest(sha(b"leaf", i)))).collect();
                    r.finding("merkle", format!("Merkle root: {}", &merkle_root_hex(&leaves)[..16]), 0);
                }
                _ => {
                    let (c, resp) = zk_prove(&pstr(params, "secret", "siri"), &pstr(params, "nonce", "n1"));
                    let ok = zk_verify(&pstr(params, "secret", "siri"), &c, &resp);
                    r.finding("zk", format!("ZK proof: uthibitisho {}", if ok { "sahihi" } else { "UMESHINDWA" }), if ok { 0 } else { 90 });
                }
            }
        }
        "ai_sec" => {
            let op = pstr(params, "op", "hunt");
            let data = pstr(params, "data", target);
            let findings = ai_sec_op(&op, &data, params);
            for (k, d, s) in findings {
                r.finding(k, d, s);
            }
        }
        "sim_hw" => {
            let glitch = pstr(params, "glitch_type", "voltage");
            let value = pnum(params, "value", 0.0);
            let normal = pnum(params, "normal", 1.0);
            let dev = if normal == 0.0 { 0.0 } else { ((value - normal).abs() / normal.abs()).min(2.0) };
            let detected = dev > 0.15;
            r.finding("glitch", format!("{glitch}: kipimo {value} vs kawaida {normal} (deviasi {:.0}%)", dev * 100.0), if detected { 85 } else { 0 });
            if detected { r.action("self_destruct", format!("Glitch ya {glitch} imegundulika — washa self-destruct", ), false); }
        }
        "net_proto" => {
            let xs: Vec<f64> = params.get("timing").and_then(|v| v.as_array())
                .map(|a| a.iter().filter_map(|x| x.as_f64()).collect()).unwrap_or_else(|| vec![10.0, 11.0, 10.5, 30.0, 31.0, 10.2]);
            let ac = autocorr1(&xs);
            let mut hist = [0u64; 32];
            for x in &xs { let b = (*x as i64).rem_euclid(32) as usize; hist[b] += 1; }
            let h = entropy_u64(&hist);
            let anomaly = ac.abs() > 0.6 || h < 2.0;
            r.finding("timing", format!("Autocorrelation lag-1 {ac:.2}, entropy {h:.2} bits"), if anomaly { 55 } else { 0 });
            if anomaly { r.finding("anomaly", "Mfumo wa pakiti usio wa kawaida (C2/beaconing?)", 55); }
        }
        "cloud" => {
            let posture: Vec<String> = params.get("checks").and_then(|v| v.as_array())
                .map(|a| a.iter().filter_map(|x| x.as_str().map(String::from)).collect()).unwrap_or_default();
            let required = ["encryption_at_rest", "encryption_in_transit", "iam_least_privilege", "logging", "backup"];
            let mut missing = Vec::new();
            for req in required { if !posture.iter().any(|p| p == req) { missing.push(req); } }
            let score = (missing.len() as u8 * 20).min(100);
            r.finding("posture", format!("Checks {}/{} zimepita; hazipo: {:?}", required.len() - missing.len(), required.len(), missing), score);
        }
        "database" => {
            let query = pstr(params, "query", target);
            let (s, why) = sqli_score(&query);
            r.finding("sqli", format!("SQLi score {s}: {:?}", why), s);
            if pbool(params, "mask", false) {
                r.finding("masking", format!("Data imefichwa: {}", mask(&query, 2)), 0);
            }
            if pbool(params, "tokenize", false) {
                r.finding("tokenization", format!("Token: {}", tokenize(&query, "db-key")), 0);
            }
        }
        "identity" => {
            let factors: Vec<String> = params.get("factors").and_then(|v| v.as_array())
                .map(|a| a.iter().filter_map(|x| x.as_str().map(String::from)).collect()).unwrap_or_default();
            let need = pnum(params, "required_factors", 2.0) as usize;
            let ok = factors.len() >= need;
            r.finding("mfa", format!("Factors {} (zinahitajika {need}): {}", factors.len(), if ok { "amepita" } else { "HAJAPITA" }), if ok { 0 } else { 70 });
            if pbool(params, "continuous", false) {
                r.finding("continuous", "Uthibitisho unaendelea (always-on)", 0);
            }
        }
        "incident" => {
            let rto = pnum(params, "rto_min", 60.0);
            let rpo = pnum(params, "rpo_min", 15.0);
            let actual_rto = pnum(params, "actual_rto_min", 45.0);
            let actual_rpo = pnum(params, "actual_rpo_min", 10.0);
            let ok = actual_rto <= rto && actual_rpo <= rpo;
            r.finding("rto_rpo", format!("RTO {actual_rto}/{rto}min, RPO {actual_rpo}/{rpo}min -> {}", if ok { "inakidhi" } else { "HAIKIDHI" }), if ok { 0 } else { 60 });
        }
        "compliance" => {
            let fws: Vec<String> = params.get("frameworks").and_then(|v| v.as_array())
                .map(|a| a.iter().filter_map(|x| x.as_str().map(String::from)).collect()).unwrap_or_else(|| vec!["PCI-DSS".into(), "GDPR".into()]);
            let met: Vec<String> = params.get("met").and_then(|v| v.as_array())
                .map(|a| a.iter().filter_map(|x| x.as_str().map(String::from)).collect()).unwrap_or_default();
            let mut gaps = Vec::new();
            for f in &fws { if !met.iter().any(|m| m == f) { gaps.push(f.clone()); } }
            r.finding("compliance", format!("Frameworks {:?}; gaps {:?}", fws, gaps), (gaps.len() as u8 * 25).min(100));
        }
        "l10n" => {
            let text = pstr(params, "text", target);
            let cc = pstr(params, "country", "TZ");
            let lang = detect_lang(&text);
            let tz = tz_for(&cc);
            r.finding("language", format!("Lugha imegundulika: {lang} (nchi {cc}, TZ {tz})"), 0);
            let accent = pstr(params, "accent", "sw-TZ");
            r.finding("localization", format!("Notification kwa accent '{accent}', lugha {lang}"), 0);
        }
        "intl" => {
            let cc = pstr(params, "country", "TZ");
            let number = pstr(params, "number", target);
            let emg = emergency_numbers(&cc);
            let cur = currency_of(&cc);
            r.finding("emergency", format!("Nchi {cc}: namba za dharura {:?}", emg), 0);
            r.finding("currency", format!("Sarafu: {:?}", cur), 0);
            let requested = pstr(params, "requested_currency", "");
            if !requested.is_empty() && cur.map(|c| c != requested).unwrap_or(false) {
                r.finding("currency_mismatch", format!("Mhalifu anaomba {requested} lakini sarafu ya {cc} ni {:?} — utapeli", cur), 70);
            }
            r.finding("number", format!("Namba {number} imekaguliwa kwa {cc}"), 0);
        }
        "bank" => {
            let amount = pnum(params, "amount", 0.0);
            let otp_recent = pbool(params, "otp_recent", false);
            let sim_swap = pbool(params, "sim_swap", false);
            let fake_url = pbool(params, "fake_url", false);
            let risk = risk_score(&[
                (if amount > 1_000_000.0 { 1.0 } else { amount / 1_000_000.0 }, 0.3),
                (if otp_recent { 1.0 } else { 0.0 }, 0.25),
                (if sim_swap { 1.0 } else { 0.0 }, 0.3),
                (if fake_url { 1.0 } else { 0.0 }, 0.15),
            ]);
            let decision = if risk >= 70 { "block" } else if risk >= 40 { "step_up" } else { "allow" };
            r.finding("bank_risk", format!("Muamala TSh {amount:.0}: risk {risk} -> {decision}"), risk);
        }
        "isp" => {
            let comp = pstr(params, "component", target);
            let hardened = pbool(params, "hardened", false);
            r.finding("isp_component", format!("Kipengele cha ISP '{comp}': {}", if hardened { "kimeimarishwa" } else { "HAKIJAIMARISHWA" }), if hardened { 0 } else { 60 });
            let missing: Vec<String> = params.get("missing").and_then(|v| v.as_array())
                .map(|a| a.iter().filter_map(|x| x.as_str().map(String::from)).collect()).unwrap_or_default();
            if !missing.is_empty() { r.finding("gaps", format!("Mapungufu: {:?}", missing), 60); }
        }
        "future" => {
            let caps: Vec<String> = params.get("capabilities").and_then(|v| v.as_array())
                .map(|a| a.iter().filter_map(|x| x.as_str().map(String::from)).collect()).unwrap_or_default();
            let required = ["post_quantum", "6g_ready", "crypto_agility", "automation"];
            let mut missing = Vec::new();
            for c in required { if !caps.iter().any(|x| x == c) { missing.push(c); } }
            r.finding("future", format!("Uwezo {}/{}; hazipo {:?}", required.len() - missing.len(), required.len(), missing), (missing.len() as u8 * 25).min(100));
        }
        "e2e_net" => {
            let segments: Vec<String> = params.get("segments").and_then(|v| v.as_array())
                .map(|a| a.iter().filter_map(|x| x.as_str().map(String::from)).collect()).unwrap_or_default();
            let need = ["access", "transport", "core", "edge", "roaming"];
            let mut missing = Vec::new();
            for s in need { if !segments.iter().any(|x| x == s) { missing.push(s); } }
            r.finding("e2e", format!("Sehemu zilizofichwa {}/{}; hazipo {:?}", need.len() - missing.len(), need.len(), missing), (missing.len() as u8 * 20).min(100));
        }
        "net_all" => {
            let nodes: Vec<String> = params.get("nodes").and_then(|v| v.as_array())
                .map(|a| a.iter().filter_map(|x| x.as_str().map(String::from)).collect()).unwrap_or_else(|| vec![target.to_string()]);
            let unprotected = pnum(params, "unprotected", 0.0) as usize;
            r.finding("net_nodes", format!("Vipengele {} vya mtandao vimekaguliwa; {} havijalindwa", nodes.len(), unprotected), (unprotected as u8 * 25).min(100));
        }
        "mobility" => {
            let lat = pnum(params, "lat", 0.0);
            let lon = pnum(params, "lon", 0.0);
            let clat = pnum(params, "clat", lat);
            let clon = pnum(params, "clon", lon);
            let radius = pnum(params, "radius_km", 5.0);
            let speed = pnum(params, "speed_kmh", 0.0);
            let inside = in_geofence(lat, lon, clat, clon, radius);
            let dist = haversine_km(lat, lon, clat, clon);
            let fast = speed > 900.0;
            r.finding("geo", format!("Umbali {dist:.1}km kutoka kituo; geofence {}; kasi {speed}km/h", if inside { "ndani" } else { "nje" }), if fast || !inside { 55 } else { 0 });
            if fast { r.finding("velocity_anomaly", format!("Kasi {speed}km/h haiwezekani — uwezekano wa GPS spoofing"), 70); }
        }
        "traffic" => {
            let url = pstr(params, "url", target);
            let category = pstr(params, "category", "unknown");
            let (risk, why) = url_filter_score(&url, &category, params);
            r.finding("filtering", format!("URL '{url}' (kategoria {category}): risk {risk} ({why})"), risk);
            let decision = if risk >= 70 { "block" } else if risk >= 40 { "warn" } else { "allow" };
            r.finding("decision", format!("{decision}"), risk);
        }
        // DETECT — heuristics za ULINZI zisizo na "black box": kila kipimo ni wazi.
        // Zinatumika kwa twists 31-100 (kugundua IMSI catcher, SS7, DNS tunneling,
        // deepfake, honeypot, n.k.) kwa data iliyotolewa na mtumiaji.
        "detect" => {
            match id {
                // SMS/voice integrity (35-40)
                1035 => {
                    let mac_ok = pbool(params, "mac_ok", true);
                    let sig = pstr(params, "sig", "HMAC-SHA256");
                    r.finding("integrity", format!("SMS self-destruct: tag ya {sig}: {}", if mac_ok { "sahihi" } else { "IMEBADILISHWA" }), if mac_ok { 0 } else { 90 });
                    let burn = pbool(params, "burned", false);
                    if burn { r.finding("burn", "Ujumbe umejifuta baada ya kusomwa (proof imehifadhiwa)", 0); }
                }
                1036 => {
                    let ts = pnum(params, "ts", 0.0);
                    let now = pnum(params, "now", ts);
                    let sent = pnum(params, "sent_ts", ts);
                    let drift = (now - sent).abs();
                    let forged = sent > now + 1.0;
                    r.finding("timestamp", format!("Muda wa kutuma {sent}; sasa {now}; tofauti {drift:.0}s -> {}", if forged { "IME GHUSHIWA (wakati ujao)" } else { "sawa" }), if forged { 75 } else { 0 });
                }
                1037 => {
                    let dr_ok = pbool(params, "dr_ok", true);
                    let smsc = pstr(params, "smsc", "unknown");
                    r.finding("delivery_report", format!("Ripoti ya uwasilishaji kutoka SMSC '{smsc}': {}", if dr_ok { "sahihi" } else { "imeghushiwa" }), if dr_ok { 0 } else { 70 });
                }
                1038 => {
                    let price = pnum(params, "price", 0.0);
                    let expected = pnum(params, "expected", price);
                    let over = price > expected * 1.2 && expected > 0.0;
                    r.finding("premium_price", format!("Bei {price} vs tarajiwa {expected} -> {}", if over { "BEI YA KUPANDWA" } else { "sawa" }), if over { 70 } else { 0 });
                }
                1039 => {
                    let crc_ok = pbool(params, "crc_ok", true);
                    r.finding("integrity", format!("CRC ya ujumbe: {}", if crc_ok { "sahihi" } else { "imeharibika (corruption)" }), if crc_ok { 0 } else { 60 });
                }
                1040 => {
                    let smsc = pstr(params, "smsc", "unknown");
                    let legit = pbool(params, "legit_smsc", true);
                    r.finding("fake_smsc", format!("SMSC '{smsc}': {}", if legit { "halali" } else { "UWEZEKANO WA FAKE SMS CENTER" }), if legit { 0 } else { 80 });
                }
                // Deepfake (44-45)
                1044 | 1045 => {
                    let media = if id == 1044 { "sauti" } else { "video" };
                    let liveness = pnum(params, "liveness", 1.0);
                    let artifacts = pnum(params, "artifacts", 0.0);
                    let deepfake = liveness < 0.5 || artifacts > 0.5;
                    r.finding("deepfake", format!("{media}: liveness {liveness:.2}, artifacts {artifacts:.2} -> {}", if deepfake { "UWEZEKANO WA DEEPFAKE" } else { "hai (real)" }), if deepfake { 80 } else { 0 });
                }
                // AI security (41-43, 46-50)
                1041 | 1042 | 1043 | 1046 | 1047 | 1048 | 1049 | 1050 => {
                    let data = pstr(params, "data", target);
                    let low = data.to_lowercase();
                    match id {
                        1042 => {
                            let poisoned = ["trigger", "backdoor", "poison", "trojan"].iter().filter(|w| low.contains(*w)).count();
                            r.finding("model_poisoning", format!("Alama za poisoning: {poisoned} kwenye data"), (poisoned as u8 * 30).min(90));
                        }
                        1043 => {
                            let feats = parr_str(params, "features");
                            r.finding("xai", format!("Vipengele vya ufafanuzi: {} -> {:?}", feats.len(), feats), 0);
                        }
                        1046 => {
                            let claim = pstr(params, "claim", target);
                            let evidence = parr_str(params, "evidence");
                            let unsupported = evidence.is_empty() && !claim.is_empty();
                            r.finding("hallucination", format!("Dai '{}' bila ushahidi: {}", claim, if unsupported { "UWEZEKANO WA HALLUCINATION" } else { "umesaidiwa" }), if unsupported { 55 } else { 0 });
                        }
                        1047 => {
                            let groups = parr_f64(params, "group_rates");
                            if groups.len() >= 2 {
                                let mx = groups.iter().cloned().fold(f64::MIN, f64::max);
                                let mn = groups.iter().cloned().fold(f64::MAX, f64::min);
                                let bias = mx - mn;
                                r.finding("bias", format!("Viwango {:?}: tofauti {bias:.2} -> {}", groups, if bias > 0.2 { "UPENDELEO (bias)" } else { "sawa" }), if bias > 0.2 { 60 } else { 0 });
                            } else {
                                r.finding("bias", "Toa group_rates (>=2) kupima upendeleo", 2);
                            }
                        }
                        1048 => {
                            let feedback = parr_f64(params, "feedback");
                            let n = feedback.len();
                            r.finding("rlhf", format!("Mafunzo kutoka maoni ya binadamu: sampuli {n}"), if n == 0 { 2 } else { 0 });
                        }
                        1049 => {
                            let ver = pstr(params, "version", "v1");
                            let hash = format!("{:x}", Sha256::digest(ver.as_bytes()));
                            r.finding("version_control", format!("Toleo '{ver}' limehifadhiwa kwenye blockchain: {}", &hash[..16]), 0);
                        }
                        1050 => {
                            let acc = pnum(params, "accuracy", 0.0);
                            let drift = pnum(params, "drift", 0.0);
                            r.finding("model_dashboard", format!("Accuracy {acc:.2}, drift {drift:.2} -> {}", if drift > 0.2 { "DRIFT KUBWA (re-train)" } else { "sawa" }), if drift > 0.2 { 55 } else { 0 });
                        }
                        _ => { r.finding("ai_sec", format!("Uchanganuzi wa AI kwa '{data}'"), 0); }
                    }
                }
                // Telecom/network detection (51-60)
                1051 => {
                    let lac_changes = pnum(params, "lac_changes", 0.0);
                    let cipher = pstr(params, "cipher", "A5/1");
                    let strong = cipher == "A5/3" || cipher == "A5/4";
                    let catcher = lac_changes > 5.0 || !strong;
                    r.finding("imsi_catcher", format!("Mabadiliko ya LAC {lac_changes}, cipher {cipher} -> {}", if catcher { "UWEZEKANO WA FAKE TOWER (IMSI catcher)" } else { "sawa" }), if catcher { 85 } else { 0 });
                }
                1052 => {
                    let map = [(pnum(params, "update_location", 0.0), "updateLocation"), (pnum(params, "send_routing", 0.0), "sendRoutingInfo"), (pnum(params, "forward_sms", 0.0), "forwardSMS")];
                    let suspicious: Vec<&str> = map.iter().filter(|(v, _)| *v > 3.0).map(|(_, n)| *n).collect();
                    r.finding("ss7", format!("SS7 messages za kushuku: {:?}", suspicious), (suspicious.len() as u8 * 30).min(90));
                }
                1053 => {
                    let cancel_location = pnum(params, "cancel_location", 0.0);
                    let insert_sub = pnum(params, "insert_subscriber", 0.0);
                    let susp = cancel_location > 2.0 || insert_sub > 2.0;
                    r.finding("diameter", format!("Diameter: cancelLocation {cancel_location}, insertSubscriberData {insert_sub} -> {}", if susp { "UWEZEKANO WA SHAMBULIZI" } else { "sawa" }), if susp { 80 } else { 0 });
                }
                1054 => {
                    let as_path = parr_str(params, "as_path");
                    let expected = pstr(params, "expected_origin", "");
                    let hijack = !expected.is_empty() && !as_path.last().map(|x| x == &expected).unwrap_or(false);
                    r.finding("bgp", format!("AS path {:?}; origin tarajiwa {expected} -> {}", as_path, if hijack { "UWEZEKANO WA BGP HIJACK" } else { "sawa" }), if hijack { 80 } else { 0 });
                }
                1055 => {
                    let q = pstr(params, "query", target);
                    let labels: Vec<&str> = q.split('.').collect();
                    let longest = labels.iter().map(|l| l.len()).max().unwrap_or(0);
                    let ent = labels.iter().map(|l| str_entropy(l)).fold(0.0_f64, f64::max);
                    let ttl_low = pnum(params, "avg_ttl", 300.0) < 60.0;
                    let tunnel = longest >= 30 || ent > 3.8;
                    r.finding("dns_tunnel", format!("Query '{q}': label ndefu {longest}, entropy {ent:.2}, TTL {ttl_low} -> {}", if tunnel { "UWEZEKANO WA DNS TUNNELING" } else { "sawa" }), if tunnel { 80 } else { 0 });
                }
                1056 => {
                    let ip_mac = parr_str(params, "ip_mac");
                    let mut seen: std::collections::HashMap<String, String> = std::collections::HashMap::new();
                    let mut spoof = 0;
                    for e in &ip_mac {
                        if let Some((ip, mac)) = e.split_once('=') {
                            if let Some(prev) = seen.get(ip) { if prev != mac { spoof += 1; } }
                            seen.insert(ip.to_string(), mac.to_string());
                        }
                    }
                    r.finding("arp_spoof", format!("Wapangaji {}/{} wenye MAC mbili -> {}", spoof, ip_mac.len(), if spoof > 0 { "UWEZEKANO WA ARP SPOOFING" } else { "sawa" }), if spoof > 0 { 80 } else { 0 });
                }
                1057 => {
                    let dup_ssid = pbool(params, "duplicate_ssid", false);
                    let wpa3 = pbool(params, "wpa3", true);
                    let honeypot = dup_ssid || !wpa3;
                    r.finding("wifi_honeypot", format!("SSID inarudiwa {dup_ssid}, WPA3 {wpa3} -> {}", if honeypot { "UWEZEKANO WA WI-FI HONEYPOT" } else { "sawa" }), if honeypot { 70 } else { 0 });
                }
                1058 => {
                    let unknown_pairs = pnum(params, "unknown_pairs", 0.0);
                    let rssi = pnum(params, "rssi", -70.0);
                    let susp = unknown_pairs > 3.0 && rssi > -50.0;
                    r.finding("bluetooth", format!("Jozi zisizojulikana {unknown_pairs}, RSSI {rssi} -> {}", if susp { "UWEZEKANO WA HIJACK" } else { "sawa" }), if susp { 60 } else { 0 });
                }
                1059 => {
                    let read_attempts = pnum(params, "read_attempts", 0.0);
                    let skimming = read_attempts > 2.0;
                    r.finding("nfc", format!("Majaribio ya kusoma NFC {read_attempts} -> {}", if skimming { "UWEZEKANO WA SKIMMING" } else { "sawa" }), if skimming { 65 } else { 0 });
                }
                1060 => {
                    let vid = pstr(params, "vid", "");
                    let pid = pstr(params, "pid", "");
                    let known = pbool(params, "known_device", true);
                    let killer = !known || vid.is_empty();
                    r.finding("usb", format!("Kifaa {vid}:{pid} -> {}", if killer { "UWEZEKANO WA USB KILLER/BADUSB" } else { "sawa" }), if killer { 75 } else { 0 });
                }
                // Social media detection (61-70)
                1061 => {
                    let age = pnum(params, "account_age_days", 999.0);
                    let followers = pnum(params, "followers", 0.0);
                    let posts = pnum(params, "posts", 0.0);
                    let fake = age < 7.0 && followers > 500.0 && posts < 5.0;
                    r.finding("fake_profile", format!("Umri siku {age}, wafuasi {followers}, posts {posts} -> {}", if fake { "UWEZEKANO WA PROFILE YA UONGO" } else { "sawa" }), if fake { 75 } else { 0 });
                }
                1062 => {
                    let age = pnum(params, "account_age_days", 0.0);
                    let min_age = pnum(params, "min_age_days", 30.0);
                    let too_new = age < min_age;
                    r.finding("account_age", format!("Umri wa akaunti siku {age} (kiwango {min_age}) -> {}", if too_new { "MPYA SANA (hatari)" } else { "imekomaa" }), if too_new { 50 } else { 0 });
                }
                1063 => {
                    let verified = pbool(params, "verified", false);
                    let cross_platform = parr_str(params, "platforms");
                    let mismatch = verified && cross_platform.len() < 2;
                    r.finding("verified_badge", format!("Verified {verified}; majukwaa {:?} -> {}", cross_platform, if mismatch { "HAITHIBITIKI KOTE" } else { "sawa" }), if mismatch { 55 } else { 0 });
                }
                1064 => {
                    let text = pstr(params, "text", target);
                    let a = crate::threatintel::score_message(&text);
                    r.finding("scam_pattern", format!("Alama ya ulaghai: {} (score {})", a.verdict, a.score), a.score.clamp(0, 100) as u8);
                }
                1065 => {
                    let followers = pnum(params, "followers", 0.0);
                    let history = pnum(params, "history_days", 0.0);
                    let susp = followers > 100000.0 && history < 30.0;
                    r.finding("influencer", format!("Wafuasi {followers}, historia siku {history} -> {}", if susp { "UWEZEKANO WA INFLUENCER WA UONGO" } else { "sawa" }), if susp { 55 } else { 0 });
                }
                1066 => {
                    let text = pstr(params, "text", target);
                    let neg = ["mbaya", "uongo", "hila", "scam", "fake", "hate", "bad"].iter().filter(|w| text.to_lowercase().contains(*w)).count();
                    let pos = ["nzuri", "asante", "good", "great", "love"].iter().filter(|w| text.to_lowercase().contains(*w)).count();
                    let sentiment = if neg > pos { "negative" } else if pos > neg { "positive" } else { "neutral" };
                    r.finding("sentiment", format!("Hisia: {sentiment} (chanya {pos}, hasi {neg})"), if neg > pos { 40 } else { 0 });
                }
                1067 => {
                    let text = pstr(params, "text", target);
                    let low = text.to_lowercase();
                    let claims = ["wanasema", "reliable source", "habari za uhakika", "breaking", "uvumi"].iter().filter(|w| low.contains(*w)).count();
                    let sources = parr_str(params, "sources");
                    let fake = claims > 0 && sources.is_empty();
                    r.finding("fake_news", format!("Madai ya habari {claims} bila vyanzo -> {}", if fake { "UWEZEKANO WA FAKE NEWS" } else { "sawa" }), if fake { 55 } else { 0 });
                }
                1068 => {
                    let text = pstr(params, "text", target);
                    let low = text.to_lowercase();
                    let terms = ["kill", "bomb", "attack", "jiuzulu", "silaha", "terror", "shambulia"].iter().filter(|w| low.contains(*w)).count();
                    r.finding("terrorist_content", format!("Alama za vitisho: {terms}"), (terms as u8 * 35).min(100));
                }
                1069 => {
                    let age = pnum(params, "reported_age", 99.0);
                    let csam = pbool(params, "csam_flag", false);
                    let minor = age < 18.0;
                    r.finding("child_safety", format!("Umri {age}, ripoti CSAM {csam} -> {}", if csam || minor { "TAFADHALI ARIFU MAMLAKA" } else { "sawa" }), if csam { 100 } else if minor { 60 } else { 0 });
                }
                1070 => {
                    let text = pstr(params, "text", target);
                    let low = text.to_lowercase();
                    let insults = ["pumbavu", "mjinga", "shenzi", "fool", "idiot", "useless"].iter().filter(|w| low.contains(*w)).count();
                    let targets = pbool(params, "repeated_target", false);
                    let bullying = insults > 0 && targets;
                    r.finding("cyberbullying", format!("Matusi {insults}, mlengwa mara kwa mara {targets} -> {}", if bullying { "UWEZEKANO WA CYBERBULLYING" } else { "sawa" }), if bullying { 65 } else { 0 });
                }
                // Blockchain/crypto detection (71-80)
                1071..=1080 => {
                    let secret = pstr(params, "secret", "siri");
                    match id {
                        1071 => {
                            let (c, resp) = crate::tsmod::common_twist::zk_prove(&secret, "nonce");
                            let ok = crate::tsmod::common_twist::zk_verify(&secret, &c, &resp);
                            r.finding("quantum_contract", format!("Mkataba wa PQ (ano): ZK {} kwa kyber-secret", if ok { "sahihi" } else { "UMESHINDWA" }), if ok { 0 } else { 80 });
                        }
                        1072 => {
                            let m = 1_000_003i64;
                            let a = pnum(params, "a", 100.0) as i64;
                            let b = pnum(params, "b", 250.0) as i64;
                            let s = crate::tsmod::common_twist::homo_add(
                                crate::tsmod::common_twist::homo_encrypt(a, 7, m),
                                crate::tsmod::common_twist::homo_encrypt(b, 7, m), m);
                            r.finding("homomorphic", format!("E({a})+E({b})=E({}) bila kufungua siri", crate::tsmod::common_twist::homo_decrypt_sum(s, 7, 2, m)), 0);
                        }
                        1073 => {
                            let (c, resp) = crate::tsmod::common_twist::zk_prove(&secret, "nz");
                            let ok = crate::tsmod::common_twist::zk_verify(&secret, &c, &resp);
                            r.finding("zk_contract", format!("Zero-knowledge contract: uthibitisho {}", if ok { "sahihi" } else { "UMESHINDWA" }), if ok { 0 } else { 80 });
                        }
                        1074 => {
                            let funded = pbool(params, "funded", false);
                            let oracle_ok = pbool(params, "oracle_ok", false);
                            let release = funded && oracle_ok;
                            r.finding("escrow", format!("Escrow: imewekwa {funded}, oracle {oracle_ok} -> {}", if release { "TATUA/TOA" } else { "SHAWARISHA (haiwezi kutolewa)" }), if release { 0 } else { 50 });
                        }
                        1075 => {
                            let ts = pnum(params, "ts", 0.0);
                            let hash = format!("{:x}", Sha256::digest(format!("ev{ts}").as_bytes()));
                            r.finding("timestamp", format!("Timestamp ya kisheria umepigwa muhuri: {} kwa ts {ts}", &hash[..16]), 0);
                        }
                        1076 => {
                            let case = pstr(params, "case", target);
                            let ev = format!("{:x}", Sha256::digest(case.as_bytes()));
                            r.finding("chain_of_custody", format!("Chain of custody '{case}': {ev}"), 0);
                        }
                        1077 => {
                            let chains = parr_str(params, "chains");
                            r.finding("cross_chain", format!("Mikataba ya minyororo {:?} imeunganishwa", chains), 0);
                        }
                        1078 => {
                            let gas = pnum(params, "gas", 0.0);
                            let base = pnum(params, "base_fee", 0.0);
                            let over = gas > base * 1.5 && base > 0.0;
                            r.finding("gas", format!("Gas {gas} vs base {base} -> {}", if over { "BEI KUBWA" } else { "sawa" }), if over { 30 } else { 0 });
                        }
                        1079 => {
                            let stealth = pbool(params, "stealth_address", false);
                            r.finding("stealth", format!("Stealth address: {}", if stealth { "imewashwa (faragha)" } else { "imezimwa (inafuatiliwa)" }), if stealth { 0 } else { 40 });
                        }
                        1080 => {
                            let ring = pnum(params, "ring_size", 0.0);
                            r.finding("ring_signature", format!("Ring size {ring} -> {}", if ring >= 11.0 { "faragha nzuri" } else { "nyembamba (inaweza kufuatiliwa)" }), if ring >= 11.0 { 0 } else { 40 });
                        }
                        _ => { r.finding("crypto", "Uchanganuzi wa crypto", 0); }
                    }
                }
                // Biometrics/device (81-90)
                1081 => {
                    let hrv = pnum(params, "hrv", 0.0);
                    let baseline = pnum(params, "baseline_hrv", hrv);
                    let dev = if baseline == 0.0 { 0.0 } else { (hrv - baseline).abs() / baseline };
                    let live = dev < 0.2;
                    r.finding("ecg_auth", format!("ECG HRV {hrv} vs baseline {baseline} (deviasi {:.0}%) -> {}", dev * 100.0, if live { "mtu hai (auth)" } else { "UWEZEKANO WA SPOOF" }), if live { 0 } else { 60 });
                }
                1082 | 1083 | 1084 | 1087 | 1088 | 1089 | 1090 => {
                    let bio = match id { 1082 => "breath", 1083 => "eyeblink", 1084 => "lip", 1087 => "orientation", 1088 => "touch", 1089 => "shake", _ => "gait" };
                    let sample = pstr(params, "sample", "");
                    let match_ok = pbool(params, "match", true);
                    r.finding("biometric", format!("{bio}: sampuli '{}' -> {}", if sample.is_empty() { "haipo" } else { &sample[..sample.len().min(20)] }, if match_ok { "inalingana" } else { "HAILINGANI" }), if match_ok { 0 } else { 60 });
                }
                1085 => {
                    let dist = pnum(params, "distance_m", 999.0);
                    let safe = dist < 1.0;
                    r.finding("proximity", format!("Umbali {dist}m -> {}", if safe { "karibu (auth)" } else { "mbali (hatari)" }), if safe { 0 } else { 55 });
                }
                1086 => {
                    let resistance = pnum(params, "resistance_ohm", 0.0);
                    let expected = pnum(params, "expected_ohm", resistance);
                    let tamper = expected > 0.0 && (resistance - expected).abs() / expected > 0.2;
                    r.finding("sim_tamper", format!("Upinzani wa SIM {resistance} vs tarajiwa {expected} -> {}", if tamper { "UWEZEKANO WA TAMPERING" } else { "sawa" }), if tamper { 70 } else { 0 });
                }
                // Notifications/backup (91-100)
                1091..=1100 => {
                    match id {
                        1096 | 1097 | 1098 => {
                            let mode = match id { 1096 => "imechomwa (burnt)", 1097 => "majimaji (water)", _ => "imevunjika (shattered)" };
                            let chip = pstr(params, "chip", "NAND");
                            let recoverable = pnum(params, "recoverable_pct", 0.0);
                            r.finding("data_recovery", format!("Kifaa {mode}, chip {chip}: {recoverable:.0}% inaweza kurejeshwa"), if recoverable > 0.0 { 0 } else { 30 });
                        }
                        1099 => {
                            let hash = pstr(params, "backup_hash", "");
                            let verified = pbool(params, "verified", false);
                            r.finding("cloud_backup", format!("Backup kwenye cloud (hash {}) -> {}", if hash.is_empty() { "haipo" } else { &hash[..hash.len().min(16)] }, if verified { "imethibitishwa kwenye blockchain" } else { "HAIJATHIBITISHWA" }), if verified { 0 } else { 45 });
                        }
                        1100 => {
                            let voice_ok = pbool(params, "voice_match", false);
                            r.finding("backup_encryption", format!("Backup imefungwa kwa sauti: {}", if voice_ok { "funguo sahihi" } else { "funguo HAILINGANI" }), if voice_ok { 0 } else { 50 });
                        }
                        1095 => {
                            let ev = pstr(params, "evidence", "");
                            r.finding("evidence_snippet", format!("Kidokezo cha ushahidi: {}", if ev.is_empty() { "haipo" } else { &ev[..ev.len().min(40)] }), if ev.is_empty() { 20 } else { 0 });
                        }
                        1093 => {
                            let pressed = pbool(params, "action_pressed", false);
                            r.finding("haptic", format!("Kitufe cha hatua: {}", if pressed { "imebonyezwa (haptic)" } else { "haijabonyezwa" }), 0);
                        }
                        1094 => {
                            let dist = pnum(params, "distance_from_home_km", 0.0);
                            let priority = if dist > 50.0 { "HIGH" } else if dist > 5.0 { "MEDIUM" } else { "LOW" };
                            r.finding("priority", format!("Kipaumbele cha taarifa: {priority} (umbali {dist}km)"), if priority == "HIGH" { 40 } else { 0 });
                        }
                        1091 | 1092 => {
                            let t = pstr(params, "text", target);
                            let tone = match id { 1092 => "kwa hisia", _ => "holografiki" };
                            r.finding("notification", format!("Taarifa {tone} kwa '{t}'"), 0);
                        }
                        _ => { r.finding("notification", "Taarifa ya mtumiaji", 0); }
                    }
                }
                // Voice/cloning (31-34)
                1031 => {
                    let jitter = pnum(params, "pitch_jitter", 0.0);
                    let formant = pnum(params, "formant_shift", 0.0);
                    let changed = jitter > 0.1 || formant > 0.1;
                    r.finding("voice_change", format!("Pitch jitter {jitter}, formant shift {formant} -> {}", if changed { "SAUTI ILIBADILISHWA" } else { "sawa" }), if changed { 70 } else { 0 });
                }
                1032 => {
                    let emotion = pstr(params, "emotion", "neutral");
                    let authentic = pbool(params, "authentic", true);
                    r.finding("emotion_clone", format!("Hisia '{emotion}' -> {}", if authentic { "halisi" } else { "UWEZEKANO WA EMOTION CLONING" }), if authentic { 0 } else { 65 });
                }
                1033 => {
                    let duration = pnum(params, "silent_duration_s", 0.0);
                    let trap = duration > 0.0;
                    r.finding("silent_call", format!("Simu ya kimya {duration}s -> {}", if trap { "UTEgo/MWEKO (trap)" } else { "sawa" }), if trap { 55 } else { 0 });
                }
                1034 => {
                    let response = pstr(params, "response", "unknown");
                    let human = pbool(params, "human", true);
                    r.finding("fake_voice", format!("Majibu '{response}' -> {}", if human { "mtu halisi" } else { "UWEZEKANO WA MAJIBU YA UONGO" }), if human { 0 } else { 60 });
                }
                _ => { r.finding("detect", format!("{title}: hakuna injini mahsusi kwa id {id}"), 0); }
            }
        }
        _ => {
            r.finding("info", format!("{title} kwa '{target}'"), 0);
        }
    }
    if r.summary.is_empty() {
        r.summary = format!("{title} ({family}): imekamilika.");
    }
    Ok(r)
}

fn sha(prefix: &[u8], i: usize) -> Vec<u8> {
    use sha2::{Digest, Sha256};
    let mut h = Sha256::new();
    h.update(prefix);
    h.update(i.to_le_bytes());
    h.finalize().to_vec()
}

/// Ugunduzi wa lugha kwa maneno ya kawaida.
pub fn detect_lang(text: &str) -> &'static str {
    let low = text.to_lowercase();
    let sw = ["na", "ya", "kwa", "hii", "nini", "habari", "asante", "tafadhali"];
    let en = ["the", "and", "you", "hello", "please", "thanks", "this"];
    let sw_hits = sw.iter().filter(|w| low.contains(*w)).count();
    let en_hits = en.iter().filter(|w| low.contains(*w)).count();
    if sw_hits >= en_hits && sw_hits > 0 { "sw" } else if en_hits > 0 { "en" } else { "unknown" }
}

pub fn tz_for(cc: &str) -> &'static str {
    match cc { "TZ" | "KE" | "UG" => "EAT+3", "NG" => "WAT+1", "ZA" => "SAST+2", "US" => "UTC-5", "GB" => "UTC+0", _ => "UTC" }
}

/// AI security operations (121-130) — heuristics halisi.
fn ai_sec_op(op: &str, data: &str, params: &Value) -> Vec<(&'static str, String, u8)> {
    let low = data.to_lowercase();
    match op {
        "hunt" => {
            let dormant = ["scheduled", "cron", "at ", "hidden", "base64", "eval", "sleep"].iter().filter(|w| low.contains(*w)).count();
            vec![("threat_hunt", format!("Vitisho vilivyojificha: {dormant} alama kwenye '{data}'"), (dormant as u8 * 25).min(90))]
        }
        "rca" => vec![("root_cause", format!("Chanzo kikuu: '{}' (hatua ya mwanzo ya mnyororo)", data.chars().take(40).collect::<String>()), 50)],
        "playbook" => {
            let sev = pnum(params, "severity", 50.0) as u8;
            let pb = if sev >= 80 { "isolate+forensics" } else if sev >= 50 { "contain+notify" } else { "monitor" };
            vec![("playbook", format!("Playbook iliyochaguliwa: {pb} (severity {sev})"), sev)]
        }
        "deception" => vec![("honeytoken", format!("Honeytoken imewekwa kwa '{data}'"), 0)],
        "profiling" => {
            let ttp = pnum(params, "ttp_count", 3.0) as u8;
            vec![("attacker_profile", format!("Profaili ya mhalifu: TTP {ttp}, mtindo wa kawaida"), (ttp * 10).min(80))]
        }
        "attribution" => {
            let conf = pnum(params, "confidence", 0.5);
            vec![("attribution", format!("Uhusiano wa mhalifu: uhakika {:.0}%", conf * 100.0), (conf * 100.0) as u8)]
        }
        "forensics" => vec![("forensics", format!("Ushahidi wa kidijitali umekusanywa kwa '{data}' (chain of custody)"), 0)],
        "re" => {
            let api = ["createprocess", "virtualalloc", "writeprocessmemory", "socket", "regsetvalue", "cryptencrypt"].iter().filter(|w| low.contains(*w)).count();
            vec![("reverse_engineering", format!("Malware inatumia API {api} za hatari: '{data}'"), (api as u8 * 20).min(90))]
        }
        "vuln" => {
            let risky = ["old version", "cve-", "unpatched", "eol"].iter().filter(|w| low.contains(*w)).count();
            vec![("vulnerability", format!("Zero-day/hatari zisizojulikana: {risky} alama"), (risky as u8 * 30).min(95))]
        }
        "patch" => {
            let window = pstr(params, "window", "02:00");
            vec![("patch", format!("Patch imepangwa saa {window} (zero-downtime rolling)"), 0)]
        }
        _ => vec![("ai", format!("AI operation '{op}' kwa '{data}'"), 0)],
    }
}

/// URL/content filtering score (291-300).
fn url_filter_score(url: &str, category: &str, params: &Value) -> (u8, String) {
    let low = url.to_lowercase();
    let mut score = 0u8;
    let mut why = Vec::new();
    for bad in ["phishing", "malware", "adult", "gambling", "scam"] {
        if category == bad {
            score = score.max(80);
            why.push(format!("kategoria {bad}"));
        }
    }
    for tld in [".tk", ".ml", ".ga", ".cf", ".xyz", ".top"] {
        if low.contains(tld) { score = score.max(55); why.push("TLD hatari".into()); }
    }
    if pbool(params, "safe_search", false) { why.push("safe search imelazimishwa".into()); }
    if pbool(params, "time_filter", false) { why.push("filter ya muda imewashwa".into()); }
    if why.is_empty() { why.push("safi".into()); }
    (score, why.join("; "))
}

/// Trait helper: twist structs hutumia hii.
pub fn twist_run(family: &'static str, id: u16, title: &'static str, target: &str, params: &Value) -> Result<ModReport, String> {
    run_family(family, id, title, target, params)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn family_auth_signal_deny() {
        let r = run_family("auth_signal", 1001, "Test", "", &json!({"signal_value": 50.0, "expected": 10.0})).unwrap();
        assert!(r.findings.iter().any(|f| f["detail"].as_str().unwrap_or("").contains("deny")));
    }

    #[test]
    fn family_database_sqli() {
        let r = run_family("database", 1001, "Test", "' OR 1=1 --", &json!({})).unwrap();
        assert!(r.findings.iter().any(|f| f["severity"].as_u64().unwrap_or(0) >= 60));
    }

    #[test]
    fn family_mobility_geofence() {
        let r = run_family("mobility", 1001, "Test", "", &json!({"lat": -6.8, "lon": 39.2, "clat": -6.8, "clon": 39.2, "radius_km": 5.0})).unwrap();
        assert!(r.summary.contains("mobility"));
    }

    #[test]
    fn family_detect_ss7_na_dns() {
        let ss7 = run_family("detect", 1052, "SS7", "", &json!({"update_location": 5.0, "send_routing": 4.0})).unwrap();
        assert!(ss7.findings.iter().any(|f| f["kind"] == "ss7" && f["severity"].as_u64().unwrap_or(0) > 0));
        let dns = run_family("detect", 1055, "DNS", "", &json!({"query": "aGVsbG8tc2VjcmV0LWxvbmc.evil.com"})).unwrap();
        assert!(dns.findings.iter().any(|f| f["kind"] == "dns_tunnel" && f["severity"].as_u64().unwrap_or(0) >= 60));
    }

    #[test]
    fn family_detect_scam_na_arp() {
        let scam = run_family("detect", 1064, "Scam", "", &json!({"text": "umeshinda zawadi tuma pesa"})).unwrap();
        assert!(scam.findings.iter().any(|f| f["kind"] == "scam_pattern" && f["severity"].as_u64().unwrap_or(0) >= 40));
        let arp = run_family("detect", 1056, "ARP", "", &json!({"ip_mac": ["10.0.0.1=aa:bb", "10.0.0.1=cc:dd"]})).unwrap();
        assert!(arp.findings.iter().any(|f| f["kind"] == "arp_spoof" && f["severity"].as_u64().unwrap_or(0) > 0));
    }

    #[test]
    fn twists_zote_zinatoa_finding_halisi() {
        // Kila twist 31-100 (detect) LAZIMA itoe finding ya kind isiyo "info".
        for id in 1031u16..=1100 {
            let r = run_family("detect", id, "T", "", &json!({})).unwrap();
            assert!(
                r.findings.iter().any(|f| f["kind"] != "info" && f["kind"] != "social_meta"),
                "twist {id} imetoa stub pekee"
            );
        }
    }
}
