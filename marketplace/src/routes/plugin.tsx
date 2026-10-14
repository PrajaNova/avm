import { PluginPage } from '../pages/PluginPage';
import { useCopy } from '../components/CopyContext';
import { pageMeta } from '../data/seo';

export const meta = ({ params }: { params: { name?: string } }) => pageMeta(`/marketplace/${params.name}`);

export default function PluginRoute() {
  return <PluginPage onCopy={useCopy()} />;
}
