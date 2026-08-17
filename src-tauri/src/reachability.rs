//! Reachability verification after binding (FR-30, FR-31).
//!
//! After the HTTP server binds, we verify that the inbound path actually works
//! rather than assuming it does. We also discriminate between "the PC firewall
//! is blocking" and "the network forbids device-to-device traffic" (client
//! isolation), because presenting "waiting for your phone" when the true state
//! is "blocked" violates Principle 5.
//!
//! The known limitation (per PLAN §2.5 research): a PC-local TCP connect to
//! the machine's own LAN IPv4 **may bypass Windows Firewall**. A successful
//! self-connect therefore proves the server is running but does not guarantee
//! external reachability. The phone connecting is the only definitive test.
//!
//! What this module provides:
//! 1. A fast self-test that confirms the server is bound and responsive.
//! 2. A timeout-based notification trigger: if no phone connects within a
//!    grace period, the UI surfaces the client-isolation explanation.
//! 3. Structured diagnostic info (interface name, LAN IP) surfaced only in
//!    failure states (FR-24).

use std::net::{Ipv4Addr, SocketAddr};
use std::time::Duration;

/// How long we wait after binding before suggesting that client isolation
/// might be the cause. Shorter than the session inactivity timeout; long
/// enough for a real person to scan a QR and have their browser load.
pub const PHONE_GRACE_PERIOD: Duration = Duration::from_secs(30);

/// Self-test connect timeout. A localhost or LAN-self connect should resolve
/// nearly instantly; anything longer signals a problem.
const SELF_TEST_TIMEOUT: Duration = Duration::from_secs(2);

// ---------------------------------------------------------------------------
// Diagnosis
// ---------------------------------------------------------------------------

/// The reachability verdict, with enough detail for the UI to render the
/// correct failure state per FR-32.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum ReachDiagnosis {
    /// Server bound and self-connect succeeded.
    /// Does NOT guarantee external reachability (see module docs).
    Bound,
    /// Failed to bind — likely port conflict or no usable LAN address.
    BindFailed,
    /// Bound but self-connect failed or timed out. Firewall blocking is the
    /// most likely cause.
    BlockedByFirewall,
    /// Grace period elapsed with no phone connection. Client isolation is
    /// the most likely cause, though simple user inattention is also possible.
    NoPhoneYet,
}

#[derive(Debug, Clone)]
pub struct ReachReport {
    pub diagnosis: ReachDiagnosis,
    /// The address the server is actually listening on.
    pub bind_addr: SocketAddr,
    /// The human-readable interface name (e.g. "Wi-Fi"), for diagnostic copy.
    pub interface_name: String,
    /// Plain-language detail for the UI, per Principle 5. Empty when Bound.
    pub detail: String,
}

impl ReachReport {
    pub fn bind_failed(interface_name: String, detail: String) -> Self {
        Self {
            diagnosis: ReachDiagnosis::BindFailed,
            bind_addr: SocketAddr::new(Ipv4Addr::UNSPECIFIED.into(), 0),
            interface_name,
            detail,
        }
    }
}

// ---------------------------------------------------------------------------
// Verification
// ---------------------------------------------------------------------------

/// Quick synchronous self-test: attempt a TCP connect to the bound address.
/// On Windows, a connection to the machine's own LAN IP from the same machine
/// **may** bypass Windows Firewall, so a true result means "server is running"
/// but not necessarily "external devices can reach us."
///
/// Uses `std::net::TcpStream::connect_timeout` — synchronous because this
/// runs once at session start, not on a hot path.
pub fn self_test(addr: SocketAddr) -> bool {
    std::net::TcpStream::connect_timeout(&addr, SELF_TEST_TIMEOUT).is_ok()
}

/// Run the full verification flow synchronously: bind info + self-test.
/// For the async timeout-based "no phone yet" path, the caller (server_manager
/// or frontend) should start a timer after calling this and emit a
/// `ReachDiagnosis::NoPhoneYet` notification if no connection occurs within
/// `PHONE_GRACE_PERIOD`.
pub fn verify(addr: SocketAddr, interface_name: &str) -> ReachReport {
    if addr.ip().is_unspecified() || addr.port() == 0 {
        return ReachReport::bind_failed(
            interface_name.to_string(),
            "No usable LAN address found. Check that your PC is connected to Wi-Fi or Ethernet."
                .to_string(),
        );
    }

    if self_test(addr) {
        ReachReport {
            diagnosis: ReachDiagnosis::Bound,
            bind_addr: addr,
            interface_name: interface_name.to_string(),
            detail: String::new(),
        }
    } else {
        ReachReport {
            diagnosis: ReachDiagnosis::BlockedByFirewall,
            bind_addr: addr,
            interface_name: interface_name.to_string(),
            detail: format!(
                "The server is running but connections to {} on {} are blocked. \
                 Windows Firewall may be preventing inbound traffic. \
                 Open AirLynk and select Repair Firewall to fix this.",
                addr, interface_name
            ),
        }
    }
}

/// The plain-language explanation surfaced when no phone connects within the
/// grace period. Follows the PLAN's directive to copy Google Nest's honesty:
/// name the cause plainly and avoid hanging.
pub fn isolation_explanation(interface_name: &str) -> String {
    format!(
        "No device connected yet. If your phone cannot reach this PC, \
         your Wi‑Fi network may be blocking device‑to‑device traffic — \
         guest, hotel, and public networks commonly do this. \
         Try:\n  • Switching both devices to a private/home network\n  \
         • Using your phone as a mobile hotspot and connecting this PC to it\n\n\
         PC address: {interface_name}"
    )
}

// ---------------------------------------------------------------------------
// Tests
// ---------------------------------------------------------------------------

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn bind_failed_diagnosis_has_no_socket() {
        let r = ReachReport::bind_failed("Wi-Fi".into(), "no address".into());
        assert_eq!(r.diagnosis, ReachDiagnosis::BindFailed);
        assert_eq!(r.bind_addr.port(), 0);
        assert_eq!(r.interface_name, "Wi-Fi");
        assert!(!r.detail.is_empty());
    }

    #[test]
    fn verify_rejects_unspecified() {
        let r = verify(
            SocketAddr::new(Ipv4Addr::UNSPECIFIED.into(), 0),
            "Wi-Fi",
        );
        assert_eq!(r.diagnosis, ReachDiagnosis::BindFailed);
    }

    #[test]
    fn verify_rejects_zero_port() {
        let r = verify(
            SocketAddr::new(Ipv4Addr::new(192, 168, 1, 15).into(), 0),
            "Wi-Fi",
        );
        assert_eq!(r.diagnosis, ReachDiagnosis::BindFailed);
    }

    #[test]
    fn diagnosis_variants_have_distinct_identities() {
        // All variants must be distinguishable — the UI maps each to a
        // different failure state (FR-32).
        assert_ne!(ReachDiagnosis::Bound, ReachDiagnosis::BindFailed);
        assert_ne!(ReachDiagnosis::Bound, ReachDiagnosis::BlockedByFirewall);
        assert_ne!(ReachDiagnosis::Bound, ReachDiagnosis::NoPhoneYet);
        assert_ne!(ReachDiagnosis::BlockedByFirewall, ReachDiagnosis::NoPhoneYet);
    }

    #[test]
    fn isolation_explanation_includes_interface_name() {
        let msg = isolation_explanation("Wi-Fi");
        assert!(msg.contains("Wi-Fi"));
        // Must include at least one concrete next action (Principle 5).
        assert!(msg.contains("hotspot") || msg.contains("private"));
    }

    #[test]
    fn self_test_unknown_address_returns_false() {
        // 192.0.2.0/24 is TEST-NET-1 per RFC 5737 — guaranteed unreachable.
        let addr = SocketAddr::new(Ipv4Addr::new(192, 0, 2, 1).into(), 9999);
        assert!(!self_test(addr));
    }

    #[test]
    fn self_test_succeeds_against_a_live_listener() {
        // Bind a real listener (test-only, loopback) and confirm self_test
        // connects to it. This proves the "Bound" path of `verify`.
        let listener = std::net::TcpListener::bind("127.0.0.1:0").unwrap();
        let addr = listener.local_addr().unwrap();
        assert!(self_test(addr));
        drop(listener);
        // After the listener closes, the same probe must fail.
        assert!(!self_test(addr));
    }

    #[test]
    fn verify_reports_bound_for_live_listener() {
        let listener = std::net::TcpListener::bind("127.0.0.1:0").unwrap();
        let addr = listener.local_addr().unwrap();
        let report = verify(addr, "TestIface");
        assert_eq!(report.diagnosis, ReachDiagnosis::Bound);
        assert_eq!(report.bind_addr, addr);
        assert_eq!(report.interface_name, "TestIface");
        assert!(report.detail.is_empty(), "Bound must carry no failure copy");
    }

    #[test]
    fn grace_period_is_reasonable() {
        // Must be shorter than the session inactivity timeout so the user
        // sees the explanation before the session expires.
        assert!(PHONE_GRACE_PERIOD.as_secs() <= 30);
    }
}