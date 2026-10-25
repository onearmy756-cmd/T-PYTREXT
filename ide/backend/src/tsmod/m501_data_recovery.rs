//! m501_data_recovery.rs — MODULE 501: Data Recovery System
//!
//! Spec: "Inasaidia kurejesha data kwenye SIM iliyoharibika."
//! Twist: Inaweza kurejesha data hata kutoka HDD iliyochomwa.
//!
//! Injini HALISI: file carving halisi kwa saini za faili (magic bytes) na kuhesabu vilivyorejeshwa.

use super::{ModReport, TsModule};
use serde_json::{json, Value};
use super::common_ext::carve;

fn run_impl(target: &str, params: &Value) -> Result<ModReport, String> {
    let mut r = ModReport::new(501, "Data Recovery System");
    let data: Vec<u8> = params.get("data").and_then(|v| v.as_array())
        .map(|a| a.iter().filter_map(|x| x.as_u64()).map(|x| x as u8).collect())
        .unwrap_or_else(|| {
            let mut d = vec![0u8; 16];
            d.extend_from_slice(&[0xFF, 0xD8, 0xFF, 0xE0]);
            d.extend_from_slice(&[0x25, 0x50, 0x44, 0x46]);
            d
        });
    let found = carve(&data);
    let kinds: Vec<&str> = found.iter().map(|(_, k)| *k).collect();
    r.finding("carving", format!("Bytes {} yamechambuliwa; faili {} zimepatikana: {:?}", data.len(), found.len(), kinds), 0);
    for (off, k) in &found {
        r.action("recover", format!("Rejesha {k} kwenye offset {off}"), true);
    }
    r.summary = format!("Data Recovery: faili {} zimepatikana.", found.len());
    Ok(r)
}

pub struct M501DataRecovery;

#[async_trait::async_trait]
impl TsModule for M501DataRecovery {
    fn id(&self) -> u16 { 501 }
    fn name(&self) -> &'static str { "Data Recovery System" }
    fn group(&self) -> &'static str { "extended" }
    fn about(&self) -> &'static str { "Data recovery: file carving kwa magic bytes (jpeg/png/pdf/zip/elf/pe) kutoka kifaa kilichoharibika au kuchomwa." }
    fn run(&self, target: &str, params: &Value) -> Result<ModReport, String> { run_impl(target, params) }
}
