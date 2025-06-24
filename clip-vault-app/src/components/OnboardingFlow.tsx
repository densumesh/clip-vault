import React, { useState, useEffect, useRef } from 'react';
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
  const [currentFeature, setCurrentFeature] = useState(0);
  const [isUserInteracting, setIsUserInteracting] = useState(false);
  const [platform, setPlatform] = useState<string>('');
  const autoAdvanceTimerRef = useRef<number | null>(null);

  const features = [
    {
      icon: '🔒',
      title: 'Encrypted Storage',
      description: 'Your clipboard history is encrypted with SQLCipher for maximum security'
    },
    {
      icon: '🔍',
      title: 'Powerful Search',
      description: 'Instantly find any copied text with fuzzy search and filtering'
    },
    {
      icon: '✏️',
      title: 'Edit & Organize',
      description: 'Edit clipboard items with syntax highlighting and smart formatting'
    },
    {
      icon: '⚡',
      title: 'Quick Access',
      description: 'Global hotkey for instant clipboard access from anywhere'
    }
  ];

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

  // Auto-advance feature cards
  useEffect(() => {
    if (step === 0 && isVisible && !isUserInteracting) {
      autoAdvanceTimerRef.current = setInterval(() => {
        setCurrentFeature(prev => (prev + 1) % features.length);
      }, 3000);

      return () => {
        if (autoAdvanceTimerRef.current) {
          clearInterval(autoAdvanceTimerRef.current);
          autoAdvanceTimerRef.current = null;
        }
      };
    }
  }, [step, isVisible, isUserInteracting, features.length]);

  // Reset user interaction after delay
  useEffect(() => {
    if (isUserInteracting) {
      const resetTimer = setTimeout(() => {
        setIsUserInteracting(false);
      }, 2000); // Resume auto-advance after 2 seconds

      return () => clearTimeout(resetTimer);
    }
  }, [isUserInteracting]);

  const nextFeature = () => {
    setIsUserInteracting(true);
    setCurrentFeature(prev => (prev + 1) % features.length);
  };

  const prevFeature = () => {
    setIsUserInteracting(true);
    setCurrentFeature(prev => (prev - 1 + features.length) % features.length);
  };

  const goToFeature = (index: number) => {
    setIsUserInteracting(true);
    setCurrentFeature(index);
  };

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
      title: 'Welcome to Clip Vault',
      content: (
        <div className="onboarding-welcome">
          <div className="welcome-icon">🔐</div>
          <h3>Secure Clipboard Manager</h3>

          <div className="feature-carousel">
            <div className="feature-cards-container">
              <div
                className="feature-cards"
                style={{ transform: `translateX(-${currentFeature * 25}%)` }}
              >
                {features.map((feature, index) => (
                  <div key={index} className="feature-card">
                    <div className="feature-card-icon">{feature.icon}</div>
                    <h4>{feature.title}</h4>
                    <p>{feature.description}</p>
                  </div>
                ))}
              </div>
            </div>

            <div className="carousel-controls">
              <button className="carousel-btn prev" onClick={prevFeature}>
                ‹
              </button>
              <div className="carousel-dots">
                {features.map((_, index) => (
                  <button
                    key={index}
                    className={`carousel-dot ${index === currentFeature ? 'active' : ''}`}
                    onClick={() => goToFeature(index)}
                  />
                ))}
              </div>
              <button className="carousel-btn next" onClick={nextFeature}>
                ›
              </button>
            </div>
          </div>
        </div>
      )
    },
    // Password setup
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