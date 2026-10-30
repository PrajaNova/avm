import raw from '../../../registry.json';

type RegistryEntry = { repo: string; desc: string; tags: string[] };

// Display names for plugins whose title is not just the capitalised name.
const SECTION_LABELS: Record<string, string> = { java: 'OpenJDK', android: 'Android SDK' };

/** The registry file (name -> { repo, desc, tags }) as the list the site renders. */
export const registry = {
  plugins: Object.entries(raw as Record<string, RegistryEntry>).map(([name, entry]) => ({
    name,
    description: entry.desc,
    tags: entry.tags,
    // `owner/repo`, which the plugin pages append to https://github.com/.
    repo: entry.repo.replace('https://github.com/', '').replace(/\.git$/, '').replace(/\/$/, ''),
    section_label: SECTION_LABELS[name] ?? name.charAt(0).toUpperCase() + name.slice(1),
  })),
};
