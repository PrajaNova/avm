import React, { useState } from 'react';
import { useLocation, useNavigate } from 'react-router';
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
import { TerminalInstallBox } from '../components/TerminalInstallBox';
import { CliReference } from '../components/CliReference';
import { ComparisonTable } from '../components/ComparisonTable';
import { FeatureGuideView } from '../components/docs/FeatureGuideView';
import { FEATURE_GUIDES } from '../data/featureGuides';
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

  const currentPage = ALL_PAGES.find((p) => p.link === location.pathname.replace(/(.)\/$/, '$1')) || ALL_PAGES[0];
  const activeSection = currentPage.id;
  const pageIndex = ALL_PAGES.indexOf(currentPage);
  const prevPage = pageIndex > 0 ? ALL_PAGES[pageIndex - 1] : null;
  const nextPage = pageIndex < ALL_PAGES.length - 1 ? ALL_PAGES[pageIndex + 1] : null;

  return (
    <div className="min-h-screen bg-page text-ink flex flex-col antialiased">
      
      <a href="#docs-content" onClick={(event) => { event.preventDefault(); document.getElementById('docs-content')?.focus(); }} className="sr-only focus:not-sr-only focus:p-4 focus:text-accent">Skip documentation navigation</a>
      {/* Mobile Bar */}
      <div className="lg:hidden sticky top-14 z-30 bg-page/95 backdrop-blur-md border-b border-line px-4 py-3 flex items-center justify-between">
        <button
          onClick={() => setIsMobileMenuOpen(true)}
          className="flex items-center gap-2 px-3 py-1.5 rounded-lg bg-surface border border-line text-xs font-mono text-secondary hover:text-ink"
        >
          <Menu className="w-4 h-4 text-accent" />
          <span>Documentation Menu</span>
        </button>

        <span className="text-xs font-mono font-medium text-accent truncate max-w-[200px]">
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
        <div id="docs-content" tabIndex={-1} className="flex-1 min-w-0 pb-16">
          
          {/* Breadcrumb Header */}
          <div className="flex items-center gap-2 text-xs font-mono text-secondary mb-6 pb-2 border-b border-line/60">
            <span>Docs</span>
            <span>/</span>
            <span className="text-secondary">{currentPage.group}</span>
            <span>/</span>
            <span className="text-accent font-semibold">{currentPage.text}</span>
          </div>

          {/* SECTION ROUTING */}

          {FEATURE_GUIDES[activeSection] ? (
            <FeatureGuideView guide={FEATURE_GUIDES[activeSection]} onCopy={onCopy} />
          ) : activeSection === 'introduction' ? (
            
            /* 2. Guide: What is avm? */
            <div className="space-y-10">
              <div>
                <h1 className="text-3xl sm:text-4xl font-semibold text-ink tracking-tight mb-4">
                  What is avm?
                </h1>
                <p className="text-secondary text-base leading-relaxed max-w-3xl">
                  <strong className="text-ink">avm</strong> is an ultra-fast, native CLI version manager and project orchestrator. 
                  It manages multiple runtime versions (like Node.js, OpenJDK Temurin, Android SDK) and anchors directory-scoped command aliases and environment variables in a single version-controlled configuration.
                </p>
              </div>

              <nav aria-label="Feature guides" className="flex flex-wrap gap-3 text-sm">
                {Object.entries(FEATURE_GUIDES).map(([id, guide]) => (
                  <button key={id} onClick={() => navigate(`/docs/manage/${id}`)} className="rounded-lg border border-line px-3 py-2 text-accent hover:bg-surface focus-visible:outline focus-visible:outline-accent">{guide.title}</button>
                ))}
              </nav>

              {/* Motivation */}
              <div className="rounded-lg bg-surface/50 border border-line p-6 sm:p-8 space-y-4">
                <h3 className="text-xl font-bold text-ink">Why avm?</h3>
                <p className="text-sm text-secondary leading-relaxed">
                  Version managers like <em>nvm</em> and <em>rbenv</em> rely on shell scripts, and most tools split their state across fragmented configuration files (<code className="text-accent font-mono text-xs bg-page px-1 py-0.5 rounded">.tool-versions</code>, <code className="text-accent font-mono text-xs bg-page px-1 py-0.5 rounded">.nvmrc</code>, <code className="text-accent font-mono text-xs bg-page px-1 py-0.5 rounded">.ruby-version</code>).
                </p>
                <div className="grid grid-cols-1 md:grid-cols-2 gap-4 pt-2 text-xs">
                  <div className="p-4 rounded-lg bg-page border border-line">
                    <span className="text-rose-400 font-bold block mb-1">Traditional Friction</span>
                    <ul className="space-y-1.5 text-secondary">
                      <li>&bull; Slower prompt load times due to shell wrappers</li>
                      <li>&bull; Compiling plugins and runtimes from source</li>
                      <li>&bull; Command drift across engineering teammates</li>
                    </ul>
                  </div>
                  <div className="p-4 rounded-lg bg-page border border-accent/20 bg-accent/10">
                    <span className="text-accent font-bold block mb-1">The avm Approach</span>
                    <ul className="space-y-1.5 text-secondary">
                      <li>&bull; A single compiled Rust binary, with plugins as separate verified executables</li>
                      <li>&bull; Reads your existing <code className="font-mono text-accent">.nvmrc</code> / <code className="font-mono text-accent">.tool-versions</code> as-is</li>
                      <li>&bull; Precompiled standalone binaries with zero compilation</li>
                      <li>&bull; Unified <code className="font-mono text-accent">.avm.json</code> for aliases, tools, and env vars</li>
                    </ul>
                  </div>
                </div>
              </div>

              {/* Core Features */}
              <div className="space-y-4">
                <h3 className="text-xl font-bold text-ink">Core Capabilities</h3>
                <div className="grid grid-cols-1 sm:grid-cols-3 gap-4">
                  <div className="p-5 rounded-lg bg-surface/60 border border-line">
                    <Package className="w-5 h-5 text-accent mb-2" />
                    <h4 className="font-bold text-ink text-sm mb-1">Plugin Marketplace</h4>
                    <p className="text-xs text-secondary">
                      Native executables fetched on demand directly from GitHub Releases.
                    </p>
                  </div>
                  <div className="p-5 rounded-lg bg-surface/60 border border-line">
                    <Terminal className="w-5 h-5 text-accent mb-2" />
                    <h4 className="font-bold text-ink text-sm mb-1">Static PATH Shims</h4>
                    <p className="text-xs text-secondary">
                      Commands resolve the selected runtime when invoked, including from IDEs.
                    </p>
                  </div>
                  <div className="p-5 rounded-lg bg-surface/60 border border-line">
                    <FileJson className="w-5 h-5 text-accent mb-2" />
                    <h4 className="font-bold text-ink text-sm mb-1">Unified Config</h4>
                    <p className="text-xs text-secondary">
                      A single <code className="text-accent font-mono text-xs">.avm.json</code> file commits your team's workflow to git.
                    </p>
                  </div>
                </div>
              </div>
            </div>

          ) : activeSection === 'getting-started' ? (

            /* 3. Guide: Getting Started */
            <div className="space-y-10">
              <div>
                <h1 className="text-3xl sm:text-4xl font-semibold text-ink tracking-tight mb-4">
                  Getting Started
                </h1>
                <p className="text-secondary text-base leading-relaxed max-w-3xl">
                  Install avm on macOS, Linux, or Windows in seconds. Prebuilt downloads are checksum-verified before installation.
                </p>
              </div>

              {/* Installation Methods */}
              <div className="rounded-lg bg-surface/50 border border-line p-6 sm:p-8 space-y-6">
                <div>
                  <h3 className="text-xl font-bold text-ink mb-1">1. Install avm CLI</h3>
                  <p className="text-xs sm:text-sm text-secondary">
                    Select your installation target (macOS Apple Silicon or Intel, Linux x86_64 or arm64, Windows x64):
                  </p>
                </div>
                <TerminalInstallBox onCopy={onCopy} />
                <div className="pt-2 border-t border-line/80 flex items-center justify-between text-xs text-secondary">
                  <span>Verify installation in your terminal:</span>
                  <code className="text-accent font-mono bg-page px-2 py-0.5 rounded border border-line">
                    avm --version
                  </code>
                </div>
              </div>

              {/* Quickstart 5 commands */}
              <div className="rounded-lg bg-surface/40 border border-line p-6 sm:p-8 space-y-4">
                <h3 className="text-xl font-bold text-ink">2. Quickstart in 60 Seconds</h3>
                <p className="text-xs sm:text-sm text-secondary">
                  Run these 5 simple commands inside any directory to pin tools and configure aliases:
                </p>

                <div className="relative p-5 rounded-lg bg-page border border-line font-mono text-xs text-secondary space-y-3">
                  <div>
                    <span className="text-secondary"># 1. Initialize .avm.json in current directory</span>
                    <div className="text-accent font-bold">$ avm init</div>
                  </div>
                  <div>
                    <span className="text-secondary"># 2. Add an official runtime provider (e.g. Node.js)</span>
                    <div className="text-accent font-bold">$ avm plugin add node</div>
                  </div>
                  <div>
                    <span className="text-secondary"># 3. Download and install a runtime version</span>
                    <div className="text-accent font-bold">$ avm node install 22.14.0</div>
                  </div>
                  <div>
                    <span className="text-secondary"># 4. Pin version for this project directory</span>
                    <div className="text-accent font-bold">$ avm node use 22.14.0</div>
                  </div>
                  <div>
                    <span className="text-secondary"># 5. Add a project alias</span>
                    <div className="text-accent font-bold">$ avm alias add dev "pnpm run dev"</div>
                  </div>

                  <button
                    onClick={() => onCopy('avm init\navm plugin add node\navm node install 22.14.0\navm node use 22.14.0\navm alias add dev "pnpm run dev"')}
                    className="absolute top-4 right-4 p-2 rounded-lg bg-muted hover:bg-muted text-secondary"
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
                <h1 className="text-3xl sm:text-4xl font-semibold text-ink tracking-tight mb-4">
                  Shell Setup & Shims
                </h1>
                <p className="text-secondary text-base leading-relaxed max-w-3xl">
                  Configure your shell to transparently intercept commands via <code className="text-accent font-mono text-xs bg-page px-1 py-0.5 rounded">~/.avm/shims</code> without manual PATH management.
                </p>
              </div>

              {/* Supported Shells */}
              <div className="grid grid-cols-1 md:grid-cols-3 gap-4">
                {SHELL_SETUP_GUIDE.map((item) => (
                  <div key={item.shell} className="p-5 rounded-lg bg-surface/60 border border-line flex flex-col justify-between">
                    <div>
                      <div className="flex items-center justify-between mb-3">
                        <span className="font-mono text-sm font-bold text-accent uppercase">{item.shell}</span>
                        <span className="font-mono text-xs text-secondary">{item.file}</span>
                      </div>
                      <pre className="font-mono text-xs text-secondary mb-4 whitespace-pre-wrap bg-page p-3 rounded-lg border border-line/80 overflow-x-auto">
                        {item.snippet}
                      </pre>
                    </div>
                    <button
                      onClick={() => onCopy(item.snippet)}
                      className="w-full py-2 rounded-lg bg-muted hover:bg-muted text-ink text-xs font-mono flex items-center justify-center gap-1.5 transition-colors"
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
                <h1 className="text-3xl sm:text-4xl font-semibold text-ink tracking-tight mb-4">
                  Core Usage & Precedence
                </h1>
                <p className="text-secondary text-base leading-relaxed max-w-3xl">
                  Understand how avm discovers configuration files, walks directory hierarchies, and safely falls back to host system binaries.
                </p>
              </div>

              <div className="rounded-lg bg-surface/60 border border-line p-6 sm:p-8 space-y-4">
                <h3 className="text-xl font-bold text-ink">Hierarchical Precedence</h3>
                <div className="space-y-3 text-xs sm:text-sm">
                  <div className="p-4 rounded-lg bg-page border border-line flex items-start gap-3">
                    <span className="w-6 h-6 rounded bg-accent text-on-accent font-bold flex items-center justify-center font-mono shrink-0">1</span>
                    <div>
                      <strong className="text-ink block mb-0.5">Merged Project Pins & Version Files</strong>
                      <span className="text-secondary">Parent JSON configs merge into child configs, with profiles and personal overrides. Explicit JSON pins win over version files; valid locks pin exact releases.</span>
                    </div>
                  </div>
                  <div className="p-4 rounded-lg bg-page border border-line flex items-start gap-3">
                    <span className="w-6 h-6 rounded bg-muted text-accent font-bold flex items-center justify-center font-mono shrink-0">2</span>
                    <div>
                      <strong className="text-ink block mb-0.5">Global Machine Fallback (~/.avm.json)</strong>
                      <span className="text-secondary">Provides developer defaults when a directory has no pinned tool or alias.</span>
                    </div>
                  </div>
                  <div className="p-4 rounded-lg bg-page border border-line flex items-start gap-3">
                    <span className="w-6 h-6 rounded bg-muted text-secondary font-bold flex items-center justify-center font-mono shrink-0">3</span>
                    <div>
                      <strong className="text-ink block mb-0.5">Host System Tool with Warning</strong>
                      <span className="text-secondary">If a requested version is missing, avm falls back to host binary instead of failing silently.</span>
                    </div>
                  </div>
                </div>
              </div>
            </div>

          ) : activeSection === 'usage-plugins' ? (

            /* 6. Usage: Plugins */
            <div className="space-y-10">
              <div>
                <h1 className="text-3xl sm:text-4xl font-semibold text-ink tracking-tight mb-4">
                  Managing Plugins
                </h1>
                <p className="text-secondary text-base leading-relaxed max-w-3xl">
                  Commands for installing, listing, updating, and removing plugins in the avm ecosystem.
                </p>
              </div>

              <div className="grid grid-cols-1 sm:grid-cols-2 gap-4 font-mono text-xs">
                <div className="p-5 rounded-lg bg-surface/60 border border-line space-y-2">
                  <div className="text-accent font-bold">avm plugin add &lt;name&gt;</div>
                  <div className="text-secondary font-sans text-xs">Downloads precompiled binary from marketplace registry and registers ToolProvider.</div>
                  <pre className="p-2.5 rounded bg-page text-secondary mt-2 overflow-x-auto">{"$ avm plugin add node"}</pre>
                </div>
                <div className="p-5 rounded-lg bg-surface/60 border border-line space-y-2">
                  <div className="text-accent font-bold">avm plugin list</div>
                  <div className="text-secondary font-sans text-xs">Lists installed plugins in ~/.avm/plugins with their versions and manifests.</div>
                  <pre className="p-2.5 rounded bg-page text-secondary mt-2 overflow-x-auto">{"$ avm plugin list"}</pre>
                </div>
                <div className="p-5 rounded-lg bg-surface/60 border border-line space-y-2">
                  <div className="text-accent font-bold">avm plugin update &lt;name&gt;</div>
                  <div className="text-secondary font-sans text-xs">Checks upstream GitHub releases for the latest plugin binary and updates it.</div>
                  <pre className="p-2.5 rounded bg-page text-secondary mt-2 overflow-x-auto">{"$ avm plugin update java"}</pre>
                </div>
                <div className="p-5 rounded-lg bg-surface/60 border border-line space-y-2">
                  <div className="text-accent font-bold">avm plugin remove &lt;name&gt;</div>
                  <div className="text-secondary font-sans text-xs">Removes the installed plugin; managed runtime versions are separate.</div>
                  <pre className="p-2.5 rounded bg-page text-secondary mt-2 overflow-x-auto">{"$ avm plugin remove ruby"}</pre>
                </div>
              </div>
            </div>

          ) : activeSection === 'usage-versions' ? (

            /* 7. Usage: Versions */
            <div className="space-y-10">
              <div>
                <h1 className="text-3xl sm:text-4xl font-semibold text-ink tracking-tight mb-4">
                  Managing Tool Versions
                </h1>
                <p className="text-secondary text-base leading-relaxed max-w-3xl">
                  Install, switch, and pin multiple versions per language or SDK across project directories.
                </p>
              </div>

              <div className="rounded-lg bg-surface/60 border border-line p-6 sm:p-8 space-y-6 font-mono text-xs">
                <div>
                  <span className="text-accent font-bold text-sm block mb-1">1. Browse Available Releases</span>
                  <p className="text-secondary font-sans text-xs mb-2">Queries upstream indices directly without cloning heavy repositories:</p>
                  <pre className="p-3 rounded-lg bg-page text-secondary border border-line overflow-x-auto">{"$ avm node versions\n$ avm java versions\n$ avm android versions"}</pre>
                </div>

                <div>
                  <span className="text-accent font-bold text-sm block mb-1">2. Install Exact or Major Versions</span>
                  <p className="text-secondary font-sans text-xs mb-2">Fetches official prebuilt distributions:</p>
                  <pre className="p-3 rounded-lg bg-page text-secondary border border-line overflow-x-auto">{"$ avm node install 22.14.0\n$ avm java install 17\n$ avm android install 34"}</pre>
                </div>

                <div>
                  <span className="text-accent font-bold text-sm block mb-1">3. Pin Local vs Global</span>
                  <pre className="p-3 rounded-lg bg-page text-secondary border border-line overflow-x-auto">{"# Pins in ./.avm.json for the current repository:\n$ avm node use 22.14.0\n\n# Pins machine default in ~/.avm.json:\n$ avm node use 22.14.0 --global"}</pre>
                </div>
                <div>
                  <span className="text-accent font-bold text-sm block mb-1">4. Try a Version Without Changing Pins</span>
                  <p className="text-secondary font-sans text-xs mb-2">Run a command with temporary tool versions, provider env and trusted project env. Missing runtimes install automatically; plugins must already be installed. Use --no-install to select installed versions only.</p>
                  <pre className="p-3 rounded-lg bg-page text-secondary border border-line overflow-x-auto">{'$ avm exec node@18 -- npm test\n$ avm x node@22 java@21 -- ./gradlew build\n$ avm run --with node@18 dev --port 3000'}</pre>
                  <p className="text-secondary font-sans text-xs mt-2">Project and global pins and lockfiles stay unchanged. Repeat --with for multiple tools and place it before alias arguments.</p>
                </div>
                <div>
                  <span className="text-accent font-bold text-sm block mb-1">Reproducible Installs for Teams and CI</span>
                  <p className="text-secondary font-sans text-xs mb-2">Install all selected tools without repinning. Lock fuzzy versions to exact releases and commit .avm.lock; frozen installs reject stale config and verify locked archive checksums.</p>
                  <pre className="p-3 rounded-lg bg-page text-secondary border border-line overflow-x-auto">{'$ avm install\n$ avm lock --platform linux_amd64,darwin_arm64,windows_amd64\n$ avm install --frozen'}</pre>
                  <p className="text-secondary font-sans text-xs mt-2">Locking fetches metadata without installing runtimes. Upgrade the Node and Java plugins for lockfile support. Android and legacy asdf plugins currently support ordinary installs only.</p>
                </div>
              </div>
            </div>

          ) : activeSection === 'usage-aliases' ? (

            /* 8. Usage: Aliases & Scripts */
            <div className="space-y-10">
              <div>
                <h1 className="text-3xl sm:text-4xl font-semibold text-ink tracking-tight mb-4">
                  Aliases & Scripts Discovery
                </h1>
                <p className="text-secondary text-base leading-relaxed max-w-3xl">
                  Eliminate command drift. Anchor project aliases inside <code className="text-accent font-mono text-xs bg-page px-1 py-0.5 rounded">.avm.json</code> and automatically discover scripts from <code className="text-accent font-mono text-xs bg-page px-1 py-0.5 rounded">package.json</code>.
                </p>
              </div>

              <div className="grid grid-cols-1 md:grid-cols-2 gap-4 text-xs font-mono">
                <div className="p-5 rounded-lg bg-surface/60 border border-line space-y-3">
                  <h4 className="text-accent font-bold text-sm font-sans">Explicit Project Aliases</h4>
                  <pre className="p-3 rounded-lg bg-page text-secondary overflow-x-auto">{"$ avm alias add dev \"pnpm run dev --port 3000\"\n$ avm alias add release 'npm run release $1'"}</pre>
                  <p className="text-secondary font-sans text-xs">
                    Supports parameter forwarding ($1, $@) and runs with injected tools and env vars.
                  </p>
                </div>

                <div className="p-5 rounded-lg bg-surface/60 border border-line space-y-3">
                  <h4 className="text-accent font-bold text-sm font-sans">Auto package.json Script Discovery</h4>
                  <pre className="p-3 rounded-lg bg-page text-secondary overflow-x-auto">{"$ avm test\n$ avm build\n$ avm lint"}</pre>
                  <p className="text-secondary font-sans text-xs">
                    Discovers scripts defined in package.json and executes them using your detected package manager (pnpm, npm, yarn, bun).
                  </p>
                </div>
              </div>
            </div>

          ) : activeSection === 'version-files' ? (

            /* Usage: Version Files */
            <div className="space-y-10">
              <div>
                <h1 className="text-3xl sm:text-4xl font-semibold text-ink tracking-tight mb-4">
                  Version Files
                </h1>
                <p className="text-secondary text-base leading-relaxed max-w-3xl">
                  avm reads the version files your projects already have, so you can try it without rewriting any config.
                </p>
              </div>

              <div className="rounded-lg bg-surface/50 border border-line p-6 sm:p-8 space-y-4 text-sm text-secondary">
                <h3 className="text-xl font-bold text-ink">Precedence (nearest directory wins)</h3>
                <ol className="list-decimal pl-5 space-y-1.5">
                  <li><code className="text-accent font-mono text-xs bg-page px-1 py-0.5 rounded">.avm.json</code> <code className="text-accent font-mono text-xs bg-page px-1 py-0.5 rounded">tools</code> merged from parent and child layers (including profiles and personal overrides)</li>
                  <li><code className="text-accent font-mono text-xs bg-page px-1 py-0.5 rounded">.tool-versions</code> (asdf format; <code className="text-accent font-mono text-xs bg-page px-1 py-0.5 rounded">nodejs</code> maps to <code className="text-accent font-mono text-xs bg-page px-1 py-0.5 rounded">node</code>)</li>
                  <li><code className="text-accent font-mono text-xs bg-page px-1 py-0.5 rounded">.nvmrc</code>, <code className="text-accent font-mono text-xs bg-page px-1 py-0.5 rounded">.node-version</code>, <code className="text-accent font-mono text-xs bg-page px-1 py-0.5 rounded">package.json</code> (<code className="text-accent font-mono text-xs bg-page px-1 py-0.5 rounded">volta.node</code>, else the <code className="text-accent font-mono text-xs bg-page px-1 py-0.5 rounded">engines.node</code> range), <code className="text-accent font-mono text-xs bg-page px-1 py-0.5 rounded">.java-version</code>, <code className="text-accent font-mono text-xs bg-page px-1 py-0.5 rounded">.sdkmanrc</code></li>
                  <li>Global <code className="text-accent font-mono text-xs bg-page px-1 py-0.5 rounded">~/.avm.json</code></li>
                </ol>
                <p>Version files are searched upward from the current directory. Partial specs resolve to the newest <em>installed</em> match:</p>
                <div className="p-4 rounded-lg bg-page border border-line font-mono text-xs space-y-1">
                  <div><span className="text-secondary"># .nvmrc: 20</span> <span className="text-secondary">→ 20.11.1</span></div>
                  <div><span className="text-secondary"># .nvmrc: lts/iron, lts/*</span> <span className="text-secondary">→ newest installed 20.x / newest LTS</span></div>
                  <div><span className="text-secondary"># package.json engines: &gt;=18 &lt;21, ^20.1, 18 || 20</span> <span className="text-secondary">→ newest installed match</span></div>
                  <div><span className="text-secondary"># .java-version: 17</span> <span className="text-secondary">→ openjdk-17.0.9+9</span></div>
                </div>
                <p>See which file won with <code className="text-accent font-mono text-xs bg-page px-1 py-0.5 rounded">avm which</code>:</p>
                <pre className="p-4 rounded-lg bg-page border border-line font-mono text-xs text-secondary overflow-x-auto">{`$ avm which node
tool 'node': 20.11.1 (from ./.nvmrc)`}</pre>
                <p>To ignore the tool-specific files (item 3), set <code className="text-accent font-mono text-xs bg-page px-1 py-0.5 rounded">"idiomatic_version_files": false</code> in <code className="text-accent font-mono text-xs bg-page px-1 py-0.5 rounded">~/.avm.json</code>.</p>
              </div>
            </div>

          ) : activeSection === 'security' ? (

            /* Usage: Security & Trust */
            <div className="space-y-10">
              <div>
                <h1 className="text-3xl sm:text-4xl font-semibold text-ink tracking-tight mb-4">
                  Security &amp; Trust
                </h1>
                <p className="text-secondary text-base leading-relaxed max-w-3xl">
                  Cloning a repository never runs its config, and every binary avm downloads is verified before it's unpacked.
                </p>
              </div>

              <div className="rounded-lg bg-surface/50 border border-line p-6 sm:p-8 space-y-4 text-sm text-secondary">
                <h3 className="text-xl font-bold text-ink">Trusting a project</h3>
                <p>
                  A project's <code className="text-accent font-mono text-xs bg-page px-1 py-0.5 rounded">.avm.json</code> aliases and <code className="text-accent font-mono text-xs bg-page px-1 py-0.5 rounded">env</code>, and its <code className="text-accent font-mono text-xs bg-page px-1 py-0.5 rounded">.env</code> files, run with your privileges and reach every shimmed <code className="text-accent font-mono text-xs bg-page px-1 py-0.5 rounded">node</code>/<code className="text-accent font-mono text-xs bg-page px-1 py-0.5 rounded">java</code>. So avm ignores them until you trust that exact file content. Tool pins only pick a version and always apply.
                </p>
                <pre className="p-4 rounded-lg bg-page border border-line font-mono text-xs text-secondary overflow-x-auto">{`$ cd cloned-repo
$ avm trust            # shows the aliases/env it enables, then trusts them
$ avm trust --list     # everything you've trusted
$ avm trust --revoke   # stop trusting this directory`}</pre>
                <ul className="space-y-1.5 list-disc pl-5">
                  <li>Trust is stored as path &rarr; sha256 in <code className="text-accent font-mono text-xs bg-page px-1 py-0.5 rounded">~/.avm/trusted.json</code>. Any edit made outside avm blocks the file again; <code className="text-accent font-mono text-xs bg-page px-1 py-0.5 rounded">avm init</code>, <code className="text-accent font-mono text-xs bg-page px-1 py-0.5 rounded">avm add</code> and <code className="text-accent font-mono text-xs bg-page px-1 py-0.5 rounded">avm env add</code> keep a trusted file trusted.</li>
                  <li>Your global <code className="text-accent font-mono text-xs bg-page px-1 py-0.5 rounded">~/.avm.json</code> is always trusted and can list <code className="text-accent font-mono text-xs bg-page px-1 py-0.5 rounded">"trusted_paths": ["~/work/**"]</code>.</li>
                  <li><code className="text-accent font-mono text-xs bg-page px-1 py-0.5 rounded">AVM_TRUST_ALL=1</code> trusts everything, for CI. It's never implied by <code className="text-accent font-mono text-xs bg-page px-1 py-0.5 rounded">CI=true</code>.</li>
                </ul>
              </div>

              <div className="rounded-lg bg-surface/50 border border-line p-6 sm:p-8 space-y-4 text-sm text-secondary">
                <h3 className="text-xl font-bold text-ink">Verified downloads</h3>
                <ul className="space-y-1.5 list-disc pl-5">
                  <li><strong className="text-ink">Plugins:</strong> <code className="text-accent font-mono text-xs bg-page px-1 py-0.5 rounded">avm plugin add</code> checks the archive against the release's <code className="text-accent font-mono text-xs bg-page px-1 py-0.5 rounded">checksums.txt</code> before extracting. A mismatch aborts with nothing installed.</li>
                  <li><strong className="text-ink">Runtimes:</strong> Node.js is checked against <code className="text-accent font-mono text-xs bg-page px-1 py-0.5 rounded">SHASUMS256.txt</code>, Temurin JDKs against foojay's sha256, and the Android cmdline-tools zip against a pinned sha256 (<code className="text-accent font-mono text-xs bg-page px-1 py-0.5 rounded">sdkmanager</code> verifies the rest).</li>
                  <li><strong className="text-ink">avm itself:</strong> <code className="text-accent font-mono text-xs bg-page px-1 py-0.5 rounded">install.sh</code>, <code className="text-accent font-mono text-xs bg-page px-1 py-0.5 rounded">install.ps1</code> and the npm installer verify <code className="text-accent font-mono text-xs bg-page px-1 py-0.5 rounded">avm-bin</code> the same way.</li>
                  <li><strong className="text-ink">Provenance:</strong> every release carries a GitHub build attestation. Check it with <code className="text-accent font-mono text-xs bg-page px-1 py-0.5 rounded">gh attestation verify &lt;archive&gt; --owner PrajaNova</code>.</li>
                  <li><code className="text-accent font-mono text-xs bg-page px-1 py-0.5 rounded">AVM_ALLOW_UNVERIFIED=1</code> is the only way to skip verification.</li>
                </ul>
              </div>
            </div>

          ) : activeSection === 'commands' ? (

            /* 10. Reference: All Commands */
            <CliReference onCopy={onCopy} />

          ) : activeSection === 'dependencies' ? (

            /* 11. Reference: Dependencies */
            <div className="space-y-10">
              <div>
                <h1 className="text-3xl sm:text-4xl font-semibold text-ink tracking-tight mb-4">
                  Dependencies & Host Environment
                </h1>
                <p className="text-secondary text-base leading-relaxed max-w-3xl">
                  avm is compiled directly to machine code with zero external runtime dependencies.
                </p>
              </div>

              <div className="grid grid-cols-1 md:grid-cols-2 gap-4 text-xs font-sans">
                <div className="p-5 rounded-lg bg-surface/60 border border-line space-y-2">
                  <h4 className="font-bold text-ink text-sm">Supported Operating Systems</h4>
                  <ul className="space-y-1 text-secondary font-mono text-xs">
                    <li>&bull; macOS (Apple Silicon arm64, Intel x86_64)</li>
                    <li>&bull; Linux (x86_64, aarch64; glibc)</li>
                    <li>&bull; Windows x64 with PowerShell, cmd.exe forwarding and native plugins</li>
                  </ul>
                </div>
                <div className="p-5 rounded-lg bg-surface/60 border border-line space-y-2">
                  <h4 className="font-bold text-ink text-sm">Runtime Dependencies</h4>
                  <p className="text-secondary text-xs">
                    <strong className="text-accent">No language runtime required.</strong> Plugin installs use host tools such as curl and tar; Android also needs a JDK (and unzip on Unix). No Node.js, Python, Ruby, or GCC is required to run the core <code className="font-mono text-accent">avm-bin</code>.
                  </p>
                </div>
              </div>
            </div>

          ) : activeSection === 'architecture' ? (

            /* 12. Reference: Architecture */
            <div className="space-y-10">
              <div>
                <h1 className="text-3xl sm:text-4xl font-semibold text-ink tracking-tight mb-4">
                  Architecture & Wire Protocol
                </h1>
                <p className="text-secondary text-base leading-relaxed max-w-3xl">
                  avm isolates plugins as standalone processes communicating over standard input/output using a strongly typed JSON wire protocol.
                </p>
              </div>

              <div className="p-6 rounded-lg bg-surface/60 border border-line space-y-4">
                <h3 className="text-lg font-bold text-ink flex items-center gap-2">
                  <Cpu className="w-5 h-5 text-accent" />
                  <span>Typed Wire Protocol Commands</span>
                </h3>
                <div className="grid grid-cols-1 md:grid-cols-2 gap-4 text-xs font-mono">
                  <div className="p-4 rounded-lg bg-page border border-line space-y-1.5">
                    <span className="text-accent font-bold uppercase tracking-wider block mb-1">Queries (Read)</span>
                    <div className="text-secondary"># Metadata:</div>
                    <div className="text-ink">avm-plugin-&lt;name&gt; manifest</div>
                    <div className="text-secondary pt-1"># Versions:</div>
                    <div className="text-ink">avm-plugin-&lt;name&gt; versions</div>
                    <div className="text-secondary pt-1"># Env exports:</div>
                    <div className="text-ink">avm-plugin-&lt;name&gt; env-vars &lt;version&gt;</div>
                  </div>
                  <div className="p-4 rounded-lg bg-page border border-line space-y-1.5">
                    <span className="text-accent font-bold uppercase tracking-wider block mb-1">Lifecycle (Write)</span>
                    <div className="text-secondary"># Install:</div>
                    <div className="text-ink">avm-plugin-&lt;name&gt; install &lt;version&gt;</div>
                    <div className="text-secondary pt-1"># Uninstall:</div>
                    <div className="text-ink">avm-plugin-&lt;name&gt; uninstall &lt;version&gt;</div>
                    <div className="text-secondary pt-1"># Executable:</div>
                    <div className="text-ink">avm-plugin-&lt;name&gt; executable-path &lt;version&gt;</div>
                  </div>
                </div>
              </div>
              <section className="rounded-lg bg-surface/50 border border-line p-6 space-y-4 text-sm text-secondary">
                <h2 className="text-xl font-bold text-ink">Discovery and plugin development</h2>
                <p>All native providers use the same protocol: bundled executables beside avm-bin, then user-installed plugins, then the permanent legacy asdf adapter. No provider implementation is linked into the CLI.</p>
                <p>Additional query verbs are is-installed and installed-versions. Optional resolve and install-locked verbs provide exact download metadata and checksum-verified installs. Native plugins can also expose custom commands, such as android avd.</p>
                <p>Run <code>avm create kotlin</code> for template instructions. Read the <a className="text-accent underline" href="https://github.com/PrajaNova/avm/blob/main/docs/plugins/CREATING_A_PLUGIN.md">plugin author guide</a> for every wire verb, the ToolProvider contract, timeouts and release packaging.</p>
              </section>
            </div>

          ) : activeSection === 'comparison' ? (

            /* 13. Reference: Comparison Matrix */
            <div className="space-y-10">
              <div>
                <h1 className="text-3xl sm:text-4xl font-semibold text-ink tracking-tight mb-4">
                  Comparison Matrix
                </h1>
                <p className="text-secondary text-base leading-relaxed max-w-3xl">
                  Side-by-side comparison of avm against asdf, vfox, mise, and proto, checked against each project's docs in September 2026. "Not documented" means we couldn't find it, not that it's confirmed absent.
                </p>
              </div>

              <ComparisonTable />
            </div>

          ) : activeSection === 'faq' ? (

            /* 16. Questions: FAQ */
            <div className="space-y-10">
              <div>
                <h1 className="text-3xl sm:text-4xl font-semibold text-ink tracking-tight mb-4">
                  Frequently Asked Questions
                </h1>
                <p className="text-secondary text-base leading-relaxed max-w-3xl">
                  Answers to common questions regarding avm's runtime model, performance, and compatibility.
                </p>
              </div>

              <div className="space-y-4 text-xs sm:text-sm">
                {[
                  { q: 'Do I need to rewrite my .nvmrc or .tool-versions?', a: 'No. avm reads .tool-versions, .nvmrc, .node-version, package.json engines/volta, .java-version and .sdkmanrc directly. avm which <tool> shows which file a version came from.' },
                  { q: "Why doesn't my project's alias run after I clone it?", a: "A project's aliases, env and .env files are ignored until you run avm trust in that directory. It shows what you're enabling first. Editing the file outside avm blocks it again." },
                  { q: 'Can I use globally installed packages across different local version pins?', a: 'Yes! avm supports global package sharing, meaning globally installed tools (like npm -g packages) remain reachable even when a directory pins a different version.' },
                  { q: 'Where are plugins downloaded and installed?', a: 'Plugins are stored in ~/.avm/plugins/avm-plugin-<name>, while downloaded runtimes reside in ~/.avm/tools/<name>/<version>.' },
                  { q: 'What happens if a requested tool version is not installed?', a: 'Plain shimmed commands warn and fall back to a system binary when available. avm exec and avm run --with install missing runtimes automatically; --no-install fails if no installed version matches.' },
                ].map((item, idx) => (
                  <div key={idx} className="p-5 rounded-lg bg-surface/60 border border-line">
                    <h4 className="font-bold text-ink text-base mb-2">{item.q}</h4>
                    <p className="text-secondary leading-relaxed text-xs sm:text-sm">{item.a}</p>
                  </div>
                ))}
              </div>
            </div>

          ) : activeSection === 'troubleshooting' ? (

            /* 17. Questions: Troubleshooting */
            <div className="space-y-10">
              <div>
                <h1 className="text-3xl sm:text-4xl font-semibold text-ink tracking-tight mb-4">
                  Troubleshooting & Diagnostics
                </h1>
                <p className="text-secondary text-base leading-relaxed max-w-3xl">
                  Diagnostic steps to resolve PATH ordering, shell hook evaluation, and emulator permissions.
                </p>
              </div>

              <div className="space-y-4 font-mono text-xs">
                <div className="p-5 rounded-lg bg-surface/60 border border-line space-y-2">
                  <h4 className="font-bold text-ink font-sans text-sm">1. Check Active Environment Variables</h4>
                  <pre className="p-3 rounded-lg bg-page text-accent overflow-x-auto">{"$ avm env"}</pre>
                  <p className="text-secondary font-sans text-xs">Prints exact exported variables including JAVA_HOME, ANDROID_HOME, and PATH order.</p>
                </div>
                <div className="p-5 rounded-lg bg-surface/60 border border-line space-y-2">
                  <h4 className="font-bold text-ink font-sans text-sm">2. Verify Shim Resolution</h4>
                  <pre className="p-3 rounded-lg bg-page text-accent overflow-x-auto">{"$ which node\n$ which java\n$ which adb"}</pre>
                  <p className="text-secondary font-sans text-xs">Should point to ~/.avm/shims/&lt;binary&gt;.</p>
                </div>
              </div>
            </div>

          ) : activeSection === 'contribute-core' ? (

            /* 18. Contribute: Core */
            <div className="space-y-10">
              <div>
                <h1 className="text-3xl sm:text-4xl font-semibold text-ink tracking-tight mb-4">
                  Contributing to avm Core
                </h1>
                <p className="text-secondary text-base leading-relaxed max-w-3xl">
                  We welcome contributions! avm is completely open source under the MIT license.
                </p>
              </div>

              <div className="p-6 rounded-lg bg-surface/60 border border-line space-y-4 font-mono text-xs">
                <span className="text-ink font-bold text-sm font-sans">Build from Source</span>
                <pre className="p-4 rounded-lg bg-page text-secondary border border-line overflow-x-auto">{"$ git clone https://github.com/PrajaNova/avm.git\n$ cd avm\n$ cargo check\n$ cargo test\n$ cargo build --release"}</pre>
              </div>
            </div>

          ) : (

            /* 17. Contribute: Docs */
            <div className="space-y-10">
              <div>
                <h1 className="text-3xl sm:text-4xl font-semibold text-ink tracking-tight mb-4">
                  Contributing Documentation
                </h1>
                <p className="text-secondary text-base leading-relaxed max-w-3xl">
                  Help improve our guides, shell integration instructions, and troubleshooting tips.
                </p>
              </div>

              <div className="p-6 rounded-lg bg-surface/60 border border-line space-y-4">
                <h4 className="text-base font-bold text-ink">Submitting Documentation Changes</h4>
                <p className="text-xs sm:text-sm text-secondary leading-relaxed">
                  Fork <a href="https://github.com/PrajaNova/avm" target="_blank" rel="noreferrer" className="text-accent underline">PrajaNova/avm</a>, improve the documentation or guide files, and open a Pull Request!
                </p>
              </div>
            </div>

          )}

          {/* Previous / Next Page Navigation Cards (like asdf-vm/VitePress) */}
          <div className="mt-16 pt-8 border-t border-line/80 flex flex-col sm:flex-row items-stretch sm:items-center justify-between gap-4">
            {prevPage ? (
              <button
                onClick={() => navigate(prevPage.link)}
                className="flex-1 flex flex-col items-start p-4 rounded-lg bg-surface/50 hover:bg-surface border border-line hover:border-line transition-all group text-left"
              >
                <div className="flex items-center gap-1.5 text-xs font-mono text-secondary group-hover:text-accent mb-1">
                  <ArrowLeft className="w-3.5 h-3.5 group-hover:-translate-x-1 transition-transform" />
                  <span>Previous page</span>
                </div>
                <span className="text-sm font-bold text-ink group-hover:text-accent transition-colors">
                  {prevPage.text}
                </span>
              </button>
            ) : (
              <div className="flex-1" />
            )}

            {nextPage && (
              <button
                onClick={() => navigate(nextPage.link)}
                className="flex-1 flex flex-col items-end p-4 rounded-lg bg-surface/50 hover:bg-surface border border-line hover:border-line transition-all group text-right"
              >
                <div className="flex items-center gap-1.5 text-xs font-mono text-secondary group-hover:text-accent mb-1">
                  <span>Next page</span>
                  <ArrowRight className="w-3.5 h-3.5 group-hover:translate-x-1 transition-transform" />
                </div>
                <span className="text-sm font-bold text-ink group-hover:text-accent transition-colors">
                  {nextPage.text}
                </span>
              </button>
            )}
          </div>

        </div>
      </div>

    </div>
  );
};
