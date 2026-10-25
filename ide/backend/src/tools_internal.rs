//! tools_internal.rs — KICHUNGUZI CHA NDANI cha target.
//!
//! KANUNI: target inayokubalika ni hostname/IP fupi salama (herufi, namba,
//! `.` `-` `_` `:`) — hii huzuia injection kwenye zana za usalama
//! (`secpack`, `threatswarm`, `fleet`) zinazoendesha scans kwa arg-array.

/// Target inayokubalika: hostname/IP fupi salama (herufi, namba, . - _ :)
pub(crate) fn valid_target(t: &str) -> bool {
    !t.is_empty() && t.len() <= 64 && t.chars().all(|c| c.is_alphanumeric() || "._-:".contains(c))
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn valid_target_inakataa_vitendosti() {
        assert!(valid_target("192.168.1.10"));
        assert!(valid_target("hr-pc-01"));
        assert!(!valid_target(""));
        assert!(!valid_target("bad; rm -rf /"));
        assert!(!valid_target("$(whoami)"));
        assert!(!valid_target(&"a".repeat(100)));
    }
}
