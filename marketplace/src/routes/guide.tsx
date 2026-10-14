import { Link, useParams } from 'react-router';
import { FeatureGuideView } from '../components/docs/FeatureGuideView';
import { NotFoundPage } from '../pages/NotFoundPage';
import { useCopy } from '../components/CopyContext';
import { SEO_GUIDES } from '../data/seoGuides';
import { pageMeta } from '../data/seo';

export const meta = ({ params }: { params: { slug?: string } }) => pageMeta(`/guides/${params.slug}`);

export default function GuideRoute() {
  const guide = SEO_GUIDES.find((g) => g.slug === useParams().slug);
  const onCopy = useCopy();
  if (!guide) return <NotFoundPage />;
  return (
    <div className="max-w-3xl mx-auto px-5 sm:px-8 py-12 pb-20">
      <FeatureGuideView guide={guide} onCopy={onCopy} />
      <nav aria-label="Related pages" className="mt-10 border-t border-line pt-6">
        <h2 className="text-lg font-medium mb-3">Related</h2>
        <ul className="space-y-2">{guide.related.map((r) => <li key={r.to}><Link to={r.to} className="text-accent underline underline-offset-4">{r.label}</Link></li>)}</ul>
      </nav>
    </div>
  );
}
