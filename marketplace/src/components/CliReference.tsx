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
    <section id="commands" className="py-20 border-t border-line scroll-mt-16">
      <div className="max-w-5xl mx-auto px-4 sm:px-6 lg:px-8">
        
        {/* Section Header */}
        <div className="text-center max-w-3xl mx-auto mb-12">
          <div className="inline-flex items-center gap-2 px-3 py-1 rounded-full bg-accent/10 border border-accent/20 text-accent font-mono text-xs font-semibold mb-3">
            <Layers className="w-3.5 h-3.5" />
            CLI Cheat Sheet
          </div>
          <h1 className="text-3xl sm:text-4xl font-semibold text-ink tracking-tight">
            Complete Command Reference
          </h1>
          <p className="mt-3 text-secondary text-sm sm:text-base">
            Every command supported by <code className="text-accent font-mono text-xs">avm-cli</code> with real usage patterns.
          </p>
        </div>

        {/* Search & Category Filter */}
        <div className="flex flex-col sm:flex-row items-stretch sm:items-center justify-between gap-4 mb-8">
          <div className="relative flex-1 max-w-md">
            <Search className="absolute left-3.5 top-1/2 -translate-y-1/2 w-4 h-4 text-secondary" />
            <input
              type="text"
              aria-label="Search commands"
              placeholder="Search commands (e.g. init, plugin, shims, versions)..."
              value={searchQuery}
              onChange={(e) => setSearchQuery(e.target.value)}
              className="w-full pl-10 pr-4 py-2.5 bg-surface/90 border border-line rounded-lg text-sm text-ink placeholder-secondary focus:outline-none focus:border-accent font-mono transition-all"
            />
          </div>

          <div className="flex items-center gap-1.5 overflow-x-auto pb-1 sm:pb-0">
            {categories.map((cat) => (
              <button
                key={cat}
                aria-pressed={selectedCategory === cat}
                onClick={() => setSelectedCategory(cat)}
                className={`px-3 py-1.5 rounded-lg text-xs font-mono transition-all ${
                  selectedCategory === cat
                    ? 'bg-accent text-on-accent font-bold shadow-md shadow-accent/20'
                    : 'text-secondary hover:text-ink bg-surface/60 border border-line'
                }`}
              >
                {cat}
              </button>
            ))}
          </div>
        </div>

        {filteredCommands.length === 0 && <p role="status" className="text-secondary py-6">No matching commands. Try another search or category.</p>}
        {/* Command List */}
        <div className="space-y-4">
          {filteredCommands.map((item, idx) => (
            <div
              key={idx}
              className="group rounded-lg bg-surface/60 border border-line hover:border-line p-4 sm:p-5 transition-all"
            >
              <div className="flex flex-col sm:flex-row sm:items-center justify-between gap-2 mb-2">
                <div className="flex items-center gap-2 flex-wrap">
                  <span className="font-mono text-sm sm:text-base font-bold text-accent">
                    {item.command}
                  </span>
                  {item.args && (
                    <span className="font-mono text-xs text-secondary bg-page px-2 py-0.5 rounded border border-line">
                      {item.args}
                    </span>
                  )}
                </div>
                <span className="self-start sm:self-center text-xs font-mono px-2 py-0.5 rounded-full bg-muted text-secondary border border-line">
                  {item.category}
                </span>
              </div>

              <p className="text-xs sm:text-sm text-secondary mb-3 leading-relaxed">
                {item.description}
              </p>

              {/* Example Bar */}
              <div className="flex items-center justify-between bg-page rounded-lg p-2.5 font-mono text-xs border border-line/80">
                <div className="flex items-center gap-2 overflow-x-auto">
                  <span className="text-accent font-bold select-none">$</span>
                  <span className="text-ink">{item.example}</span>
                </div>
                <button
                  onClick={() => onCopy(item.example)}
                  className="shrink-0 ml-2 p-1 text-secondary hover:text-ink rounded hover:bg-muted transition-colors"
                  title="Copy example"
                >
                  <Copy className="w-3.5 h-3.5" />
                </button>
              </div>

              {/* Output Preview */}
              {item.outputExample && (
                <div className="mt-2.5 pt-2.5 border-t border-line/60">
                  <div className="text-xs font-mono text-secondary uppercase tracking-wider mb-1">Example Output:</div>
                  <pre className="p-2 rounded bg-page/60 font-mono text-xs text-secondary whitespace-pre-wrap leading-relaxed">
                    {item.outputExample}
                  </pre>
                </div>
              )}
            </div>
          ))}
        </div>

        {/* Group summary reminder */}
        <div className="mt-8 p-4 rounded-lg bg-surface/40 border border-line text-center text-xs text-secondary font-mono">
          Tip: run <code className="text-accent">avm --help</code> or <code className="text-accent">avm &lt;command&gt; --help</code>. Use <code className="text-accent">avm --env ci &lt;command&gt;</code> to select profiles for one invocation.
        </div>

      </div>
    </section>
  );
};
