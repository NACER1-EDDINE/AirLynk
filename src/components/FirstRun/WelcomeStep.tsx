interface WelcomeStepProps {
  onContinue: () => void;
}

export function WelcomeStep({ onContinue }: WelcomeStepProps) {
  return (
    <div
      className="fr-step-content welcome-step"
      style={{
        color: 'var(--color-fr-text)',
        fontFamily: 'var(--font-sans)',
      }}
    >
      <div className="welcome-hero">
        <div
          className="welcome-icon"
          aria-hidden="true"
          style={{
            color: 'var(--color-fr-accent)',
          }}
        >
          <svg viewBox="0 0 48 48" fill="none" xmlns="http://www.w3.org/2000/svg" width="48" height="48">
            <rect x="4" y="12" width="40" height="28" rx="2" stroke="currentColor" strokeWidth="1.5" />
            <path d="M12 12V8C12 6.34 13.34 5 15 5H33C34.66 5 36 6.34 36 8V12" stroke="currentColor" strokeWidth="1.5" strokeLinecap="round" strokeLinejoin="round" />
            <path d="M15 40H33" stroke="currentColor" strokeWidth="1.5" strokeLinecap="round" />
            <path d="M24 20V32" stroke="currentColor" strokeWidth="1.5" strokeLinecap="round" />
            <path d="M16 26H32" stroke="currentColor" strokeWidth="1.5" strokeLinecap="round" />
          </svg>
        </div>
        <h1 className="welcome-headline">AirLynk</h1>
        <p className="welcome-subtitle">Move files between your PC and phone. No cloud. No account. No cable.</p>
      </div>

      <div className="welcome-meta">Setup takes under a minute.</div>

      <div className="welcome-body">
        <p>AirLynk works like a sealed courier sleeve. You drop files in, the sleeve seals, and a QR label appears on the outside. Scan it with your phone's camera — the sleeve opens on their screen, and the files transfer directly over your local network.</p>
        <p>Nothing leaves your network. Nothing is stored anywhere else. It works with the internet unplugged.</p>
      </div>

      <button
        className="fr-btn fr-btn-primary"
        onClick={onContinue}
        autoFocus
        style={{
          minWidth: 180,
          minHeight: 50,
          fontSize: 18,
          fontWeight: 700,
          letterSpacing: '-0.02em',
          borderRadius: 12,
          boxShadow: '0 10px 30px rgba(59, 130, 246, 0.22)',
        }}
      >
        Continue
      </button>
    </div>
  );
}