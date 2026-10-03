import React, { useState } from 'react';
import { Copy, Terminal } from 'lucide-react';
import { INSTALL_METHODS } from '../data/guideData';

interface TerminalInstallBoxProps {
  onCopy: (text: string) => void;
}

export const TerminalInstallBox: React.FC<TerminalInstallBoxProps> = ({ onCopy }) => {
  const [selectedMethod, setSelectedMethod] = useState(INSTALL_METHODS[0].id);

  const currentMethod = INSTALL_METHODS.find((m) => m.id === selectedMethod) || INSTALL_METHODS[0];

  return (
      <div className="w-full max-w-xl mx-auto flex flex-col items-center">
        <div className="w-full flex flex-col items-center">
          <div className="flex items-center justify-center gap-1.5 p-1 mb-4 rounded-xl bg-slate-900/80 border border-slate-700/80 backdrop-blur-sm flex-wrap shadow-sm">
            {INSTALL_METHODS.map((m) => {
              const isSelected = selectedMethod === m.id;
              return (
                <button
                  key={m.id}
                  aria-pressed={isSelected}
                  onClick={() => setSelectedMethod(m.id)}
                  className={`px-3.5 py-1.5 rounded-lg text-xs font-mono font-medium transition-all ${
                    isSelected
                      ? 'bg-slate-800 text-emerald-400 border border-slate-600 shadow-sm'
                      : 'text-slate-400 border border-transparent hover:text-slate-200 hover:bg-slate-800/40'
                  }`}
                >
                  <span>{m.name}</span>
                  {m.tag && isSelected && (
                    <span className="ml-1.5 text-[9px] px-1.5 py-0.2 rounded-full bg-emerald-500/20 text-emerald-300 font-sans border border-emerald-500/30 uppercase tracking-wider">
                      {m.tag}
                    </span>
                  )}
                </button>
              );
            })}
          </div>

          {/* Terminal Box */}
          <div className="w-full rounded-xl bg-slate-900/95 border border-slate-700/90 shadow-2xl overflow-hidden text-left backdrop-blur-md">
            
            {/* Header: Window Dots, Title, and Fixed Copy Button */}
            <div className="flex items-center justify-between px-4 py-2.5 border-b border-slate-800 bg-slate-950/70">
              <div className="flex items-center gap-2.5">
                <div className="flex items-center gap-1.5">
                  <span className="w-2.5 h-2.5 rounded-full bg-rose-500/80" />
                  <span className="w-2.5 h-2.5 rounded-full bg-amber-500/80" />
                  <span className="w-2.5 h-2.5 rounded-full bg-emerald-500/80" />
                </div>
                <div className="h-3 w-px bg-slate-800" />
                <div className="flex items-center gap-1.5 text-[11px] font-mono text-slate-500">
                  <Terminal className="w-3 h-3 text-emerald-400" />
                  <span>terminal &mdash; zsh</span>
                </div>
              </div>

              <button
                onClick={() => onCopy(currentMethod.command)}
                className="flex items-center gap-1.5 px-2.5 py-1 rounded-md bg-emerald-400 text-slate-950 text-[11px] font-sans font-semibold hover:bg-emerald-300 active:scale-95 transition-all shadow-sm"
                aria-label="Copy install command"
                title={`Copy: ${currentMethod.command}`}
              >
                <Copy className="w-3.5 h-3.5" />
                <span>Copy</span>
              </button>
            </div>

            {/* Code Body - horizontal scroll isolated exclusively to command text */}
            <div className="relative px-4 py-3.5 font-mono text-[13px]">
              <div className="overflow-x-auto pr-2 pb-1">
                <div className="flex items-center gap-2.5 min-w-max">
                  <span className="text-emerald-400 select-none font-bold">$</span>
                  <span className="text-slate-100 select-all font-mono whitespace-nowrap">
                    {currentMethod.command}
                  </span>
                </div>
              </div>
            </div>

          </div>
        </div>

        {/* Method Description */}
        <p className="text-[12px] text-slate-400 mt-3 text-center leading-relaxed">
          {currentMethod.description}
        </p>
      </div>
  );
};
