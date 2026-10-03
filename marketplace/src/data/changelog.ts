// avm's CHANGELOG.md, fetched live so a new release shows up here without
// redeploying the site (deploys are manual).
export const CHANGELOG_URL = 'https://raw.githubusercontent.com/PrajaNova/avm/main/CHANGELOG.md';
export const CHANGELOG_SOURCE = 'https://github.com/PrajaNova/avm/blob/main/CHANGELOG.md';

export interface ChangelogRelease {
  version: string; // "0.4.0-beta-1", or "Unreleased"
  date?: string;
  groups: { heading: string; items: string[] }[];
}

/** Parse Keep a Changelog markdown: `## [version] - date`, `### Group`, `- item` (wrapped lines join the item). */
export function parseChangelog(md: string): ChangelogRelease[] {
  const releases: ChangelogRelease[] = [];
  let release: ChangelogRelease | undefined;
  let group: ChangelogRelease['groups'][number] | undefined;

  for (const line of md.split('\n')) {
    const version = line.match(/^## \[([^\]]+)\](?:\s*-\s*(\S+))?/);
    if (version) {
      release = { version: version[1], date: version[2], groups: [] };
      releases.push(release);
      group = undefined;
      continue;
    }
    if (!release) continue;
    const heading = line.match(/^### (.+)/);
    if (heading) {
      group = { heading: heading[1].trim(), items: [] };
      release.groups.push(group);
      continue;
    }
    const item = line.match(/^[-*] (.+)/);
    if (item && group) {
      group.items.push(item[1].trim());
    } else if (line.trim() && group?.items.length) {
      group.items[group.items.length - 1] += ' ' + line.trim();
    }
  }
  return releases.filter((r) => r.groups.some((g) => g.items.length));
}

/** Display order and labels; unknown headings keep their own name, last. */
export const GROUP_ORDER: Record<string, { label: string; rank: number }> = {
  Added: { label: 'New features', rank: 0 },
  Security: { label: 'Security', rank: 1 },
  Changed: { label: 'Changed', rank: 2 },
  Fixed: { label: 'Fixed', rank: 3 },
  Deprecated: { label: 'Deprecated', rank: 4 },
  Removed: { label: 'Removed', rank: 5 },
};
