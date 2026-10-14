import { Copy } from 'lucide-react';
import { FeatureGuide } from '../../data/featureGuides';

export function FeatureGuideView({ guide, onCopy }: { guide: FeatureGuide; onCopy: (text: string) => void }) {
  return (
    <article className="space-y-8">
      <header>
        <h1 className="text-3xl sm:text-4xl font-extrabold text-white tracking-tight mb-4">{guide.title}</h1>
        <p className="text-slate-300 leading-relaxed max-w-3xl">{guide.intro}</p>
      </header>
      {guide.sections.map((section) => (
        <section key={section.title} className="rounded-2xl bg-slate-900/50 border border-slate-800 p-5 sm:p-7 space-y-4">
          <h2 className="text-xl font-bold text-white">{section.title}</h2>
          {section.paragraphs.map((paragraph) => <p key={paragraph} className="text-sm text-slate-300 leading-relaxed">{paragraph}</p>)}
          {section.code && (
            <div className="rounded-xl bg-slate-950 border border-slate-800 overflow-hidden">
              <div className="flex justify-end border-b border-slate-800 px-2">
                <button onClick={() => onCopy(section.code!)} aria-label={`Copy ${section.title} example`} className="flex items-center gap-2 min-h-11 px-3 text-xs text-slate-300 hover:text-white focus-visible:outline focus-visible:outline-emerald-400">
                  <Copy aria-hidden="true" className="w-4 h-4" /> Copy example
                </button>
              </div>
              <pre className="p-4 overflow-x-auto text-xs leading-relaxed text-slate-200"><code>{section.code}</code></pre>
            </div>
          )}
        </section>
      ))}
    </article>
  );
}
