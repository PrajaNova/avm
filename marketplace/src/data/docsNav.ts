// Docs navigation: the single list of docs pages used by the sidebar, routes, prerendering and the sitemap.
export interface SidebarItem {
  id: string;
  text: string;
  link: string;
  external?: boolean;
  hasVideo?: boolean;
  badge?: string;
}

export interface SidebarGroup {
  text: string;
  items: SidebarItem[];
}

export const ASDF_SIDEBAR_CONFIG: SidebarGroup[] = [
  {
    text: 'Guide',
    items: [
      { id: 'introduction', text: 'What is avm?', link: '/docs/guide/introduction' },
      { id: 'getting-started', text: 'Getting Started', link: '/docs/guide/getting-started' },
      { id: 'shell-setup', text: 'Shell Setup & Shims', link: '/docs/guide/shell-setup' },
    ],
  },
  {
    text: 'Usage',
    items: [
      { id: 'usage-core', text: 'Core (CLI & Precedence)', link: '/docs/manage/core' },
      { id: 'usage-plugins', text: 'Plugins (Add, List, Update)', link: '/docs/manage/plugins' },
      { id: 'usage-versions', text: 'Versions (Browse, Install, Pin)', link: '/docs/manage/versions' },
      { id: 'configuration', text: 'Config, Profiles & Monorepos', link: '/docs/manage/configuration' },
      { id: 'lockfiles', text: 'Lockfiles & CI', link: '/docs/manage/lockfiles' },
      { id: 'temporary-runtimes', text: 'Temporary Runtimes', link: '/docs/manage/temporary-runtimes' },
      { id: 'environment', text: 'Environment Variables', link: '/docs/manage/environment' },
      { id: 'maintenance', text: 'Updates & Cleanup', link: '/docs/manage/maintenance' },
      { id: 'windows', text: 'Windows & IDEs', link: '/docs/manage/windows' },
      { id: 'usage-aliases', text: 'Aliases & Scripts Discovery', link: '/docs/manage/aliases' },
      { id: 'version-files', text: 'Version Files (.nvmrc, .tool-versions)', link: '/docs/manage/version-files', badge: 'New' },
      { id: 'security', text: 'Security & Trust', link: '/docs/manage/security', badge: 'New' },
    ],
  },
  {
    text: 'Reference',
    items: [
      { id: 'commands', text: 'All Commands', link: '/docs/manage/commands' },
      { id: 'dependencies', text: 'Dependencies & Host Env', link: '/docs/manage/dependencies' },
      { id: 'architecture', text: 'Architecture & Wire Protocol', link: '/docs/manage/architecture' },
      { id: 'comparison', text: 'Comparison Matrix', link: '/docs/manage/comparison' },
    ],
  },
  {
    text: 'Questions',
    items: [
      { id: 'faq', text: 'FAQ', link: '/docs/more/faq' },
      { id: 'troubleshooting', text: 'Troubleshooting & Diagnostics', link: '/docs/more/troubleshooting' },
      {
        id: 'github-issues',
        text: 'GitHub Issues',
        link: 'https://github.com/PrajaNova/avm/issues',
        external: true,
      },
    ],
  },
  {
    text: 'Contribute',
    items: [
      { id: 'contribute-core', text: 'Core avm', link: '/docs/contribute/core' },
      { id: 'contribute-docs', text: 'Documentation', link: '/docs/contribute/documentation' },
    ],
  },
];
