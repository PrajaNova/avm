import { NotFoundPage } from '../pages/NotFoundPage';
import { pageMeta } from '../data/seo';

export const meta = () => pageMeta('/not-found');

export default function NotFoundRoute() {
  return <NotFoundPage />;
}
