import React from 'react';
import {
  ExternalLink,
  Copy,
  Package,
  Layers,
  Sparkles,
  FileJson,
  Cpu
} from 'lucide-react';
import registry from '../../../registry.json';
import { EXTENDED_PLUGIN_METADATA } from '../../data/registryData';
import { PLUGIN_WALKTHROUGHS } from '../../data/pluginWalkthroughData';
import { WalkthroughVideo } from '../WalkthroughVideo';

interface PluginDocViewProps {
  pluginName: string;
  onCopy: (text: string) => void;
}

export const PluginDocView: React.FC<PluginDocViewProps> = ({
  pluginName,
  onCopy,
}) => {
  const plugin = registry.plugins.find((p) => p.name === pluginName);
  if (!plugin) return <p className="text-slate-400">Unknown plugin: {pluginName}</p>;
  const cleanName = plugin.name;
  const meta = EXTENDED_PLUGIN_METADATA[cleanName];
  const walkthrough = PLUGIN_WALKTHROUGHS[cleanName];

  return (
    <div className="space-y-12">
      
      {/* Breadcrumb & Header */}
      <div>
        <div className="flex items-center gap-2 text-xs font-mono text-slate-400 mb-3">
          <span>Documentation</span>
          <span>/</span>
          <span>Plugins</span>
          <span>/</span>
          <span className="text-emerald-400 font-semibold">{cleanName}</span>
        </div>

        <div className="flex flex-col sm:flex-row sm:items-center justify-between gap-4 pb-6 border-b border-slate-800">
          <div>
            <div className="flex items-center gap-3 mb-2">
              <h1 className="text-3xl sm:text-4xl font-extrabold text-white tracking-tight">
                {plugin.section_label} Plugin
              </h1>
              <span className="text-xs font-mono px-2.5 py-0.5 rounded-full bg-emerald-500/15 text-emerald-400 border border-emerald-500/30">
                Official
              </span>
            </div>
            <p className="text-slate-400 text-sm font-mono">
              avm-plugin-{cleanName} &bull; Precompiled Native Executable
            </p>
          </div>

          <div className="flex items-center gap-3 shrink-0">
            <a
              href={`https://github.com/${plugin.repo}`}
              target="_blank"
              rel="noreferrer"
              className="flex items-center gap-2 px-3.5 py-2 rounded-xl bg-slate-900 hover:bg-slate-800 border border-slate-800 text-slate-300 hover:text-white transition-all text-xs font-mono"
            >
              <span>GitHub</span>
              <ExternalLink className="w-3.5 h-3.5 text-slate-400" />
            </a>
          </div>
        </div>
      </div>

      {/* Embedded Video Tutorial & Interactive Walkthrough */}
      {walkthrough && (
      <div>
        <div className="flex items-center justify-between gap-4 mb-4">
          <div>
            <div className="flex items-center gap-2 text-xs font-mono text-emerald-400 font-bold uppercase tracking-wider mb-1">
              <Sparkles className="w-3.5 h-3.5" />
              <span>Interactive Tutorial & CLI Demo</span>
            </div>
            <h2 className="text-xl sm:text-2xl font-bold text-white">
              {walkthrough.title}
            </h2>
          </div>
          <span className="hidden sm:inline-block text-xs font-mono text-slate-500">
            {walkthrough.steps.length} interactive steps
          </span>
        </div>

        {/* Multi-mode Walkthrough Component */}
        <WalkthroughVideo
          steps={walkthrough.steps}
          title={walkthrough.title}
          badge={walkthrough.badge}
          className="w-full mb-0"
          onCopy={onCopy}
        />
      </div>
      )}

      {/* Quick Install Bar */}
      <div className="rounded-2xl bg-slate-900/60 border border-slate-800 p-6 sm:p-7">
        <h3 className="text-base font-bold text-white mb-2 flex items-center gap-2">
          <Package className="w-4 h-4 text-emerald-400" />
          <span>Quick Installation</span>
        </h3>
        <p className="text-xs sm:text-sm text-slate-300 mb-4 leading-relaxed">
          Install the precompiled standalone executable directly into <code className="text-emerald-300 font-mono text-xs bg-slate-950 px-1.5 py-0.5 rounded">~/.avm/plugins/avm-plugin-{cleanName}</code>:
        </p>

        <div className="relative rounded-xl bg-slate-950 border border-slate-800 p-4 font-mono text-xs sm:text-sm text-emerald-300 flex items-center justify-between">
          <span>avm plugin add {cleanName}</span>
          <button
            onClick={() => onCopy(`avm plugin add ${cleanName}`)}
            className="p-1.5 rounded-lg bg-slate-800 hover:bg-slate-700 text-slate-300 transition-colors"
            title="Copy command"
          >
                          <Copy className="w-4 h-4" />
          </button>
        </div>
      </div>

      {/* Key Commands Table */}
      {meta && (
        <div className="space-y-4">
          <div className="flex items-center gap-2 text-white font-bold text-lg">
            <Layers className="w-5 h-5 text-emerald-400" />
            <span>Key Commands</span>
          </div>

          <div className="rounded-xl border border-slate-800 overflow-hidden bg-slate-900/40">
            <table className="w-full text-left text-xs border-collapse">
              <thead>
                <tr className="border-b border-slate-800 bg-slate-950/80 font-mono text-slate-400">
                  <th className="p-3.5 font-semibold">Command</th>
                  <th className="p-3.5 font-semibold">Purpose</th>
                  <th className="p-3.5 text-right font-semibold">Copy</th>
                </tr>
              </thead>
              <tbody className="divide-y divide-slate-800/60 font-mono">
                {meta.keyCommands.map((cmd, idx) => (
                  <tr key={idx} className="hover:bg-slate-800/30 transition-colors">
                    <td className="p-3.5 text-emerald-300 font-semibold whitespace-nowrap">
                      {cmd.command}
                    </td>
                    <td className="p-3.5 text-slate-300 font-sans text-xs">
                      {cmd.description}
                    </td>
                    <td className="p-3.5 text-right">
                      <button
                        onClick={() => onCopy(cmd.command)}
                        className="p-1.5 rounded-lg text-slate-400 hover:text-white hover:bg-slate-800 transition-colors"
                        title="Copy command"
                      >
                                                  <Copy className="w-3.5 h-3.5" />
                      </button>
                    </td>
                  </tr>
                ))}
              </tbody>
            </table>
          </div>
        </div>
      )}

      {/* Environment Injections */}
      {meta && (
        <div className="rounded-2xl bg-slate-900/50 border border-slate-800 p-6 sm:p-7">
          <h3 className="text-base font-bold text-white mb-2 flex items-center gap-2">
            <Cpu className="w-4 h-4 text-emerald-400" />
            <span>Environment Variable Injections</span>
          </h3>
          <p className="text-xs sm:text-sm text-slate-300 mb-4 leading-relaxed">
            When you run an alias (e.g. <code className="text-emerald-300 font-mono text-xs bg-slate-950 px-1 py-0.5 rounded">avm dev</code>) or call shims directly, the plugin exports the following environment variables:
          </p>

          <div className="grid grid-cols-1 sm:grid-cols-2 gap-3">
            {meta.envVars.map((env, idx) => (
              <div
                key={idx}
                className="p-3 rounded-xl bg-slate-950 border border-slate-800/80 font-mono text-xs flex items-center justify-between text-slate-300"
              >
                <span className="text-emerald-400 font-semibold">{env}</span>
                <span className="text-[11px] text-slate-500">Auto-injected</span>
              </div>
            ))}
          </div>
        </div>
      )}

      {/* Sample .avm.json Snippet */}
      {meta && (
        <div className="space-y-3">
          <div className="flex items-center gap-2 text-white font-bold text-lg">
            <FileJson className="w-5 h-5 text-emerald-400" />
            <span>Sample Project Configuration (.avm.json)</span>
          </div>

          <div className="relative rounded-2xl bg-slate-950 border border-slate-800 p-5 font-mono text-xs text-slate-300 overflow-x-auto shadow-lg">
            <pre>{meta.sampleConfig}</pre>
            <button
              onClick={() => onCopy(meta.sampleConfig)}
              className="absolute top-4 right-4 p-2 rounded-lg bg-slate-800 hover:bg-slate-700 text-slate-300 transition-colors"
              title="Copy config"
            >
                              <Copy className="w-4 h-4" />
            </button>
          </div>
        </div>
      )}

      {/* Feature Highlights */}
      {meta && (
        <div className="rounded-2xl bg-slate-900/30 border border-slate-800 p-6 sm:p-7 space-y-3">
          <h3 className="text-base font-bold text-white">Features & Highlights</h3>
          <ul className="grid grid-cols-1 sm:grid-cols-2 gap-3 text-xs text-slate-300">
            {meta.highlights.map((highlight, idx) => (
              <li
                key={idx}
                className="flex items-start gap-2.5 p-3 rounded-xl bg-slate-950/60 border border-slate-800/80"
              >
                <span className="w-1.5 h-1.5 rounded-full bg-emerald-400 shrink-0 mt-1.5" />
                <span>{highlight}</span>
              </li>
            ))}
          </ul>
        </div>
      )}

    </div>
  );
};
