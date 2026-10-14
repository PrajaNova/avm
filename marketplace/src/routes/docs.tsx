import { useParams } from 'react-router';
import { DocsPage } from '../pages/DocsPage';
import { NotFoundPage } from '../pages/NotFoundPage';
import { useCopy } from '../components/CopyContext';
import { pageMeta, seoFor } from '../data/seo';

export const meta = ({ params }: { params: { '*'?: string } }) => pageMeta(`/docs${params['*'] ? `/${params['*']}` : ''}`);

export default function DocsRoute() {
  const splat = useParams()['*'];
  const onCopy = useCopy();
  const path = `/docs${splat ? `/${splat}` : ''}`;
  return seoFor(path) ? <DocsPage onCopy={onCopy} /> : <NotFoundPage />;
}
