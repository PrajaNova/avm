export interface FeatureGuide {
  title: string;
  intro: string;
  sections: { title: string; paragraphs: string[]; code?: string }[];
}

// User-facing behavior is checked against the CLI parser and docs/ops guides.
export const FEATURE_GUIDES: Record<string, FeatureGuide> = {
  configuration: {
    title: 'Configuration, Profiles & Monorepos',
    intro: 'Keep team tools, aliases and environment variables in .avm.json. Parent directories supply shared settings; each child can override individual keys.',
    sections: [
      { title: 'Project configuration', paragraphs: ['Use valid JSON: aliases, env and tools map names to strings. root is a boolean. Comments are not accepted. This example establishes a project boundary and a Node version range.'], code: '{\n  "root": true,\n  "tools": { "node": "22" },\n  "aliases": { "dev": "npm run dev", "build": "npm run build" },\n  "env": { "NODE_ENV": "development" }\n}' },
      { title: 'Merge order', paragraphs: ['Global ~/.avm.json supplies defaults. Project JSON files merge from parents to the current directory; nearer directories win for each key. Within each directory, the order is .avm.json, selected .avm.<profile>.json files in profile order, then .avm.local.json.', 'All child layers override all parent layers. Missing keys are inherited. A base .avm.json with root: true includes that directory and stops JSON traversal above it; global defaults still apply. It does not stop the separate searches for .env, version files or .avm.lock.'] },
      { title: 'Select environment profiles', paragraphs: ['AVM_ENV selects profiles for subsequent commands and shell refreshes. The global --env option overrides it for one invocation. Later profiles win on shared keys; --env with an empty string selects no profiles. Names may contain letters, digits, underscores and hyphens.'], code: '# Bash / zsh\nexport AVM_ENV=ci,staging\navm which node\navm --env production run build\navm --env "" list\n\n# PowerShell\n$env:AVM_ENV = "ci,staging"' },
      { title: 'Personal overrides and edits', paragraphs: ['Put personal overrides in .avm.local.json; avm init adds that filename to .gitignore. Config-editing commands write the current directory’s base .avm.json, or ~/.avm.json with --global. They do not edit a selected profile.', 'Review and trust every contributing file that supplies aliases or env. Trust parent files explicitly. Use which and list to inspect the winning values and their origins.'], code: 'avm --env ci trust\navm trust ../.avm.json\navm which build\navm which node\navm list' },
      { title: 'Global settings', paragraphs: ['trusted_paths and idiomatic_version_files are read from the global base ~/.avm.json only. trusted_paths grants trust to matching directories; use it only for repositories you control. Set idiomatic_version_files to false to disable tool-specific discovery such as .nvmrc and .java-version; .tool-versions remains supported.', 'Legacy flat alias maps are accepted and converted to the structured format on the next save. Invalid JSON causes an error and is not replaced with an empty configuration.'], code: '{\n  "tools": { "node": "22" },\n  "trusted_paths": ["~/work/my-project/**"],\n  "idiomatic_version_files": true\n}' },
    ],
  },
  lockfiles: {
    title: 'Lockfiles & CI Installs',
    intro: 'Commit .avm.lock to keep fuzzy project selections on exact releases and verify the downloaded archives during installation.',
    sections: [
      { title: 'Install the selected tools', paragraphs: ['Install the required plugins first. avm install installs all effective tool selections without changing pins. Unlocked installs include global selections; Java installs first when selected so Android can use a managed JDK.'], code: 'avm plugin add node\navm plugin add java\navm install' },
      { title: 'Create a project lock', paragraphs: ['avm lock resolves releases and checksum metadata without installing runtimes. It writes sorted JSON atomically. It updates the nearest existing lock, or creates one next to the nearest project config, resolving selections from that directory.', 'Locks include project JSON and version-file selections, excluding personal global pins. By default only the host platform is included; --platform selects the explicit comma-separated set. Commit the resulting .avm.lock.'], code: 'avm lock\navm lock --platform linux_amd64,darwin_arm64,windows_amd64' },
      { title: 'Frozen installation in CI', paragraphs: ['--frozen fails when the lock is missing, selected tool requests differ, or this platform has no locked asset. The installed native plugins must match the locked name and version; executable checksums are checked where recorded. All selections and plugin checks are validated before installation starts.', 'Use the same profile when creating and consuming the lock. Trust project aliases/env only after reviewing them. CI=true does not grant trust automatically.'], code: 'avm --env ci lock\n# Commit .avm.lock, then on the CI machine:\navm --env ci install --frozen\navm --env ci trust\navm --env ci run build' },
      { title: 'Updates and limits', paragraphs: ['With a valid lock, install, aliases, shims, env and which use its exact versions. Changed tool requests produce an error directing you to avm lock; run it explicitly to regenerate. Environment and alias edits alone do not invalidate the lock.', 'Node and Java support verified locked installs. Android and legacy asdf plugins support ordinary unlocked installs only. Plugin executable hashes are recorded for the host that creates the lock; other platforms warn and check plugin name/version when their hash is absent.', 'Existing installed runtimes are reused without scanning every file. New archives are verified before extraction, and AVM_ALLOW_UNVERIFIED cannot bypass a locked checksum. A later download failure may leave earlier successful installs in place.'] },
    ],
  },
  'temporary-runtimes': {
    title: 'Temporary Runtimes',
    intro: 'Try a tool version for one command or alias while preserving project pins, global pins and lockfiles.',
    sections: [
      { title: 'Run a command', paragraphs: ['Install the provider plugin first. avm exec (also avm x) resolves each tool@version, installs a missing runtime, and supplies its executable directory and provider environment to the child process. Arguments after -- are passed to the command.'], code: 'avm exec node@20 -- node --version\navm x node@22 java@21 -- npm run build\navm exec --no-install node@20 -- node --version' },
      { title: 'Run a project alias', paragraphs: ['Use --with before the alias arguments; repeat it for multiple tools. Trusted project environment variables apply. --no-install fails if the selected version is unavailable locally. Temporary selections resolve independently of project lock pins.'], code: 'avm run --with node@20 test\navm run --with node@22 --with java@21 build\navm run --no-install --with node@20 test' },
    ],
  },
  maintenance: {
    title: 'Updates & Cleanup',
    intro: 'Inspect runtime and plugin updates, control major-version changes, and remove unused managed runtimes.',
    sections: [
      { title: 'Inspect and upgrade runtimes', paragraphs: ['outdated reports the requested spec, current selection, newest match in range, newest release overall and pin source. --json prints machine-readable output. upgrade defaults to the current range (the major line for an exact pin); select tool names to narrow it.', '--bump allows crossing a major version and asks for confirmation; -y confirms in scripts. --dry-run previews changes. Inherited, profile and version-file pins remain at their source: bump tells you to edit them there. Regenerate .avm.lock after intentional pin changes.'], code: 'avm outdated\navm outdated --json\navm upgrade node --dry-run\navm upgrade node\navm upgrade node --bump --dry-run\navm upgrade node --bump -y\navm lock' },
      { title: 'Update plugins and avm', paragraphs: ['Plugin updates are separate from runtime updates. Use --all to update every installed plugin. self-update replaces direct installations after checksum verification; package-manager installations receive the appropriate upgrade command instead.', 'AVM_NO_UPDATE_CHECK=1 disables the background update notice. GITHUB_TOKEN or GH_TOKEN can help with GitHub API rate limits. Updating a plugin may require regenerating project locks that pin its identity.'], code: 'avm plugin outdated --json\navm plugin update node\navm plugin update --all\navm self-update\navm self-update --version 0.4.1' },
      { title: 'Prune unused versions', paragraphs: ['prune protects global pins and pins from tracked projects, including inactive profiles. It previews removal candidates and asks before deleting; --dry-run makes no changes and -y confirms.', 'Use --older-than 90d to limit candidates by last use. --include-unrecorded also considers versions with no recorded usage. AVM cannot protect a project it has never tracked: inspect the preview before confirming. Unreadable tracked configuration aborts pruning.'], code: 'avm prune --dry-run\navm prune --older-than 90d --dry-run\navm prune --older-than 90d -y\navm prune --include-unrecorded --dry-run' },
    ],
  },
  environment: {
    title: 'Environment Variables',
    intro: 'Combine project settings with provider variables such as JAVA_HOME, ANDROID_HOME and ANDROID_SDK_ROOT.',
    sections: [
      { title: 'Manage configured values', paragraphs: ['env add and env remove edit the local base configuration, or the global base with -g. env list shows merged configured variables; bare env prints shell statements including provider variables and PATH integration. Project aliases/env and .env files require trust.'], code: 'avm env add NODE_ENV development\navm env add -g EDITOR vim\navm env list\navm env remove NODE_ENV\navm env --shell sh\navm env --shell pwsh\navm env --shell cmd' },
      { title: 'Shell refresh and profiles', paragraphs: ['Activate shell-init to apply the printed environment automatically. Bash and PowerShell refresh at prompts; zsh also refreshes on directory changes. The wrapper refreshes around avm commands. Leaving a project restores the original value or unsets a previously absent variable.', 'avm env --env production prints exports for that invocation only. Set AVM_ENV in the shell to select the same profile for subsequent commands. cmd.exe has a forwarding avm command but does not provide automatic shell refresh.'] },
      { title: 'Storage and diagnostics', paragraphs: ['AVM_HOME changes the managed storage root (normally ~/.avm); AVM_PLUGIN_DIR overrides the plugin discovery directory. AVM_DEBUG=1 enables additional diagnostics.', 'AVM_PLUGIN_INSTALL_TIMEOUT is measured in seconds and defaults to 1800, covering plugin download, verification and extraction. Android has additional timeout controls described in its plugin guide.'] },
    ],
  },
  windows: {
    title: 'Windows & IDE Setup',
    intro: 'Use AVM on Windows x64 with native Node, Java and Android providers, PowerShell integration, and executable shims for IDEs.',
    sections: [
      { title: 'PowerShell', paragraphs: ['The PowerShell installer adds avm-bin to the user PATH and installs a shell hook in your profile. Restart your terminal after installation. Re-run shell setup in an existing session after upgrading.'], code: 'irm https://raw.githubusercontent.com/PrajaNova/avm/main/install.ps1 | iex\nInvoke-Expression ((& avm-bin shell-init pwsh) -join "`n")\navm plugin add node\navm node install 22\nnode --version' },
      { title: 'cmd.exe and IDEs', paragraphs: ['The installer includes avm.bat for cmd.exe command forwarding. Automatic environment refresh requires PowerShell; use avm env --shell cmd to inspect CMD assignments.', 'Shims are .exe launchers on Windows so IDEs can invoke managed tools. Add the directory printed by shims path to your Windows user PATH, or configure your IDE to use those executables. Restart IDEs and terminals afterward. shims install regenerates launchers, including global package commands. shims activate edits Unix shell startup files; it does not update the Windows user PATH.'], code: 'avm shims install\navm shims path\navm which node' },
      { title: 'Host tools', paragraphs: ['The core binary needs no Node or Python runtime. Installation workflows use host utilities such as curl and tar; Unix Android installs also need unzip. Android sdkmanager requires a working JDK, and emulator acceleration depends on the host.', 'Interactive version pickers support Windows consoles. For scripts or terminals without interactive input, use explicit install/use commands.'] },
    ],
  },
};
