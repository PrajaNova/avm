import React from 'react';
import { Link } from 'react-router';
import {
  ExternalLink,
  Copy,
  Package,
  Layers,
  Sparkles,
  FileJson,
  Cpu
} from 'lucide-react';
import { registry } from '../../data/registry';
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
  if (!plugin) return <p className="text-secondary">Unknown plugin: {pluginName}</p>;
  const cleanName = plugin.name;
  const meta = EXTENDED_PLUGIN_METADATA[cleanName];
  const walkthrough = PLUGIN_WALKTHROUGHS[cleanName];

  return (
    <div className="space-y-12">
      
      {/* Breadcrumb & Header */}
      <div>
        <div className="flex items-center gap-2 text-xs font-mono text-secondary mb-3">
          <Link to="/marketplace" className="hover:text-accent">Plugins</Link>
          <span>/</span>
          <span className="text-accent font-semibold">{cleanName}</span>
        </div>

        <div className="flex flex-col sm:flex-row sm:items-center justify-between gap-4 pb-6 border-b border-line">
          <div>
            <div className="flex items-center gap-3 mb-2">
              <h1 className="text-3xl sm:text-4xl font-semibold text-ink tracking-tight">
                {plugin.section_label} plugin for AVM
              </h1>
              <span className="text-xs font-mono px-2.5 py-0.5 rounded-full bg-accent/15 text-accent border border-accent/30">
                Official
              </span>
            </div>
            <p className="text-secondary text-sm font-mono">
              avm-plugin-{cleanName} &bull; Precompiled Native Executable
            </p>
          </div>

          <div className="flex items-center gap-3 shrink-0">
            <a
              href={`https://github.com/${plugin.repo}/tree/main/plugins/avm-plugin-${cleanName}`}
              target="_blank"
              rel="noreferrer"
              className="flex items-center gap-2 px-3.5 py-2 rounded-lg bg-surface hover:bg-muted border border-line text-secondary hover:text-ink transition-all text-xs font-mono"
            >
              <span>Source on GitHub</span>
              <ExternalLink className="w-3.5 h-3.5 text-secondary" />
            </a>
          </div>
        </div>
      </div>

      {/* Embedded Video Tutorial & Interactive Walkthrough */}
      {walkthrough && (
      <div>
        <div className="flex items-center justify-between gap-4 mb-4">
          <div>
            <div className="flex items-center gap-2 text-xs font-mono text-accent font-bold uppercase tracking-wider mb-1">
              <Sparkles className="w-3.5 h-3.5" />
              <span>Interactive Tutorial & CLI Demo</span>
            </div>
            <h2 className="text-xl sm:text-2xl font-bold text-ink">
              {walkthrough.title}
            </h2>
          </div>
          <span className="hidden sm:inline-block text-xs font-mono text-secondary">
            {walkthrough.steps.length} interactive steps
          </span>
        </div>

        <p className="text-xs text-secondary mb-4">Illustrative walkthrough: release versions, output and timings vary by platform and network. Install a JDK before setting up Android.</p>
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
      <div className="rounded-lg bg-surface/60 border border-line p-6 sm:p-7">
        <h3 className="text-base font-bold text-ink mb-2 flex items-center gap-2">
          <Package className="w-4 h-4 text-accent" />
          <span>Quick Installation</span>
        </h3>
        <p className="text-xs sm:text-sm text-secondary mb-4 leading-relaxed">
          Install the precompiled standalone executable directly into <code className="text-accent font-mono text-xs bg-page px-1.5 py-0.5 rounded">~/.avm/plugins/avm-plugin-{cleanName}</code>:
        </p>

        <div className="relative rounded-lg bg-page border border-line p-4 font-mono text-xs sm:text-sm text-accent flex items-center justify-between">
          <span>avm plugin install {cleanName}</span>
          <button
            onClick={() => onCopy(`avm plugin install ${cleanName}`)}
            className="p-1.5 rounded-lg bg-muted hover:bg-muted text-secondary transition-colors"
            title="Copy command"
          >
                          <Copy className="w-4 h-4" />
          </button>
        </div>
      </div>

      {/* Key Commands Table */}
      {meta && (
        <div className="space-y-4">
          <div className="flex items-center gap-2 text-ink font-bold text-lg">
            <Layers className="w-5 h-5 text-accent" />
            <span>Key Commands</span>
          </div>

          <div className="rounded-lg border border-line overflow-x-auto bg-surface/40">
            <table className="w-full text-left text-xs border-collapse">
              <thead>
                <tr className="border-b border-line bg-page/80 font-mono text-secondary">
                  <th className="p-3.5 font-semibold">Command</th>
                  <th className="p-3.5 font-semibold">Purpose</th>
                  <th className="p-3.5 text-right font-semibold">Copy</th>
                </tr>
              </thead>
              <tbody className="divide-y divide-line/60 font-mono">
                {meta.keyCommands.map((cmd, idx) => (
                  <tr key={idx} className="hover:bg-muted/30 transition-colors">
                    <td className="p-3.5 text-accent font-semibold whitespace-nowrap">
                      {cmd.command}
                    </td>
                    <td className="p-3.5 text-secondary font-sans text-xs">
                      {cmd.description}
                    </td>
                    <td className="p-3.5 text-right">
                      <button
                        onClick={() => onCopy(cmd.command)}
                        className="p-1.5 rounded-lg text-secondary hover:text-ink hover:bg-muted transition-colors"
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
        <div className="rounded-lg bg-surface/50 border border-line p-6 sm:p-7">
          <h3 className="text-base font-bold text-ink mb-2 flex items-center gap-2">
            <Cpu className="w-4 h-4 text-accent" />
            <span>Environment Variable Injections</span>
          </h3>
          <p className="text-xs sm:text-sm text-secondary mb-4 leading-relaxed">
            When you run an alias (e.g. <code className="text-accent font-mono text-xs bg-page px-1 py-0.5 rounded">avm dev</code>) or call shims directly, the plugin exports the following environment variables:
          </p>

          <div className="grid grid-cols-1 sm:grid-cols-2 gap-3">
            {meta.envVars.map((env, idx) => (
              <div
                key={idx}
                className="p-3 rounded-lg bg-page border border-line/80 font-mono text-xs flex items-center justify-between text-secondary"
              >
                <span className="text-accent font-semibold">{env}</span>
                <span className="text-xs text-secondary">Auto-injected</span>
              </div>
            ))}
          </div>
        </div>
      )}

      {cleanName === 'android' && (
        <section className="rounded-lg bg-surface/50 border border-line p-6 space-y-4 text-sm text-secondary">
          <h2 className="text-xl font-bold text-ink">SDK setup and overrides</h2>
          <p>Install a JDK first, then select an installed SDK with <code>avm android use 34</code>. AVD commands operate on that selected SDK. Raw adb, sdkmanager, avdmanager and emulator commands are also available through shims.</p>
          <p>ANDROID_BUILD_TOOLS_VERSION overrides the build-tools selection. ANDROID_SYSTEM_IMAGE_API selects a system-image API ID. For a custom command-line tools build, set both ANDROID_CMDLINE_TOOLS_BUILD and ANDROID_CMDLINE_TOOLS_SHA256.</p>
          <p>AVM_ANDROID_CURL_TIMEOUT, AVM_ANDROID_UNZIP_TIMEOUT and AVM_ANDROID_SDKMANAGER_TIMEOUT are measured in seconds. Large SDK and emulator downloads may need longer timeouts.</p>
          <p>Android components are installed and verified by sdkmanager. Their revisions are not captured by .avm.lock, so use ordinary installs for Android.</p>
        </section>
      )}
      {cleanName === 'node' && (
        <section className="rounded-lg bg-surface/50 border border-line p-6 space-y-4 text-sm text-secondary">
          <h2 className="text-xl font-bold text-ink">Scripts and global npm commands</h2>
          <p>Scripts from the current directory’s package.json are available as aliases. Package-manager detection checks Bun lockfiles, then pnpm-lock.yaml, then yarn.lock, and otherwise uses npm. Explicit project aliases win over discovered scripts.</p>
          <p>Global package executables remain reachable through the globally selected Node installation when a project selects another version. Successful package-manager installs regenerate shims; run <code>avm shims install</code> if a command is missing.</p>
        </section>
      )}

      {/* Sample .avm.json Snippet */}
      {meta && (
        <div className="space-y-3">
          <div className="flex items-center gap-2 text-ink font-bold text-lg">
            <FileJson className="w-5 h-5 text-accent" />
            <span>Sample Project Configuration (.avm.json)</span>
          </div>

          <div className="relative rounded-lg bg-page border border-line p-5 font-mono text-xs text-secondary overflow-x-auto shadow-sm">
            <pre>{meta.sampleConfig}</pre>
            <button
              onClick={() => onCopy(meta.sampleConfig)}
              className="absolute top-4 right-4 p-2 rounded-lg bg-muted hover:bg-muted text-secondary transition-colors"
              title="Copy config"
            >
                              <Copy className="w-4 h-4" />
            </button>
          </div>
        </div>
      )}

      {/* Feature Highlights */}
      {meta && (
        <div className="rounded-lg bg-surface/30 border border-line p-6 sm:p-7 space-y-3">
          <h3 className="text-base font-bold text-ink">Features & Highlights</h3>
          <ul className="grid grid-cols-1 sm:grid-cols-2 gap-3 text-xs text-secondary">
            {meta.highlights.map((highlight, idx) => (
              <li
                key={idx}
                className="flex items-start gap-2.5 p-3 rounded-lg bg-page/60 border border-line/80"
              >
                <span className="w-1.5 h-1.5 rounded-full bg-accent shrink-0 mt-1.5" />
                <span>{highlight}</span>
              </li>
            ))}
          </ul>
        </div>
      )}

    </div>
  );
};
