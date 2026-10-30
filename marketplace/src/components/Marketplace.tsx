import React, { useState } from 'react';
import { Link } from 'react-router';
import { Search, Package, Copy } from 'lucide-react';
import { Platforms } from './OsIcons';
import registry from '../../registry.json';

// Node and Java support .avm.lock frozen installs; Android and asdf plugins do not yet.
const LOCKABLE = new Set(['node', 'java']);

interface MarketplaceProps {
  onCopy: (text: string) => void;
}

export const Marketplace: React.FC<MarketplaceProps> = ({ onCopy }) => {
  const [searchQuery, setSearchQuery] = useState('');

  const q = searchQuery.toLowerCase();
  const filteredPlugins = registry.plugins.filter((p) =>
    [p.name, p.description, p.section_label, p.repo].some((f) => f.toLowerCase().includes(q))
  );

  return (
    <section id="marketplace" className="py-20 border-t border-line scroll-mt-16">
      <div className="max-w-6xl mx-auto px-4 sm:px-6 lg:px-8">
        
        {/* Section Header */}
        <div className="flex flex-col md:flex-row md:items-end justify-between gap-6 mb-12">
          <div>
            <div className="flex items-center gap-2 mb-2">
              <span className="text-xs font-mono font-bold tracking-widest text-accent uppercase">
                Plugin Ecosystem
              </span>
              <span className="text-xs font-mono px-2 py-0.5 rounded-full bg-muted text-secondary border border-line">
                registry.json (v1)
              </span>
            </div>
            <h1 className="text-3xl sm:text-4xl font-semibold text-ink tracking-tight">
              AVM Plugin Marketplace
            </h1>
            <p className="mt-2 text-secondary text-sm sm:text-base max-w-2xl">
              Plugins are standalone precompiled executables fetched on-demand. 
              Install with <code className="text-accent font-mono text-xs bg-surface px-1.5 py-0.5 rounded">avm plugin install &lt;name&gt;</code>.
            </p>
          </div>
        </div>

        {/* Search & Filter Bar */}
        <div className="flex flex-col sm:flex-row items-stretch sm:items-center justify-between gap-4 mb-8">
          {/* Search box */}
          <div className="relative flex-1 max-w-md">
            <Search className="absolute left-3.5 top-1/2 -translate-y-1/2 w-4 h-4 text-secondary" />
            <input
              type="text"
              placeholder="Search plugins by name, runtime, or description..."
              value={searchQuery}
              onChange={(e) => setSearchQuery(e.target.value)}
              className="w-full pl-10 pr-4 py-2.5 bg-surface/90 border border-line rounded-lg text-sm text-ink placeholder-secondary focus:outline-none focus:border-accent focus:ring-1 focus:ring-accent transition-all font-mono"
            />
          </div>
        </div>

        {/* Plugin Cards Grid */}
        <div className="grid grid-cols-1 md:grid-cols-2 lg:grid-cols-3 gap-6 mb-12">
          {filteredPlugins.map((plugin) => (
            <article key={plugin.name} className="group relative rounded-lg bg-surface border border-line hover:border-accent p-6 flex flex-col justify-between transition-colors">
              <div>
                <div className="flex items-start justify-between gap-3 mb-3">
                  <div>
                    <h3 className="text-xl font-medium text-ink">
                      <Link to={`/marketplace/${plugin.name}`} className="after:absolute after:inset-0 hover:text-accent">{plugin.name}</Link>
                    </h3>
                    <span className="text-sm font-mono text-secondary">avm-plugin-{plugin.name}</span>
                  </div>
                  <span className="text-sm text-secondary border border-line rounded-md px-2 py-0.5">{plugin.section_label}</span>
                </div>
                <p className="text-secondary mb-5 line-clamp-3">{plugin.description}</p>
                <dl className="grid grid-cols-[auto_1fr] gap-x-4 gap-y-1 text-sm mb-6">
                  <dt className="text-secondary">Platforms</dt><dd className="text-ink"><Platforms /></dd>
                  <dt className="text-secondary">Lockfile</dt><dd className="text-ink">{LOCKABLE.has(plugin.name) ? 'Frozen installs supported' : 'Standard installs only'}</dd>
                </dl>
              </div>
              <div className="relative z-10 flex items-center justify-between bg-page border border-line rounded-md pl-3 pr-1 py-1 font-mono text-sm">
                <span className="text-ink truncate mr-2">avm plugin install {plugin.name}</span>
                <button
                  onClick={() => onCopy(`avm plugin install ${plugin.name}`)}
                  className="inline-flex items-center justify-center w-10 h-10 rounded text-secondary hover:text-ink hover:bg-muted shrink-0"
                  aria-label={`Copy install command for ${plugin.name}`}
                >
                  <Copy className="w-4 h-4" />
                </button>
              </div>
            </article>
          ))}
        </div>

        {/* Empty state if search returned 0 */}
        {filteredPlugins.length === 0 && (
          <div className="text-center py-16 bg-surface/40 rounded-lg border border-line mb-12">
            <Package className="w-12 h-12 text-secondary mx-auto mb-3" />
            <h3 className="text-lg font-bold text-ink mb-1">No matching plugins found</h3>
            <p className="text-sm text-secondary max-w-sm mx-auto mb-4">
              Try adjusting your query.
            </p>
            <button
              onClick={() => setSearchQuery('')}
              className="px-4 py-2 rounded-lg bg-muted hover:bg-muted text-xs font-mono text-ink transition-colors"
            >
              Clear search
            </button>
          </div>
        )}

      </div>
    </section>
  );
};
