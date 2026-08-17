import { useEffect, useState, useCallback } from "react";
import {
  getNetworkInfo,
  checkFirewall,
  verifyReachability,
  repairFirewall,
} from "../../lib/ipc";
import type { NetworkInfoResult, FirewallStatusResult, ReachabilityResult } from "../../lib/ipc";

interface NetworkCheckStepProps {
  onContinue: () => void;
  onBack: () => void;
}

interface NetworkInfoState extends NetworkInfoResult {
  loading: boolean;
  error?: string;
}

interface FirewallState extends FirewallStatusResult {
  loading: boolean;
  error?: string;
}

interface ReachabilityState extends ReachabilityResult {
  loading: boolean;
  error?: string;
}

export function NetworkCheckStep({ onContinue, onBack }: NetworkCheckStepProps) {
  const [isRefreshing, setIsRefreshing] = useState(false);
  const [isRepairing, setIsRepairing] = useState(false);
  const [networkInfo, setNetworkInfo] = useState<NetworkInfoState>({
    lanIp: "",
    interfaceName: "",
    port: 0,
    loading: true,
  });
  const [firewall, setFirewall] = useState<FirewallState>({
    status: "unknown",
    loading: true,
  });
  const [reachability, setReachability] = useState<ReachabilityState>({
    diagnosis: "bindFailed",
    bindAddr: "",
    interfaceName: "",
    detail: "",
    loading: true,
  });

  const readError = (error: unknown, fallback: string): string => {
    if (error instanceof Error && error.message.trim()) return error.message;
    if (typeof error === "string" && error.trim()) return error;
    return fallback;
  };

  const loadAll = useCallback(async () => {
    setIsRefreshing(true);
    setNetworkInfo((s) => ({ ...s, loading: true, error: undefined }));
    setFirewall((s) => ({ ...s, loading: true, error: undefined }));
    setReachability((s) => ({ ...s, loading: true, error: undefined }));

    const [netResult, firewallResult] = await Promise.allSettled([
      getNetworkInfo(),
      checkFirewall(),
    ]);

    let netInfo: NetworkInfoResult | null = null;
    if (netResult.status === "fulfilled") {
      netInfo = netResult.value;
      setNetworkInfo({ ...netInfo, loading: false, error: undefined });
    } else {
      setNetworkInfo((s) => ({
        ...s,
        loading: false,
        error: readError(netResult.reason, "Failed to get network info"),
      }));
    }

    if (firewallResult.status === "fulfilled") {
      setFirewall({ ...firewallResult.value, loading: false, error: undefined });
    } else {
      setFirewall((s) => ({
        ...s,
        loading: false,
        error: readError(firewallResult.reason, "Failed to check firewall"),
      }));
    }

    if (netInfo?.lanIp) {
      try {
        const reach = await verifyReachability(
          `${netInfo.lanIp}:${netInfo.port}`,
          netInfo.interfaceName
        );
        setReachability({ ...reach, loading: false, error: undefined });
      } catch (error) {
        setReachability((s) => ({
          ...s,
          loading: false,
          error: readError(error, "Failed to verify reachability"),
        }));
      }
    } else {
      setReachability((s) => ({
        ...s,
        loading: false,
        diagnosis: "bindFailed",
        detail:
          "No LAN address found. Check that your PC is connected to Wi-Fi or Ethernet.",
      }));
    }
    setIsRefreshing(false);
  }, []);

  useEffect(() => {
    loadAll();
  }, [loadAll]);

  const handleRepair = useCallback(async () => {
    try {
      setIsRepairing(true);
      await repairFirewall();
      await loadAll();
    } catch (error) {
      setFirewall((s) => ({
        ...s,
        loading: false,
        error: readError(error, "Failed to repair firewall"),
      }));
    } finally {
      setIsRepairing(false);
    }
  }, [loadAll]);

  useEffect(() => {
    if (reachability.diagnosis !== "bound" || reachability.loading || reachability.error) {
      return;
    }
    const timeout = window.setTimeout(() => {
      setReachability((current) =>
        current.diagnosis === "bound"
          ? {
              ...current,
              diagnosis: "noPhoneYet",
              detail:
                "Server is ready. If your phone still cannot open the link, your Wi-Fi may block device-to-device traffic. Try a private network or a mobile hotspot.",
            }
          : current
      );
    }, 30_000);
    return () => window.clearTimeout(timeout);
  }, [reachability]);

  const canContinue =
    Boolean(networkInfo.lanIp) &&
    (reachability.diagnosis === "bound" || reachability.diagnosis === "noPhoneYet");

  const checkIconStyle = {
    width: 20,
    height: 20,
    flexShrink: 0,
    display: "flex",
    alignItems: 'center',
    justifyContent: 'center',
    fontSize: 14,
  };

  const checkLabelStyle = {
    fontFamily: 'var(--font-sans)',
    fontSize: 'var(--font-size-text)',
    color: 'var(--color-fr-text)',
  };

  const checkValueStyle = {
    fontFamily: 'var(--font-mono)',
    fontSize: 'var(--font-size-micro)',
    fontVariantNumeric: 'tabular-nums lining-nums',
  };

  const checkExplanationStyle = {
    fontFamily: 'var(--font-sans)',
    fontSize: 'var(--font-size-micro)',
    color: 'var(--color-fr-muted)',
    marginTop: 'var(--space-1)',
  };

  return (
    <div
      className="fr-step-content network-check-step"
      style={{
        display: 'flex',
        flexDirection: 'column',
        height: '100%',
        padding: 'var(--space-5) var(--space-5)',
        color: 'var(--color-fr-text)',
        fontFamily: 'var(--font-sans)',
      }}
    >
      <h2
        className="fr-step-headline"
        style={{
          fontSize: 'var(--font-size-display)',
          fontWeight: 700,
          margin: '0 0 var(--space-5)',
          color: 'var(--color-fr-text)',
        }}
      >
        Network Check
      </h2>
      <p
        className="fr-step-subtitle"
        style={{
          fontSize: 'var(--font-size-text)',
          color: 'var(--color-fr-muted)',
          margin: '0 0 var(--space-5)',
        }}
      >
        Verifying your network is ready for direct transfers.
      </p>
      <div
        style={{
          display: "flex",
          justifyContent: "space-between",
          alignItems: "center",
          gap: "var(--space-3)",
        }}
      >
        <p style={{ margin: 0, fontSize: "var(--font-size-micro)", color: "var(--color-fr-muted)" }}>
          Keep this open while checks complete.
        </p>
        <button
          className="fr-btn fr-btn-secondary"
          onClick={loadAll}
          disabled={isRefreshing || isRepairing}
          style={{
            background: "transparent",
            color: "var(--color-fr-text)",
            border: "1px solid var(--color-fr-muted)",
            borderRadius: "var(--radius-standard)",
            padding: "8px 14px",
            fontFamily: "var(--font-sans)",
            fontSize: "var(--font-size-micro)",
            fontWeight: 600,
            cursor: isRefreshing || isRepairing ? "not-allowed" : "pointer",
          }}
        >
          {isRefreshing ? "Refreshing..." : "Refresh"}
        </button>
      </div>

      <div
        className="fr-checks"
        style={{
          flex: 1,
          display: 'flex',
          flexDirection: 'column',
          gap: 'var(--space-4)',
          overflowY: 'auto',
        }}
      >
        {/* Network Info */}
        <div
          className="fr-check"
          style={{
            display: 'flex',
            flexDirection: 'column',
            gap: 'var(--space-2)',
            padding: 'var(--space-4)',
            background: 'rgba(255,255,255,0.03)',
            borderRadius: 'var(--radius-standard)',
            border: '1px solid rgba(139,147,156,0.2)',
          }}
        >
          <div
            className="fr-check-header"
            style={{
              display: 'flex',
              alignItems: 'center',
              gap: 'var(--space-3)',
            }}
          >
            <span
              className={`fr-check-icon ${
                networkInfo.loading ? "loading" : networkInfo.lanIp ? "success" : "error"
              }`}
              aria-hidden="true"
              style={{
                ...checkIconStyle,
                color: networkInfo.loading ? 'var(--color-fr-muted)' : networkInfo.lanIp ? 'var(--color-fr-accent)' : 'var(--color-fr-error)',
              }}
            >
              {networkInfo.loading ? (
                <svg
                  width="20"
                  height="20"
                  viewBox="0 0 20 20"
                  className="fr-spinner"
                  aria-hidden="true"
                >
                  <circle
                    cx="10"
                    cy="10"
                    r="8"
                    stroke="currentColor"
                    strokeWidth="2"
                    fill="none"
                    strokeDasharray="31.4 31.4"
                    strokeLinecap="round"
                  />
                </svg>
              ) : networkInfo.lanIp ? "✓" : "✗"}
            </span>
            <span style={checkLabelStyle}>Network Interface</span>
          </div>
          {networkInfo.loading ? (
            <div style={{ ...checkValueStyle, color: "var(--color-fr-muted)" }}>Detecting...</div>
          ) : networkInfo.error ? (
            <div style={{ ...checkValueStyle, color: "var(--color-fr-error)" }}>{networkInfo.error}</div>
          ) : (
            <>
              <div style={{ ...checkValueStyle, color: "var(--color-fr-text)" }}>{networkInfo.lanIp}</div>
              <p style={checkExplanationStyle}>Interface: {networkInfo.interfaceName} · Port: {networkInfo.port}</p>
            </>
          )}
        </div>

        {/* Firewall */}
        <div
          className="fr-check"
          style={{
            display: 'flex',
            flexDirection: 'column',
            gap: 'var(--space-2)',
            padding: 'var(--space-4)',
            background: 'rgba(255,255,255,0.03)',
            borderRadius: 'var(--radius-standard)',
            border: '1px solid rgba(139,147,156,0.2)',
          }}
        >
          <div
            className="fr-check-header"
            style={{
              display: 'flex',
              alignItems: 'center',
              gap: 'var(--space-3)',
            }}
          >
            <span
              className={`fr-check-icon ${firewall.loading ? "loading" : firewall.status === "clean" ? "success" : "error"}`}
              aria-hidden="true"
              style={{
                ...checkIconStyle,
                color: firewall.loading ? 'var(--color-fr-muted)' : firewall.status === "clean" ? 'var(--color-fr-accent)' : 'var(--color-fr-error)',
              }}
            >
              {firewall.loading ? (
                <svg width="20" height="20" viewBox="0 0 20 20" style={{ animation: 'spin 1s linear infinite' }}>
                  <circle cx="10" cy="10" r="8" stroke="currentColor" strokeWidth="2" fill="none" strokeDasharray="31.4 31.4" strokeLinecap="round" />
                </svg>
              ) : firewall.status === "clean" ? '✓' : '✗'}
            </span>
            <span style={checkLabelStyle}>Windows Firewall</span>
          </div>
          {firewall.loading ? (
            <div style={{ ...checkValueStyle, color: 'var(--color-fr-muted)' }}>Checking...</div>
          ) : firewall.error ? (
            <div style={{ ...checkValueStyle, color: 'var(--color-fr-error)' }}>{firewall.error}</div>
          ) : firewall.status === "clean" ? (
            <div style={{ ...checkValueStyle, color: 'var(--color-fr-accent)' }}>Allow rule active</div>
          ) : firewall.status === "missingAllowRule" ? (
            <>
              <div style={{ ...checkValueStyle, color: 'var(--color-fr-error)' }}>✗ ALLOW RULE MISSING</div>
              <p style={checkExplanationStyle}>The inbound allow rule was not created. Run repair to add it.</p>
              <button
                className="fr-btn fr-btn-secondary"
                onClick={handleRepair}
                disabled={firewall.loading || isRepairing}
                style={{
                  marginTop: 'var(--space-2)',
                  alignSelf: 'flex-start',
                  background: 'transparent',
                  color: 'var(--color-fr-accent)',
                  border: '1px solid var(--color-fr-accent)',
                  borderRadius: 'var(--radius-standard)',
                  padding: '8px 16px',
                  fontFamily: 'var(--font-sans)',
                  fontSize: 'var(--font-size-text)',
                  fontWeight: 600,
                  cursor: 'pointer',
                }}
              >
                Repair Firewall
              </button>
            </>
          ) : firewall.status === "staleBlockRules" ? (
            <>
              <div style={{ ...checkValueStyle, color: 'var(--color-fr-error)' }}>✗ FIREWALL BLOCKED</div>
              <p style={checkExplanationStyle}>Stale block rules detected: {firewall.blocks?.join(', ') || 'unknown'}. Run repair to clear them.</p>
              <button
                className="fr-btn fr-btn-secondary"
                onClick={handleRepair}
                disabled={firewall.loading || isRepairing}
                style={{
                  marginTop: 'var(--space-2)',
                  alignSelf: 'flex-start',
                  background: 'transparent',
                  color: 'var(--color-fr-accent)',
                  border: '1px solid var(--color-fr-accent)',
                  borderRadius: 'var(--radius-standard)',
                  padding: '8px 16px',
                  fontFamily: 'var(--font-sans)',
                  fontSize: 'var(--font-size-text)',
                  fontWeight: 600,
                  cursor: 'pointer',
                }}
              >
                Repair Firewall
              </button>
            </>
          ) : (
            <div style={{ ...checkValueStyle, color: 'var(--color-fr-error)' }}>Unknown state</div>
          )}
        </div>

        {/* Reachability */}
        <div
          className="fr-check"
          style={{
            display: 'flex',
            flexDirection: 'column',
            gap: 'var(--space-2)',
            padding: 'var(--space-4)',
            background: 'rgba(255,255,255,0.03)',
            borderRadius: 'var(--radius-standard)',
            border: '1px solid rgba(139,147,156,0.2)',
          }}
        >
          <div
            className="fr-check-header"
            style={{
              display: 'flex',
              alignItems: 'center',
              gap: 'var(--space-3)',
            }}
          >
            <span
              className={`fr-check-icon ${
                reachability.loading
                  ? "loading"
                  : reachability.diagnosis === "bound"
                    ? "success"
                    : reachability.diagnosis === "noPhoneYet"
                      ? "warning"
                      : "error"
              }`}
              aria-hidden="true"
              style={{
                ...checkIconStyle,
                color:
                  reachability.loading
                    ? "var(--color-fr-muted)"
                    : reachability.diagnosis === "bound"
                      ? "var(--color-fr-accent)"
                      : reachability.diagnosis === "noPhoneYet"
                        ? "var(--color-fr-text)"
                        : "var(--color-fr-error)",
              }}
            >
              {reachability.loading ? (
                <svg width="20" height="20" viewBox="0 0 20 20" style={{ animation: 'spin 1s linear infinite' }}>
                  <circle cx="10" cy="10" r="8" stroke="currentColor" strokeWidth="2" fill="none" strokeDasharray="31.4 31.4" strokeLinecap="round" />
                </svg>
              ) : reachability.diagnosis === "bound" ? '✓' : reachability.diagnosis === "noPhoneYet" ? '⚠' : '✗'}
            </span>
            <span style={checkLabelStyle}>Reachability</span>
          </div>
          {reachability.loading ? (
            <div style={{ ...checkValueStyle, color: 'var(--color-fr-muted)' }}>Verifying...</div>
          ) : reachability.error ? (
            <div style={{ ...checkValueStyle, color: 'var(--color-fr-error)' }}>{reachability.error}</div>
          ) : reachability.diagnosis === "bound" ? (
            <div style={{ ...checkValueStyle, color: 'var(--color-fr-accent)' }}>Ready — server reachable</div>
          ) : reachability.diagnosis === "bindFailed" ? (
            <>
              <div style={{ ...checkValueStyle, color: 'var(--color-fr-error)' }}>✗ BIND FAILED</div>
              <p style={checkExplanationStyle}>{reachability.detail || 'Could not bind to LAN address.'}</p>
            </>
          ) : reachability.diagnosis === "blockedByFirewall" ? (
            <>
              <div style={{ ...checkValueStyle, color: 'var(--color-fr-error)' }}>✗ BLOCKED BY FIREWALL</div>
              <p style={checkExplanationStyle}>{reachability.detail || 'Inbound connection blocked. Run firewall repair.'}</p>
              <button
                className="fr-btn fr-btn-secondary"
                onClick={handleRepair}
                disabled={firewall.loading || isRepairing}
                style={{
                  marginTop: 'var(--space-2)',
                  alignSelf: 'flex-start',
                  background: 'transparent',
                  color: 'var(--color-fr-accent)',
                  border: '1px solid var(--color-fr-accent)',
                  borderRadius: 'var(--radius-standard)',
                  padding: '8px 16px',
                  fontFamily: 'var(--font-sans)',
                  fontSize: 'var(--font-size-text)',
                  fontWeight: 600,
                  cursor: 'pointer',
                }}
              >
                Repair Firewall
              </button>
            </>
          ) : reachability.diagnosis === "noPhoneYet" ? (
            <>
              <div style={{ ...checkValueStyle, color: "var(--color-fr-text)" }}>
                ⚠ NO PHONE YET — network may isolate devices
              </div>
              <p style={checkExplanationStyle}>{reachability.detail || 'Server bound, but no phone connected. If this persists, your network may have device isolation enabled.'}</p>
            </>
          ) : (
            <div style={{ ...checkValueStyle, color: 'var(--color-fr-error)' }}>Unknown diagnosis</div>
          )}
        </div>
      </div>

      <div
        className="fr-step-actions"
        style={{
          display: 'flex',
          justifyContent: 'space-between',
          marginTop: 'var(--space-4)',
          paddingTop: 'var(--space-4)',
          borderTop: '1px solid rgba(139,147,156,0.2)',
        }}
      >
        <button
          className="fr-btn fr-btn-secondary"
          onClick={onBack}
          style={{
            background: 'transparent',
            color: 'var(--color-fr-text)',
            border: '1px solid var(--color-fr-muted)',
            borderRadius: 'var(--radius-standard)',
            padding: '12px 24px',
            fontFamily: 'var(--font-sans)',
            fontSize: 'var(--font-size-text)',
            fontWeight: 600,
            cursor: 'pointer',
          }}
        >
          Back
        </button>
        <button
          className="fr-btn fr-btn-primary"
          onClick={onContinue}
          disabled={!canContinue}
          style={{
            background: canContinue ? 'var(--color-fr-accent)' : 'rgba(139,147,156,0.3)',
            color: canContinue ? 'var(--color-fr-text)' : 'var(--color-fr-muted)',
            border: 'none',
            borderRadius: 'var(--radius-standard)',
            padding: '12px 24px',
            fontFamily: 'var(--font-sans)',
            fontSize: 'var(--font-size-text)',
            fontWeight: 600,
            cursor: canContinue ? 'pointer' : 'not-allowed',
            opacity: canContinue ? 1 : 0.6,
          }}
        >
          Continue
        </button>
      </div>
    </div>
  );
}