//! LAN IPv4 discovery (FR-34).
//!
//! Multi-adapter machines are the normal case — Hyper-V, WSL, VirtualBox,
//! VMware, VPN clients — and picking the wrong address yields a QR code that
//! silently cannot be reached. The pure `pick_lan_ipv4` is unit-tested with
//! synthetic adapter tables; `discover_lan_ipv4` wraps the live enumeration.

use std::net::{Ipv4Addr, SocketAddr};

use if_addrs::{IfAddr, Interface};

#[derive(Debug, Clone, PartialEq, Eq, thiserror::Error)]
pub enum NetError {
    #[error("no usable LAN IPv4 address found")]
    NoLanAddress,
    #[error("interface enumeration failed: {0}")]
    Enumeration(String),
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct LanAddress {
    pub name: String,
    pub ip: Ipv4Addr,
}

/// Lowercase substrings that mark an adapter as virtual — never a safe choice
/// for the phone-facing server. Name-based heuristics are inherently fuzzy;
/// the tested guarantee is that the KNOWN offenders are rejected and that a
/// real adapter is always preferred over a virtual one.
const VIRTUAL_MARKERS: &[&str] = &[
    "hyper-v", "vethernet", "vswitch", "default switch", "wsl", "virtualbox",
    "vbox", "vmware", "vmnet", "vmnat", "veth", "docker", "nat", "tap-",
    "tap ", "wintun", "tun", "tailscale", "zerotier", "wireguard", "openvpn",
    "nordvpn", "expressvpn", "surfshark", "anyconnect", "cisco", "vpn",
    "hamachi", "loopback",
];

fn is_virtual(name: &str) -> bool {
    let lower = name.to_ascii_lowercase();
    VIRTUAL_MARKERS.iter().any(|m| lower.contains(m))
}

fn is_usable(ip: Ipv4Addr) -> bool {
    !ip.is_loopback() && !ip.is_link_local() && !ip.is_unspecified()
}

fn is_private(ip: Ipv4Addr) -> bool {
    match ip.octets() {
        [10, _, _, _] => true,
        [172, b, _, _] => (16..=31).contains(&b),
        [192, 168, _, _] => true,
        _ => false,
    }
}

/// Pick the best LAN IPv4 from an interface table. Tiers:
/// 1. non-virtual, private RFC1918
/// 2. non-virtual, public-ish (still a LAN address; e.g. a router-assigned
///    public IP on a LAN interface)
/// Virtual adapters are NEVER usable by a phone on the same Wi-Fi (Hyper-V
/// internal switches and VPN tunnels are unreachable), so they are filtered
/// out entirely rather than chosen as a fallback — a broken QR that silently
/// fails would violate Principle 5. Within a tier, deterministic:
/// lexicographically smallest adapter name. Returns None when nothing usable
/// exists (FR-34).
pub fn pick_lan_ipv4(ifaces: &[Interface]) -> Option<LanAddress> {
    let mut candidates: Vec<&Interface> = ifaces
        .iter()
        .filter(|i| match i.addr {
            IfAddr::V4(ref v4) => is_usable(v4.ip) && !is_virtual(&i.name),
            _ => false,
        })
        .collect();
    candidates.sort_by(|a, b| a.name.cmp(&b.name));

    let tier = |i: &Interface| -> u8 {
        match i.addr {
            IfAddr::V4(ref v4) if is_private(v4.ip) => 0,
            _ => 1,
        }
    };

    candidates
        .into_iter()
        .min_by_key(|i| (tier(i), i.name.clone()))
        .map(|i| match i.addr {
            IfAddr::V4(ref v4) => LanAddress {
                name: i.name.clone(),
                ip: v4.ip,
            },
            _ => unreachable!(),
        })
}

/// Live enumeration + pick. The phone-facing server binds to this address
/// (SEC-7) and the QR encodes it.
pub fn discover_lan_ipv4() -> Result<LanAddress, NetError> {
    let ifaces = if_addrs::get_if_addrs().map_err(|e| NetError::Enumeration(e.to_string()))?;
    pick_lan_ipv4(&ifaces).ok_or(NetError::NoLanAddress)
}

/// Build the socket address the phone-facing server binds to: the discovered
/// LAN IPv4 with an ephemeral port (0). The OS assigns the actual port;
/// `ServerManager` reads it back from the bound listener and combines it with
/// the LAN IP for the QR URL. SEC-7 requires binding to the specific LAN
/// address, never `0.0.0.0`.
pub fn bind_addr() -> Result<SocketAddr, NetError> {
    let lan = discover_lan_ipv4()?;
    Ok(SocketAddr::new(lan.ip.into(), 0))
}

#[cfg(test)]
mod tests {
    use super::*;
    use if_addrs::IfAddr;

    fn iface(name: &str, ip: [u8; 4]) -> Interface {
        Interface {
            name: name.to_string(),
            addr: IfAddr::V4(if_addrs::Ifv4Addr {
                ip: Ipv4Addr::from(ip),
                netmask: Ipv4Addr::from([255, 255, 255, 0]),
                prefixlen: 24,
                broadcast: None,
            }),
            index: None,
            #[cfg(windows)]
            adapter_name: String::new(),
        }
    }

    fn names(ifaces: &[Interface]) -> Vec<String> {
        ifaces.iter().map(|i| i.name.clone()).collect()
    }

    #[test]
    fn rejects_loopback_only() {
        let ifaces = vec![iface("Loopback Pseudo-Interface 1", [127, 0, 0, 1])];
        assert_eq!(pick_lan_ipv4(&ifaces), None);
    }

    #[test]
    fn rejects_link_local_only() {
        let ifaces = vec![iface("Ethernet", [169, 254, 12, 34])];
        assert_eq!(pick_lan_ipv4(&ifaces), None);
    }

    #[test]
    fn rejects_unspecified() {
        let ifaces = vec![iface("Ethernet", [0, 0, 0, 0])];
        assert_eq!(pick_lan_ipv4(&ifaces), None);
    }

    #[test]
    fn picks_private_over_public() {
        let ifaces = vec![
            iface("Ethernet", [8, 8, 8, 8]),
            iface("Wi-Fi", [192, 168, 1, 15]),
        ];
        let picked = pick_lan_ipv4(&ifaces).unwrap();
        assert_eq!(picked.name, "Wi-Fi");
        assert_eq!(picked.ip, Ipv4Addr::new(192, 168, 1, 15));
    }

    #[test]
    fn prefers_non_virtual_over_virtual() {
        let ifaces = vec![
            iface("vEthernet (Default Switch)", [172, 20, 0, 1]),
            iface("Ethernet", [192, 168, 1, 15]),
        ];
        let picked = pick_lan_ipv4(&ifaces).unwrap();
        assert_eq!(picked.name, "Ethernet");
        assert_eq!(picked.ip, Ipv4Addr::new(192, 168, 1, 15));
    }

    #[test]
    fn picks_real_private_over_virtual_private() {
        // Hyper-V often claims 172.16-31; the real adapter must win.
        let ifaces = vec![
            iface("vEthernet (Default Switch)", [172, 26, 4, 1]),
            iface("Ethernet", [10, 0, 0, 42]),
        ];
        let picked = pick_lan_ipv4(&ifaces).unwrap();
        assert_eq!(picked.name, "Ethernet");
    }

    #[test]
    fn rejects_all_virtual_when_no_real_adapter_exists() {
        // A virtual adapter's address can never be reached by a phone on the
        // same Wi-Fi; per FR-34 the app must degrade with an explanation.
        let ifaces = vec![
            iface("vEthernet (WSL)", [172, 20, 0, 1]),
            iface("Tailscale", [100, 100, 100, 100]),
        ];
        assert_eq!(pick_lan_ipv4(&ifaces), None);
    }

    #[test]
    fn deterministic_when_multiple_real_adapters() {
        let a = vec![
            iface("Wi-Fi", [192, 168, 1, 5]),
            iface("Ethernet", [192, 168, 1, 6]),
        ];
        let b = vec![
            iface("Ethernet", [192, 168, 1, 6]),
            iface("Wi-Fi", [192, 168, 1, 5]),
        ];
        assert_eq!(pick_lan_ipv4(&a), pick_lan_ipv4(&b));
        assert_eq!(pick_lan_ipv4(&a).unwrap().name, "Ethernet");
    }

    #[test]
    fn recognizes_common_virtual_adapter_names() {
        for name in [
            "vEthernet (Default Switch)",
            "vEthernet (WSL (Hyper-V firewall))",
            "VirtualBox Host-Only Network",
            "VMware Network Adapter VMnet8",
            "TAP-Windows Adapter V9",
            "Tailscale",
            "WireGuard Tunnel",
            "NordVPN Tunnel",
            "OpenVPN TAP-Windows6",
            "Cisco AnyConnect Secure Mobility Client",
        ] {
            assert!(is_virtual(name), "{name} should be virtual");
        }
    }

    #[test]
    fn does_not_overflag_real_adapter_names() {
        for name in ["Ethernet", "Wi-Fi", "Local Area Connection", "Ethernet 2"] {
            assert!(!is_virtual(name), "{name} should not be virtual");
        }
    }

    #[test]
    fn mixed_environment_picks_expected() {
        let ifaces = vec![
            iface("Loopback Pseudo-Interface 1", [127, 0, 0, 1]),
            iface("vEthernet (Default Switch)", [172, 20, 0, 1]),
            iface("VMware Network Adapter VMnet8", [192, 168, 99, 1]),
            iface("Ethernet", [192, 168, 1, 15]),
        ];
        let picked = pick_lan_ipv4(&ifaces).unwrap();
        assert_eq!(picked.name, "Ethernet");
        assert_eq!(picked.ip, Ipv4Addr::new(192, 168, 1, 15));
    }
}
