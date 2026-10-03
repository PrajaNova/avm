import React, { useEffect, useState } from 'react';
import { ExternalLink, Sparkles, Tag } from 'lucide-react';
import { CHANGELOG_SOURCE, CHANGELOG_URL, ChangelogRelease, GROUP_ORDER, parseChangelog } from '../data/changelog';

const REPO = 'https://github.com/PrajaNova/avm';

/** Inline markdown used in the changelog: `code`, **bold**, [text](url), and #123 issue refs. */
function Inline({ text }: { text: string }) {
  const parts = text.split(/(`[^`]+`|\*\*[^*]+\*\*|\[[^\]]+\]\([^)]+\)|#\d+\b)/g);
  return (
    <>
      {parts.map((part, i) => {
        if (part.startsWith('`') && part.endsWith('`') && part.length > 1) {
          return (
            <code key={i} className="text-emerald-300 font-mono text-[0.85em] bg-slate-950 px-1 py-0.5 rounded">
              {part.slice(1, -1)}
            </code>
          );
        }
        if (part.startsWith('**') && part.endsWith('**')) {
          return <strong key={i} className="text-white">{part.slice(2, -2)}</strong>;
        }
        const link = part.match(/^\[([^\]]+)\]\(([^)]+)\)$/);
        if (link) {
          return (
            <a key={i} href={link[2]} target="_blank" rel="noreferrer" className="text-emerald-400 hover:underline">
              {link[1]}
            </a>
          );
        }
        if (/^#\d+$/.test(part)) {
          return (
            <a key={i} href={`${REPO}/issues/${part.slice(1)}`} target="_blank" rel="noreferrer" className="text-emerald-400 hover:underline">
              {part}
            </a>
          );
        }
        return <React.Fragment key={i}>{part}</React.Fragment>;
      })}
    </>
  );
}

/** Same-named groups merged (some old entries repeat a heading), in display order. */
function orderedGroups(release: ChangelogRelease) {
  const merged = new Map<string, string[]>();
  for (const g of release.groups) merged.set(g.heading, [...(merged.get(g.heading) ?? []), ...g.items]);
  return [...merged.entries()]
    .filter(([, items]) => items.length)
    .sort(([a], [b]) => (GROUP_ORDER[a]?.rank ?? 99) - (GROUP_ORDER[b]?.rank ?? 99));
}

const anchor = (version: string) => `v-${version.replace(/[^a-zA-Z0-9-]/g, '-')}`;

export const ChangelogPage: React.FC = () => {
  const [releases, setReleases] = useState<ChangelogRelease[] | null>(null);
  const [failed, setFailed] = useState(false);

  useEffect(() => {
    fetch(CHANGELOG_URL)
      .then((r) => (r.ok ? r.text() : Promise.reject(r.status)))
      .then((md) => setReleases(parseChangelog(md)))
      .catch(() => setFailed(true));
  }, []);

  const latest = releases?.find((r) => r.version !== 'Unreleased')?.version;

  return (
    <div className="max-w-6xl mx-auto px-4 sm:px-6 lg:px-8 py-12">
      <div className="mb-10">
        <p className="text-[11px] font-mono tracking-widest text-emerald-400 uppercase mb-2">CHANGELOG</p>
        <h1 className="text-3xl sm:text-4xl font-extrabold text-white tracking-tight mb-3">What's new in avm</h1>
        <p className="text-slate-300 max-w-3xl">
          Every release, version by version: new features first, then changes and fixes. Issue numbers link to the
          discussion behind each change.
        </p>
      </div>

      {failed && (
        <div className="p-5 rounded-2xl bg-slate-900/60 border border-slate-800 text-sm text-slate-300">
          Couldn't load the changelog right now. Read it on{' '}
          <a href={CHANGELOG_SOURCE} target="_blank" rel="noreferrer" className="text-emerald-400 hover:underline">GitHub</a>.
        </div>
      )}
      {!releases && !failed && <div className="text-sm font-mono text-slate-400">Loading releases…</div>}

      {releases && (
        <div className="flex gap-10">
          {/* Version index */}
          <nav className="hidden lg:block w-48 shrink-0">
            <div className="sticky top-24 space-y-1 text-sm font-mono">
              {releases.map((r) => (
                <a key={r.version} href={`#${anchor(r.version)}`} onClick={(e) => { e.preventDefault(); document.getElementById(anchor(r.version))?.scrollIntoView({ behavior: 'smooth' }); }}
                  className="block px-3 py-1.5 rounded-lg text-slate-400 hover:text-white hover:bg-slate-900">
                  {r.version === 'Unreleased' ? 'Unreleased' : `v${r.version}`}
                </a>
              ))}
            </div>
          </nav>

          <div className="flex-1 min-w-0 space-y-8">
            {releases.map((r) => {
              const unreleased = r.version === 'Unreleased';
              return (
                <section key={r.version} id={anchor(r.version)} className="scroll-mt-24 rounded-2xl bg-slate-900/50 border border-slate-800 p-6 sm:p-8">
                  <header className="flex flex-wrap items-center gap-3 mb-6">
                    <h2 className="text-2xl font-bold text-white font-mono">{unreleased ? 'Unreleased' : `v${r.version}`}</h2>
                    {r.version === latest && (
                      <span className="px-2 py-0.5 rounded-full text-[11px] font-mono bg-emerald-500/15 text-emerald-300 border border-emerald-500/30">Latest</span>
                    )}
                    {r.version.includes('-') && (
                      <span className="px-2 py-0.5 rounded-full text-[11px] font-mono bg-amber-500/15 text-amber-300 border border-amber-500/30">Pre-release</span>
                    )}
                    {unreleased && (
                      <span className="px-2 py-0.5 rounded-full text-[11px] font-mono bg-slate-700/50 text-slate-300 border border-slate-600">Coming next</span>
                    )}
                    {r.date && <span className="text-sm font-mono text-slate-400">{r.date}</span>}
                    {!unreleased && (
                      <a href={`${REPO}/releases/tag/v${r.version}`} target="_blank" rel="noreferrer"
                        className="ml-auto flex items-center gap-1 text-xs font-mono text-slate-400 hover:text-emerald-400">
                        <Tag className="w-3.5 h-3.5" /> Release <ExternalLink className="w-3 h-3" />
                      </a>
                    )}
                  </header>

                  <div className="space-y-6">
                    {orderedGroups(r).map(([heading, items]) => {
                      const isNew = heading === 'Added';
                      return (
                        <div key={heading}>
                          <h3 className={`flex items-center gap-2 text-xs font-mono uppercase tracking-wider mb-3 ${isNew ? 'text-emerald-400' : 'text-slate-400'}`}>
                            {isNew && <Sparkles className="w-3.5 h-3.5" />}
                            {GROUP_ORDER[heading]?.label ?? heading}
                          </h3>
                          <ul className="space-y-2.5 text-sm leading-relaxed text-slate-300">
                            {items.map((item, i) => (
                              <li key={i} className={`pl-4 border-l-2 ${isNew ? 'border-emerald-500/40' : 'border-slate-700'}`}>
                                <Inline text={item} />
                              </li>
                            ))}
                          </ul>
                        </div>
                      );
                    })}
                  </div>
                </section>
              );
            })}
          </div>
        </div>
      )}
    </div>
  );
};
