import React, { useState, useEffect } from 'react';
import { ClipboardService } from '../services/clipboardService';

interface OnboardingFlowProps {
  isVisible: boolean;
  onComplete: (settings: OnboardingSettings) => void;
}

export interface OnboardingSettings {
  password: string;
  keyCombo: string;
  sessionTimeMinutes: number;
}

export const OnboardingFlow: React.FC<OnboardingFlowProps> = ({ isVisible, onComplete }) => {
  const [step, setStep] = useState(0);
  const [password, setPassword] = useState('');
  const [confirmPassword, setConfirmPassword] = useState('');
  const [keyCombo, setKeyCombo] = useState('');
  const [sessionTime, setSessionTime] = useState(15);
  const [platform, setPlatform] = useState<string>('');


  // Platform detection and default shortcut setup
  useEffect(() => {
    const detectPlatform = async () => {
      const detectedPlatform = await ClipboardService.getPlatform();
      setPlatform(detectedPlatform);

      // Set default key combo based on platform
      if (!keyCombo) {
        if (detectedPlatform === 'macos') {
          setKeyCombo('Cmd+Shift+C');
        } else {
          setKeyCombo('Ctrl+Shift+C');
        }
      }
    };

    if (isVisible) {
      detectPlatform();
    }
  }, [isVisible, keyCombo]);


  // Generate platform-aware key combo options
  const getKeyComboOptions = () => {
    const isMac = platform === 'macos';
    const cmdKey = isMac ? '⌘' : 'Ctrl';
    const altKey = isMac ? '⌥' : 'Alt';

    return [
      { value: `${isMac ? 'Cmd' : 'Ctrl'}` + `+Shift+C`, label: `${cmdKey} + Shift + C` },
      { value: `${isMac ? 'Cmd' : 'Ctrl'}` + `+Shift+V`, label: `${cmdKey} + Shift + V` },
      { value: `${isMac ? 'Cmd' : 'Ctrl'}` + `+${isMac ? 'Option' : 'Alt'}+V`, label: `${cmdKey} + ${altKey} + V` },
      { value: `${isMac ? 'Cmd' : 'Ctrl'}` + `+${isMac ? 'Option' : 'Alt'}+C`, label: `${cmdKey} + ${altKey} + C` },
    ];
  };

  const steps = [
    // Welcome step
    {
      title: 'Create Your Master Password',
      content: (
        <div className="onboarding-password">
          <div className="password-info">
            <p>Your master password encrypts all clipboard data. Choose a strong password you'll remember.</p>
          </div>

          <div className="setting-group">
            <label htmlFor="password">Master Password</label>
            <input
              id="password"
              type="password"
              value={password}
              onChange={(e) => setPassword(e.target.value)}
              placeholder="Enter a strong password (minimum 6 characters)"
              autoFocus
            />
            {password && password.length < 6 && (
              <div className="input-hint">Password must be at least 6 characters</div>
            )}
          </div>

          <div className="setting-group">
            <label htmlFor="confirmPassword">Confirm Password</label>
            <input
              id="confirmPassword"
              type="password"
              value={confirmPassword}
              onChange={(e) => setConfirmPassword(e.target.value)}
              placeholder="Confirm your password"
            />
            {password && confirmPassword && password === confirmPassword && (
              <div className="success-message">✓ Passwords match</div>
            )}
            {password && confirmPassword && password !== confirmPassword && (
              <div className="error-message">❌ Passwords don't match</div>
            )}
          </div>


        </div>
      )
    },
    // Settings configuration
    {
      title: 'Configure Settings',
      content: (
        <div className="onboarding-settings">
          <div className="setting-group">
            <label htmlFor="keyCombo">Global Hotkey</label>
            <select
              id="keyCombo"
              value={keyCombo}
              onChange={(e) => setKeyCombo(e.target.value)}
            >
              {getKeyComboOptions().map((option) => (
                <option key={option.value} value={option.value}>
                  {option.label}
                </option>
              ))}
            </select>
            <small>Press this key combination to open Clip Vault from anywhere</small>
          </div>

          <div className="setting-group">
            <label htmlFor="sessionTime">Auto-lock Session (minutes)</label>
            <select
              id="sessionTime"
              value={sessionTime}
              onChange={(e) => setSessionTime(Number(e.target.value))}
            >
              <option value={5}>5 minutes</option>
              <option value={15}>15 minutes</option>
              <option value={30}>30 minutes</option>
              <option value={60}>1 hour</option>
              <option value={120}>2 hours</option>
              <option value={0}>Never</option>
            </select>
            <small>Automatically lock the vault after this period of inactivity</small>
          </div>
        </div>
      )
    }
  ];

  // Early return after all hooks are defined
  if (!isVisible) return null;

  const currentStep = steps[step];
  const isLastStep = step === steps.length - 1;
  const canProceed = step === 0 ||
    (step === 1 && password && confirmPassword && password === confirmPassword && password.length >= 6) ||
    (step === 2);

  const handleNext = () => {
    if (isLastStep) {
      onComplete({
        password,
        keyCombo,
        sessionTimeMinutes: sessionTime
      });
    } else {
      setStep(step + 1);
    }
  };

  const handleBack = () => {
    if (step > 0) {
      setStep(step - 1);
    }
  };

  return (
    <div className="modal-overlay onboarding-overlay">
      <div className="modal onboarding-modal">
        <div className="onboarding-header">
          <h2>{currentStep.title}</h2>
          <div className="step-indicator">
            {steps.map((_, index) => (
              <div
                key={index}
                className={`step-dot ${index <= step ? 'active' : ''}`}
              />
            ))}
          </div>
        </div>

        <div className="onboarding-content">
          {currentStep.content}
        </div>

        <div className="onboarding-footer">
          {step > 0 && (
            <button
              className="modal-button secondary"
              onClick={handleBack}
            >
              Back
            </button>
          )}
          <button
            className="modal-button primary"
            onClick={handleNext}
            disabled={!canProceed}
          >
            {isLastStep ? 'Create Vault' : 'Next'}
          </button>
        </div>
      </div>
    </div>
  );
};