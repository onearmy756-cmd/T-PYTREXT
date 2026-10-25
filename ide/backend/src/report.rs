//! report.rs — Ripoti ya PDF ya MTECH OS (brand ya Mbilinyi Tech, bila dependencies).
//!
//! PDF 1.4 halisi (bila dependencies), logo vector,
//! Helvetica. Ripoti ina jobs zote (id, PC, OS, hali, %, ujumbe).

use serde_json::json;

/// Huandaa maandishi kwa PDF string literal yenye WinAnsiEncoding.
///
/// Baiti 0x80–0xFF LAZIMA zitoke kama octal escapes (`\267`). Kikasha cha PDF
/// huchukua baiti moja kwa kila herufi; kuziweka kama UTF-8 (Rust `String`)
/// huzalisha herufi mbili — "·" (0xB7) huonekana kama "Â·".
fn pdf_escape(s: &str) -> String {
    let mut out = String::with_capacity(s.len());
    for c in s.chars() {
        let cp = c as u32;
        let byte: u32 = if cp == 0x2026 {
            0x85 // ellipsis ya WinAnsi
        } else if (32..=126).contains(&cp) || (160..=255).contains(&cp) {
            cp
        } else if (0x2000..0x2100).contains(&cp) {
            b'-' as u32 // dashes, quotes za Kitypo
        } else if cp >= 0x1F000 {
            b' ' as u32 // emoji
        } else if cp < 32 {
            b' ' as u32
        } else {
            b'?' as u32
        };
        if byte < 128 {
            match byte as u8 {
                b'\\' => out.push_str("\\\\"),
                b'(' => out.push_str("\\("),
                b')' => out.push_str("\\)"),
                b => out.push(b as char),
            }
        } else {
            out.push_str(&format!("\\{byte:03o}"));
        }
    }
    out
}

struct PdfBuilder {
    objs: Vec<String>,
}

impl PdfBuilder {
    fn new() -> Self {
        PdfBuilder { objs: Vec::new() }
    }
    fn obj(&mut self, body: &str) -> usize {
        self.objs
            .push(format!("{} 0 obj\n{body}\nendobj\n", self.objs.len() + 1));
        self.objs.len()
    }
    fn finish(self) -> Vec<u8> {
        let mut out = Vec::new();
        out.extend_from_slice(b"%PDF-1.4\n%\xE2\xE3\xCF\xD3\n");
        let mut positions: Vec<usize> = Vec::with_capacity(self.objs.len());
        for o in &self.objs {
            positions.push(out.len());
            out.extend_from_slice(o.as_bytes());
        }
        let info_n = self.objs.len() + 1;
        let info_pos = out.len();
        out.extend_from_slice(
            format!("{info_n} 0 obj\n<< /Title (Ripoti ya MTECH OS) /Producer (MTECH OS - Mbilinyi Tech) >>\nendobj\n")
                .as_bytes(),
        );
        let xref_pos = out.len();
        let mut xref = format!("xref\n0 {}\n", info_n + 1);
        xref.push_str("0000000000 65535 f \n");
        for p in &positions {
            xref.push_str(&format!("{p:010} 00000 n \n"));
        }
        xref.push_str(&format!("{info_pos:010} 00000 n \n"));
        out.extend_from_slice(xref.as_bytes());
        out.extend_from_slice(
            format!(
                "trailer\n<< /Size {} /Root 1 0 R /Info {} 0 R >>\nstartxref\n{}\n%%EOF\n",
                info_n + 1,
                info_n,
                xref_pos
            )
            .as_bytes(),
        );
        out
    }
}

fn txt(x: f64, y: f64, size: f64, bold: bool, r: f64, g: f64, bl: f64, s: &str) -> String {
    format!(
        "BT /{} {} Tf {} {} {} rg {} {} Td ({}) Tj ET\n",
        if bold { "F2" } else { "F1" },
        size,
        r,
        g,
        bl,
        x,
        y,
        pdf_escape(s)
    )
}

fn rect_fill(x: f64, y: f64, w: f64, h: f64, r: f64, g: f64, bl: f64) -> String {
    format!("{} {} {} rg {x} {y} {w} {h} re f\n", r, g, bl)
}

fn hline(x1: f64, y: f64, x2: f64, w: f64, r: f64, g: f64, bl: f64) -> String {
    format!("{} {} {} RG {w} w {x1} {y} m {x2} {y} l S\n", r, g, bl)
}

/// Logo ya mfumo (vector) — ripoti ina brand ya MTECH OS tu
fn vector_logo_ops() -> String {
    let mut s = String::new();
    s.push_str("0 0.898 1 rg 50 4 m 50 96 l 4 50 l h f\n");
    s.push_str("0 0.51 0.56 rg 50 96 m 50 4 l 96 50 l h f\n");
    s.push_str("1 1 1 rg\n");
    s.push_str("28 30 11 52 re f\n");
    s.push_str("28 71 32 11 re f\n");
    s.push_str("28 48 25 11 re f\n");
    s
}

fn build_pdf(draw: &str) -> Vec<u8> {
    // NB: namba za obj LAZIMA zilingane na mpangilio wa push (1-based):
    // 1 Catalog · 2 Pages · 3 Page · 4 Contents · 5 F1 · 6 F2 · 7 LOGO
    let mut b = PdfBuilder::new();
    b.obj("<< /Type /Catalog /Pages 2 0 R >>");
    b.obj("<< /Type /Pages /Kids [3 0 R] /Count 1 >>");
    b.obj(
        "<< /Type /Page /Parent 2 0 R /MediaBox [0 0 595 842] /Resources << /Font << /F1 5 0 R /F2 6 0 R >> /XObject << /LOGO 7 0 R >> >> /Contents 4 0 R >>",
    );
    let content = format!("<< /Length {} >>\nstream\n{}\nendstream", draw.len(), draw);
    b.obj(&content);
    b.obj("<< /Type /Font /Subtype /Type1 /BaseFont /Helvetica /Encoding /WinAnsiEncoding >>");
    b.obj("<< /Type /Font /Subtype /Type1 /BaseFont /Helvetica-Bold /Encoding /WinAnsiEncoding >>");
    let logo_ops = vector_logo_ops();
    b.obj(&format!(
        "<< /Type /XObject /Subtype /Form /BBox [0 0 100 100] /Resources << >> /Length {} >>\nstream\n{}\nendstream",
        logo_ops.len(),
        logo_ops
    ));
    b.finish()
}

fn short(s: &str, n: usize) -> String {
    if s.chars().count() > n {
        format!("{}\u{2026}", s.chars().take(n).collect::<String>())
    } else {
        s.to_string()
    }
}

/// PDF kamili: kichwa (logo MTECH OS) + muhtasari + jedwali la jobs.
pub fn jobs_report_pdf(summary: &serde_json::Value, jobs: &[serde_json::Value]) -> Vec<u8> {
    let mut d = String::new();
    // kichwa: logo + brand
    d.push_str("q 44 0 0 44 40 762 cm /LOGO Do Q\n");
    d.push_str(&txt(96.0, 788.0, 20.0, true, 0.0, 0.51, 0.56, "MTECH OS"));
    d.push_str(&txt(
        96.0,
        772.0,
        8.5,
        false,
        0.33,
        0.43,
        0.47,
        "Mbilinyi Tech · Umiliki ni wako, leseni ni yako, faida ni yako",
    ));
    d.push_str(&txt(
        400.0,
        788.0,
        16.0,
        true,
        0.04,
        0.13,
        0.15,
        "RIPOTI YA DEPLOY",
    ));
    let stamp = chrono::Local::now().format("%Y-%m-%d %H:%M").to_string();
    d.push_str(&txt(
        400.0,
        772.0,
        11.0,
        false,
        0.33,
        0.43,
        0.47,
        &format!("FD-{stamp}"),
    ));
    d.push_str(&rect_fill(40.0, 756.0, 515.0, 2.5, 0.0, 0.51, 0.56));

    // muhtasari
    let getn = |v: &serde_json::Value, k: &str| v[k].as_i64().unwrap_or(0);
    d.push_str(&txt(
        40.0,
        726.0,
        10.0,
        true,
        0.04,
        0.13,
        0.15,
        "Muhtasari:",
    ));
    d.push_str(&txt(
        40.0,
        710.0,
        10.0,
        false,
        0.1,
        0.16,
        0.2,
        &format!(
            "jumla {} · zinasubiri idhini {} · zinaendesha {} · kamili {} · zimeshindwa {}",
            getn(summary, "total"),
            getn(summary, "awaiting_your_approval"),
            getn(summary, "running"),
            getn(summary, "done"),
            getn(summary, "failed")
        ),
    ));

    // jedwali la jobs
    let mut y = 686.0;
    d.push_str(&rect_fill(40.0, y - 6.0, 515.0, 22.0, 0.88, 0.97, 0.98));
    d.push_str(&txt(48.0, y, 9.5, true, 0.0, 0.29, 0.33, "Job"));
    d.push_str(&txt(175.0, y, 9.5, true, 0.0, 0.29, 0.33, "PC"));
    d.push_str(&txt(265.0, y, 9.5, true, 0.0, 0.29, 0.33, "OS"));
    d.push_str(&txt(325.0, y, 9.5, true, 0.0, 0.29, 0.33, "Hali"));
    d.push_str(&txt(395.0, y, 9.5, true, 0.0, 0.29, 0.33, "%"));
    d.push_str(&txt(425.0, y, 9.5, true, 0.0, 0.29, 0.33, "Ujumbe"));
    y -= 22.0;

    for (i, j) in jobs.iter().enumerate() {
        if y < 100.0 {
            break;
        }
        let id = j["id"].as_str().unwrap_or("?");
        let pc = j["device_name"].as_str().unwrap_or("?");
        let os = j["os_type"].as_str().unwrap_or("?");
        let st = j["status"].as_str().unwrap_or("?");
        let msg = j["message"].as_str().unwrap_or("");
        let prog = j["progress"].as_u64().unwrap_or(0);
        if i % 2 == 1 {
            d.push_str(&rect_fill(40.0, y - 6.0, 515.0, 20.0, 0.96, 0.99, 1.0));
        }
        d.push_str(&txt(
            48.0,
            y,
            8.5,
            false,
            0.1,
            0.16,
            0.2,
            &short(&id[..8.min(id.len())], 8),
        ));
        d.push_str(&txt(175.0, y, 8.5, false, 0.1, 0.16, 0.2, &short(pc, 10)));
        d.push_str(&txt(265.0, y, 8.5, false, 0.1, 0.16, 0.2, &short(os, 6)));
        d.push_str(&txt(325.0, y, 8.5, false, 0.1, 0.16, 0.2, &short(st, 8)));
        d.push_str(&txt(
            395.0,
            y,
            8.5,
            false,
            0.1,
            0.16,
            0.2,
            &format!("{prog}%"),
        ));
        d.push_str(&txt(425.0, y, 8.5, false, 0.1, 0.16, 0.2, &short(msg, 36)));
        y -= 20.0;
    }
    if jobs.is_empty() {
        d.push_str(&txt(
            48.0,
            y,
            9.5,
            false,
            0.42,
            0.47,
            0.5,
            "Hakuna jobs bado.",
        ));
    }

    // footer
    d.push_str(&hline(40.0, 70.0, 555.0, 0.8, 0.78, 0.86, 0.87));
    d.push_str(&txt(
        40.0,
        56.0,
        8.0,
        false,
        0.42,
        0.47,
        0.5,
        "Ripoti hii imetengenezwa na MTECH OS (Mbilinyi Tech) — kazi zinafuata idhini ya msimamizi (HITL).",
    ));
    build_pdf(&d)
}

/// PDF ya ripoti ya usalama (admin console) — brand ya MBILINYI TECH + muhuri
/// wa SHA-256 unaoonekana. Single-page, bila dependencies.
pub fn security_report_pdf(res: &crate::admin_console::AnalysisResult, seal: &crate::pkicrypto::Seal) -> Vec<u8> {
    let mut d = String::new();
    // kichwa: logo + brand
    d.push_str("q 44 0 0 44 40 762 cm /LOGO Do Q\n");
    d.push_str(&txt(96.0, 788.0, 20.0, true, 0.0, 0.51, 0.56, "MBILINYI TECH"));
    d.push_str(&txt(
        96.0,
        772.0,
        8.5,
        false,
        0.33,
        0.43,
        0.47,
        "MTECH OS · Cyber Security & Digital Forensics",
    ));
    d.push_str(&txt(
        380.0,
        788.0,
        15.0,
        true,
        0.04,
        0.13,
        0.15,
        "RIPOTI YA USALAMA",
    ));
    d.push_str(&txt(
        380.0,
        772.0,
        10.0,
        false,
        0.33,
        0.43,
        0.47,
        &format!("Kielezo #{} kwenye mnyororo", seal.chain_index),
    ));
    d.push_str(&rect_fill(40.0, 756.0, 515.0, 2.5, 0.0, 0.51, 0.56));

    // Muhtasari
    d.push_str(&txt(40.0, 726.0, 10.5, true, 0.04, 0.13, 0.15, "Muhtasari:"));
    d.push_str(&txt(
        40.0,
        709.0,
        10.0,
        false,
        0.1,
        0.16,
        0.2,
        &format!("Aina: {} · Lengo: {} · Ukali: {}", res.job, short(&res.target, 40), res.severity.to_uppercase()),
    ));
    d.push_str(&txt(
        40.0,
        694.0,
        10.0,
        false,
        0.1,
        0.16,
        0.2,
        &format!("Imetiwa muhuri: {} · Algorithm: {}", seal.at, seal.alg),
    ));

    // Jedwali la findings
    let mut y = 660.0;
    d.push_str(&rect_fill(40.0, y - 6.0, 515.0, 22.0, 0.88, 0.97, 0.98));
    d.push_str(&txt(48.0, y, 9.5, true, 0.0, 0.29, 0.33, "Aina"));
    d.push_str(&txt(120.0, y, 9.5, true, 0.0, 0.29, 0.33, "Lengo"));
    d.push_str(&txt(280.0, y, 9.5, true, 0.0, 0.29, 0.33, "Hukumu"));
    d.push_str(&txt(380.0, y, 9.5, true, 0.0, 0.29, 0.33, "Alama"));
    d.push_str(&txt(430.0, y, 9.5, true, 0.0, 0.29, 0.33, "Maelezo"));
    y -= 22.0;

    for (i, f) in res.findings.iter().enumerate() {
        if y < 190.0 {
            break;
        }
        if i % 2 == 1 {
            d.push_str(&rect_fill(40.0, y - 6.0, 515.0, 20.0, 0.96, 0.99, 1.0));
        }
        d.push_str(&txt(48.0, y, 8.5, false, 0.1, 0.16, 0.2, &short(&f.kind, 12)));
        d.push_str(&txt(120.0, y, 8.5, false, 0.1, 0.16, 0.2, &short(&f.target, 28)));
        d.push_str(&txt(280.0, y, 8.5, false, 0.1, 0.16, 0.2, &short(&f.verdict, 16)));
        d.push_str(&txt(380.0, y, 8.5, false, 0.1, 0.16, 0.2, &f.score.to_string()));
        d.push_str(&txt(430.0, y, 8.5, false, 0.1, 0.16, 0.2, &short(&f.detail, 42)));
        y -= 20.0;
    }

    // Kizuizi cha muhuri (unaonekana)
    d.push_str(&rect_fill(40.0, 150.0, 515.0, 0.8, 0.78, 0.86, 0.87));
    d.push_str(&txt(40.0, 132.0, 10.0, true, 0.02, 0.42, 0.36, "MUHURI WA KIDIJITALI (ECDSA P-256 + SHA-256)"));
    d.push_str(&txt(40.0, 116.0, 8.0, false, 0.1, 0.16, 0.2, &format!("SHA-256 (maudhui): {}", seal.content_hash)));
    d.push_str(&txt(40.0, 102.0, 8.0, false, 0.1, 0.16, 0.2, &format!("Fingerprint ya funguo: {}", seal.key_fingerprint)));
    d.push_str(&txt(40.0, 88.0, 8.0, false, 0.1, 0.16, 0.2, &format!("Sahihi: {}", short(&seal.signature, 64))));
    d.push_str(&txt(40.0, 74.0, 8.0, false, 0.1, 0.16, 0.2, &format!("Hash ya awali: {}", short(&seal.prev_hash, 64))));

    // footer
    d.push_str(&hline(40.0, 58.0, 555.0, 0.8, 0.78, 0.86, 0.87));
    d.push_str(&txt(
        40.0,
        44.0,
        7.5,
        false,
        0.42,
        0.47,
        0.5,
        "Ripoti hii imetengenezwa na MTECH OS (Mbilinyi Tech). Uthibitisho: /api/crypto/verify-seal.",
    ));
    build_pdf(&d)
}

/// JSON ya health (inatumika kwenye /report/pdf meta)
pub fn report_meta() -> serde_json::Value {
    json!({ "kind": "deploy_report", "producer": "MTECH OS", "logo": "mtech-vector" })
}

// ---------- PDF ya kurasa nyingi (AI Scribe, Computer Solutions) ----------

/// Kijenzi cha PDF cha kurasa nyingi — hutumika kwa hati ndefu (SOAP, dakika
/// za mkutano, ripoti za utatuzi). Hushiriki fonti na logo ya `build_pdf`.
struct MultiPdf {
    pages: Vec<String>,
    cur: String,
    y: f64,
}

impl MultiPdf {
    fn new() -> Self {
        MultiPdf { pages: Vec::new(), cur: String::new(), y: 0.0 }
    }

    /// Anza ukurasa mpya wenye kichwa cha brand.
    fn new_page(&mut self, title: &str, subtitle: &str) {
        if !self.cur.is_empty() {
            self.pages.push(std::mem::take(&mut self.cur));
        }
        self.cur.push_str("q 34 0 0 34 40 780 cm /LOGO Do Q\n");
        self.cur.push_str(&txt(84.0, 798.0, 15.0, true, 0.0, 0.51, 0.56, "MTECH OS"));
        self.cur.push_str(&txt(
            84.0, 785.0, 7.5, false, 0.33, 0.43, 0.47,
            "Mbilinyi Tech · Umiliki ni wako, leseni ni yako, faida ni yako",
        ));
        self.cur.push_str(&txt(360.0, 798.0, 12.0, true, 0.04, 0.13, 0.15, title));
        if !subtitle.is_empty() {
            self.cur.push_str(&txt(360.0, 785.0, 8.5, false, 0.33, 0.43, 0.47, subtitle));
        }
        self.cur.push_str(&rect_fill(40.0, 774.0, 515.0, 2.0, 0.0, 0.51, 0.56));
        self.y = 752.0;
    }

    fn ensure(&mut self, need: f64, title: &str, subtitle: &str) {
        if self.y - need < 70.0 {
            self.new_page(title, subtitle);
        }
    }

    fn heading(&mut self, s: &str, title: &str, subtitle: &str) {
        self.ensure(26.0, title, subtitle);
        self.cur.push_str(&txt(40.0, self.y, 12.0, true, 0.0, 0.29, 0.33, s));
        self.y -= 4.0;
        self.cur.push_str(&hline(40.0, self.y, 555.0, 0.8, 0.78, 0.86, 0.87));
        self.y -= 16.0;
    }

    /// Aya — hufunika maneno (≈92 herufi kwa mstari).
    fn para(&mut self, s: &str, title: &str, subtitle: &str) {
        for line in wrap_text(s, 92) {
            self.ensure(16.0, title, subtitle);
            self.cur.push_str(&txt(40.0, self.y, 9.5, false, 0.1, 0.16, 0.2, &line));
            self.y -= 13.0;
        }
    }

    fn bullet(&mut self, s: &str, title: &str, subtitle: &str) {
        let lines = wrap_text(s, 88);
        for (i, line) in lines.iter().enumerate() {
            self.ensure(16.0, title, subtitle);
            if i == 0 {
                self.cur.push_str(&txt(44.0, self.y, 9.5, false, 0.0, 0.51, 0.56, "\u{2022}"));
            }
            self.cur.push_str(&txt(54.0, self.y, 9.5, false, 0.1, 0.16, 0.2, line));
            self.y -= 13.0;
        }
    }

    fn spacer(&mut self, h: f64) {
        self.y -= h;
    }

    fn finish(mut self, title: &str, subtitle: &str, footer: &str) -> Vec<u8> {
        if self.cur.is_empty() {
            self.new_page(title, subtitle);
        }
        self.cur.push_str(&hline(40.0, 62.0, 555.0, 0.8, 0.78, 0.86, 0.87));
        self.cur.push_str(&txt(40.0, 50.0, 8.0, false, 0.42, 0.47, 0.5, footer));
        self.pages.push(std::mem::take(&mut self.cur));
        build_pdf_multi(&self.pages)
    }
}

/// Gawa maandishi kwa mistari ya urefu `max` (huvunja kwa maneno).
pub fn wrap_text(s: &str, max: usize) -> Vec<String> {
    let mut out: Vec<String> = Vec::new();
    let mut line = String::new();
    for word in s.split_whitespace() {
        if line.is_empty() {
            line = word.to_string();
        } else if line.chars().count() + 1 + word.chars().count() <= max {
            line.push(' ');
            line.push_str(word);
        } else {
            out.push(std::mem::take(&mut line));
            line = word.to_string();
        }
    }
    if !line.is_empty() {
        out.push(line);
    }
    if out.is_empty() {
        out.push(String::new());
    }
    out
}

/// PDF 1.4 yenye kurasa nyingi (bila dependencies).
fn build_pdf_multi(pages: &[String]) -> Vec<u8> {
    let n = pages.len().max(1);
    let mut b = PdfBuilder::new();
    b.obj("<< /Type /Catalog /Pages 2 0 R >>"); // 1
    // Kids: ukurasa wa kwanza = obj 3; kila ukurasa 2 objs (page, contents).
    let kids: Vec<String> = (0..n).map(|i| format!("{} 0 R", 3 + i * 2)).collect();
    b.obj(&format!(
        "<< /Type /Pages /Kids [{}] /Count {n} >>",
        kids.join(" ")
    )); // 2
    let font1 = 3 + n * 2;
    let font2 = font1 + 1;
    let logo = font2 + 1;
    for (i, content) in pages.iter().enumerate() {
        b.obj(&format!(
            "<< /Type /Page /Parent 2 0 R /MediaBox [0 0 595 842] /Resources << /Font << /F1 {font1} 0 R /F2 {font2} 0 R >> /XObject << /LOGO {logo} 0 R >> >> /Contents {} 0 R >>",
            4 + i * 2
        ));
        b.obj(&format!(
            "<< /Length {} >>\nstream\n{}\nendstream",
            content.len(),
            content
        ));
    }
    b.obj("<< /Type /Font /Subtype /Type1 /BaseFont /Helvetica /Encoding /WinAnsiEncoding >>");
    b.obj("<< /Type /Font /Subtype /Type1 /BaseFont /Helvetica-Bold /Encoding /WinAnsiEncoding >>");
    let logo_ops = vector_logo_ops();
    b.obj(&format!(
        "<< /Type /XObject /Subtype /Form /BBox [0 0 100 100] /Resources << >> /Length {} >>\nstream\n{}\nendstream",
        logo_ops.len(),
        logo_ops
    ));
    b.finish()
}

/// Hati ya jumla: kichwa, sehemu (heading + aya/bullets), muhuri, QR.
/// Hutumika kwa AI Scribe (SOAP, dakika, notes) na Computer Solutions.
pub fn document_pdf(
    title: &str,
    subtitle: &str,
    sections: &[(String, String, Vec<String>)],
    footer: &str,
) -> Vec<u8> {
    let mut p = MultiPdf::new();
    p.new_page(title, subtitle);
    for (h, body, bullets) in sections {
        if !h.is_empty() {
            p.heading(h, title, subtitle);
        }
        if !body.trim().is_empty() {
            p.para(body, title, subtitle);
            p.spacer(6.0);
        }
        for bl in bullets {
            p.bullet(bl, title, subtitle);
        }
        p.spacer(4.0);
    }
    p.finish(title, subtitle, footer)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn wrap_huvunja_kwa_maneno_bila_kukata() {
        let lines = wrap_text("habari ya asubuhi kwa wanafunzi wote wa chuo", 20);
        assert!(lines.iter().all(|l| l.chars().count() <= 20), "mstari mrefu: {lines:?}");
        assert!(lines.join(" ").contains("wanafunzi"), "maneno yamekatwa: {lines:?}");
        assert_eq!(wrap_text("", 20), vec![String::new()]);
    }

    #[test]
    fn hati_ya_kurasa_nyingi_ina_kurasa_na_maudhui() {
        let sections = vec![
            ("MUHTASARI".to_string(), "Mgonjwa ana homa kali siku tatu.".to_string(), vec![]),
            (
                "DAWA".to_string(),
                String::new(),
                vec!["Paracetamol 500mg — kila saa 6".to_string(), "Maji mengi".to_string()],
            ),
        ];
        let pdf = document_pdf("SOAP NOTE", "Kliniki ya Chuo", &sections, "AI Generated — Review Required");
        assert!(pdf.starts_with(b"%PDF-1.4"));
        assert!(pdf.windows(5).any(|w| w == b"%%EOF"));
        let t = String::from_utf8_lossy(&pdf);
        assert!(t.contains("/Type /Pages"), "Pages tree haipo");
        assert!(t.contains("SOAP NOTE"));
        assert!(t.contains("Paracetamol"), "bullet haipo");
        assert!(t.contains("Review Required"), "muhuri haipo");
    }

    /// Hati ndefu LAZIMA igawanywe kwa kurasa nyingi, na PDF iwe halali.
    #[test]
    fn hati_ndefu_hugawanywa_kwa_kurasa() {
        let long_body = "sentensi ndefu ya uchambuzi wa kliniki. ".repeat(120);
        let sections = vec![("MAELEZO".to_string(), long_body, vec![])];
        let pdf = document_pdf("DAKIKA ZA MKUTANO", "Chuo", &sections, "footer");
        let t = String::from_utf8_lossy(&pdf);
        assert!(t.contains("/Count 2") || t.contains("/Count 3") || t.contains("/Count 4"), "haikugawanywa");
        assert!(pdf.starts_with(b"%PDF-1.4"));

        let p = std::env::temp_dir().join(format!("doc_test_{}.pdf", std::process::id()));
        std::fs::write(&p, &pdf).unwrap();
        let has = |bin: &str| {
            std::process::Command::new("sh")
                .arg("-c")
                .arg(format!("command -v {bin}"))
                .output()
                .map(|o| o.status.success())
                .unwrap_or(false)
        };
        if has("mutool") {
            let o = std::process::Command::new("mutool")
                .args(["info", p.to_str().unwrap()])
                .output()
                .unwrap();
            assert!(o.status.success(), "mutool ilikataa PDF: {}", String::from_utf8_lossy(&o.stderr));
        } else if has("gs") {
            let o = std::process::Command::new("gs")
                .args(["-o", "/dev/null", "-sDEVICE=nullpage", p.to_str().unwrap()])
                .output()
                .unwrap();
            assert!(o.status.success(), "gs ilikataa PDF");
        }
        let _ = std::fs::remove_file(&p);
    }

    #[test]
    fn pdf_hutumia_winansi_si_utf8() {
        // "·" (U+00B7) lazima itoke kama octal \267; UTF-8 (0xC2 0xB7) husababisha "Â·".
        let e = pdf_escape("A · B — C … D");
        assert!(e.contains("\\267"), "middot haipo: {e}");
        assert!(!e.contains('\u{c2}'), "UTF-8 0xC2 imebaki: {e}");
        assert!(!e.contains('\u{b7}'), "0xB7 mbichi imebaki: {e}");
        assert!(e.contains("B - C"), "em-dash haikubadilishwa: {e}");
        // Mipaka ya escaping ya PDF.
        assert_eq!(pdf_escape("(x) \\ y"), "\\(x\\) \\\\ y");
    }

    #[test]
    fn pdf_ina_kichwa_na_jobs() {
        let summary = json!({ "total": 2, "awaiting_your_approval": 1, "running": 0, "done": 1, "failed": 0 });
        let jobs = vec![
            json!({ "id": "aaaaaaaa-bbbb-cccc", "device_name": "PC-01", "os_type": "ubuntu", "status": "done", "progress": 100, "message": "Kamili" }),
            json!({ "id": "dddddddd-eeee-ffff", "device_name": "PC-02", "os_type": "auto", "status": "awaiting_approval", "progress": 10, "message": "Subiri idhini" }),
        ];
        let pdf = jobs_report_pdf(&summary, &jobs);
        assert!(pdf.starts_with(b"%PDF-1.4"));
        assert!(pdf.windows(5).any(|w| w == b"%%EOF"));
        let t = String::from_utf8_lossy(&pdf);
        assert!(t.contains("MTECH OS"));
        assert!(!t.contains("FUNDI"), "brand ya kale haipo");
        assert!(t.contains("PC-01"));
        assert!(t.contains("PC-02"));
    }

    /// Uthibitisho HALISI: PDF inapaswa kuparse/kuonyeshwa na kifaa cha nje
    /// (mutool/gs). Hii inagundua xref/offset mbovu ambayo test ya bytes haiwezi.
    #[test]
    fn pdf_ni_halali_kwa_mutool_au_gs() {
        use std::process::Command;
        let summary = json!({ "total": 3, "awaiting_your_approval": 0, "running": 1, "done": 2, "failed": 0 });
        let jobs = vec![
            json!({ "id": "11111111-2222-3333", "device_name": "PC-A", "os_type": "debian", "status": "done", "progress": 100, "message": "Kamili" }),
            json!({ "id": "44444444-5555-6666", "device_name": "PC-B", "os_type": "win11", "status": "running", "progress": 42, "message": "Inaendelea" }),
            json!({ "id": "77777777-8888-9999", "device_name": "PC-C", "os_type": "kali", "status": "failed", "progress": 5, "message": "Kosa la mtandao" }),
        ];
        let pdf = jobs_report_pdf(&summary, &jobs);
        let p = std::env::temp_dir().join(format!("report_test_{}.pdf", std::process::id()));
        std::fs::write(&p, &pdf).unwrap();

        let has = |bin: &str| {
            Command::new("sh")
                .arg("-c")
                .arg(format!("command -v {bin}"))
                .output()
                .map(|o| o.status.success())
                .unwrap_or(false)
        };

        if has("mutool") {
            let o = Command::new("mutool")
                .args(["draw", "-F", "txt", "-o", "-"])
                .arg(&p)
                .output()
                .expect("mutool");
            assert!(o.status.success(), "mutool imeshindwa: {}", String::from_utf8_lossy(&o.stderr));
            let txt = String::from_utf8_lossy(&o.stdout);
            assert!(txt.contains("MTECH OS"), "maandishi: {txt}");
            assert!(txt.contains("PC-A") && txt.contains("PC-C"));
            assert!(txt.contains("42%"), "progress haipo: {txt}");
        } else if has("gs") {
            let o = Command::new("gs")
                .args(["-q", "-dNOPAUSE", "-dBATCH", "-sDEVICE=nullpage"])
                .arg(&p)
                .output()
                .expect("gs");
            assert!(o.status.success(), "gs imeshindwa: {}", String::from_utf8_lossy(&o.stderr));
        } else {
            eprintln!("skip: mutool/gs haipo");
        }
        let _ = std::fs::remove_file(&p);
    }
}
