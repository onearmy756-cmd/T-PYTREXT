//! m45_usb.rs — MODULE 45: USB SECURITY
//!
//! Spec: "Inalinda USB connections, inagundua BadUSB attack. Twist: inagundua
//! BadUSB attack (keyboard emulation)."
//!
//! Hii si catalog: inasoma vifaa halisi vya USB (/sys/bus/usb/devices) — VID/PID
//! + device class — na kugundua BadUSB: kifaa kinachojitangaza kama HID keyboard
//! (class 03) lakini si keyboard halisi, au kifaa cha kuhifadhi chenye HID.

use super::{ModReport, TsModule};
use serde_json::{json, Value};

pub struct M45Usb;

/// Vifaa vya HID vya kuaminika (VID:PID) — keyboards za kawaida.
const KNOWN_KEYBOARDS: &[(&str, &str)] = &[
    ("046d", "c31c"), // Logitech
    ("046d", "c52b"),
    ("045e", "0750"), // Microsoft
    ("04f2", "0112"),
];

/// Kifaa cha USB kutoka sysfs.
#[derive(Debug, Clone, PartialEq)]
pub struct UsbDevice {
    pub vid: String,
    pub pid: String,
    pub class: u8,
    pub product: String,
    pub serial: String,
}

/// Changanua vid/pid kutoka idVendor/idProduct.
pub fn parse_vidpid(vendor: &str, product: &str) -> (String, String) {
    (
        vendor.trim().to_ascii_lowercase(),
        product.trim().to_ascii_lowercase(),
    )
}

/// Je, kifaa ni BadUSB hatarishi? (HID keyboard isiyojulikana, au composite
/// yenye HID pamoja na storage.)
pub fn badusb_risk(d: &UsbDevice) -> Option<&'static str> {
    let is_hid = d.class == 3 || d.class == 0; // 0 = composite inaweza kuwa HID
    if d.class == 3 {
        let known = KNOWN_KEYBOARDS
            .iter()
            .any(|(v, p)| *v == d.vid && *p == d.pid);
        if !known {
            return Some("hid_unknown_keyboard");
        }
    }
    if is_hid && d.class == 0 && d.serial.is_empty() && d.product.to_lowercase().contains("keyboard")
    {
        return Some("composite_hid_keyboard");
    }
    None
}

impl TsModule for M45Usb {
    fn id(&self) -> u16 {
        45
    }
    fn name(&self) -> &'static str {
        "USB Security"
    }
    fn group(&self) -> &'static str {
        "device"
    }
    fn about(&self) -> &'static str {
        "Husoma vifaa halisi vya USB (VID/PID/class) na kugundua BadUSB (HID \
         keyboard emulation isiyojulikana)."
    }

    fn run(&self, target: &str, params: &Value) -> Result<ModReport, String> {
        let op = params.get("op").and_then(|v| v.as_str()).unwrap_or("audit");
        match op {
            "audit" => self.audit(),
            "scan" => self.scan(params),
            other => Err(format!("op '{other}' haipo: audit|scan")),
        }
    }
}

impl M45Usb {
    /// Soma vifaa halisi kutoka /sys/bus/usb/devices.
    fn read_devices(&self) -> Vec<UsbDevice> {
        let mut out = Vec::new();
        let Ok(rd) = std::fs::read_dir("/sys/bus/usb/devices") else {
            return out;
        };
        for e in rd.flatten() {
            let p = e.path();
            let vendor = std::fs::read_to_string(p.join("idVendor")).unwrap_or_default();
            let product_id = std::fs::read_to_string(p.join("idProduct")).unwrap_or_default();
            if vendor.trim().is_empty() || product_id.trim().is_empty() {
                continue;
            }
            let class = std::fs::read_to_string(p.join("bDeviceClass"))
                .ok()
                .and_then(|s| u8::from_str_radix(s.trim(), 16).ok())
                .unwrap_or(0);
            let product = std::fs::read_to_string(p.join("product")).unwrap_or_default();
            let serial = std::fs::read_to_string(p.join("serial")).unwrap_or_default();
            let (vid, pid) = parse_vidpid(&vendor, &product_id);
            out.push(UsbDevice {
                vid,
                pid,
                class,
                product: product.trim().to_string(),
                serial: serial.trim().to_string(),
            });
        }
        out
    }

    fn audit(&self) -> Result<ModReport, String> {
        let mut r = ModReport::new(45, self.name());
        let devs = self.read_devices();
        r.finding("usb_devices", format!("Vifaa vya USB: {}", devs.len()), 2);
        let mut alerts = 0;
        for d in &devs {
            if let Some(kind) = badusb_risk(d) {
                alerts += 1;
                r.finding(
                    "badusb",
                    format!(
                        "{}:{} ({}) — {kind}",
                        d.vid, d.pid, d.product
                    ),
                    80,
                );
            }
        }
        r.summary = format!("Vifaa {}; alerts {alerts}.", devs.len());
        Ok(r)
    }

    fn scan(&self, params: &Value) -> Result<ModReport, String> {
        let mut r = ModReport::new(45, self.name());
        let arr = params
            .get("devices")
            .and_then(|v| v.as_array())
            .ok_or("params.devices (array) inahitajika")?;
        let mut alerts = 0;
        for d in arr {
            let dev = UsbDevice {
                vid: d.get("vid").and_then(|v| v.as_str()).unwrap_or("").to_ascii_lowercase(),
                pid: d.get("pid").and_then(|v| v.as_str()).unwrap_or("").to_ascii_lowercase(),
                class: d.get("class").and_then(|v| v.as_u64()).unwrap_or(0) as u8,
                product: d.get("product").and_then(|v| v.as_str()).unwrap_or("").to_string(),
                serial: d.get("serial").and_then(|v| v.as_str()).unwrap_or("").to_string(),
            };
            if let Some(kind) = badusb_risk(&dev) {
                alerts += 1;
                r.finding(
                    "badusb",
                    format!("{}:{} ({}) — {kind}", dev.vid, dev.pid, dev.product),
                    80,
                );
            }
        }
        r.summary = format!("Vifaa vimechanganuliwa; alerts {alerts}.");
        Ok(r)
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn keyboard_isiyojulikana_ni_hatari() {
        let d = UsbDevice {
            vid: "dead".into(),
            pid: "beef".into(),
            class: 3,
            product: "Keyboard".into(),
            serial: String::new(),
        };
        assert_eq!(badusb_risk(&d), Some("hid_unknown_keyboard"));
    }

    #[test]
    fn keyboard_inayojulikana_ni_salama() {
        let d = UsbDevice {
            vid: "046d".into(),
            pid: "c31c".into(),
            class: 3,
            product: "USB Keyboard".into(),
            serial: "ABC".into(),
        };
        assert_eq!(badusb_risk(&d), None);
    }

    #[test]
    fn parse_vidpid_inaleta_lowercase() {
        assert_eq!(parse_vidpid("046D\n", "C31C\n"), ("046d".into(), "c31c".into()));
    }
}
