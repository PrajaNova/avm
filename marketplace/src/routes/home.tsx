import { HomePage } from '../pages/HomePage';
import { useCopy } from '../components/CopyContext';
import { pageMeta } from '../data/seo';

export const meta = () => pageMeta('/');

export default function Home() {
  return <HomePage onCopy={useCopy()} />;
}
