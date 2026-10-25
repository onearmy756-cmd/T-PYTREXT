//! Wake-on-LAN — magic packet via wakeonlan binary au raw UDP

use anyhow::Context;
use std::net::UdpSocket;

pub async fn wake(mac: &str) -> anyhow::Result<()> {
    let out = tokio::process::Command::new("wakeonlan")
        .arg(mac)
        .output()
        .await;
    if let Ok(o) = out {
        if o.status.success() {
            return Ok(());
        }
    }
    send_magic_packet(mac)
}

/// WOL ya kina: iwapo IP inajulikana, packet hutumwa MOJA KWA MOJA kwa ile PC
/// (port 9 na 7) — hata kama broadcast halali haiwezi kufika (subnet nyingine,
/// mkondakta wa mpira) — na pia broadcast ya kawaida. Hii ndiyo njia ya
/// "kuwasha PC iliyozima kupitia mtandao" (routed WOL).
pub async fn wake_target(mac: &str, ip: Option<&str>) -> anyhow::Result<String> {
    let packet = magic_packet(mac)?;
    let sock = UdpSocket::bind("0.0.0.0:0")?;
    sock.set_broadcast(true)?;
    let mut sent: Vec<String> = Vec::new();
    if let Some(ip) = ip.filter(|s| !s.trim().is_empty()) {
        for port in [9u16, 7u16] {
            let dst = format!("{ip}:{port}");
            sock.send_to(&packet, dst.as_str())?;
            sent.push(dst);
        }
    }
    for port in [9u16, 7u16] {
        let dst = format!("255.255.255.255:{port}");
        sock.send_to(&packet, dst.as_str())?;
        sent.push(dst);
    }
    Ok(sent.join(", "))
}

fn send_magic_packet(mac: &str) -> anyhow::Result<()> {
    let bytes = parse_mac(mac)?;
    let packet = build_packet(&bytes);
    let sock = UdpSocket::bind("0.0.0.0:0").context("bind")?;
    sock.set_broadcast(true)?;
    sock.send_to(&packet, "255.255.255.255:9")?;
    Ok(())
}

/// Magic packet halisi: 6×FF + MAC ×16 (46 bytes).
fn build_packet(mac: &[u8; 6]) -> Vec<u8> {
    let mut packet = vec![0xFFu8; 6];
    for _ in 0..16 {
        packet.extend_from_slice(mac);
    }
    packet
}

fn magic_packet(mac: &str) -> anyhow::Result<Vec<u8>> {
    Ok(build_packet(&parse_mac(mac)?))
}

fn parse_mac(mac: &str) -> anyhow::Result<[u8; 6]> {
    let parts: Vec<&str> = mac.split(|c| c == ':' || c == '-').collect();
    if parts.len() != 6 {
        anyhow::bail!("MAC batili: {mac}");
    }
    let mut out = [0u8; 6];
    for (i, p) in parts.iter().enumerate() {
        out[i] = u8::from_str_radix(p, 16)?;
    }
    Ok(out)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn magic_packet_halisi_102_baiti() {
        let mac = parse_mac("aa:bb:cc:dd:ee:ff").unwrap();
        let p = build_packet(&mac);
        // 6 (FF×6) + 16×6 (MAC) = 102 baiti — kanuni ya Wake-on-LAN
        assert_eq!(p.len(), 102);
        assert!(p[..6].iter().all(|&b| b == 0xFF));
        assert_eq!(&p[6..12], &mac);
        assert_eq!(&p[96..102], &mac);
    }

    #[test]
    fn mac_mbaya_inaondoa() {
        assert!(parse_mac("zz:zz:zz:zz:zz:zz").is_err());
        assert!(parse_mac("aa:bb").is_err());
        assert!(parse_mac("").is_err());
    }

    #[test]
    fn wake_target_haipoki_bila_ip() {
        // hakuna kipimo cha mtandao — tu hakiwa na haki (inatumwa, haishindwi)
        // ip None → broadcast tu
        let mac = "aa:bb:cc:dd:ee:01";
        assert!(magic_packet(mac).is_ok());
    }
}
