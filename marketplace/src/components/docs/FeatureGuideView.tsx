import { Copy } from 'lucide-react';
import { FeatureGuide } from '../../data/featureGuides';

export function FeatureGuideView({ guide, onCopy }: { guide: FeatureGuide; onCopy: (text: string) => void }) {
  return (
    <article className="space-y-8">
      <header>
        <h1 className="text-3xl sm:text-4xl font-semibold text-ink tracking-tight mb-4">{guide.title}</h1>
        <p className="text-secondary leading-relaxed max-w-3xl">{guide.intro}</p>
      </header>
      {guide.sections.map((section) => (
        <section key={section.title} className="rounded-lg bg-surface/50 border border-line p-5 sm:p-7 space-y-4">
          <h2 className="text-xl font-bold text-ink">{section.title}</h2>
          {section.paragraphs.map((paragraph) => <p key={paragraph} className="text-sm text-secondary leading-relaxed">{paragraph}</p>)}
          {section.code && (
            <div className="rounded-lg bg-page border border-line overflow-hidden">
              <div className="flex justify-end border-b border-line px-2">
                <button onClick={() => onCopy(section.code!)} aria-label={`Copy ${section.title} example`} className="flex items-center gap-2 min-h-11 px-3 text-xs text-secondary hover:text-ink focus-visible:outline focus-visible:outline-accent">
                  <Copy aria-hidden="true" className="w-4 h-4" /> Copy example
                </button>
              </div>
              <pre className="p-4 overflow-x-auto text-xs leading-relaxed text-ink"><code>{section.code}</code></pre>
            </div>
          )}
        </section>
      ))}
    </article>
  );
}
