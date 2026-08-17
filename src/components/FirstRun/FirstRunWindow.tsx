import { useState, useCallback } from "react";
import { WelcomeStep } from "./WelcomeStep";
import { NetworkCheckStep } from "./NetworkCheckStep";
import { HandoffStep } from "./HandoffStep";

export type FirstRunStep = 0 | 1 | 2;

interface FirstRunWindowProps {
  onComplete: (mode: "send" | "receive") => void;
}

export function FirstRunWindow({ onComplete }: FirstRunWindowProps) {
  const [step, setStep] = useState<FirstRunStep>(0);

  const goNext = useCallback(() => {
    setStep((s) => (s < 2 ? (s + 1 as FirstRunStep) : s));
  }, []);

  const goBack = useCallback(() => {
    setStep((s) => (s > 0 ? (s - 1 as FirstRunStep) : s));
  }, []);

  const handleHandoff = useCallback(
    (mode: "send" | "receive") => {
      onComplete(mode);
    },
    [onComplete]
  );

  const handleSkip = useCallback(() => {
    // Skip first run, go to main app (unsealed state)
    localStorage.setItem('airlynk.firstRun', 'true');
    onComplete('send'); // Default to send mode
  }, [onComplete]);

  const renderStep = () => {
    switch (step) {
      case 0:
        return <WelcomeStep onContinue={goNext} />;
      case 1:
        return <NetworkCheckStep onContinue={goNext} onBack={goBack} />;
      case 2:
        return <HandoffStep onStart={handleHandoff} onBack={goBack} />;
      default:
        return null;
    }
  };

  return (
    <div className="first-run-window" data-tauri-drag-region>
      <header className="fr-header">
        <div className="fr-window-controls">
          <button
            className="fr-window-btn"
            onClick={handleSkip}
            aria-label="Skip and open AirLynk"
            title="Skip and open AirLynk"
          >
            ×
          </button>
        </div>
        <div className="fr-stepper" role="tablist" aria-label="Setup progress">
          <button
            role="tab"
            aria-selected={step === 0}
            aria-controls="step-0-panel"
            id="step-0-tab"
            className={`fr-step ${step === 0 ? "active" : ""} ${step > 0 ? "complete" : ""}`}
            disabled
          >
            <span className="fr-step-dot" aria-hidden="true" />
            <span className="fr-step-label">Welcome</span>
          </button>
          <div className="fr-step-connector" aria-hidden="true" />
          <button
            role="tab"
            aria-selected={step === 1}
            aria-controls="step-1-panel"
            id="step-1-tab"
            className={`fr-step ${step === 1 ? "active" : ""} ${step > 1 ? "complete" : ""}`}
            disabled
          >
            <span className="fr-step-dot" aria-hidden="true" />
            <span className="fr-step-label">Network</span>
          </button>
          <div className="fr-step-connector" aria-hidden="true" />
          <button
            role="tab"
            aria-selected={step === 2}
            aria-controls="step-2-panel"
            id="step-2-tab"
            className={`fr-step ${step === 2 ? "active" : ""}`}
            disabled
          >
            <span className="fr-step-dot" aria-hidden="true" />
            <span className="fr-step-label">Ready</span>
          </button>
        </div>
      </header>
      <main className="fr-content" role="tabpanel" aria-labelledby={`step-${step}-tab`} id={`step-${step}-panel`}>
        {renderStep()}
      </main>
      <footer className="fr-footer">
        <div className="fr-footer-spacer" />
      </footer>
    </div>
  );
}