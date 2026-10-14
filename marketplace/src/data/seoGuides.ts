import type { FeatureGuide } from './featureGuides';

// Search-focused articles. Every claim is limited to what the docs and the comparison table already state.
export interface SeoGuide extends FeatureGuide {
  slug: string;
  description: string;
  related: { label: string; to: string }[];
}

export const SEO_GUIDES: SeoGuide[] = [
  {
    slug: 'nvm-alternative',
    title: 'An nvm alternative for Node.js, Java and Android',
    description: 'Looking for an nvm alternative? AVM reads your .nvmrc, switches Node.js per project, and also manages Java and the Android SDK from one tool.',
    intro: 'nvm manages Node.js and nothing else. If your projects also need a JDK or the Android SDK, or you want versions to follow the project directory without extra scripts, AVM covers all three and reads the version files you already have.',
    related: [
      { label: 'Version files reference', to: '/docs/manage/version-files' },
      { label: 'Compare AVM with asdf, mise and others', to: '/docs/manage/comparison' },
      { label: 'Switch Node.js versions per project', to: '/guides/switch-node-versions-per-project' },
    ],
    sections: [
      { title: 'Why people move away from nvm', paragraphs: ['nvm is a shell script focused on Node.js. Java and the Android SDK need separate tools, and each tool has its own install, switching and configuration habits.', 'AVM handles Node.js, Java (Temurin OpenJDK) and the Android SDK through plugins with one command style, one project file (.avm.json) and one lockfile format for Node.js and Java.'] },
      { title: 'Keep your .nvmrc', paragraphs: ['You do not need to rewrite anything. AVM reads .nvmrc, .node-version and package.json (volta.node, or the engines.node range) by default, searching upward from the current directory. Partial specs such as 20 or lts/* resolve to the newest installed match.', 'Run avm which to see which file decided the version.'], code: '$ avm which node\ntool \'node\': 20.11.1 (from ./.nvmrc)' },
      { title: 'Install and switch', paragraphs: ['Install AVM, add the shell hook, add the Node.js plugin and install a version. avm node use pins it in the project; add --global to set your machine default.'], code: 'echo \'eval "$(avm shell-init)"\' >> ~/.zshrc\nsource ~/.zshrc\navm plugin add node\navm node install 22\navm node use 22' },
      { title: 'What you gain over nvm', paragraphs: ['One tool for Node.js, Java and the Android SDK. Checksum-verified downloads. A committed .avm.lock with exact versions and archive SHA-256 for Node.js and Java, enforced in CI with avm install --frozen. Native Windows support with PowerShell integration.', 'Project aliases and environment variables live next to the versions in .avm.json, and cloned projects stay inactive until you run avm trust.'] },
      { title: 'What to know before switching', paragraphs: ['AVM keeps its own installs under ~/.avm, separate from nvm, so install the versions you need again with avm node install. If a pinned version is not installed, AVM falls back to the system binary with a warning instead of failing.', 'Android and legacy asdf plugins support ordinary installs only, not lockfiles.'] },
    ],
  },
  {
    slug: 'switch-node-versions-per-project',
    title: 'Switch Node.js versions per project',
    description: 'Pin a different Node.js version in each repository and have it applied automatically when you enter the folder, with AVM and a committed .avm.json.',
    intro: 'Different repositories need different Node.js versions. Pin the version in each project once, commit it, and every teammate and CI job uses the same one, without remembering to run a switch command.',
    related: [
      { label: 'Configuration, profiles and monorepos', to: '/docs/manage/configuration' },
      { label: 'Lockfiles and CI', to: '/docs/manage/lockfiles' },
      { label: 'An nvm alternative', to: '/guides/nvm-alternative' },
    ],
    sections: [
      { title: 'Pin the version in the project', paragraphs: ['Run avm node use inside the repository. It writes the version to .avm.json in that directory, so the choice travels with the code.'], code: 'cd my-app\navm plugin add node\navm node install 22.14.0\navm node use 22.14.0' },
      { title: 'It follows the directory', paragraphs: ['AVM searches upward from the current directory, nearest file first. Entering another project with a different pin switches Node.js for that project; leaving it falls back to your global default in ~/.avm.json. Plain commands such as node and npm are intercepted through shims, so the right version runs without extra steps.'] },
      { title: 'Use files you already have', paragraphs: ['If the repository has .nvmrc, .node-version, .tool-versions or an engines.node range in package.json, AVM reads them without any migration. Add .avm.json only when you want to override or add tools such as Java.'] },
      { title: 'Try another version without changing the pin', paragraphs: ['avm exec runs one command with a temporary version. Project and global pins and lockfiles stay unchanged; a missing runtime is installed automatically unless you pass --no-install.'], code: 'avm exec node@18 -- npm test\navm x node@22 java@21 -- ./gradlew build' },
      { title: 'Make it reproducible for the team', paragraphs: ['Lock fuzzy versions such as 22 to exact releases and commit .avm.lock. New teammates run avm install, and CI runs avm install --frozen, which rejects stale configuration and verifies the locked archive checksums.'], code: 'avm lock\ngit add .avm.json .avm.lock\n# new machine or CI\navm install --frozen' },
    ],
  },
  {
    slug: 'manage-java-versions',
    title: 'Manage multiple Java versions on macOS, Linux and Windows',
    description: 'Install several JDKs, pin one per project and get JAVA_HOME set automatically, using AVM and Temurin OpenJDK builds on macOS, Linux and Windows.',
    intro: 'Projects often need different JDKs, and setting JAVA_HOME by hand is easy to get wrong. AVM installs Temurin OpenJDK builds, pins one version per project and sets JAVA_HOME for you.',
    related: [
      { label: 'Java plugin', to: '/marketplace/java' },
      { label: 'Environment variables', to: '/docs/manage/environment' },
      { label: 'Lockfiles and CI', to: '/docs/manage/lockfiles' },
    ],
    sections: [
      { title: 'Install the Java plugin and a JDK', paragraphs: ['The Java plugin downloads Eclipse Temurin builds. List what is available, then install a major or exact version.'], code: 'avm plugin add java\navm java versions\navm java install 21' },
      { title: 'Pin a JDK per project', paragraphs: ['avm java use writes the version to .avm.json for the current repository; add --global for a machine default. AVM also reads .java-version and .sdkmanrc, so existing projects work unchanged.'], code: 'avm java use 21\navm java use 17 --global' },
      { title: 'JAVA_HOME without editing profiles', paragraphs: ['The plugin provides JAVA_HOME for the selected version when commands run through AVM, so you do not export it in your shell profile. Check the result with avm which java.'] },
      { title: 'Build with another JDK once', paragraphs: ['Run a single command against a different JDK without touching your pins.'], code: 'avm x java@17 -- ./gradlew build' },
      { title: 'Lock the JDK for CI', paragraphs: ['Java supports lockfiles. avm lock records exact releases and archive checksums for each platform, and avm install --frozen verifies them.'], code: 'avm lock --platform linux_amd64,darwin_arm64,windows_amd64\navm install --frozen' },
    ],
  },
];
