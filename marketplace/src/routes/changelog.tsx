import { ChangelogPage } from '../pages/ChangelogPage';
import { pageMeta } from '../data/seo';

export const meta = () => pageMeta('/changelog');

export default function ChangelogRoute() {
  return <ChangelogPage />;
}
