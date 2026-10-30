import React, { useState, useEffect } from 'react';
import {
  Play,
  Pause,
  RotateCcw,
  Terminal,
  Copy,
} from 'lucide-react';

export interface WalkthroughStep {
  id: string;
  stepNumber: string;
  label: string;
  command: string;
  comment: string;
  output: string;
  timing: string;
}

const DEMO_STEPS: WalkthroughStep[] = [
  {
    id: 'install',
    stepNumber: '01',
    label: 'Install avm',
    command: 'curl -fsSL https://raw.githubusercontent.com/prajanova/avm/main/install.sh | bash',
    comment: '# Install native precompiled avm binary into ~/.local/bin',
    output: "Downloading avm-bin for darwin-arm64...\n✓ Installed /Users/dev/.local/bin/avm-bin\n✓ Created shims directory in ~/.avm/shims\n✓ Shell hook ready: eval \"$(avm shell-init)\"\n\navm v0.1.0-alpha successfully installed! Run 'avm --version' to verify.",
    timing: '0.7s',
  },
  {
    id: 'init',
    stepNumber: '02',
    label: 'Initialize',
    command: 'avm init',
    comment: '# Create version-controlled .avm.json in current directory',
    output: '✓ Created .avm.json in /Users/dev/workspace/web-app\n  Ready for project-scoped aliases, environment variables, and tool pins.',
    timing: '<1ms',
  },
  {
    id: 'plugin',
    stepNumber: '03',
    label: 'Add Plugin',
    command: 'avm plugin install node',
    comment: '# Fetch precompiled native plugin from marketplace registry',
    output: "Fetching 'node' from PrajaNova/avm-plugin-node...\n✓ Installed node v0.2.0",
    timing: '1.1s',
  },
  {
    id: 'pin',
    stepNumber: '04',
    label: 'Pin Tool',
    command: 'avm node use 22.14.0',
    comment: '# Pin Node runtime version specifically for this directory',
    output: 'Resolving Node.js 22.14.0 via provider index...\nDownloading node-v22.14.0-darwin-arm64.tar.gz...\n✓ Extracted to ~/.avm/tools/node/22.14.0\n✓ Set local tools.node = "22.14.0"',
    timing: '2.4s',
  },
  {
    id: 'alias',
    stepNumber: '05',
    label: 'Add Alias',
    command: 'avm alias add dev "pnpm run dev --filter web"',
    comment: '# Anchor project commands so every teammate runs identical flags',
    output: "✓ Added local alias 'dev' → 'pnpm run dev --filter web'\n  Recorded in .avm.json under \"aliases\"",
    timing: '<1ms',
  },
  {
    id: 'run',
    stepNumber: '06',
    label: 'Execute',
    command: 'avm dev',
    comment: '# Run directly with injected runtime version and environment variables',
    output: '[avm] Injected node v22.14.0 into PATH (<0.3ms overhead)\n> web@1.0.0 dev\n> vite dev --port 3000\n\n  VITE v6.1.0  ready in 142 ms\n\n  ➜  Local:   http://localhost:3000/\n  ➜  Network: use --host to expose\n  ➜  press h + enter to show help',
    timing: '<0.3ms',
  },
];

export interface WalkthroughVideoProps {
  onCopy: (text: string) => void;
  steps?: WalkthroughStep[];
  title?: string;
  badge?: string;
  className?: string;
}

export const WalkthroughVideo: React.FC<WalkthroughVideoProps> = ({
  onCopy,
  steps: customSteps,
  title,
  badge,
  className,
}) => {
  const steps = customSteps || DEMO_STEPS;
  const [currentStepIndex, setCurrentStepIndex] = useState(0);
  const [isPlaying, setIsPlaying] = useState(true);
  const [playbackSpeed, setPlaybackSpeed] = useState<number>(1);
  const [typedCommand, setTypedCommand] = useState('');
  const [showOutput, setShowOutput] = useState(false);

  // Reset when steps change
  useEffect(() => {
    setCurrentStepIndex(0);
    setIsPlaying(true);
  }, [customSteps]);

  const currentStep = steps[currentStepIndex] || steps[0];

  // Typing animation & auto-advance timer
  useEffect(() => {
    if (!isPlaying || !currentStep) return;

    setTypedCommand('');
    setShowOutput(false);

    let charIndex = 0;
    const fullText = currentStep.command;
    const typingIntervalMs = Math.max(15, Math.floor(40 / playbackSpeed));

    const typeTimer = setInterval(() => {
      if (charIndex < fullText.length) {
        setTypedCommand(fullText.slice(0, charIndex + 1));
        charIndex++;
      } else {
        clearInterval(typeTimer);
        setShowOutput(true);

        // Pause on output before advancing to next step
        const dwellTimeMs = Math.floor(2400 / playbackSpeed);
        const advanceTimer = setTimeout(() => {
          setCurrentStepIndex((prev) => {
            if (prev < steps.length - 1) {
              return prev + 1;
            } else {
              setIsPlaying(false);
              return prev;
            }
          });
        }, dwellTimeMs);

        return () => clearTimeout(advanceTimer);
      }
    }, typingIntervalMs);

    return () => clearInterval(typeTimer);
  }, [currentStepIndex, isPlaying, playbackSpeed, currentStep, steps.length]);

  const handleStepClick = (index: number) => {
    setCurrentStepIndex(index);
    setTypedCommand(steps[index].command);
    setShowOutput(true);
    setIsPlaying(false);
  };

  const handleTogglePlay = () => {
    if (!isPlaying && currentStepIndex === steps.length - 1) {
      // Replay from beginning
      setCurrentStepIndex(0);
      setIsPlaying(true);
    } else {
      setIsPlaying(!isPlaying);
    }
  };

  const handleRestart = () => {
    setCurrentStepIndex(0);
    setIsPlaying(true);
  };

  const cycleSpeed = () => {
    if (playbackSpeed === 1) setPlaybackSpeed(1.5);
    else if (playbackSpeed === 1.5) setPlaybackSpeed(2);
    else setPlaybackSpeed(1);
  };

  return (
    <div className={className || "w-full max-w-5xl mx-auto px-4 sm:px-6 lg:px-8 mb-20"}>
      <div className="rounded-lg border border-line/80 bg-surface/90 shadow-sm overflow-hidden backdrop-blur-xl">
        
        {/* Top Header Bar */}
        <div className="flex flex-wrap items-center justify-between gap-3 border-b border-line/80 px-4 sm:px-6 py-3.5 bg-page/70">
          <div className="flex items-center gap-3">
            <div className="flex items-center gap-1.5">
              <span className="w-3 h-3 rounded-full bg-rose-500/80 inline-block" />
              <span className="w-3 h-3 rounded-full bg-amber-500/80 inline-block" />
              <span className="w-3 h-3 rounded-full bg-accent/80 inline-block" />
            </div>
            <div className="h-4 w-px bg-muted hidden sm:block" />
            <div className="flex items-center gap-2 text-xs font-mono text-secondary">
              <Terminal className="w-3.5 h-3.5 text-accent" />
              <span className="text-ink font-semibold hidden sm:inline">{title || 'avm walkthrough'}</span>
              <span className="text-secondary hidden sm:inline">&mdash;</span>
              <span className="text-secondary text-xs">{badge || '60-second tour'}</span>
            </div>
          </div>

          <div className="flex items-center gap-2">
            <span className="text-xs font-mono px-2 py-0.5 rounded bg-accent/10 text-accent border border-accent/20 font-medium">
              Interactive CLI
            </span>
          </div>
        </div>

        {/* Step Progression Scrubber */}
        <div className="border-b border-line bg-page/40 px-4 sm:px-6 py-2.5 overflow-x-auto">
          <div className="flex items-center gap-2 min-w-max">
            {steps.map((step, idx) => {
              const isActive = idx === currentStepIndex;
              const isCompleted = idx < currentStepIndex;
              return (
                <button
                  key={step.id}
                  onClick={() => handleStepClick(idx)}
                  className={`flex items-center gap-2 px-3 py-1.5 rounded-lg text-xs font-mono transition-all ${
                    isActive
                      ? 'bg-accent/10 text-accent border border-accent/30'
                      : isCompleted
                      ? 'text-secondary bg-muted/60 border border-line hover:border-line'
                      : 'text-secondary border border-transparent hover:text-secondary'
                  }`}
                >
                  <span
                    className={`w-4 h-4 rounded-full flex items-center justify-center text-xs font-bold ${
                      isActive
                        ? 'bg-accent text-on-accent'
                        : isCompleted
                        ? 'bg-muted text-accent'
                        : 'bg-muted text-secondary'
                    }`}
                  >
                    {isCompleted ? '✓' : step.stepNumber}
                  </span>
                  <span>{step.label}</span>
                </button>
              );
            })}
          </div>
        </div>

        {/* Main Content Area - Interactive Terminal */}
        <div className="relative min-h-[340px] sm:min-h-[380px] bg-page flex flex-col justify-between">
          <div className="p-5 sm:p-7 font-mono text-xs sm:text-sm overflow-x-auto flex-1">
            
            {/* Step info commentary */}
            <div className="flex flex-wrap items-center justify-between gap-2 pb-3 mb-3 border-b border-line/80 text-xs">
              <div className="text-secondary font-sans flex items-center gap-2">
                <span className="text-accent font-mono font-semibold">
                  Step {currentStep.stepNumber} of {String(steps.length).padStart(2, '0')}:
                </span>
                <span className="text-secondary font-medium">{currentStep.label}</span>
              </div>
              <div className="flex items-center gap-2 text-xs font-mono text-secondary">
                <span>Execution overhead:</span>
                <span className="text-accent font-semibold">{currentStep.timing}</span>
              </div>
            </div>

            {/* Explanatory comment line */}
            <div className="text-secondary italic select-none mb-3">
              {currentStep.comment}
            </div>

            {/* Typed command prompt */}
            <div className="flex items-center gap-2 text-ink font-semibold text-sm sm:text-base mb-4">
              <span className="text-accent select-none">$</span>
              <span className="text-ink">{typedCommand}</span>
              <span
                className={`w-2 h-4 bg-accent inline-block transition-opacity ${
                  isPlaying ? '' : 'opacity-80'
                }`}
              />
            </div>

            {/* Output block */}
            {showOutput && (
              <div>
                <pre className="text-secondary whitespace-pre-wrap pl-4 border-l-2 border-line/80 text-xs sm:text-sm font-mono leading-relaxed bg-surface/30 p-3 rounded-r-lg">
                  {currentStep.output}
                </pre>
              </div>
            )}
          </div>

          {/* Bottom Control Bar */}
          <div className="flex flex-wrap items-center justify-between gap-3 border-t border-line bg-page/90 px-4 sm:px-6 py-3 text-xs font-mono">
            <div className="flex items-center gap-2">
              <button
                onClick={handleTogglePlay}
                className="flex items-center gap-1.5 px-3 py-1.5 rounded-md bg-accent text-on-accent font-semibold hover:bg-accent transition-colors"
              >
                {isPlaying ? (
                  <>
                    <Pause className="w-3.5 h-3.5 fill-slate-950" />
                    <span>Pause</span>
                  </>
                ) : (
                  <>
                    <Play className="w-3.5 h-3.5 fill-slate-950" />
                    <span>{currentStepIndex === steps.length - 1 ? 'Replay' : 'Resume'}</span>
                  </>
                )}
              </button>

              <button
                onClick={handleRestart}
                className="p-1.5 rounded-md border border-line text-secondary hover:text-ink hover:border-line transition-colors"
                title="Restart from Step 1"
              >
                <RotateCcw className="w-3.5 h-3.5" />
              </button>

              <button
                onClick={cycleSpeed}
                className="px-2.5 py-1 rounded border border-line text-secondary hover:border-line transition-colors text-xs"
                title="Change playback speed"
              >
                {playbackSpeed}x Speed
              </button>
            </div>

            <div className="flex items-center gap-3">
              <button
                onClick={() => onCopy(currentStep.command)}
                className="flex items-center gap-1.5 px-2.5 py-1.5 rounded border border-line text-secondary hover:text-ink hover:border-line transition-colors text-xs"
              >
                <Copy className="w-3.5 h-3.5" />
                <span>{`Copy $ ${currentStep.command}`}</span>
              </button>

              <div className="text-xs text-secondary hidden sm:inline">
                Pure Rust &bull; verified plugins
              </div>
            </div>
          </div>

        </div>

      </div>
    </div>
  );
};
