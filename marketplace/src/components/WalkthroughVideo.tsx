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
    command: 'avm plugin add node',
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
      <div className="rounded-2xl border border-slate-700/80 bg-slate-900/90 shadow-2xl overflow-hidden backdrop-blur-xl">
        
        {/* Top Header Bar */}
        <div className="flex flex-wrap items-center justify-between gap-3 border-b border-slate-700/80 px-4 sm:px-6 py-3.5 bg-slate-950/70">
          <div className="flex items-center gap-3">
            <div className="flex items-center gap-1.5">
              <span className="w-3 h-3 rounded-full bg-rose-500/80 inline-block" />
              <span className="w-3 h-3 rounded-full bg-amber-500/80 inline-block" />
              <span className="w-3 h-3 rounded-full bg-emerald-500/80 inline-block" />
            </div>
            <div className="h-4 w-px bg-slate-800 hidden sm:block" />
            <div className="flex items-center gap-2 text-xs font-mono text-slate-400">
              <Terminal className="w-3.5 h-3.5 text-emerald-400" />
              <span className="text-slate-200 font-semibold hidden sm:inline">{title || 'avm walkthrough'}</span>
              <span className="text-slate-600 hidden sm:inline">&mdash;</span>
              <span className="text-slate-400 text-[11px]">{badge || '60-second tour'}</span>
            </div>
          </div>

          <div className="flex items-center gap-2">
            <span className="text-xs font-mono px-2 py-0.5 rounded bg-emerald-500/10 text-emerald-400 border border-emerald-500/20 font-medium">
              Interactive CLI
            </span>
          </div>
        </div>

        {/* Step Progression Scrubber */}
        <div className="border-b border-slate-800 bg-slate-950/40 px-4 sm:px-6 py-2.5 overflow-x-auto">
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
                      ? 'bg-emerald-500/10 text-emerald-400 border border-emerald-500/30'
                      : isCompleted
                      ? 'text-slate-300 bg-slate-800/60 border border-slate-700 hover:border-slate-600'
                      : 'text-slate-500 border border-transparent hover:text-slate-300'
                  }`}
                >
                  <span
                    className={`w-4 h-4 rounded-full flex items-center justify-center text-[10px] font-bold ${
                      isActive
                        ? 'bg-emerald-400 text-slate-950'
                        : isCompleted
                        ? 'bg-slate-700 text-emerald-400'
                        : 'bg-slate-800 text-slate-500'
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
        <div className="relative min-h-[340px] sm:min-h-[380px] bg-slate-950 flex flex-col justify-between">
          <div className="p-5 sm:p-7 font-mono text-xs sm:text-sm overflow-x-auto flex-1">
            
            {/* Step info commentary */}
            <div className="flex flex-wrap items-center justify-between gap-2 pb-3 mb-3 border-b border-slate-800/80 text-xs">
              <div className="text-slate-400 font-sans flex items-center gap-2">
                <span className="text-emerald-400 font-mono font-semibold">
                  Step {currentStep.stepNumber} of {String(steps.length).padStart(2, '0')}:
                </span>
                <span className="text-slate-300 font-medium">{currentStep.label}</span>
              </div>
              <div className="flex items-center gap-2 text-[11px] font-mono text-slate-500">
                <span>Execution overhead:</span>
                <span className="text-emerald-400 font-semibold">{currentStep.timing}</span>
              </div>
            </div>

            {/* Explanatory comment line */}
            <div className="text-slate-500 italic select-none mb-3">
              {currentStep.comment}
            </div>

            {/* Typed command prompt */}
            <div className="flex items-center gap-2 text-slate-100 font-semibold text-sm sm:text-base mb-4">
              <span className="text-emerald-400 select-none">$</span>
              <span className="text-slate-100">{typedCommand}</span>
              <span
                className={`w-2 h-4 bg-emerald-400 inline-block transition-opacity ${
                  isPlaying ? 'animate-pulse' : 'opacity-80'
                }`}
              />
            </div>

            {/* Output block */}
            {showOutput && (
              <div>
                <pre className="text-slate-300 whitespace-pre-wrap pl-4 border-l-2 border-slate-700/80 text-xs sm:text-sm font-mono leading-relaxed bg-slate-900/30 p-3 rounded-r-lg">
                  {currentStep.output}
                </pre>
              </div>
            )}
          </div>

          {/* Bottom Control Bar */}
          <div className="flex flex-wrap items-center justify-between gap-3 border-t border-slate-800 bg-slate-950/90 px-4 sm:px-6 py-3 text-xs font-mono">
            <div className="flex items-center gap-2">
              <button
                onClick={handleTogglePlay}
                className="flex items-center gap-1.5 px-3 py-1.5 rounded-md bg-emerald-400 text-slate-950 font-semibold hover:bg-emerald-300 transition-colors"
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
                className="p-1.5 rounded-md border border-slate-700 text-slate-400 hover:text-slate-200 hover:border-slate-600 transition-colors"
                title="Restart from Step 1"
              >
                <RotateCcw className="w-3.5 h-3.5" />
              </button>

              <button
                onClick={cycleSpeed}
                className="px-2.5 py-1 rounded border border-slate-700 text-slate-300 hover:border-slate-500 transition-colors text-[11px]"
                title="Change playback speed"
              >
                {playbackSpeed}x Speed
              </button>
            </div>

            <div className="flex items-center gap-3">
              <button
                onClick={() => onCopy(currentStep.command)}
                className="flex items-center gap-1.5 px-2.5 py-1.5 rounded border border-slate-700 text-slate-300 hover:text-white hover:border-slate-500 transition-colors text-xs"
              >
                <Copy className="w-3.5 h-3.5" />
                <span>{`Copy $ ${currentStep.command}`}</span>
              </button>

              <div className="text-[11px] text-slate-500 hidden sm:inline">
                Pure Rust &bull; verified plugins
              </div>
            </div>
          </div>

        </div>

      </div>
    </div>
  );
};
