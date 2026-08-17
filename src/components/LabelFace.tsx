import { useEffect, useCallback } from 'react';
import { LabelFaceProps } from '../types';

export const LabelFace = ({ qrCodeSvg, displayCode, fileName, progress, onClose }: LabelFaceProps) => {
  const handleKeyDown = useCallback(
    (e: KeyboardEvent) => {
      if (e.key === 'Escape') {
        onClose();
      }
    },
    [onClose]
  );

  useEffect(() => {
    window.addEventListener('keydown', handleKeyDown);
    return () => window.removeEventListener('keydown', handleKeyDown);
  }, [handleKeyDown]);

  const processedQrSvg = qrCodeSvg
    .replace(/<path[^>]*fill="#000000"[^>]*>/g, '<path fill="var(--color-label-bg)" />')
    .replace(/<rect[^>]*fill="#ffffff"[^>]*>/g, '')
    .replace(/<svg([^>]*)>/, '<svg$1 style="background: transparent;">');

  const pct = progress && progress.total > 0 ? Math.min(100, Math.round((progress.current / progress.total) * 100)) : 0;

  return (
    <div
      className="label-face"
      onClick={onClose}
      style={{
        display: 'flex',
        flexDirection: 'column',
        alignItems: 'center',
        justifyContent: 'space-between',
        width: '100%',
        height: '100%',
        background: 'linear-gradient(180deg, rgba(12,18,24,0.96) 0%, rgba(7,10,13,0.98) 100%)',
        borderRadius: 'var(--radius-standard)',
        boxSizing: 'border-box',
        padding: 'var(--space-5) var(--space-4) var(--space-4)',
        position: 'relative',
      }}
      role="button"
      tabIndex={0}
      aria-label="QR code label. Press Escape or click to return."
      onKeyDown={(e) => {
        if (e.key === 'Enter' || e.key === ' ') {
          e.preventDefault();
          onClose();
        }
      }}
    >
      <div style={{ display: 'flex', alignItems: 'center', justifyContent: 'space-between', width: '100%', marginBottom: 'var(--space-3)' }}>
        <div style={{ fontSize: 'var(--font-size-micro)', letterSpacing: '0.2em', textTransform: 'uppercase', color: 'var(--color-label-muted)' }}>
          AirLynk
        </div>
        <div style={{ fontFamily: 'var(--font-mono)', fontSize: 12, color: 'var(--color-label-muted)', letterSpacing: '0.12em' }}>
          {displayCode}
        </div>
      </div>

      <div className="qr-container" style={{ display: 'flex', flexDirection: 'column', alignItems: 'center', gap: 'var(--space-4)', width: '100%' }}>
        <div
          className="qr-code"
          style={{
            width: 220,
            height: 220,
            background: '#f3f7fb',
            borderRadius: 18,
            padding: 'var(--space-3)',
            boxSizing: 'border-box',
            display: 'flex',
            alignItems: 'center',
            justifyContent: 'center',
            boxShadow: '0 20px 40px rgba(0,0,0,0.32)',
          }}
          dangerouslySetInnerHTML={{ __html: processedQrSvg }}
        />

        <div className="label-info" style={{ textAlign: 'center', width: '100%' }}>
          <div style={{ fontSize: 13, color: 'var(--color-label-muted)', textTransform: 'uppercase', letterSpacing: '0.12em', marginBottom: 'var(--space-2)' }}>
            Ready to receive
          </div>

          <div
            className="file-name"
            style={{
              fontSize: 18,
              color: 'var(--color-label-text)',
              fontWeight: 600,
              whiteSpace: 'nowrap',
              overflow: 'hidden',
              textOverflow: 'ellipsis',
              marginBottom: 'var(--space-3)',
            }}
          >
            {fileName ?? 'AirLynk transfer'}
          </div>

          <div style={{ width: '100%', height: 8, borderRadius: 999, background: 'rgba(255,255,255,0.08)', overflow: 'hidden', marginBottom: 'var(--space-2)' }}>
            <div style={{ width: pct + '%', height: '100%', borderRadius: 999, background: 'linear-gradient(90deg, #2dd4bf, #38bdf8)' }} />
          </div>

          <div style={{ display: 'flex', justifyContent: 'space-between', alignItems: 'center', color: 'var(--color-label-muted)', fontSize: 12, letterSpacing: '0.08em', textTransform: 'uppercase' }}>
            <span>{progress && progress.total > 0 ? pct + '% sent' : 'Ready'}</span>
            <span>{displayCode}</span>
          </div>
        </div>
      </div>
    </div>
  );
};
