import { PluginDocView } from '../components/docs/PluginDocView';
import { useParams } from 'react-router-dom';
import registry from '../../registry.json';
import { usePageMeta } from '../components/usePageMeta';

export const PluginPage = ({ onCopy }: { onCopy: (text: string) => void }) => {
  const { name = '' } = useParams();
  const plugin = registry.plugins.find((p) => p.name === name);
  usePageMeta(plugin ? `${plugin.section_label} plugin for AVM` : 'Plugin not found | AVM', !plugin);
  return (
    <div className="max-w-5xl mx-auto px-5 sm:px-8 py-10 pb-20">
      <PluginDocView pluginName={name} onCopy={onCopy} />
    </div>
  );
};
