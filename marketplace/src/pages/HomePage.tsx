import { Link } from 'react-router-dom';
import {
  Copy,
  ArrowRight,
  FileJson,
  FolderGit2,
  FileSearch,
  Lock,
  Monitor,
  Package,
  Sparkles,
  Share2,
  ShieldCheck,
  Workflow,
  AlertTriangle,
  Layers,
  Smartphone,
  Terminal,
} from 'lucide-react';
import { ComparisonTable } from '../components/ComparisonTable';
import { WalkthroughVideo } from '../components/WalkthroughVideo';
import { TerminalInstallBox } from '../components/TerminalInstallBox';

interface HomePageProps {
  onCopy: (text: string) => void;
}

export const HomePage: React.FC<HomePageProps> = ({ onCopy }) => {
  const whyAvm = [
    {
      num: '01',
      title: 'Command drift across projects',
      body: 'Different teammates run slightly different commands, forgotten flags, or outdated scripts. avm anchors project-specific aliases inside a version-controlled .avm.json — everyone executes the identical workflow.',
    },
    {
      num: '02',
      title: 'Juggling tool versions per project',
      body: 'Conflicting Node, OpenJDK, and Android SDK versions across projects mean brittle manual environment switches. avm resolves the right version per directory, automatically, the moment you cd in.',
    },
    {
      num: '03',
      title: 'A shell rc file full of one-off hacks',
      body: '.bashrc/.zshrc become dumping grounds for fragmented aliases and machine-specific PATH edits. avm keeps aliases and env vars scoped to a project, or clean and global — never both, tangled.',
    },
  ];

  const useCases = [
    {
      icon: Layers,
      title: 'Multi-Runtime Monorepos',
      desc: 'Monorepos mixing multiple Node or tool versions across packages with zero path collisions.',
    },
    {
      icon: Smartphone,
      title: 'Mobile Engineering Teams',
      desc: 'Mobile teams juggling Temurin OpenJDKs and Android SDK API levels per application repository.',
    },
    {
      icon: Terminal,
      title: 'Unified Team Workflows',
      desc: 'Engineering teams replacing brittle, hand-copied shell scripts with version-controlled aliases.',
    },
  ];

  const corePillars = [
    { icon: FileJson, title: 'Project & global alias resolution', desc: 'Aliases in .avm.json execute with injected environment variables and tool versions.' },
    { icon: FolderGit2, title: 'Directory-aware local precedence', desc: 'Local project configuration takes immediate precedence over global ~/.avm.json settings.' },
    { icon: FileSearch, title: 'Works with your existing version files', desc: '.nvmrc, .node-version, .tool-versions, package.json engines, .java-version and .sdkmanrc are read as-is — no config rewrite to try avm.' },
    { icon: Lock, title: 'Nothing runs until you trust it', desc: "A cloned repo's .avm.json aliases/env and .env files stay inert until you run avm trust; editing them re-blocks." },
    { icon: ShieldCheck, title: 'Verified downloads', desc: 'Plugins, avm itself, and Node/Java/Android downloads are sha256-checked before extraction; releases carry build provenance attestations.' },
    { icon: Monitor, title: 'macOS, Linux, and Windows', desc: 'Apple Silicon and Intel Macs, Linux x64/arm64, and Windows via PowerShell (phase 1).' },
    { icon: Package, title: 'Runtime plugin marketplace', desc: 'avm plugin add fetches a precompiled native binary on demand from GitHub Releases — never built from source.' },
    { icon: Sparkles, title: 'Automatic package.json discovery', desc: 'Scripts in package.json become instant avm <script> commands, using your detected package manager.' },
    { icon: Share2, title: 'Global package sharing across versions', desc: 'A globally pinned tool (like npm -g packages) stays reachable even when a different local version is active.' },
    { icon: Layers, title: 'Isolated subprocess wire protocol', desc: 'Plugins run as standalone OS processes speaking a typed JSON-over-stdio contract — failures stay isolated.' },
    { icon: Workflow, title: 'Workflow orchestration', desc: 'Unifies multi-runtime dependencies, env vars, and team scripts into one reproducible development experience.' },
    { icon: AlertTriangle, title: 'Safe host fallback', desc: "If a managed version isn't installed, avm falls back to the host/system tool with a clear warning." },
  ];

  const workspaceCrates = [
    { name: 'crates/avm-cli · cli/', desc: 'The avm-bin binary: Clap command routing, shell-init (sh and PowerShell), trust.' },
    { name: 'crates/avm-cli · config, resolver, version_files', desc: '.avm.json parsing, local/global merge, version files (.nvmrc, .tool-versions, …), alias/env/tool resolution.' },
    { name: 'crates/avm-cli · shims, runtime', desc: 'Shims in ~/.avm/shims, plugin discovery, the protocol host, the verified marketplace installer, and the asdf adapter.' },
    { name: 'crates/avm-plugin-api', desc: 'The ToolProvider trait, the wire protocol, the runner every plugin uses, and shared sha256 verification helpers.' },
  ];

  return (
    <>
      <div className="w-full flex flex-col items-center">

        {/* ===================== HERO ===================== */}
        <section className="w-full max-w-4xl mx-auto px-4 sm:px-6 lg:px-8 pt-20 pb-16 flex flex-col items-center text-center">
          <div className="inline-flex items-center gap-2 px-3.5 py-1.5 rounded-full border border-slate-700 bg-slate-900/60 text-[11px] font-mono tracking-wider text-emerald-400 mb-7 shadow-sm">
            <span className="w-1.5 h-1.5 rounded-full bg-emerald-400 animate-pulse" />
            <span>RUST-NATIVE &middot; RUNTIME PLUGIN MARKETPLACE</span>
          </div>
          <Link
            to="/changelog"
            className="-mt-4 mb-7 text-xs font-mono text-slate-400 hover:text-emerald-400 transition-colors"
          >
            New in v0.4.0-beta-1: config trust, version files, verified downloads, Intel &amp; Windows &rarr;
          </Link>

          <h1 className="font-sans font-bold text-4xl sm:text-5xl md:text-6xl leading-[1.12] tracking-tight text-slate-100 mb-6 max-w-3xl">
            The version manager for your whole project.
          </h1>

          <p className="max-w-2xl text-[16px] leading-relaxed text-slate-300 mb-10">
            One unified tool for project aliases, per-directory tool versions, and env vars &mdash;
            plugins fetched on demand, lightning fast, nothing to compile.
          </p>

          {/* Terminal Install Box */}
          <TerminalInstallBox onCopy={onCopy} />

          <div className="flex flex-wrap items-center justify-center gap-4 mt-8">
            <Link
              to="/marketplace"
              className="px-6 py-2.5 rounded-lg bg-emerald-400 text-slate-950 font-sans font-semibold text-[13px] hover:bg-emerald-300 transition-colors shadow-lg shadow-emerald-500/10 flex items-center gap-2"
            >
              <span>Explore Marketplace</span>
              <ArrowRight className="w-4 h-4" />
            </Link>
            <Link
              to="/docs"
              className="px-6 py-2.5 rounded-lg border border-slate-700 bg-slate-900/60 text-slate-200 font-sans font-semibold text-[13px] hover:border-slate-500 hover:text-white transition-colors"
            >
              Read Documentation
            </Link>
          </div>
        </section>

        {/* ===================== INTERACTIVE TERMINAL WALKTHROUGH ===================== */}
        <section className="w-full">
          <div className="text-center mb-6 max-w-2xl mx-auto px-4">
            <p className="text-[11px] font-mono tracking-widest text-emerald-400 uppercase mb-2">
              60-Second Walkthrough
            </p>
            <h2 className="font-sans font-bold text-2xl sm:text-3xl text-slate-100">
              See avm in action
            </h2>
            <p className="text-[13px] text-slate-400 mt-2">
              Follow the live interactive terminal walkthrough to see how avm installs, initializes,
              downloads plugins, pins tool versions, registers aliases, and executes commands in under a millisecond.
            </p>
          </div>
          <WalkthroughVideo onCopy={onCopy} />
        </section>

        {/* ===================== WHY AVM ===================== */}
        <section className="w-full max-w-5xl mx-auto px-4 sm:px-6 lg:px-8 pb-24">
          <div className="text-center mb-12">
            <p className="text-[11px] font-mono tracking-widest text-emerald-400 uppercase mb-2">WHY AVM</p>
            <h2 className="font-sans font-bold text-2xl sm:text-3xl text-slate-100">
              Solve everyday tool and shell friction
            </h2>
          </div>
          <div className="grid grid-cols-1 md:grid-cols-3 gap-6">
            {whyAvm.map((item, idx) => (
              <div
                key={idx}
                className="flex flex-col p-6 rounded-xl bg-slate-900/60 border border-slate-700/80 hover:border-slate-600 transition-all hover:shadow-lg"
              >
                <div className="flex items-center justify-between mb-4">
                  <span className="font-mono text-xs font-bold text-emerald-400 px-2.5 py-1 rounded bg-emerald-500/10 border border-emerald-500/20">
                    {item.num}
                  </span>
                </div>
                <h3 className="font-sans font-semibold text-[17px] text-slate-100 mb-2.5 leading-snug">
                  {item.title}
                </h3>
                <p className="text-[13px] leading-relaxed text-slate-300">
                  {item.body}
                </p>
              </div>
            ))}
          </div>
        </section>

        {/* ===================== COMPARISON ===================== */}
        <section className="w-full max-w-5xl mx-auto px-4 sm:px-6 lg:px-8 pb-24 flex flex-col items-center">
          <div className="text-center mb-10">
            <p className="text-[11px] font-mono tracking-widest text-emerald-400 uppercase mb-2">HOW IT COMPARES</p>
            <h2 className="font-sans font-bold text-2xl sm:text-3xl text-slate-100">
              avm, next to asdf, vfox, mise, and proto
            </h2>
          </div>

          <ComparisonTable />
        </section>

        {/* ===================== USE CASES ===================== */}
        <section className="w-full max-w-5xl mx-auto px-4 sm:px-6 lg:px-8 pb-24">
          <div className="text-center mb-10">
            <p className="text-[11px] font-mono tracking-widest text-emerald-400 uppercase mb-2">BUILT FOR</p>
            <h2 className="font-sans font-bold text-2xl sm:text-3xl text-slate-100">
              Real teams, real workflows
            </h2>
          </div>
          <div className="grid grid-cols-1 md:grid-cols-3 gap-5">
            {useCases.map((useCase, idx) => {
              const Icon = useCase.icon;
              return (
                <div
                  key={idx}
                  className="flex flex-col justify-between p-6 rounded-xl bg-slate-900/60 border border-slate-700/80 hover:border-slate-600 transition-all group"
                >
                  <div>
                    <div className="w-10 h-10 rounded-lg bg-slate-800 border border-slate-700 flex items-center justify-center mb-4 text-emerald-400 group-hover:border-emerald-500/40 transition-colors">
                      <Icon className="w-5 h-5" />
                    </div>
                    <h3 className="font-sans font-semibold text-[16px] text-slate-100 mb-2">
                      {useCase.title}
                    </h3>
                    <p className="text-[13px] leading-relaxed text-slate-300">
                      {useCase.desc}
                    </p>
                  </div>
                  <div className="mt-4 pt-3 border-t border-slate-800 flex items-center justify-between text-xs font-mono text-emerald-400">
                    <span>Supported out-of-the-box</span>
                    <ArrowRight className="w-4 h-4 group-hover:translate-x-1 transition-transform" />
                  </div>
                </div>
              );
            })}
          </div>
        </section>

        {/* ===================== QUICK START ===================== */}
        <section className="w-full max-w-5xl mx-auto px-4 sm:px-6 lg:px-8 pb-24">
          <div className="text-center mb-10">
            <p className="text-[11px] font-mono tracking-widest text-emerald-400 uppercase mb-2">GET STARTED</p>
            <h2 className="font-sans font-bold text-2xl sm:text-3xl text-slate-100 mb-2">
              Quick start in 60 seconds
            </h2>
            <p className="text-[13px] text-slate-400 max-w-xl mx-auto">
              From an empty project to pinned runtime versions and project aliases, in five quick commands.
            </p>
          </div>

          <div className="relative rounded-xl bg-slate-900/90 border border-slate-700/80 p-6 font-mono text-[13px] text-slate-200 mb-6 shadow-xl backdrop-blur-sm">
            <div className="space-y-3">
              <div>
                <span className="text-slate-500 block mb-0.5"># 1. Initialize .avm.json in the current directory</span>
                <span className="text-emerald-400 font-semibold">avm init</span>
              </div>
              <div>
                <span className="text-slate-500 block mb-0.5"># 2. Add a project-scoped alias</span>
                <span className="text-emerald-400 font-semibold">avm alias add dev "pnpm run dev"</span>
              </div>
              <div>
                <span className="text-slate-500 block mb-0.5"># 3. Install a runtime plugin on demand</span>
                <span className="text-emerald-400 font-semibold">avm plugin add node</span>
              </div>
              <div>
                <span className="text-slate-500 block mb-0.5"># 4. Pin the project's runtime version</span>
                <span className="text-emerald-400 font-semibold">avm node use 20.11.1</span>
              </div>
              <div>
                <span className="text-slate-500 block mb-0.5"># 5. Run it directly with injected environment and tools</span>
                <span className="text-emerald-400 font-semibold">avm dev</span>
              </div>
            </div>

            <button
              onClick={() =>
                onCopy(
                  'avm init\navm alias add dev "pnpm run dev"\navm plugin add node\navm node use 20.11.1\navm dev'
                )
              }
              className="absolute top-5 right-5 p-2 rounded-lg bg-slate-800 hover:bg-slate-700 text-slate-300 border border-slate-700 hover:border-slate-600 transition-colors shadow-sm"
              title="Copy all commands"
              aria-label="Copy quickstart commands"
            >
              <Copy className="w-4 h-4" />
            </button>
          </div>

          <div className="flex flex-col sm:flex-row sm:items-center justify-between gap-4 p-5 rounded-xl border border-slate-700/80 bg-slate-900/50 text-[13px]">
            <div>
              <span className="font-semibold text-slate-100 block mb-1">
                Shell setup for transparent command interception
              </span>
              <span className="text-slate-400 text-xs">
                Directly resolves plain commands like <code className="text-emerald-400 bg-slate-800 px-1.5 py-0.5 rounded">node</code> through avm shims.
              </span>
            </div>
            <div className="shrink-0 flex items-center gap-3 bg-slate-800/90 border border-slate-700 px-3.5 py-2 rounded-lg font-mono text-emerald-400 text-xs shadow-sm">
              <span>eval "$(avm shell-init)"</span>
              <button
                onClick={() => onCopy('eval "$(avm shell-init)"')}
                className="hover:text-emerald-300 p-1 text-slate-400 hover:text-white transition-colors"
                title="Copy shell init"
                aria-label="Copy shell init command"
              >
                <Copy className="w-3.5 h-3.5" />
              </button>
            </div>
          </div>
        </section>

        {/* ===================== CONFIG SPEC ===================== */}
        <section className="w-full max-w-5xl mx-auto px-4 sm:px-6 lg:px-8 pb-24">
          <div className="text-center mb-10">
            <p className="text-[11px] font-mono tracking-widest text-emerald-400 uppercase mb-2">SPECIFICATION</p>
            <h2 className="font-sans font-bold text-2xl sm:text-3xl text-slate-100 mb-2">The .avm.json spec</h2>
            <p className="text-[13px] text-slate-400">One clean JSON document replacing a pile of fragmented dotfiles.</p>
          </div>

          <div className="grid grid-cols-1 lg:grid-cols-2 gap-6 items-stretch">
            <div className="relative rounded-xl bg-slate-900/90 border border-slate-700/80 p-6 font-mono text-[12px] text-slate-300 shadow-xl flex flex-col justify-between">
              <pre className="overflow-x-auto leading-relaxed">
{`{
  "aliases": {
    "dev": "pnpm run dev",
    "release": "npm run release $1"
  },
  "env": {
    "NODE_ENV": "development",
    "API_URL": "https://api.local"
  },
  "tools": {
    "node": "20.11.1"
  }
}`}
              </pre>
              <button
                onClick={() =>
                  onCopy('{\n  "aliases": {\n    "dev": "pnpm run dev",\n    "release": "npm run release $1"\n  },\n  "env": {\n    "NODE_ENV": "development",\n    "API_URL": "https://api.local"\n  },\n  "tools": {\n    "node": "20.11.1"\n  }\n}'
                  )
                }
                className="absolute top-5 right-5 p-2 rounded-lg bg-slate-800 hover:bg-slate-700 text-slate-300 border border-slate-700 transition-colors shadow-sm"
                title="Copy .avm.json template"
                aria-label="Copy config JSON"
              >
                <Copy className="w-4 h-4" />
              </button>
            </div>

            <div className="flex flex-col gap-3 justify-between">
              <div className="p-4 rounded-xl border border-slate-700/80 bg-slate-900/50">
                <span className="font-semibold text-emerald-400 text-sm block mb-1">Local overrides global</span>
                <span className="text-slate-300 text-xs leading-relaxed">
                  Local <code className="text-slate-100 bg-slate-800 px-1 py-0.5 rounded">./.avm.json</code> overrides machine-wide{' '}
                  <code className="text-slate-100 bg-slate-800 px-1 py-0.5 rounded">~/.avm.json</code> cleanly.
                </span>
              </div>
              <div className="p-4 rounded-xl border border-slate-700/80 bg-slate-900/50">
                <span className="font-semibold text-emerald-400 text-sm block mb-1">Env vars merge</span>
                <span className="text-slate-300 text-xs leading-relaxed">
                  Environment variables merge transparently, with local values taking precedence over global ones.
                </span>
              </div>
              <div className="p-4 rounded-xl border border-slate-700/80 bg-slate-900/50">
                <span className="font-semibold text-emerald-400 text-sm block mb-1">Tool lookup order</span>
                <span className="text-slate-300 text-xs leading-relaxed">
                  Local pin first, then global pin, then host system binary with a safe warning.
                </span>
              </div>
              <div className="p-4 rounded-xl border border-slate-700/80 bg-slate-900/50">
                <span className="font-semibold text-emerald-400 text-sm block mb-1">Legacy compatibility</span>
                <span className="text-slate-300 text-xs leading-relaxed">
                  Older flat-map configuration files are recognized and migrated automatically.
                </span>
              </div>
            </div>
          </div>
        </section>

        {/* ===================== CORE CAPABILITIES ===================== */}
        <section className="w-full max-w-5xl mx-auto px-4 sm:px-6 lg:px-8 pb-24">
          <div className="text-center mb-10">
            <p className="text-[11px] font-mono tracking-widest text-emerald-400 uppercase mb-2">CORE CAPABILITIES</p>
            <h2 className="font-sans font-bold text-2xl sm:text-3xl text-slate-100">What avm handles today</h2>
          </div>

          <div className="grid grid-cols-1 sm:grid-cols-2 lg:grid-cols-3 gap-5">
            {corePillars.map((pillar, idx) => {
              const Icon = pillar.icon;
              return (
                <div
                  key={idx}
                  className="p-5 rounded-xl border border-slate-700/80 bg-slate-900/50 hover:border-slate-600 transition-all hover:shadow-md"
                >
                  <div className="w-8 h-8 rounded-md bg-emerald-500/10 border border-emerald-500/20 flex items-center justify-center mb-3">
                    <Icon className="w-4 h-4 text-emerald-400" />
                  </div>
                  <p className="text-[14px] font-semibold text-slate-100 mb-1.5">{pillar.title}</p>
                  <p className="text-[12px] leading-relaxed text-slate-400">{pillar.desc}</p>
                </div>
              );
            })}
          </div>
        </section>

        {/* ===================== UNDER THE HOOD ===================== */}
        <section className="w-full max-w-5xl mx-auto px-4 sm:px-6 lg:px-8 pb-24">
          <div className="text-center mb-10">
            <p className="text-[11px] font-mono tracking-widest text-emerald-400 uppercase mb-2">UNDER THE HOOD</p>
            <h2 className="font-sans font-bold text-2xl sm:text-3xl text-slate-100 mb-2">Modular crate architecture</h2>
            <p className="text-[13px] text-slate-400">
              Runtime providers (node, java, android) are independent plugins fetched on demand &mdash; never bloated into the CLI binary.
            </p>
          </div>

          <div className="grid grid-cols-1 gap-3">
            {workspaceCrates.map((crate, idx) => (
              <div
                key={idx}
                className="p-4 sm:p-5 rounded-xl border border-slate-700/80 bg-slate-900/50 flex flex-col sm:flex-row sm:items-baseline justify-between gap-2 sm:gap-6 hover:border-slate-600 transition-colors"
              >
                <span className="font-mono text-xs sm:text-sm font-semibold text-emerald-400 shrink-0">
                  {crate.name}
                </span>
                <span className="text-[12px] sm:text-[13px] text-slate-300 flex-1">
                  {crate.desc}
                </span>
              </div>
            ))}
          </div>
        </section>

        {/* ===================== BOTTOM CTA ===================== */}
        <section className="w-full max-w-4xl mx-auto px-4 sm:px-6 lg:px-8 pb-24">
          <div className="rounded-2xl border border-slate-700/80 bg-gradient-to-b from-slate-900/80 to-slate-950 p-8 sm:p-10 text-center flex flex-col items-center gap-6 shadow-2xl">
            <div className="max-w-xl">
              <h2 className="font-sans font-bold text-2xl sm:text-3xl text-slate-100 mb-3">
                Ready to supercharge your workspace?
              </h2>
              <p className="text-[14px] text-slate-300 leading-relaxed">
                Explore precompiled runtime plugins for Node.js, OpenJDK Temurin, and the Android SDK in the marketplace with instant sub-millisecond execution and zero compilation overhead.
              </p>
            </div>
            <div className="flex flex-wrap items-center justify-center gap-4">
              <Link
                to="/marketplace"
                className="px-6 py-3 rounded-lg bg-emerald-400 text-slate-950 font-sans font-semibold text-sm hover:bg-emerald-300 transition-colors shadow-lg shadow-emerald-500/10 flex items-center gap-2"
              >
                <span>Explore Marketplace</span>
                <ArrowRight className="w-4 h-4" />
              </Link>
              <Link
                to="/docs"
                className="px-6 py-3 rounded-lg border border-slate-700 text-slate-200 font-sans font-semibold text-sm hover:border-slate-500 hover:text-white transition-colors"
              >
                Read Documentation
              </Link>
            </div>
          </div>
        </section>

      </div>
    </>
  );
};
