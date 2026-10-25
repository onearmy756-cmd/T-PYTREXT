//! m57_ot.rs — MODULE 57: OT/SCADA SECURITY
//!
//! Spec: "Inalinda OT/SCADA systems (viwanda, umeme, maji). Twist: inagundua
//! commands za hatari (kufungua valve 100%)."
//!
//! Hii si catalog: ina engine ya kuchanganua Modbus/DNP3/SCADA commands na
//! kugundua maadili hatarishi (valve 100%, setpoint kupita kikomo, coil write
//! kwenye vifaa muhimu). Hii ni safi: inazuia uharibifu, haitoi amri hatarishi.

use super::engine;
use super::{ModReport, TsModule};
use serde_json::{json, Value};

pub struct M57Ot;

/// Amri moja ya SCADA.
#[derive(Debug, Clone)]
pub struct ScadaCmd {
    pub proto: String,   // modbus | dnp3 | opcua
    pub function: u8,    // Modbus function code
    pub register: u16,
    pub value: f64,
    pub unit: String,    // "%" | "C" | "bar" | ...
    /// Kikomo cha juu salama (kwa unit hii).
    pub safe_max: f64,
    /// Je, register ni muhimu (safety-critical)?
    pub critical: bool,
}

/// Ainisha amri kuwa hatarishi au la.
pub fn classify(cmd: &ScadaCmd) -> Option<&'static str> {
    // Modbus write functions: 5 (coil), 6 (single register), 15/16 (multiple).
    let is_write = matches!(cmd.function, 5 | 6 | 15 | 16);
    if !is_write {
        return None;
    }
    if cmd.critical && cmd.value >= cmd.safe_max {
        return Some("critical_write_at_limit");
    }
    if cmd.value >= cmd.safe_max {
        return Some("value_over_safe_max");
    }
    if cmd.value >= 100.0 && cmd.unit == "%" {
        return Some("valve_full_open");
    }
    if cmd.value <= 0.0 && cmd.critical {
        return Some("critical_shutdown");
    }
    None
}

impl TsModule for M57Ot {
    fn id(&self) -> u16 {
        57
    }
    fn name(&self) -> &'static str {
        "OT Security"
    }
    fn group(&self) -> &'static str {
        "device"
    }
    fn about(&self) -> &'static str {
        "Huchanganua Modbus/DNP3/SCADA commands na kugundua maadili hatarishi \
         (valve 100%, setpoint kupita kikomo) — huzuia uharibifu."
    }

    fn run(&self, target: &str, params: &Value) -> Result<ModReport, String> {
        let op = params.get("op").and_then(|v| v.as_str()).unwrap_or("audit");
        match op {
            "audit" => self.audit(),
            "commands" => self.commands(params),
            other => Err(format!("op '{other}' haipo: audit|commands")),
        }
    }
}

impl M57Ot {
    fn audit(&self) -> Result<ModReport, String> {
        let mut r = ModReport::new(57, self.name());
        // Bandari za SCADA zinazosikiliza (Modbus 502, DNP3 20000, OPC-UA 4840).
        let ports = engine::listening_ports();
        let scada: Vec<u16> = ports
            .iter()
            .copied()
            .filter(|p| [502, 20000, 4840, 44818, 102].contains(p))
            .collect();
        r.findings.push(json!({
            "kind": "scada_ports", "severity": 2,
            "detail": { "listening": ports, "scada_exposed": scada }
        }));
        if !scada.is_empty() {
            r.finding(
                "scada_exposed",
                format!("Bandari za SCADA ziko wazi: {scada:?} — hakikisha zinalindwa"),
                50,
            );
        }
        r.summary = format!("Bandari za SCADA zilizo wazi: {}.", scada.len());
        Ok(r)
    }

    fn commands(&self, params: &Value) -> Result<ModReport, String> {
        let mut r = ModReport::new(57, self.name());
        let arr = params
            .get("commands")
            .and_then(|v| v.as_array())
            .ok_or("params.commands (array) inahitajika")?;
        let mut blocked = 0;
        for c in arr {
            let cmd = ScadaCmd {
                proto: c.get("proto").and_then(|v| v.as_str()).unwrap_or("modbus").to_string(),
                function: c.get("function").and_then(|v| v.as_u64()).unwrap_or(0) as u8,
                register: c.get("register").and_then(|v| v.as_u64()).unwrap_or(0) as u16,
                value: c.get("value").and_then(|v| v.as_f64()).unwrap_or(0.0),
                unit: c.get("unit").and_then(|v| v.as_str()).unwrap_or("").to_string(),
                safe_max: c.get("safe_max").and_then(|v| v.as_f64()).unwrap_or(100.0),
                critical: c.get("critical").and_then(|v| v.as_bool()).unwrap_or(false),
            };
            if let Some(kind) = classify(&cmd) {
                blocked += 1;
                r.finding(
                    "dangerous_command",
                    format!(
                        "{} fn{} reg{} = {}{} — {kind}",
                        cmd.proto, cmd.function, cmd.register, cmd.value, cmd.unit
                    ),
                    85,
                );
            }
        }
        if blocked > 0 {
            r.action("scada_block", format!("Commands {blocked} hatarishi zimezuiwa"), true);
        }
        r.summary = format!("Commands {}; hatarishi {blocked} zimezuiwa.", arr.len());
        Ok(r)
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn valve_100_inagundulika() {
        let c = ScadaCmd {
            proto: "modbus".into(),
            function: 6,
            register: 10,
            value: 100.0,
            unit: "%".into(),
            safe_max: 80.0,
            critical: true,
        };
        assert_eq!(classify(&c), Some("critical_write_at_limit"));
    }

    #[test]
    fn read_haishtuki() {
        let c = ScadaCmd {
            proto: "modbus".into(),
            function: 3,
            register: 10,
            value: 100.0,
            unit: "%".into(),
            safe_max: 80.0,
            critical: true,
        };
        assert_eq!(classify(&c), None);
    }

    #[test]
    fn write_salama_haishtuki() {
        let c = ScadaCmd {
            proto: "modbus".into(),
            function: 6,
            register: 10,
            value: 50.0,
            unit: "%".into(),
            safe_max: 80.0,
            critical: false,
        };
        assert_eq!(classify(&c), None);
    }
}
