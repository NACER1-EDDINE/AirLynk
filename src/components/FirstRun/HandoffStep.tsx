import { useRef } from "react";

interface HandoffStepProps {
  onStart: (mode: "send" | "receive") => void;
  onBack: () => void;
}

export function HandoffStep({ onStart, onBack }: HandoffStepProps) {
  const sendBtnRef = useRef<HTMLButtonElement>(null);
  const receiveBtnRef = useRef<HTMLButtonElement>(null);

  return (
    <div
      className="fr-step-content handoff-step"
      style={{
        display: 'flex',
        flexDirection: 'column',
        alignItems: 'center',
        justifyContent: 'center',
        height: '100%',
        padding: 'var(--space-6) var(--space-5)',
        textAlign: 'center',
        color: 'var(--color-fr-text)',
        fontFamily: 'var(--font-sans)',
      }}
    >
      <h2
        className="fr-step-headline"
        style={{
          fontSize: 'var(--font-size-display)',
          fontWeight: 700,
          margin: '0 0 var(--space-2)',
          color: 'var(--color-fr-text)',
        }}
      >
        Ready.
      </h2>
      <p
        className="handoff-subtitle"
        style={{
          fontSize: 'var(--font-size-text)',
          color: 'var(--color-fr-muted)',
          margin: '0 0 var(--space-6)',
        }}
      >
        Click Send or Receive to start your first transfer.
      </p>

      <div
        className="handoff-buttons"
        role="group"
        aria-label="Transfer mode"
        style={{
          display: 'flex',
          gap: 'var(--space-3)',
          width: '100%',
          maxWidth: 340,
        }}
      >
        <button
          ref={sendBtnRef}
          className="fr-btn fr-btn-primary handoff-btn"
          onClick={() => onStart("send")}
          style={{
            flex: 1,
            padding: '16px 24px',
            fontSize: 'var(--font-size-text)',
            fontWeight: 600,
            fontFamily: 'var(--font-sans)',
            background: 'var(--color-fr-accent)',
            color: 'var(--color-fr-text)',
            border: 'none',
            borderRadius: 'var(--radius-standard)',
            cursor: 'pointer',
          }}
        >
          Send Files
        </button>
        <button
          ref={receiveBtnRef}
          className="fr-btn fr-btn-secondary handoff-btn"
          onClick={() => onStart("receive")}
          style={{
            flex: 1,
            padding: '16px 24px',
            fontSize: 'var(--font-size-text)',
            fontWeight: 600,
            fontFamily: 'var(--font-sans)',
            background: 'transparent',
            color: 'var(--color-fr-text)',
            border: '1px solid var(--color-fr-muted)',
            borderRadius: 'var(--radius-standard)',
            cursor: 'pointer',
          }}
        >
          Receive Files
        </button>
      </div>

      <button
        className="fr-btn fr-btn-secondary handoff-back"
        onClick={onBack}
        style={{
          marginTop: 'var(--space-4)',
          width: '100%',
          maxWidth: 340,
          padding: '12px 24px',
          fontFamily: 'var(--font-sans)',
          fontSize: 'var(--font-size-text)',
          fontWeight: 600,
          background: 'transparent',
          color: 'var(--color-fr-text)',
          border: '1px solid var(--color-fr-muted)',
          borderRadius: 'var(--radius-standard)',
          cursor: 'pointer',
        }}
      >
        Back
      </button>
    </div>
  );
}