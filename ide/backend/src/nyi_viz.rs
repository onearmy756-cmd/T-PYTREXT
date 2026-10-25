//! nyi_viz.rs — VISUALIZATION ya 2NYI: chati za SVG kutoka data HALISI.
//!
//! KANUNI: pure Rust, HAKUNA dependency mpya, HAKUNA JavaScript inayohitajika.
//! SVG inaonyeshwa kwenye browser yoyote (na kwenye chat/IDE) bila mtandao.
//! Data inaingia kama jozi (label, value) au mfululizo wa namba — kile
//! kinachoonekana ndicho kilichopimwa; hatubuni data.
//!
//! Aina: line, bar, pie, scatter, sparkline.
//! Hurudisha SVG (string) — tayari kwa <img src="data:image/svg+xml;base64,...">
//! au kuwekwa moja kwa moja kwenye HTML.

use serde_json::Value;

/// Jozi moja ya data (label + value).
#[derive(Debug, Clone)]
pub struct Point {
    pub label: String,
    pub value: f64,
}

/// Chambua data kutoka JSON: `[{"label":"a","value":1}, ...]` au `[1,2,3]`.
pub fn parse_points(v: &Value) -> Vec<Point> {
    let mut out = Vec::new();
    match v {
        Value::Array(a) => {
            for (i, item) in a.iter().enumerate() {
                match item {
                    Value::Object(o) => {
                        let label = o
                            .get("label")
                            .or_else(|| o.get("x"))
                            .and_then(|x| x.as_str())
                            .map(|s| s.to_string())
                            .unwrap_or_else(|| i.to_string());
                        let value = o
                            .get("value")
                            .or_else(|| o.get("y"))
                            .and_then(|x| x.as_f64())
                            .unwrap_or(0.0);
                        out.push(Point { label, value });
                    }
                    Value::Number(n) => out.push(Point {
                        label: i.to_string(),
                        value: n.as_f64().unwrap_or(0.0),
                    }),
                    _ => {}
                }
            }
        }
        Value::Object(o) => {
            // {"a":1,"b":2} — label = key (mpangilio wa BTreeMap = deterministic).
            let mut keys: Vec<&String> = o.keys().collect();
            keys.sort();
            for k in keys {
                out.push(Point {
                    label: k.clone(),
                    value: o[k].as_f64().unwrap_or(0.0),
                });
            }
        }
        _ => {}
    }
    out
}

/// Chati kamili ya SVG. `kind` = line|bar|pie|scatter|sparkline.
pub fn chart_svg(kind: &str, title: &str, pts: &[Point]) -> String {
    if pts.is_empty() {
        return format!(
            "<svg xmlns=\"http://www.w3.org/2000/svg\" width=\"320\" height=\"80\">\
             <text x=\"12\" y=\"44\" font-family=\"sans-serif\" font-size=\"13\" fill=\"#888\">\
             {}</text></svg>",
            esc(title)
        );
    }
    match kind {
        "pie" => pie_svg(title, pts),
        "bar" => bar_svg(title, pts),
        "scatter" => scatter_svg(title, pts),
        "sparkline" => sparkline_svg(pts),
        _ => line_svg(title, pts),
    }
}

const W: f64 = 720.0;
const H: f64 = 400.0;
const PAD_L: f64 = 64.0;
const PAD_R: f64 = 24.0;
const PAD_T: f64 = 48.0;
const PAD_B: f64 = 72.0;

const PALETTE: [&str; 8] = [
    "#4f8cff", "#22c55e", "#f59e0b", "#ef4444", "#a855f7", "#06b6d4", "#ec4899", "#84cc16",
];

fn bounds(pts: &[Point]) -> (f64, f64) {
    let mut lo = f64::INFINITY;
    let mut hi = f64::NEG_INFINITY;
    for p in pts {
        lo = lo.min(p.value);
        hi = hi.max(p.value);
    }
    if !lo.is_finite() || !hi.is_finite() {
        return (0.0, 1.0);
    }
    if (hi - lo).abs() < f64::EPSILON {
        // Mstari tambarare — toa nafasi juu/chini ili usigonge kingo.
        let pad = if hi.abs() < f64::EPSILON { 1.0 } else { hi.abs() * 0.1 };
        return (lo - pad, hi + pad);
    }
    (lo, hi)
}

fn grid_lines(lo: f64, hi: f64, y_of: impl Fn(f64) -> f64) -> String {
    let mut s = String::new();
    for i in 0..=4 {
        let t = i as f64 / 4.0;
        let val = lo + (hi - lo) * t;
        let y = y_of(val);
        s.push_str(&format!(
            "<line x1=\"{PAD_L}\" y1=\"{y:.1}\" x2=\"{:.1}\" y2=\"{y:.1}\" stroke=\"#e5e7eb\" stroke-width=\"1\"/>\
             <text x=\"{:.1}\" y=\"{:.1}\" text-anchor=\"end\" font-family=\"sans-serif\" font-size=\"11\" fill=\"#6b7280\">{}</text>",
            W - PAD_R,
            PAD_L - 8.0,
            y + 4.0,
            fmt_num(val)
        ));
    }
    s
}

fn line_svg(title: &str, pts: &[Point]) -> String {
    let (lo, hi) = bounds(pts);
    let plot_w = W - PAD_L - PAD_R;
    let plot_h = H - PAD_T - PAD_B;
    let x_of = |i: usize| PAD_L + plot_w * (i as f64) / ((pts.len().max(2) - 1) as f64);
    let y_of = |v: f64| PAD_T + plot_h * (1.0 - (v - lo) / (hi - lo));

    let mut s = header(title);
    s.push_str(&grid_lines(lo, hi, y_of));
    let d: Vec<String> = pts
        .iter()
        .enumerate()
        .map(|(i, p)| format!("{}{:.1},{:.1}", if i == 0 { "M" } else { "L" }, x_of(i), y_of(p.value)))
        .collect();
    s.push_str(&format!(
        "<path d=\"{}\" fill=\"none\" stroke=\"{}\" stroke-width=\"2.5\"/>",
        d.join(" "),
        PALETTE[0]
    ));
    for (i, p) in pts.iter().enumerate() {
        s.push_str(&format!(
            "<circle cx=\"{:.1}\" cy=\"{:.1}\" r=\"3.5\" fill=\"{}\"/>",
            x_of(i),
            y_of(p.value),
            PALETTE[0]
        ));
    }
    s.push_str(&x_labels(pts, x_of));
    s.push_str("</svg>");
    s
}

fn bar_svg(title: &str, pts: &[Point]) -> String {
    let (lo, hi) = bounds(pts);
    let lo = lo.min(0.0);
    let plot_w = W - PAD_L - PAD_R;
    let plot_h = H - PAD_T - PAD_B;
    let n = pts.len();
    let slot = plot_w / n as f64;
    let bw = (slot * 0.68).max(2.0);
    let y_of = |v: f64| PAD_T + plot_h * (1.0 - (v - lo) / (hi - lo));

    let mut s = header(title);
    s.push_str(&grid_lines(lo, hi, y_of));
    for (i, p) in pts.iter().enumerate() {
        let x = PAD_L + slot * i as f64 + (slot - bw) / 2.0;
        let y = y_of(p.value.max(lo));
        let h = (y_of(lo) - y).abs();
        s.push_str(&format!(
            "<rect x=\"{x:.1}\" y=\"{y:.1}\" width=\"{bw:.1}\" height=\"{h:.1}\" rx=\"3\" fill=\"{}\"/>",
            PALETTE[i % PALETTE.len()]
        ));
    }
    s.push_str(&x_labels(pts, |i| PAD_L + slot * i as f64 + slot / 2.0));
    s.push_str("</svg>");
    s
}

fn scatter_svg(title: &str, pts: &[Point]) -> String {
    let (lo, hi) = bounds(pts);
    let plot_w = W - PAD_L - PAD_R;
    let plot_h = H - PAD_T - PAD_B;
    let x_of = |i: usize| PAD_L + plot_w * (i as f64) / ((pts.len().max(2) - 1) as f64);
    let y_of = |v: f64| PAD_T + plot_h * (1.0 - (v - lo) / (hi - lo));

    let mut s = header(title);
    s.push_str(&grid_lines(lo, hi, y_of));
    for (i, p) in pts.iter().enumerate() {
        s.push_str(&format!(
            "<circle cx=\"{:.1}\" cy=\"{:.1}\" r=\"5\" fill=\"{}\" fill-opacity=\"0.75\"/>",
            x_of(i),
            y_of(p.value),
            PALETTE[i % PALETTE.len()]
        ));
    }
    s.push_str(&x_labels(pts, x_of));
    s.push_str("</svg>");
    s
}

fn pie_svg(title: &str, pts: &[Point]) -> String {
    let total: f64 = pts.iter().map(|p| p.value.max(0.0)).sum();
    let cx = W / 2.0;
    let cy = H / 2.0 + 8.0;
    let r = 120.0;
    let mut s = header(title);
    if total <= 0.0 {
        s.push_str("<text x=\"360\" y=\"200\" text-anchor=\"middle\" font-family=\"sans-serif\" fill=\"#888\">data haina thamani</text></svg>");
        return s;
    }
    let mut angle = -std::f64::consts::FRAC_PI_2;
    for (i, p) in pts.iter().enumerate() {
        let frac = p.value.max(0.0) / total;
        let sweep = frac * std::f64::consts::TAU;
        let x1 = cx + r * angle.cos();
        let y1 = cy + r * angle.sin();
        let x2 = cx + r * (angle + sweep).cos();
        let y2 = cy + r * (angle + sweep).sin();
        let large = if sweep > std::f64::consts::PI { 1 } else { 0 };
        s.push_str(&format!(
            "<path d=\"M{cx:.1},{cy:.1} L{x1:.1},{y1:.1} A{r:.1},{r:.1} 0 {large} 1 {x2:.1},{y2:.1} Z\" \
             fill=\"{}\" stroke=\"#fff\" stroke-width=\"1.5\"/>",
            PALETTE[i % PALETTE.len()]
        ));
        // Legend (jina + %).
        let ly = PAD_T + 8.0 + i as f64 * 20.0;
        s.push_str(&format!(
            "<rect x=\"{:.1}\" y=\"{ly:.1}\" width=\"12\" height=\"12\" rx=\"2\" fill=\"{}\"/>\
             <text x=\"{:.1}\" y=\"{:.1}\" font-family=\"sans-serif\" font-size=\"12\" fill=\"#374151\">{} ({:.0}%)</text>",
            W - 250.0,
            PALETTE[i % PALETTE.len()],
            W - 232.0,
            ly + 10.0,
            esc(&truncate(&p.label, 18)),
            frac * 100.0
        ));
        angle += sweep;
    }
    s.push_str("</svg>");
    s
}

fn sparkline_svg(pts: &[Point]) -> String {
    let w = 240.0;
    let h = 48.0;
    let (lo, hi) = bounds(pts);
    let x_of = |i: usize| 4.0 + (w - 8.0) * (i as f64) / ((pts.len().max(2) - 1) as f64);
    let y_of = |v: f64| 4.0 + (h - 8.0) * (1.0 - (v - lo) / (hi - lo));
    let d: Vec<String> = pts
        .iter()
        .enumerate()
        .map(|(i, p)| format!("{}{:.1},{:.1}", if i == 0 { "M" } else { "L" }, x_of(i), y_of(p.value)))
        .collect();
    format!(
        "<svg xmlns=\"http://www.w3.org/2000/svg\" width=\"{w}\" height=\"{h}\">\
         <path d=\"{}\" fill=\"none\" stroke=\"{}\" stroke-width=\"2\"/></svg>",
        d.join(" "),
        PALETTE[0]
    )
}

fn header(title: &str) -> String {
    format!(
        "<svg xmlns=\"http://www.w3.org/2000/svg\" width=\"{W}\" height=\"{H}\" \
         viewBox=\"0 0 {W} {H}\" font-family=\"sans-serif\">\
         <rect width=\"{W}\" height=\"{H}\" fill=\"#ffffff\"/>\
         <text x=\"{:.1}\" y=\"28\" font-size=\"16\" font-weight=\"600\" fill=\"#111827\">{}</text>",
        PAD_L / 2.0,
        esc(title)
    )
}

fn x_labels(pts: &[Point], x_of: impl Fn(usize) -> f64) -> String {
    let mut s = String::new();
    let step = (pts.len() / 12).max(1);
    for (i, p) in pts.iter().enumerate() {
        if i % step != 0 && i + 1 != pts.len() {
            continue;
        }
        s.push_str(&format!(
            "<text x=\"{:.1}\" y=\"{:.1}\" text-anchor=\"middle\" font-size=\"10\" fill=\"#6b7280\">{}</text>",
            x_of(i),
            H - PAD_B + 18.0,
            esc(&truncate(&p.label, 10))
        ));
    }
    s
}

fn fmt_num(v: f64) -> String {
    if v.abs() >= 1000.0 || (v != 0.0 && v.abs() < 0.01) {
        format!("{v:.2e}")
    } else if (v - v.round()).abs() < 1e-9 {
        format!("{}", v.round() as i64)
    } else {
        format!("{v:.2}")
    }
}

fn truncate(s: &str, max: usize) -> String {
    if s.chars().count() <= max {
        return s.to_string();
    }
    s.chars().take(max).collect::<String>() + "…"
}

/// Escape ya XML — data ya mtumiaji isivunje SVG.
fn esc(s: &str) -> String {
    s.replace('&', "&amp;")
        .replace('<', "&lt;")
        .replace('>', "&gt;")
        .replace('"', "&quot;")
        .replace('\'', "&apos;")
}

/// Data-URL ya SVG (tayari kwa <img> kwenye chat/IDE).
pub fn data_url(svg: &str) -> String {
    let b64 = b64(svg.as_bytes());
    format!("data:image/svg+xml;base64,{b64}")
}

/// Base64 ya ndani (hakuna dependency).
fn b64(data: &[u8]) -> String {
    const T: &[u8; 64] = b"ABCDEFGHIJKLMNOPQRSTUVWXYZabcdefghijklmnopqrstuvwxyz0123456789+/";
    let mut out = String::with_capacity(data.len().div_ceil(3) * 4);
    for chunk in data.chunks(3) {
        let b = [chunk[0], *chunk.get(1).unwrap_or(&0), *chunk.get(2).unwrap_or(&0)];
        let n = ((b[0] as u32) << 16) | ((b[1] as u32) << 8) | b[2] as u32;
        out.push(T[(n >> 18) as usize & 63] as char);
        out.push(T[(n >> 12) as usize & 63] as char);
        out.push(if chunk.len() > 1 { T[(n >> 6) as usize & 63] as char } else { '=' });
        out.push(if chunk.len() > 2 { T[n as usize & 63] as char } else { '=' });
    }
    out
}

#[cfg(test)]
mod tests {
    use super::*;
    use serde_json::json;

    fn pts() -> Vec<Point> {
        parse_points(&json!([
            {"label": "Jan", "value": 12.0},
            {"label": "Feb", "value": 31.0},
            {"label": "Mac", "value": 7.0}
        ]))
    }

    #[test]
    fn parse_points_hukubali_aina_zote() {
        assert_eq!(parse_points(&json!([1, 2, 3])).len(), 3);
        assert_eq!(parse_points(&json!({"a": 1, "b": 2})).len(), 2);
        assert_eq!(parse_points(&json!([{"label": "x", "value": 5}])).len(), 1);
        assert_eq!(parse_points(&json!("junk")).len(), 0);
    }

    #[test]
    fn chart_zote_huzalisha_svg_halali() {
        for kind in ["line", "bar", "pie", "scatter", "sparkline"] {
            let svg = chart_svg(kind, "Mauzo", &pts());
            assert!(svg.starts_with("<svg"), "{kind}: {svg}");
            assert!(svg.trim_end().ends_with("</svg>"), "{kind}");
        }
    }

    #[test]
    fn data_tupu_hairuki() {
        let svg = chart_svg("line", "Tupu", &[]);
        assert!(svg.contains("Tupu"));
        assert!(svg.ends_with("</svg>"));
    }

    #[test]
    fn thamani_zote_sawa_hairuki() {
        let p = parse_points(&json!([5, 5, 5, 5]));
        let svg = chart_svg("line", "Flat", &p);
        assert!(svg.contains("path"));
    }

    #[test]
    fn esc_huzuia_ukandamizaji_wa_xml() {
        let svg = chart_svg("bar", "<script>x</script>", &pts());
        assert!(!svg.contains("<script>"), "hatari ya XSS kwenye SVG");
        assert!(svg.contains("&lt;script&gt;"));
    }

    #[test]
    fn data_url_ni_base64_sahihi() {
        let u = data_url("<svg></svg>");
        assert!(u.starts_with("data:image/svg+xml;base64,"));
        // "PHN2Zz48L3N2Zz4=" ni base64 ya "<svg></svg>"
        assert!(u.ends_with("PHN2Zz48L3N2Zz4="), "{u}");
    }
}
