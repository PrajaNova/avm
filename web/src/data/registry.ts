import raw from '../../../registry.json';

/** One plugin as listed in the repo's registry.json; `repo` is `owner/repo`, which the plugin pages append to https://github.com/. */
export type RegistryPlugin = { name: string; description: string; section_label: string; repo: string };

/** The registry file, as published at the repo root (`{ schema_version, plugins: [...] }`). */
export const registry: { schema_version: number; plugins: RegistryPlugin[] } = raw;
