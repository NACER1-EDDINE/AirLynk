import { memo, useEffect, useRef, useMemo } from 'react';
import { motion } from 'framer-motion';
import { VoidWashProps } from '../types';
import { prefersReducedMotion } from '../lib/motion';

const failureCopy: Record<string, { headline: string; body: string }> = {
  firewall: {
    headline: 'Blocked by firewall',
    body: 'Windows Firewall blocked the connection. Click Retry to run the repair (needs admin), or Dismiss to try another network.',
  },
  isolation: {
    headline: 'Device isolation active',
    body: 'Your network prevents device-to-device traffic (AP/client isolation). This cannot be fixed from the app — use a personal hotspot or a different network.',
  },
  'phone-left': {
    headline: 'Phone left the network',
    body: 'The phone disconnected before the transfer finished. Make sure both devices stay on Wi-Fi and the phone screen stays on.',
  },
  'disk-full': {
    headline: 'Not enough space',
    body: 'The destination drive is full. Free up space in Downloads and click Retry.',
  },
  cancelled: {
    headline: 'Transfer cancelled',
    body: 'You cancelled the transfer. Click Retry to start a new session.',
  },
};

export const VoidWash = memo(function VoidWash({ cause, onRetry, onDismiss }: VoidWashProps) {
  const containerRef = useRef<HTMLDivElement>(null);
  const prefersReduced = useMemo(() => prefersReducedMotion(), []);
  const copy = failureCopy[cause] ?? failureCopy.cancelled;

  useEffect(() => {
    if (containerRef.current && prefersReduced) {
      containerRef.current.style.opacity = '0.08';
    }
  }, [prefersReduced]);

  const duration = prefersReduced ? 0 : 0.35;

  return (
    <motion.div
      ref={containerRef}
      className="void-wash"
      initial="hidden"
      animate="visible"
      variants={{
        hidden: { opacity: 0, y: 8 },
        visible: { opacity: 1, y: 0, transition: { duration, ease: [0.2, 0, 0, 1] } },
      }}
      style={{
        position: 'absolute',
        inset: 0,
        zIndex: 10,
        borderRadius: 'var(--radius-standard)',
        background: 'var(--color-sleeve-bg)',
        display: 'flex',
        flexDirection: 'column',
        alignItems: 'center',
        justifyContent: 'center',
        padding: 'var(--space-5)',
        boxSizing: 'border-box',
      }}
      role="alert"
      aria-live="assertive"
    >
      <div
        className="void-content"
        style={{
          maxWidth: 280,
          textAlign: 'center',
          display: 'flex',
          flexDirection: 'column',
          alignItems: 'center',
          gap: 'var(--space-3)',
        }}
      >
        <h2
          className="void-headline"
          style={{
            fontFamily: 'var(--font-sans)',
            fontSize: 'var(--font-size-display)',
            fontWeight: 700,
            color: 'var(--color-void)',
            margin: 0,
            lineHeight: 'var(--line-height-tight)',
          }}
        >
          {copy.headline}
        </h2>
        <p
          className="void-body"
          style={{
            fontFamily: 'var(--font-sans)',
            fontSize: 'var(--font-size-text)',
            color: 'var(--color-ink)',
            margin: 0,
            lineHeight: 'var(--line-height-normal)',
          }}
        >
          {copy.body}
        </p>
        <div
          className="void-actions"
          style={{
            display: 'flex',
            gap: 'var(--space-3)',
            marginTop: 'var(--space-2)',
          }}
        >
          <button
            className="void-retry"
            onClick={onRetry}
            style={{
              background: 'var(--color-void)',
              color: 'var(--color-label-text)',
              border: 'none',
              borderRadius: 'var(--radius-standard)',
              padding: '10px 20px',
              fontFamily: 'var(--font-sans)',
              fontSize: 'var(--font-size-text)',
              fontWeight: 600,
              cursor: 'pointer',
              transition: 'background 0.15s ease',
            }}
            onMouseEnter={(e) => {
              e.currentTarget.style.background = '#C0362C';
            }}
            onMouseLeave={(e) => {
              e.currentTarget.style.background = 'var(--color-void)';
            }}
            onFocus={(e) => {
              e.currentTarget.style.outline = '2px solid var(--color-void)';
              e.currentTarget.style.outlineOffset = '2px';
            }}
            onBlur={(e) => {
              e.currentTarget.style.outline = 'none';
            }}
          >
            Retry
          </button>
          <button
            className="void-dismiss"
            onClick={onDismiss}
            style={{
              background: 'transparent',
              color: 'var(--color-ink)',
              border: '1px solid var(--color-structure)',
              borderRadius: 'var(--radius-standard)',
              padding: '10px 20px',
              fontFamily: 'var(--font-sans)',
              fontSize: 'var(--font-size-text)',
              fontWeight: 600,
              cursor: 'pointer',
              transition: 'border-color 0.15s ease',
            }}
            onMouseEnter={(e) => {
              e.currentTarget.style.borderColor = 'var(--color-ink)';
            }}
            onMouseLeave={(e) => {
              e.currentTarget.style.borderColor = 'var(--color-structure)';
            }}
            onFocus={(e) => {
              e.currentTarget.style.outline = '2px solid var(--color-signal)';
              e.currentTarget.style.outlineOffset = '2px';
            }}
            onBlur={(e) => {
              e.currentTarget.style.outline = 'none';
            }}
          >
            Dismiss
          </button>
        </div>
      </div>
    </motion.div>
  );
});