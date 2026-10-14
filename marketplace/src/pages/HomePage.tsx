import { Link } from 'react-router';
import { ArrowRight, Check, Terminal, FileCode2, ShieldCheck, Layers, GitBranch, ArrowUpRight } from 'lucide-react';
import { SEO_GUIDES } from '../data/seoGuides';
import { Platforms } from '../components/OsIcons';
import { TerminalInstallBox } from '../components/TerminalInstallBox';

export const HomePage = ({ onCopy }: { onCopy: (text: string) => void }) => (
  <div>
    <section className="max-w-7xl mx-auto px-5 sm:px-8 pt-16 lg:pt-24 pb-16 lg:pb-24 grid lg:grid-cols-[1.05fr_1fr] gap-12 lg:gap-20 items-center">
      <div>
        <p className="eyebrow mb-6">One workspace. All your runtimes.</p>
        <h1 className="text-5xl sm:text-6xl lg:text-[68px] leading-[1.06] tracking-[-0.045em] font-medium text-ink mb-6">The right runtime.<br /><span className="text-accent">In every project.</span></h1>
        <p className="text-lg text-secondary max-w-lg mb-8">Manage Node.js, Java, and Android SDK versions alongside your project commands and environment. Set it once. Get back to building.</p>
        <div className="flex flex-wrap gap-3"><Link to="/docs/guide/getting-started" className="button-primary">Get started <ArrowRight className="w-4 h-4" /></Link><Link to="/docs/guide/introduction" className="button-secondary">Read the docs</Link></div>
        <div className="flex flex-wrap gap-x-5 gap-y-2 mt-7 text-sm text-secondary"><Platforms /><span className="flex items-center gap-1.5"><Check className="w-4 h-4 text-accent" /> Open source · MIT</span></div>
      </div>
      <div className="min-w-0">
        <div className="rounded-lg border border-line bg-surface shadow-sm overflow-hidden">
          <div className="flex items-center justify-between px-5 py-3 border-b border-line text-sm text-secondary"><span className="flex items-center gap-2"><FileCode2 className="w-4 h-4" /> .avm.json</span><span>your-project /</span></div>
          <pre className="p-6 text-sm leading-7 font-mono text-ink"><code>{'{\n  "tools": {\n    "node": "22",\n    "java": "21"\n  },\n  "aliases": {\n    "dev": "npm run dev"\n  }\n}'}</code></pre>
          <div className="terminal-surface p-6 border-t border-line font-mono text-sm leading-8">
            <div className="flex items-center gap-2 text-secondary text-xs mb-3"><Terminal className="w-4 h-4" /> A consistent starting point</div>
            <div><span className="text-accent select-none">$ </span>avm install</div>
            <div><span className="text-accent select-none">$ </span>avm lock</div>
            <div><span className="text-accent select-none">$ </span>avm run dev</div>
          </div>
        </div>
        <p className="text-sm text-secondary mt-4 flex items-center gap-2"><GitBranch className="w-4 h-4" /> Configuration your whole team can share.</p>
      </div>
    </section>

    <section id="install" className="border-y border-line bg-surface py-10">
      <div className="max-w-7xl mx-auto px-5 sm:px-8 flex flex-col gap-6">
        <div className="max-w-2xl"><p className="eyebrow mb-2">Start here</p><h2 className="text-2xl font-medium tracking-tight">Your next project, ready.</h2><p className="text-secondary mt-2 text-sm">Install AVM, then follow the <Link to="/docs/guide/shell-setup" className="text-accent underline underline-offset-4">shell setup guide</Link>.</p></div>
        <TerminalInstallBox onCopy={onCopy} />
      </div>
    </section>

    <section className="max-w-7xl mx-auto px-5 sm:px-8 py-20 lg:py-24">
      <div className="max-w-2xl mb-12"><p className="eyebrow mb-4">Less setup. More building.</p><h2 className="text-3xl sm:text-4xl font-medium tracking-tight">A small tool for the work<br className="hidden sm:block" /> around your code.</h2></div>
      <div className="grid md:grid-cols-3 gap-10">
        {[
          { icon: Layers, title: 'A version for every project', text: 'Keep different runtimes across repositories and packages. AVM reads existing version files and merges shared monorepo configuration.', link: '/docs/manage/configuration', label: 'Explore configuration' },
          { icon: GitBranch, title: 'A repeatable first day', text: 'Install selected tools in one command. Commit exact Node and Java versions with checksums, then enforce them in CI.', link: '/docs/manage/lockfiles', label: 'Reproducible installs' },
          { icon: Terminal, title: 'Your workflow, in one place', text: 'Keep aliases and environment variables beside your code. Try a different runtime for one command without changing your project pins.', link: '/docs/manage/temporary-runtimes', label: 'Temporary runtimes' },
        ].map(({ icon: Icon, title, text, link, label }, i) => <article key={title} className="border-t border-line pt-6"><div className="flex justify-between items-center text-accent mb-5"><Icon className="w-6 h-6" /><span className="font-mono text-xs text-secondary">0{i + 1}</span></div><h3 className="text-xl font-medium mb-3">{title}</h3><p className="text-secondary mb-5">{text}</p><Link to={link} className="text-sm text-accent inline-flex items-center gap-2 font-medium">{label}<ArrowRight className="w-4 h-4" /></Link></article>)}
      </div>
    </section>

    <section className="border-y border-line bg-surface py-16 lg:py-20">
      <div className="max-w-7xl mx-auto px-5 sm:px-8">
        <div className="flex flex-wrap items-end justify-between gap-5 mb-10"><div><p className="eyebrow mb-3">The tools you already use</p><h2 className="text-3xl sm:text-4xl font-medium tracking-tight">One home for your runtimes.</h2></div><Link to="/marketplace" className="text-accent text-sm flex items-center gap-2">Browse plugins <ArrowUpRight className="w-4 h-4" /></Link></div>
        <div className="grid md:grid-cols-3 gap-5">
          {[['node','Node.js','Runtime versions, package scripts, and global npm commands.','Verified lockfiles'],['java','Java','Temurin OpenJDK builds with automatic JAVA_HOME setup.','Verified lockfiles'],['android','Android SDK','SDK tools, system images, and virtual device management.','Standard installs']].map(([id,title,description,capability]) => <Link key={id} to={`/marketplace/${id}`} className="group p-6 border border-line rounded-lg hover:border-accent transition-colors"><div className="flex items-center justify-between mb-5"><h3 className="text-xl font-medium">{title}</h3><ArrowUpRight className="w-5 h-5 text-secondary group-hover:text-accent" /></div><p className="text-secondary mb-5">{description}</p><div className="space-y-2 text-sm"><span className="block text-accent">{capability}</span><Platforms className="text-secondary" /></div></Link>)}
        </div>
      </div>
    </section>

    <section className="max-w-7xl mx-auto px-5 sm:px-8 py-20 lg:py-24 grid lg:grid-cols-2 gap-12 lg:gap-24">
      <div><ShieldCheck className="w-7 h-7 text-accent mb-5" /><h2 className="text-3xl sm:text-4xl font-medium tracking-tight mb-5">Confidence comes<br />from clear guarantees.</h2><p className="text-secondary">Understand what gets installed, which configuration applies, and what your project is allowed to run.</p><Link to="/docs/manage/security" className="inline-flex items-center gap-2 text-accent text-sm font-medium mt-6">Read about security <ArrowRight className="w-4 h-4" /></Link></div>
      <div className="space-y-7">
        {[['Verified downloads','Native plugin archives and supported runtime downloads are checked before installation. Android components are verified by sdkmanager.'],['Explicit configuration trust','Cloned project aliases and environment settings stay inactive until you trust their contents. Tool selections still apply.'],['Clear limits, documented','Node and Java support frozen installs. Android and legacy asdf plugins currently use standard installs.']].map(([title,body]) => <div key={title} className="border-b border-line pb-6"><h3 className="text-lg font-medium mb-2">{title}</h3><p className="text-secondary text-sm">{body}</p></div>)}
      </div>
    </section>
    <section className="max-w-7xl mx-auto px-5 sm:px-8 pb-20">
      <h2 className="text-2xl sm:text-3xl font-medium tracking-tight mb-8">Guides</h2>
      <div className="grid md:grid-cols-3 gap-5">
        {SEO_GUIDES.map((guide) => <Link key={guide.slug} to={`/guides/${guide.slug}`} className="group p-6 border border-line rounded-lg hover:border-accent transition-colors"><h3 className="text-lg font-medium mb-3 group-hover:text-accent">{guide.title}</h3><p className="text-secondary text-sm">{guide.description}</p></Link>)}
      </div>
    </section>
    <section className="max-w-7xl mx-auto px-5 sm:px-8 pb-16"><div className="border-t border-line pt-10 flex flex-wrap items-center justify-between gap-6"><div><h2 className="text-2xl font-medium mb-2">Make your workspace feel settled.</h2><p className="text-secondary">Start with one project. Keep the version files you already have.</p></div><Link to="/docs/guide/getting-started" className="button-primary">Set up AVM <ArrowRight className="w-4 h-4" /></Link></div><div className="mt-8 flex flex-wrap gap-6 text-sm text-secondary"><Link to="/docs/manage/comparison" className="hover:text-accent">Compare version managers</Link><Link to="/docs/manage/architecture" className="hover:text-accent">Explore the architecture</Link><Link to="/changelog" className="hover:text-accent">Follow the changelog</Link></div></section>
  </div>
);
