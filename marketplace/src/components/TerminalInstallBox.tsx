import React, { useState } from 'react';
import { Copy } from 'lucide-react';
import { INSTALL_METHODS } from '../data/guideData';

interface TerminalInstallBoxProps {
  onCopy: (text: string) => void;
}

export const TerminalInstallBox: React.FC<TerminalInstallBoxProps> = ({ onCopy }) => {
  const [selectedMethod, setSelectedMethod] = useState(INSTALL_METHODS[0].id);
  const currentMethod = INSTALL_METHODS.find((m) => m.id === selectedMethod) || INSTALL_METHODS[0];

  return (
    <div className="w-full min-w-0 rounded-lg border border-line bg-page overflow-hidden">
      <div role="group" aria-label="Install method" className="flex flex-wrap gap-1 p-2 border-b border-line bg-muted/50">
        {INSTALL_METHODS.map((m) => {
          const isSelected = selectedMethod === m.id;
          return (
            <button
              key={m.id}
              aria-pressed={isSelected}
              onClick={() => setSelectedMethod(m.id)}
              className={`min-h-10 px-4 rounded-md text-sm font-medium transition-colors ${
                isSelected ? 'bg-surface text-accent border border-line' : 'text-secondary border border-transparent hover:text-ink'
              }`}
            >
              {m.name}
              {m.tag && isSelected && <span className="ml-2 text-xs text-secondary">{m.tag}</span>}
            </button>
          );
        })}
      </div>

      <div className="flex items-center gap-3 px-5 py-5 font-mono">
        <span className="text-accent select-none text-lg">$</span>
        <div className="min-w-0 flex-1 overflow-x-auto">
          <span className="text-ink select-all whitespace-nowrap text-base sm:text-lg">{currentMethod.command}</span>
        </div>
        <button
          onClick={() => onCopy(currentMethod.command)}
          className="button-primary shrink-0 !min-h-10 !px-4 !py-2 font-sans"
          aria-label="Copy install command"
        >
          <Copy className="w-4 h-4" />
          <span>Copy</span>
        </button>
      </div>

      <p className="px-5 py-3 border-t border-line text-sm text-secondary">{currentMethod.description}</p>
    </div>
  );
};
