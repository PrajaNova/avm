import { Marketplace } from '../components/Marketplace';
import { useCopy } from '../components/CopyContext';
import { pageMeta } from '../data/seo';

export const meta = () => pageMeta('/marketplace');

export default function MarketplaceRoute() {
  return <div className="py-6"><Marketplace onCopy={useCopy()} /></div>;
}
