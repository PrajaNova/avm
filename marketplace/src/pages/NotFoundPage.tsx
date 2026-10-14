import { Link } from 'react-router-dom';
import { usePageMeta } from '../components/usePageMeta';

export const NotFoundPage = () => {
  usePageMeta('Page not found | AVM', true);
  return (
    <div className="max-w-2xl mx-auto px-5 sm:px-8 py-24">
      <h1 className="text-4xl font-medium tracking-tight mb-4">Page not found</h1>
      <p className="text-secondary mb-8">That address doesn't exist. Try the documentation or the plugin list.</p>
      <div className="flex flex-wrap gap-3"><Link to="/docs/guide/getting-started" className="button-primary">Get started</Link><Link to="/marketplace" className="button-secondary">Browse plugins</Link></div>
    </div>
  );
};
