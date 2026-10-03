import React, { useState } from 'react';
import { useLocation, useNavigate } from 'react-router-dom';
import {
  Menu,
  Copy,
  Terminal,
  FileJson,
  Package,
  Cpu,
  ArrowRight,
  ArrowLeft
} from 'lucide-react';
import { DocsSidebar, ASDF_SIDEBAR_CONFIG } from '../components/docs/DocsSidebar';
import { PluginDocView } from '../components/docs/PluginDocView';
import { TerminalInstallBox } from '../components/TerminalInstallBox';
import { CliReference } from '../components/CliReference';
import { ComparisonTable } from '../components/ComparisonTable';
import { SHELL_SETUP_GUIDE } from '../data/guideData';

interface DocsPageProps {
  onCopy: (text: string) => void;
}

// Flat list of navigable pages for Next/Previous links
const ALL_PAGES = ASDF_SIDEBAR_CONFIG.flatMap((g) =>
  g.items.filter((i) => !i.external).map((i) => ({ ...i, group: g.text }))
);

export const DocsPage: React.FC<DocsPageProps> = ({ onCopy }) => {
  const location = useLocation();
  const navigate = useNavigate();
  const [isMobileMenuOpen, setIsMobileMenuOpen] = useState(false);

  const currentPage = ALL_PAGES.find((p) => p.link === location.pathname) || ALL_PAGES[0];
  const activeSection = currentPage.id;
  const pageIndex = ALL_PAGES.indexOf(currentPage);
  const prevPage = pageIndex > 0 ? ALL_PAGES[pageIndex - 1] : null;
  const nextPage = pageIndex < ALL_PAGES.length - 1 ? ALL_PAGES[pageIndex + 1] : null;

  return (
    <div className="min-h-screen bg-slate-950 text-slate-100 flex flex-col antialiased">
      
      {/* Mobile Bar */}
      <div className="lg:hidden sticky top-14 z-30 bg-slate-950/95 backdrop-blur-md border-b border-slate-800 px-4 py-3 flex items-center justify-between">
        <button
          onClick={() => setIsMobileMenuOpen(true)}
          className="flex items-center gap-2 px-3 py-1.5 rounded-lg bg-slate-900 border border-slate-800 text-xs font-mono text-slate-300 hover:text-white"
        >
          <Menu className="w-4 h-4 text-emerald-400" />
          <span>Documentation Menu</span>
        </button>

        <span className="text-xs font-mono font-medium text-emerald-400 truncate max-w-[200px]">
          {currentPage.text}
        </span>
      </div>

      {/* Main Body */}
      <div className="flex-1 max-w-7xl w-full mx-auto px-4 sm:px-6 lg:px-8 py-8 flex gap-8">
        
        {/* asdf-vm style Sidebar */}
        <DocsSidebar
          activeSection={activeSection}
          onSelectSection={navigate}
          isOpenMobile={isMobileMenuOpen}
          onCloseMobile={() => setIsMobileMenuOpen(false)}
        />

        {/* Content View */}
        <main className="flex-1 min-w-0 pb-16">
          
          {/* Breadcrumb Header */}
          <div className="flex items-center gap-2 text-xs font-mono text-slate-400 mb-6 pb-2 border-b border-slate-800/60">
            <span>Docs</span>
            <span>/</span>
            <span className="text-slate-400">{currentPage.group}</span>
            <span>/</span>
            <span className="text-emerald-400 font-semibold">{currentPage.text}</span>
          </div>

          {/* SECTION ROUTING */}

          {/* 1. Official Plugin Views with Embedded Video Tutorials */}
          {activeSection.startsWith('plugin-') ? (
            <PluginDocView
              pluginName={activeSection.replace('plugin-', '')}
              onCopy={onCopy}
            />
          ) : activeSection === 'introduction' ? (
            
            /* 2. Guide: What is avm? */
            <div className="space-y-10">
              <div>
                <h1 className="text-3xl sm:text-4xl font-extrabold text-white tracking-tight mb-4">
                  What is avm?
                </h1>
                <p className="text-slate-300 text-base leading-relaxed max-w-3xl">
                  <strong className="text-white">avm</strong> is an ultra-fast, native CLI version manager and project orchestrator. 
                  It manages multiple runtime versions (like Node.js, OpenJDK Temurin, Android SDK) and anchors directory-scoped command aliases and environment variables in a single version-controlled configuration.
                </p>
              </div>

              {/* Motivation */}
              <div className="rounded-2xl bg-slate-900/50 border border-slate-800 p-6 sm:p-8 space-y-4">
                <h3 className="text-xl font-bold text-white">Why avm?</h3>
                <p className="text-sm text-slate-300 leading-relaxed">
                  Version managers like <em>nvm</em> and <em>rbenv</em> rely on shell scripts, and most tools split their state across fragmented configuration files (<code className="text-emerald-300 font-mono text-xs bg-slate-950 px-1 py-0.5 rounded">.tool-versions</code>, <code className="text-emerald-300 font-mono text-xs bg-slate-950 px-1 py-0.5 rounded">.nvmrc</code>, <code className="text-emerald-300 font-mono text-xs bg-slate-950 px-1 py-0.5 rounded">.ruby-version</code>).
                </p>
                <div className="grid grid-cols-1 md:grid-cols-2 gap-4 pt-2 text-xs">
                  <div className="p-4 rounded-xl bg-slate-950 border border-slate-800">
                    <span className="text-rose-400 font-bold block mb-1">Traditional Friction</span>
                    <ul className="space-y-1.5 text-slate-400">
                      <li>&bull; Slower prompt load times due to shell wrappers</li>
                      <li>&bull; Compiling plugins and runtimes from source</li>
                      <li>&bull; Command drift across engineering teammates</li>
                    </ul>
                  </div>
                  <div className="p-4 rounded-xl bg-slate-950 border border-emerald-500/20 bg-emerald-950/10">
                    <span className="text-emerald-400 font-bold block mb-1">The avm Approach</span>
                    <ul className="space-y-1.5 text-slate-300">
                      <li>&bull; A single compiled Rust binary, with plugins as separate verified executables</li>
                      <li>&bull; Reads your existing <code className="font-mono text-emerald-300">.nvmrc</code> / <code className="font-mono text-emerald-300">.tool-versions</code> as-is</li>
                      <li>&bull; Precompiled standalone binaries with zero compilation</li>
                      <li>&bull; Unified <code className="font-mono text-emerald-300">.avm.json</code> for aliases, tools, and env vars</li>
                    </ul>
                  </div>
                </div>
              </div>

              {/* Core Features */}
              <div className="space-y-4">
                <h3 className="text-xl font-bold text-white">Core Capabilities</h3>
                <div className="grid grid-cols-1 sm:grid-cols-3 gap-4">
                  <div className="p-5 rounded-xl bg-slate-900/60 border border-slate-800">
                    <Package className="w-5 h-5 text-emerald-400 mb-2" />
                    <h4 className="font-bold text-white text-sm mb-1">Plugin Marketplace</h4>
                    <p className="text-xs text-slate-400">
                      Native executables fetched on demand directly from GitHub Releases.
                    </p>
                  </div>
                  <div className="p-5 rounded-xl bg-slate-900/60 border border-slate-800">
                    <Terminal className="w-5 h-5 text-emerald-400 mb-2" />
                    <h4 className="font-bold text-white text-sm mb-1">Static PATH Shims</h4>
                    <p className="text-xs text-slate-400">
                      Zero subshell overhead. Commands resolve immediately on invocation.
                    </p>
                  </div>
                  <div className="p-5 rounded-xl bg-slate-900/60 border border-slate-800">
                    <FileJson className="w-5 h-5 text-emerald-400 mb-2" />
                    <h4 className="font-bold text-white text-sm mb-1">Unified Config</h4>
                    <p className="text-xs text-slate-400">
                      A single <code className="text-emerald-300 font-mono text-[11px]">.avm.json</code> file commits your team's workflow to git.
                    </p>
                  </div>
                </div>
              </div>
            </div>

          ) : activeSection === 'getting-started' ? (

            /* 3. Guide: Getting Started */
            <div className="space-y-10">
              <div>
                <h1 className="text-3xl sm:text-4xl font-extrabold text-white tracking-tight mb-4">
                  Getting Started
                </h1>
                <p className="text-slate-300 text-base leading-relaxed max-w-3xl">
                  Install avm on macOS, Linux, or Windows in seconds. Every installer verifies the binary's sha256 before installing it.
                </p>
              </div>

              {/* Installation Methods */}
              <div className="rounded-2xl bg-slate-900/50 border border-slate-800 p-6 sm:p-8 space-y-6">
                <div>
                  <h3 className="text-xl font-bold text-white mb-1">1. Install avm CLI</h3>
                  <p className="text-xs sm:text-sm text-slate-400">
                    Select your installation target (macOS Apple Silicon or Intel, Linux x86_64 or arm64, Windows x64):
                  </p>
                </div>
                <TerminalInstallBox onCopy={onCopy} />
                <div className="pt-2 border-t border-slate-800/80 flex items-center justify-between text-xs text-slate-400">
                  <span>Verify installation in your terminal:</span>
                  <code className="text-emerald-400 font-mono bg-slate-950 px-2 py-0.5 rounded border border-slate-800">
                    avm --version
                  </code>
                </div>
              </div>

              {/* Quickstart 5 commands */}
              <div className="rounded-2xl bg-slate-900/40 border border-slate-800 p-6 sm:p-8 space-y-4">
                <h3 className="text-xl font-bold text-white">2. Quickstart in 60 Seconds</h3>
                <p className="text-xs sm:text-sm text-slate-300">
                  Run these 5 simple commands inside any directory to pin tools and configure aliases:
                </p>

                <div className="relative p-5 rounded-xl bg-slate-950 border border-slate-800 font-mono text-xs text-slate-300 space-y-3">
                  <div>
                    <span className="text-slate-500"># 1. Initialize .avm.json in current directory</span>
                    <div className="text-emerald-400 font-bold">$ avm init</div>
                  </div>
                  <div>
                    <span className="text-slate-500"># 2. Add an official runtime provider (e.g. Node.js)</span>
                    <div className="text-emerald-400 font-bold">$ avm plugin add node</div>
                  </div>
                  <div>
                    <span className="text-slate-500"># 3. Download and install a runtime version</span>
                    <div className="text-emerald-400 font-bold">$ avm node install 22.14.0</div>
                  </div>
                  <div>
                    <span className="text-slate-500"># 4. Pin version for this project directory</span>
                    <div className="text-emerald-400 font-bold">$ avm node use 22.14.0</div>
                  </div>
                  <div>
                    <span className="text-slate-500"># 5. Add a project alias</span>
                    <div className="text-emerald-400 font-bold">$ avm alias add dev "pnpm run dev"</div>
                  </div>

                  <button
                    onClick={() => onCopy('avm init\navm plugin add node\navm node install 22.14.0\navm node use 22.14.0\navm alias add dev "pnpm run dev"')}
                    className="absolute top-4 right-4 p-2 rounded-lg bg-slate-800 hover:bg-slate-700 text-slate-300"
                    title="Copy all"
                  >
                    <Copy className="w-4 h-4" />
                  </button>
                </div>
              </div>
            </div>

          ) : activeSection === 'shell-setup' ? (

            /* 4. Guide: Shell Setup */
            <div className="space-y-10">
              <div>
                <h1 className="text-3xl sm:text-4xl font-extrabold text-white tracking-tight mb-4">
                  Shell Setup & Shims
                </h1>
                <p className="text-slate-300 text-base leading-relaxed max-w-3xl">
                  Configure your shell to transparently intercept commands via <code className="text-emerald-300 font-mono text-xs bg-slate-950 px-1 py-0.5 rounded">~/.avm/shims</code> without manual PATH management.
                </p>
              </div>

              {/* Supported Shells */}
              <div className="grid grid-cols-1 md:grid-cols-3 gap-4">
                {SHELL_SETUP_GUIDE.map((item) => (
                  <div key={item.shell} className="p-5 rounded-2xl bg-slate-900/60 border border-slate-800 flex flex-col justify-between">
                    <div>
                      <div className="flex items-center justify-between mb-3">
                        <span className="font-mono text-sm font-bold text-emerald-400 uppercase">{item.shell}</span>
                        <span className="font-mono text-xs text-slate-500">{item.file}</span>
                      </div>
                      <pre className="font-mono text-xs text-slate-300 mb-4 whitespace-pre-wrap bg-slate-950 p-3 rounded-xl border border-slate-800/80">
                        {item.snippet}
                      </pre>
                    </div>
                    <button
                      onClick={() => onCopy(item.snippet)}
                      className="w-full py-2 rounded-xl bg-slate-800 hover:bg-slate-700 text-slate-200 text-xs font-mono flex items-center justify-center gap-1.5 transition-colors"
                    >
                      <Copy className="w-3.5 h-3.5" />
                      <span>Copy Hook Script</span>
                    </button>
                  </div>
                ))}
              </div>
            </div>

          ) : activeSection === 'usage-core' ? (

            /* 5. Usage: Core */
            <div className="space-y-10">
              <div>
                <h1 className="text-3xl sm:text-4xl font-extrabold text-white tracking-tight mb-4">
                  Core Usage & Precedence
                </h1>
                <p className="text-slate-300 text-base leading-relaxed max-w-3xl">
                  Understand how avm discovers configuration files, walks directory hierarchies, and safely falls back to host system binaries.
                </p>
              </div>

              <div className="rounded-2xl bg-slate-900/60 border border-slate-800 p-6 sm:p-8 space-y-4">
                <h3 className="text-xl font-bold text-white">Hierarchical Precedence</h3>
                <div className="space-y-3 text-xs sm:text-sm">
                  <div className="p-4 rounded-xl bg-slate-950 border border-slate-800 flex items-start gap-3">
                    <span className="w-6 h-6 rounded bg-emerald-500 text-slate-950 font-bold flex items-center justify-center font-mono shrink-0">1</span>
                    <div>
                      <strong className="text-white block mb-0.5">Local Project Pin (./.avm.json)</strong>
                      <span className="text-slate-400">Takes absolute priority when running commands from inside that directory tree.</span>
                    </div>
                  </div>
                  <div className="p-4 rounded-xl bg-slate-950 border border-slate-800 flex items-start gap-3">
                    <span className="w-6 h-6 rounded bg-slate-800 text-emerald-400 font-bold flex items-center justify-center font-mono shrink-0">2</span>
                    <div>
                      <strong className="text-white block mb-0.5">Global Machine Fallback (~/.avm.json)</strong>
                      <span className="text-slate-400">Provides developer defaults when a directory has no pinned tool or alias.</span>
                    </div>
                  </div>
                  <div className="p-4 rounded-xl bg-slate-950 border border-slate-800 flex items-start gap-3">
                    <span className="w-6 h-6 rounded bg-slate-800 text-slate-400 font-bold flex items-center justify-center font-mono shrink-0">3</span>
                    <div>
                      <strong className="text-white block mb-0.5">Host System Tool with Warning</strong>
                      <span className="text-slate-400">If a requested version is missing, avm falls back to host binary instead of failing silently.</span>
                    </div>
                  </div>
                </div>
              </div>
            </div>

          ) : activeSection === 'usage-plugins' ? (

            /* 6. Usage: Plugins */
            <div className="space-y-10">
              <div>
                <h1 className="text-3xl sm:text-4xl font-extrabold text-white tracking-tight mb-4">
                  Managing Plugins
                </h1>
                <p className="text-slate-300 text-base leading-relaxed max-w-3xl">
                  Commands for installing, listing, updating, and removing plugins in the avm ecosystem.
                </p>
              </div>

              <div className="grid grid-cols-1 sm:grid-cols-2 gap-4 font-mono text-xs">
                <div className="p-5 rounded-2xl bg-slate-900/60 border border-slate-800 space-y-2">
                  <div className="text-emerald-400 font-bold">avm plugin add &lt;name&gt;</div>
                  <div className="text-slate-400 font-sans text-xs">Downloads precompiled binary from marketplace registry and registers ToolProvider.</div>
                  <pre className="p-2.5 rounded bg-slate-950 text-slate-300 mt-2">$ avm plugin add node</pre>
                </div>
                <div className="p-5 rounded-2xl bg-slate-900/60 border border-slate-800 space-y-2">
                  <div className="text-emerald-400 font-bold">avm plugin list</div>
                  <div className="text-slate-400 font-sans text-xs">Lists installed plugins in ~/.avm/plugins with their versions and manifests.</div>
                  <pre className="p-2.5 rounded bg-slate-950 text-slate-300 mt-2">$ avm plugin list</pre>
                </div>
                <div className="p-5 rounded-2xl bg-slate-900/60 border border-slate-800 space-y-2">
                  <div className="text-emerald-400 font-bold">avm plugin update &lt;name&gt;</div>
                  <div className="text-slate-400 font-sans text-xs">Checks upstream GitHub releases for the latest plugin binary and updates it.</div>
                  <pre className="p-2.5 rounded bg-slate-950 text-slate-300 mt-2">$ avm plugin update java</pre>
                </div>
                <div className="p-5 rounded-2xl bg-slate-900/60 border border-slate-800 space-y-2">
                  <div className="text-emerald-400 font-bold">avm plugin remove &lt;name&gt;</div>
                  <div className="text-slate-400 font-sans text-xs">Removes provider binary and cleans up shims.</div>
                  <pre className="p-2.5 rounded bg-slate-950 text-slate-300 mt-2">$ avm plugin remove ruby</pre>
                </div>
              </div>
            </div>

          ) : activeSection === 'usage-versions' ? (

            /* 7. Usage: Versions */
            <div className="space-y-10">
              <div>
                <h1 className="text-3xl sm:text-4xl font-extrabold text-white tracking-tight mb-4">
                  Managing Tool Versions
                </h1>
                <p className="text-slate-300 text-base leading-relaxed max-w-3xl">
                  Install, switch, and pin multiple versions per language or SDK across project directories.
                </p>
              </div>

              <div className="rounded-2xl bg-slate-900/60 border border-slate-800 p-6 sm:p-8 space-y-6 font-mono text-xs">
                <div>
                  <span className="text-emerald-400 font-bold text-sm block mb-1">1. Browse Available Releases</span>
                  <p className="text-slate-400 font-sans text-xs mb-2">Queries upstream indices directly without cloning heavy repositories:</p>
                  <pre className="p-3 rounded-xl bg-slate-950 text-slate-300 border border-slate-800">$ avm node versions\n$ avm java versions\n$ avm android versions</pre>
                </div>

                <div>
                  <span className="text-emerald-400 font-bold text-sm block mb-1">2. Install Exact or Major Versions</span>
                  <p className="text-slate-400 font-sans text-xs mb-2">Fetches official prebuilt distributions:</p>
                  <pre className="p-3 rounded-xl bg-slate-950 text-slate-300 border border-slate-800">$ avm node install 22.14.0\n$ avm java install 17\n$ avm android install 34</pre>
                </div>

                <div>
                  <span className="text-emerald-400 font-bold text-sm block mb-1">3. Pin Local vs Global</span>
                  <pre className="p-3 rounded-xl bg-slate-950 text-slate-300 border border-slate-800"># Pins in ./.avm.json for the current repository:\n$ avm node use 22.14.0\n\n# Pins machine default in ~/.avm.json:\n$ avm node use 22.14.0 --global</pre>
                </div>
              </div>
            </div>

          ) : activeSection === 'usage-aliases' ? (

            /* 8. Usage: Aliases & Scripts */
            <div className="space-y-10">
              <div>
                <h1 className="text-3xl sm:text-4xl font-extrabold text-white tracking-tight mb-4">
                  Aliases & Scripts Discovery
                </h1>
                <p className="text-slate-300 text-base leading-relaxed max-w-3xl">
                  Eliminate command drift. Anchor project aliases inside <code className="text-emerald-300 font-mono text-xs bg-slate-950 px-1 py-0.5 rounded">.avm.json</code> and automatically discover scripts from <code className="text-emerald-300 font-mono text-xs bg-slate-950 px-1 py-0.5 rounded">package.json</code>.
                </p>
              </div>

              <div className="grid grid-cols-1 md:grid-cols-2 gap-4 text-xs font-mono">
                <div className="p-5 rounded-2xl bg-slate-900/60 border border-slate-800 space-y-3">
                  <h4 className="text-emerald-400 font-bold text-sm font-sans">Explicit Project Aliases</h4>
                  <pre className="p-3 rounded-xl bg-slate-950 text-slate-300">$ avm alias add dev "pnpm run dev --port 3000"\n$ avm alias add release "npm run release $1"</pre>
                  <p className="text-slate-400 font-sans text-xs">
                    Supports parameter forwarding ($1, $@) and runs with injected tools and env vars.
                  </p>
                </div>

                <div className="p-5 rounded-2xl bg-slate-900/60 border border-slate-800 space-y-3">
                  <h4 className="text-emerald-400 font-bold text-sm font-sans">Auto package.json Script Discovery</h4>
                  <pre className="p-3 rounded-xl bg-slate-950 text-slate-300">$ avm test\n$ avm build\n$ avm lint</pre>
                  <p className="text-slate-400 font-sans text-xs">
                    Discovers scripts defined in package.json and executes them using your detected package manager (pnpm, npm, yarn, bun).
                  </p>
                </div>
              </div>
            </div>

          ) : activeSection === 'version-files' ? (

            /* Usage: Version Files */
            <div className="space-y-10">
              <div>
                <h1 className="text-3xl sm:text-4xl font-extrabold text-white tracking-tight mb-4">
                  Version Files
                </h1>
                <p className="text-slate-300 text-base leading-relaxed max-w-3xl">
                  avm reads the version files your projects already have, so you can try it without rewriting any config.
                </p>
              </div>

              <div className="rounded-2xl bg-slate-900/50 border border-slate-800 p-6 sm:p-8 space-y-4 text-sm text-slate-300">
                <h3 className="text-xl font-bold text-white">Precedence (nearest directory wins)</h3>
                <ol className="list-decimal pl-5 space-y-1.5">
                  <li><code className="text-emerald-300 font-mono text-xs bg-slate-950 px-1 py-0.5 rounded">.avm.json</code> <code className="text-emerald-300 font-mono text-xs bg-slate-950 px-1 py-0.5 rounded">tools</code> in the current directory</li>
                  <li><code className="text-emerald-300 font-mono text-xs bg-slate-950 px-1 py-0.5 rounded">.tool-versions</code> (asdf format; <code className="text-emerald-300 font-mono text-xs bg-slate-950 px-1 py-0.5 rounded">nodejs</code> maps to <code className="text-emerald-300 font-mono text-xs bg-slate-950 px-1 py-0.5 rounded">node</code>)</li>
                  <li><code className="text-emerald-300 font-mono text-xs bg-slate-950 px-1 py-0.5 rounded">.nvmrc</code>, <code className="text-emerald-300 font-mono text-xs bg-slate-950 px-1 py-0.5 rounded">.node-version</code>, <code className="text-emerald-300 font-mono text-xs bg-slate-950 px-1 py-0.5 rounded">package.json</code> (<code className="text-emerald-300 font-mono text-xs bg-slate-950 px-1 py-0.5 rounded">volta.node</code>, else the <code className="text-emerald-300 font-mono text-xs bg-slate-950 px-1 py-0.5 rounded">engines.node</code> range), <code className="text-emerald-300 font-mono text-xs bg-slate-950 px-1 py-0.5 rounded">.java-version</code>, <code className="text-emerald-300 font-mono text-xs bg-slate-950 px-1 py-0.5 rounded">.sdkmanrc</code></li>
                  <li>Global <code className="text-emerald-300 font-mono text-xs bg-slate-950 px-1 py-0.5 rounded">~/.avm.json</code></li>
                </ol>
                <p>Version files are searched upward from the current directory. Partial specs resolve to the newest <em>installed</em> match:</p>
                <div className="p-4 rounded-xl bg-slate-950 border border-slate-800 font-mono text-xs space-y-1">
                  <div><span className="text-slate-500"># .nvmrc: 20</span> <span className="text-slate-400">→ 20.11.1</span></div>
                  <div><span className="text-slate-500"># .nvmrc: lts/iron, lts/*</span> <span className="text-slate-400">→ newest installed 20.x / newest LTS</span></div>
                  <div><span className="text-slate-500"># package.json engines: &gt;=18 &lt;21, ^20.1, 18 || 20</span> <span className="text-slate-400">→ newest installed match</span></div>
                  <div><span className="text-slate-500"># .java-version: 17</span> <span className="text-slate-400">→ openjdk-17.0.9+9</span></div>
                </div>
                <p>See which file won with <code className="text-emerald-300 font-mono text-xs bg-slate-950 px-1 py-0.5 rounded">avm which</code>:</p>
                <pre className="p-4 rounded-xl bg-slate-950 border border-slate-800 font-mono text-xs text-slate-300">{`$ avm which node
tool 'node': 20.11.1 (from ./.nvmrc)`}</pre>
                <p>To ignore the tool-specific files (item 3), set <code className="text-emerald-300 font-mono text-xs bg-slate-950 px-1 py-0.5 rounded">"idiomatic_version_files": false</code> in <code className="text-emerald-300 font-mono text-xs bg-slate-950 px-1 py-0.5 rounded">~/.avm.json</code>.</p>
              </div>
            </div>

          ) : activeSection === 'security' ? (

            /* Usage: Security & Trust */
            <div className="space-y-10">
              <div>
                <h1 className="text-3xl sm:text-4xl font-extrabold text-white tracking-tight mb-4">
                  Security &amp; Trust
                </h1>
                <p className="text-slate-300 text-base leading-relaxed max-w-3xl">
                  Cloning a repository never runs its config, and every binary avm downloads is verified before it's unpacked.
                </p>
              </div>

              <div className="rounded-2xl bg-slate-900/50 border border-slate-800 p-6 sm:p-8 space-y-4 text-sm text-slate-300">
                <h3 className="text-xl font-bold text-white">Trusting a project</h3>
                <p>
                  A project's <code className="text-emerald-300 font-mono text-xs bg-slate-950 px-1 py-0.5 rounded">.avm.json</code> aliases and <code className="text-emerald-300 font-mono text-xs bg-slate-950 px-1 py-0.5 rounded">env</code>, and its <code className="text-emerald-300 font-mono text-xs bg-slate-950 px-1 py-0.5 rounded">.env</code> files, run with your privileges and reach every shimmed <code className="text-emerald-300 font-mono text-xs bg-slate-950 px-1 py-0.5 rounded">node</code>/<code className="text-emerald-300 font-mono text-xs bg-slate-950 px-1 py-0.5 rounded">java</code>. So avm ignores them until you trust that exact file content. Tool pins only pick a version and always apply.
                </p>
                <pre className="p-4 rounded-xl bg-slate-950 border border-slate-800 font-mono text-xs text-slate-300">{`$ cd cloned-repo
$ avm trust            # shows the aliases/env it enables, then trusts them
$ avm trust --list     # everything you've trusted
$ avm trust --revoke   # stop trusting this directory`}</pre>
                <ul className="space-y-1.5 list-disc pl-5">
                  <li>Trust is stored as path &rarr; sha256 in <code className="text-emerald-300 font-mono text-xs bg-slate-950 px-1 py-0.5 rounded">~/.avm/trusted.json</code>. Any edit made outside avm blocks the file again; <code className="text-emerald-300 font-mono text-xs bg-slate-950 px-1 py-0.5 rounded">avm init</code>, <code className="text-emerald-300 font-mono text-xs bg-slate-950 px-1 py-0.5 rounded">avm add</code> and <code className="text-emerald-300 font-mono text-xs bg-slate-950 px-1 py-0.5 rounded">avm env add</code> keep a trusted file trusted.</li>
                  <li>Your global <code className="text-emerald-300 font-mono text-xs bg-slate-950 px-1 py-0.5 rounded">~/.avm.json</code> is always trusted and can list <code className="text-emerald-300 font-mono text-xs bg-slate-950 px-1 py-0.5 rounded">"trusted_paths": ["~/work/**"]</code>.</li>
                  <li><code className="text-emerald-300 font-mono text-xs bg-slate-950 px-1 py-0.5 rounded">AVM_TRUST_ALL=1</code> trusts everything, for CI. It's never implied by <code className="text-emerald-300 font-mono text-xs bg-slate-950 px-1 py-0.5 rounded">CI=true</code>.</li>
                </ul>
              </div>

              <div className="rounded-2xl bg-slate-900/50 border border-slate-800 p-6 sm:p-8 space-y-4 text-sm text-slate-300">
                <h3 className="text-xl font-bold text-white">Verified downloads</h3>
                <ul className="space-y-1.5 list-disc pl-5">
                  <li><strong className="text-white">Plugins:</strong> <code className="text-emerald-300 font-mono text-xs bg-slate-950 px-1 py-0.5 rounded">avm plugin add</code> checks the archive against the release's <code className="text-emerald-300 font-mono text-xs bg-slate-950 px-1 py-0.5 rounded">checksums.txt</code> before extracting. A mismatch aborts with nothing installed.</li>
                  <li><strong className="text-white">Runtimes:</strong> Node.js is checked against <code className="text-emerald-300 font-mono text-xs bg-slate-950 px-1 py-0.5 rounded">SHASUMS256.txt</code>, Temurin JDKs against foojay's sha256, and the Android cmdline-tools zip against a pinned sha256 (<code className="text-emerald-300 font-mono text-xs bg-slate-950 px-1 py-0.5 rounded">sdkmanager</code> verifies the rest).</li>
                  <li><strong className="text-white">avm itself:</strong> <code className="text-emerald-300 font-mono text-xs bg-slate-950 px-1 py-0.5 rounded">install.sh</code>, <code className="text-emerald-300 font-mono text-xs bg-slate-950 px-1 py-0.5 rounded">install.ps1</code> and the npm installer verify <code className="text-emerald-300 font-mono text-xs bg-slate-950 px-1 py-0.5 rounded">avm-bin</code> the same way.</li>
                  <li><strong className="text-white">Provenance:</strong> every release carries a GitHub build attestation. Check it with <code className="text-emerald-300 font-mono text-xs bg-slate-950 px-1 py-0.5 rounded">gh attestation verify &lt;archive&gt; --owner PrajaNova</code>.</li>
                  <li><code className="text-emerald-300 font-mono text-xs bg-slate-950 px-1 py-0.5 rounded">AVM_ALLOW_UNVERIFIED=1</code> is the only way to skip verification.</li>
                </ul>
              </div>
            </div>

          ) : activeSection === 'configuration' ? (

            /* 9. Reference: Configuration */
            <div className="space-y-10">
              <div>
                <h1 className="text-3xl sm:text-4xl font-extrabold text-white tracking-tight mb-4">
                  Configuration (.avm.json)
                </h1>
                <p className="text-slate-300 text-base leading-relaxed max-w-3xl">
                  Complete JSON specification for the <code className="text-emerald-300 font-mono text-xs bg-slate-950 px-1 py-0.5 rounded">.avm.json</code> file.
                </p>
              </div>

              <div className="relative rounded-2xl bg-slate-950 border border-slate-800 p-5 font-mono text-xs text-slate-300 overflow-x-auto shadow-xl">
<pre>{`{
  // Project-specific aliases with injected environment variables
  "aliases": {
    "dev": "pnpm run dev",
    "release": "npm run release $1",
    "build:apk": "./gradlew assembleRelease"
  },
  // Injected environment variables for this workspace
  "env": {
    "NODE_ENV": "development",
    "API_URL": "https://api.internal.dev"
  },
  // Pinned tool versions (intercepted via shims or avm commands)
  "tools": {
    "node": "22.14.0",
    "java": "17.0.13+11",
    "android": "34"
  }
}`}</pre>
                <button
                  onClick={() => onCopy(`{\n  "aliases": {\n    "dev": "pnpm run dev"\n  },\n  "tools": {\n    "node": "22.14.0"\n  }\n}`)}
                  className="absolute top-4 right-4 p-2 rounded-lg bg-slate-800 hover:bg-slate-700 text-slate-300"
                >
                  <Copy className="w-4 h-4" />
                </button>
              </div>
            </div>

          ) : activeSection === 'commands' ? (

            /* 10. Reference: All Commands */
            <CliReference onCopy={onCopy} />

          ) : activeSection === 'dependencies' ? (

            /* 11. Reference: Dependencies */
            <div className="space-y-10">
              <div>
                <h1 className="text-3xl sm:text-4xl font-extrabold text-white tracking-tight mb-4">
                  Dependencies & Host Environment
                </h1>
                <p className="text-slate-300 text-base leading-relaxed max-w-3xl">
                  avm is compiled directly to machine code with zero external runtime dependencies.
                </p>
              </div>

              <div className="grid grid-cols-1 md:grid-cols-2 gap-4 text-xs font-sans">
                <div className="p-5 rounded-2xl bg-slate-900/60 border border-slate-800 space-y-2">
                  <h4 className="font-bold text-white text-sm">Supported Operating Systems</h4>
                  <ul className="space-y-1 text-slate-300 font-mono text-xs">
                    <li>&bull; macOS (Apple Silicon arm64, Intel x86_64)</li>
                    <li>&bull; Linux (x86_64, aarch64; glibc)</li>
                    <li>&bull; Windows x64 via PowerShell (phase 1; Windows plugins coming)</li>
                  </ul>
                </div>
                <div className="p-5 rounded-2xl bg-slate-900/60 border border-slate-800 space-y-2">
                  <h4 className="font-bold text-white text-sm">Runtime Dependencies</h4>
                  <p className="text-slate-400 text-xs">
                    <strong className="text-emerald-400">Zero dependencies.</strong> No Node.js, Python, Ruby, or GCC is required to run the core <code className="font-mono text-emerald-300">avm-bin</code>.
                  </p>
                </div>
              </div>
            </div>

          ) : activeSection === 'architecture' ? (

            /* 12. Reference: Architecture */
            <div className="space-y-10">
              <div>
                <h1 className="text-3xl sm:text-4xl font-extrabold text-white tracking-tight mb-4">
                  Architecture & Wire Protocol
                </h1>
                <p className="text-slate-300 text-base leading-relaxed max-w-3xl">
                  avm isolates plugins as standalone processes communicating over standard input/output using a strongly typed JSON wire protocol.
                </p>
              </div>

              <div className="p-6 rounded-2xl bg-slate-900/60 border border-slate-800 space-y-4">
                <h3 className="text-lg font-bold text-white flex items-center gap-2">
                  <Cpu className="w-5 h-5 text-emerald-400" />
                  <span>Typed Wire Protocol Commands</span>
                </h3>
                <div className="grid grid-cols-1 md:grid-cols-2 gap-4 text-xs font-mono">
                  <div className="p-4 rounded-xl bg-slate-950 border border-slate-800 space-y-1.5">
                    <span className="text-emerald-400 font-bold uppercase tracking-wider block mb-1">Queries (Read)</span>
                    <div className="text-slate-400"># Metadata:</div>
                    <div className="text-slate-200">avm-plugin-&lt;name&gt; manifest</div>
                    <div className="text-slate-400 pt-1"># Versions:</div>
                    <div className="text-slate-200">avm-plugin-&lt;name&gt; versions</div>
                    <div className="text-slate-400 pt-1"># Env exports:</div>
                    <div className="text-slate-200">avm-plugin-&lt;name&gt; env-vars &lt;version&gt;</div>
                  </div>
                  <div className="p-4 rounded-xl bg-slate-950 border border-slate-800 space-y-1.5">
                    <span className="text-emerald-400 font-bold uppercase tracking-wider block mb-1">Lifecycle (Write)</span>
                    <div className="text-slate-400"># Install:</div>
                    <div className="text-slate-200">avm-plugin-&lt;name&gt; install &lt;version&gt;</div>
                    <div className="text-slate-400 pt-1"># Uninstall:</div>
                    <div className="text-slate-200">avm-plugin-&lt;name&gt; uninstall &lt;version&gt;</div>
                    <div className="text-slate-400 pt-1"># Executable:</div>
                    <div className="text-slate-200">avm-plugin-&lt;name&gt; executable-path &lt;version&gt;</div>
                  </div>
                </div>
              </div>
            </div>

          ) : activeSection === 'comparison' ? (

            /* 13. Reference: Comparison Matrix */
            <div className="space-y-10">
              <div>
                <h1 className="text-3xl sm:text-4xl font-extrabold text-white tracking-tight mb-4">
                  Comparison Matrix
                </h1>
                <p className="text-slate-300 text-base leading-relaxed max-w-3xl">
                  Side-by-side comparison of avm against asdf, vfox, mise, and proto, checked against each project's docs in September 2026. "Not documented" means we couldn't find it, not that it's confirmed absent.
                </p>
              </div>

              <ComparisonTable />
            </div>

          ) : activeSection === 'faq' ? (

            /* 16. Questions: FAQ */
            <div className="space-y-10">
              <div>
                <h1 className="text-3xl sm:text-4xl font-extrabold text-white tracking-tight mb-4">
                  Frequently Asked Questions
                </h1>
                <p className="text-slate-300 text-base leading-relaxed max-w-3xl">
                  Answers to common questions regarding avm's runtime model, performance, and compatibility.
                </p>
              </div>

              <div className="space-y-4 text-xs sm:text-sm">
                {[
                  { q: 'Do I need to rewrite my .nvmrc or .tool-versions?', a: 'No. avm reads .tool-versions, .nvmrc, .node-version, package.json engines/volta, .java-version and .sdkmanrc directly. avm which <tool> shows which file a version came from.' },
                  { q: "Why doesn't my project's alias run after I clone it?", a: "A project's aliases, env and .env files are ignored until you run avm trust in that directory. It shows what you're enabling first. Editing the file outside avm blocks it again." },
                  { q: 'Can I use globally installed packages across different local version pins?', a: 'Yes! avm supports global package sharing, meaning globally installed tools (like npm -g packages) remain reachable even when a directory pins a different version.' },
                  { q: 'Where are plugins downloaded and installed?', a: 'Plugins are stored in ~/.avm/plugins/avm-plugin-<name>, while downloaded runtimes reside in ~/.avm/tools/<name>/<version>.' },
                  { q: 'What happens if a requested tool version is not installed?', a: 'avm gracefully prints a clear diagnostic notice and executes your host system binary fallback if available, rather than abruptly aborting.' },
                ].map((item, idx) => (
                  <div key={idx} className="p-5 rounded-2xl bg-slate-900/60 border border-slate-800">
                    <h4 className="font-bold text-white text-base mb-2">{item.q}</h4>
                    <p className="text-slate-300 leading-relaxed text-xs sm:text-sm">{item.a}</p>
                  </div>
                ))}
              </div>
            </div>

          ) : activeSection === 'troubleshooting' ? (

            /* 17. Questions: Troubleshooting */
            <div className="space-y-10">
              <div>
                <h1 className="text-3xl sm:text-4xl font-extrabold text-white tracking-tight mb-4">
                  Troubleshooting & Diagnostics
                </h1>
                <p className="text-slate-300 text-base leading-relaxed max-w-3xl">
                  Diagnostic steps to resolve PATH ordering, shell hook evaluation, and emulator permissions.
                </p>
              </div>

              <div className="space-y-4 font-mono text-xs">
                <div className="p-5 rounded-2xl bg-slate-900/60 border border-slate-800 space-y-2">
                  <h4 className="font-bold text-white font-sans text-sm">1. Check Active Environment Variables</h4>
                  <pre className="p-3 rounded-xl bg-slate-950 text-emerald-300">$ avm env</pre>
                  <p className="text-slate-400 font-sans text-xs">Prints exact exported variables including JAVA_HOME, ANDROID_HOME, and PATH order.</p>
                </div>
                <div className="p-5 rounded-2xl bg-slate-900/60 border border-slate-800 space-y-2">
                  <h4 className="font-bold text-white font-sans text-sm">2. Verify Shim Resolution</h4>
                  <pre className="p-3 rounded-xl bg-slate-950 text-emerald-300">$ which node\n$ which java\n$ which adb</pre>
                  <p className="text-slate-400 font-sans text-xs">Should point to ~/.avm/shims/&lt;binary&gt;.</p>
                </div>
              </div>
            </div>

          ) : activeSection === 'contribute-core' ? (

            /* 18. Contribute: Core */
            <div className="space-y-10">
              <div>
                <h1 className="text-3xl sm:text-4xl font-extrabold text-white tracking-tight mb-4">
                  Contributing to avm Core
                </h1>
                <p className="text-slate-300 text-base leading-relaxed max-w-3xl">
                  We welcome contributions! avm is completely open source under the MIT license.
                </p>
              </div>

              <div className="p-6 rounded-2xl bg-slate-900/60 border border-slate-800 space-y-4 font-mono text-xs">
                <span className="text-white font-bold text-sm font-sans">Build from Source</span>
                <pre className="p-4 rounded-xl bg-slate-950 text-slate-300 border border-slate-800">$ git clone https://github.com/PrajaNova/avm.git\n$ cd avm\n$ cargo check\n$ cargo test\n$ cargo build --release</pre>
              </div>
            </div>

          ) : (

            /* 17. Contribute: Docs */
            <div className="space-y-10">
              <div>
                <h1 className="text-3xl sm:text-4xl font-extrabold text-white tracking-tight mb-4">
                  Contributing Documentation
                </h1>
                <p className="text-slate-300 text-base leading-relaxed max-w-3xl">
                  Help improve our guides, shell integration instructions, and troubleshooting tips.
                </p>
              </div>

              <div className="p-6 rounded-2xl bg-slate-900/60 border border-slate-800 space-y-4">
                <h4 className="text-base font-bold text-white">Submitting Documentation Changes</h4>
                <p className="text-xs sm:text-sm text-slate-300 leading-relaxed">
                  Fork <a href="https://github.com/PrajaNova/avm" target="_blank" rel="noreferrer" className="text-emerald-400 underline">PrajaNova/avm</a>, improve the documentation or guide files, and open a Pull Request!
                </p>
              </div>
            </div>

          )}

          {/* Previous / Next Page Navigation Cards (like asdf-vm/VitePress) */}
          <div className="mt-16 pt-8 border-t border-slate-800/80 flex flex-col sm:flex-row items-stretch sm:items-center justify-between gap-4">
            {prevPage ? (
              <button
                onClick={() => navigate(prevPage.link)}
                className="flex-1 flex flex-col items-start p-4 rounded-xl bg-slate-900/50 hover:bg-slate-900 border border-slate-800 hover:border-slate-700 transition-all group text-left"
              >
                <div className="flex items-center gap-1.5 text-[11px] font-mono text-slate-400 group-hover:text-emerald-400 mb-1">
                  <ArrowLeft className="w-3.5 h-3.5 group-hover:-translate-x-1 transition-transform" />
                  <span>Previous page</span>
                </div>
                <span className="text-sm font-bold text-white group-hover:text-emerald-300 transition-colors">
                  {prevPage.text}
                </span>
              </button>
            ) : (
              <div className="flex-1" />
            )}

            {nextPage && (
              <button
                onClick={() => navigate(nextPage.link)}
                className="flex-1 flex flex-col items-end p-4 rounded-xl bg-slate-900/50 hover:bg-slate-900 border border-slate-800 hover:border-slate-700 transition-all group text-right"
              >
                <div className="flex items-center gap-1.5 text-[11px] font-mono text-slate-400 group-hover:text-emerald-400 mb-1">
                  <span>Next page</span>
                  <ArrowRight className="w-3.5 h-3.5 group-hover:translate-x-1 transition-transform" />
                </div>
                <span className="text-sm font-bold text-white group-hover:text-emerald-300 transition-colors">
                  {nextPage.text}
                </span>
              </button>
            )}
          </div>

        </main>
      </div>

    </div>
  );
};
