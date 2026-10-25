//! tools.rs — ENGINE YA HUDUMA (ndani ya server pekee).
//!
//! KANUNI KUU YA MMILIKI: **mteja hauoni zana wala lugha — anaona HUDUMA tu**
//! (mf. network_scanner, digital_forensic). Jina la zana (binary), amri, na
//! matokeo ghafi HAYATOKI nje ya server haya:
//!   - `TOOL_REGISTRY` ni `pub(crate)` — haisainishwi kwenye API yoyote.
//!   - `sanitize_output()` inasafisha kila matokeo kabla ya kwenda UI/API:
//!     amri, paths za zana, na majina yake yanafichwa (redacted).
//!   - `brain.rs` inapokea HINTS za kawaida (bila majina ya zana) — recall
//!     ya kawaida inaendelea.
//!
//! Backend ya utekelezaji halisi iko `tools_internal.rs` (mod fiche).

/// Registry ya NDANI (server-side pekee): huduma → zana halisi + invocation.
#[derive(Debug, Clone)]
pub(crate) struct ToolBinding {
    pub service: &'static str,
    pub tool: &'static str, // jina la binary (HILI HAITOKI NJE)
    pub kind: ToolKind,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub(crate) enum ToolKind {
    /// Amri ya system (Linux/Unix tools za ndani)
    Command,
    /// Kazi ya pure-Rust (hakuna binary ya nje)
    BuiltIn,
}

/// TOOL_REGISTRY — siri ya server. Kila huduma ya umma ina binding ya ndani.
pub(crate) const TOOL_REGISTRY: &[ToolBinding] = &[
    ToolBinding {
        service: "network_scanner",
        tool: "nmap",
        kind: ToolKind::Command,
    },
    ToolBinding {
        service: "health_check",
        tool: "builtin-health",
        kind: ToolKind::BuiltIn,
    },
    ToolBinding {
        service: "digital_forensic",
        tool: "sleuthkit",
        kind: ToolKind::Command,
    },
    ToolBinding {
        service: "malware_scan",
        tool: "clamav",
        kind: ToolKind::Command,
    },
    ToolBinding {
        service: "os_install",
        tool: "builtin-osinstall",
        kind: ToolKind::BuiltIn,
    },
    ToolBinding {
        service: "app_install",
        tool: "builtin-appinstall",
        kind: ToolKind::BuiltIn,
    },
    ToolBinding {
        service: "driver_update",
        tool: "builtin-drivers",
        kind: ToolKind::BuiltIn,
    },
    ToolBinding {
        service: "device_management",
        tool: "builtin-devmgmt",
        kind: ToolKind::BuiltIn,
    },
];

/// SANITIZE: safisha matokeo ghafi ya zana kabla ya kwenda kwa mteja.
/// Inaficha: jina la binary, amri zilizotumika, paths za binaries.
pub(crate) fn sanitize_output(raw: &str) -> String {
    let mut out = raw.to_string();
    for b in TOOL_REGISTRY {
        // Jina la binary kamili na fupi — vyote vinafichwa
        out = out.replace(b.tool, "[huduma-ya-ndani]");
        out = out.replace(&format!("/usr/bin/{}", b.tool), "[huduma-ya-ndani]");
        out = out.replace(&format!("/usr/sbin/{}", b.tool), "[huduma-ya-ndani]");
        out = out.replace(&format!("/bin/{}", b.tool), "[huduma-ya-ndani]");
    }
    // Amri za kawaida zinazoonyesha zana
    for cmd_hint in [
        "nmap -",
        "nmap ",
        "nmap--",
        "fls ",
        "icat ",
        "clamscan ",
        "freshclam",
    ] {
        out = out.replace(cmd_hint, "[scan-ya-ndani] ");
    }
    // SAFISHA: ondoa vibaya vya UTF-8 (U+FFFD kutoka bytes mbaya za modeli AI)
    // na alama za ziada za "?" zilizokusanyika
    out = out.replace('\u{FFFD}', "");
    while out.contains("??") {
        out = out.replace("??", "?");
    }
    out
}

/// Resolver: huduma ya umma → binding ya ndani (server-side pekee).
pub(crate) fn resolve(service: &str) -> Option<&'static ToolBinding> {
    TOOL_REGISTRY.iter().find(|b| b.service == service)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn registry_ina_bindings_za_huduma_zote() {
        for b in TOOL_REGISTRY {
            assert!(!b.service.is_empty());
            assert!(!b.tool.is_empty());
            assert!(resolve(b.service).is_some());
        }
    }

    #[test]
    fn sanitize_inaficha_majina_ya_zana() {
        let raw = "scan ilifanyika na nmap v7.9 kutoka /usr/bin/nmap; clamscan cleaned 3 files; fls listing done";
        let clean = sanitize_output(raw);
        assert!(!clean.contains("nmap"));
        assert!(!clean.contains("clamscan"));
        assert!(!clean.contains("fls"));
        assert!(clean.contains("[huduma-ya-ndani]"));
    }
}
