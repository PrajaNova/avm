import { PluginDocView } from '../components/docs/PluginDocView';
import { useParams } from 'react-router-dom';

export const PluginPage = ({ onCopy }: { onCopy: (text: string) => void }) => {
  const { name = '' } = useParams();
  return (
    <div className="max-w-5xl mx-auto px-5 sm:px-8 py-10 pb-20">
      <PluginDocView pluginName={name} onCopy={onCopy} />
    </div>
  );
};
