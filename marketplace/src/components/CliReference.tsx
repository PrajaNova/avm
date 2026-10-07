import { useState } from 'react';
import { Layers, Search, Copy } from 'lucide-react';
import { CLI_COMMANDS } from '../data/cliReference';

interface CliReferenceProps {
  onCopy: (text: string) => void;
}

export const CliReference: React.FC<CliReferenceProps> = ({ onCopy }) => {
  const [searchQuery, setSearchQuery] = useState('');
  const [selectedCategory, setSelectedCategory] = useState<string>('All');

  const categories = ['All', 'Core', 'Aliases', 'Plugins', 'Versions', 'Shims', 'Env'];

  const filteredCommands = CLI_COMMANDS.filter((cmd) => {
    const matchesSearch =
      cmd.command.toLowerCase().includes(searchQuery.toLowerCase()) ||
      cmd.description.toLowerCase().includes(searchQuery.toLowerCase()) ||
      (cmd.args && cmd.args.toLowerCase().includes(searchQuery.toLowerCase()));

    const matchesCategory =
      selectedCategory === 'All' || cmd.category === selectedCategory;

    return matchesSearch && matchesCategory;
  });

  return (
    <section id="commands" className="py-20 border-t border-slate-900 scroll-mt-16">
      <div className="max-w-5xl mx-auto px-4 sm:px-6 lg:px-8">
        
        {/* Section Header */}
        <div className="text-center max-w-3xl mx-auto mb-12">
          <div className="inline-flex items-center gap-2 px-3 py-1 rounded-full bg-emerald-500/10 border border-emerald-500/20 text-emerald-400 font-mono text-xs font-semibold mb-3">
            <Layers className="w-3.5 h-3.5" />
            CLI Cheat Sheet
          </div>
          <h2 className="text-3xl sm:text-4xl font-extrabold text-white tracking-tight">
            Complete Command Reference
          </h2>
          <p className="mt-3 text-slate-400 text-sm sm:text-base">
            Every command supported by <code className="text-emerald-400 font-mono text-xs">avm-cli</code> with real usage patterns.
          </p>
        </div>

        {/* Search & Category Filter */}
        <div className="flex flex-col sm:flex-row items-stretch sm:items-center justify-between gap-4 mb-8">
          <div className="relative flex-1 max-w-md">
            <Search className="absolute left-3.5 top-1/2 -translate-y-1/2 w-4 h-4 text-slate-400" />
            <input
              type="text"
              placeholder="Search commands (e.g. init, plugin, shims, versions)..."
              value={searchQuery}
              onChange={(e) => setSearchQuery(e.target.value)}
              className="w-full pl-10 pr-4 py-2.5 bg-slate-900/90 border border-slate-800 rounded-xl text-sm text-white placeholder-slate-500 focus:outline-none focus:border-emerald-500 font-mono transition-all"
            />
          </div>

          <div className="flex items-center gap-1.5 overflow-x-auto pb-1 sm:pb-0">
            {categories.map((cat) => (
              <button
                key={cat}
                onClick={() => setSelectedCategory(cat)}
                className={`px-3 py-1.5 rounded-lg text-xs font-mono transition-all ${
                  selectedCategory === cat
                    ? 'bg-emerald-500 text-slate-950 font-bold shadow-md shadow-emerald-500/20'
                    : 'text-slate-400 hover:text-slate-200 bg-slate-900/60 border border-slate-800'
                }`}
              >
                {cat}
              </button>
            ))}
          </div>
        </div>

        {/* Command List */}
        <div className="space-y-4">
          {filteredCommands.map((item, idx) => (
            <div
              key={idx}
              className="group rounded-xl bg-slate-900/60 border border-slate-800 hover:border-slate-700 p-4 sm:p-5 transition-all"
            >
              <div className="flex flex-col sm:flex-row sm:items-center justify-between gap-2 mb-2">
                <div className="flex items-center gap-2 flex-wrap">
                  <span className="font-mono text-sm sm:text-base font-bold text-emerald-400">
                    {item.command}
                  </span>
                  {item.args && (
                    <span className="font-mono text-xs text-slate-400 bg-slate-950 px-2 py-0.5 rounded border border-slate-800">
                      {item.args}
                    </span>
                  )}
                </div>
                <span className="self-start sm:self-center text-[10px] font-mono px-2 py-0.5 rounded-full bg-slate-800 text-slate-300 border border-slate-700">
                  {item.category}
                </span>
              </div>

              <p className="text-xs sm:text-sm text-slate-300 mb-3 leading-relaxed">
                {item.description}
              </p>

              {/* Example Bar */}
              <div className="flex items-center justify-between bg-slate-950 rounded-lg p-2.5 font-mono text-xs border border-slate-800/80">
                <div className="flex items-center gap-2 overflow-x-auto">
                  <span className="text-emerald-500 font-bold select-none">$</span>
                  <span className="text-slate-200">{item.example}</span>
                </div>
                <button
                  onClick={() => onCopy(item.example)}
                  className="shrink-0 ml-2 p-1 text-slate-400 hover:text-white rounded hover:bg-slate-800 transition-colors"
                  title="Copy example"
                >
                  <Copy className="w-3.5 h-3.5" />
                </button>
              </div>

              {/* Output Preview */}
              {item.outputExample && (
                <div className="mt-2.5 pt-2.5 border-t border-slate-800/60">
                  <div className="text-[10px] font-mono text-slate-500 uppercase tracking-wider mb-1">Example Output:</div>
                  <pre className="p-2 rounded bg-slate-950/60 font-mono text-[11px] text-slate-400 whitespace-pre-wrap leading-relaxed">
                    {item.outputExample}
                  </pre>
                </div>
              )}
            </div>
          ))}
        </div>

        {/* Group summary reminder */}
        <div className="mt-8 p-4 rounded-xl bg-slate-900/40 border border-slate-800 text-center text-xs text-slate-400 font-mono">
          Tip: run <code className="text-emerald-400">avm --help</code> (or <code className="text-emerald-400">avm Tip: You can also run <code className="text-emerald-400">avm all</code> in your terminal to see grouped help in real-time.lt;commandTip: You can also run <code className="text-emerald-400">avm all</code> in your terminal to see grouped help in real-time.gt; --help</code>) in your terminal for the same reference.
        </div>

      </div>
    </section>
  );
};
