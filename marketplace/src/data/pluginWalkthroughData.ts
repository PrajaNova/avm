import { WalkthroughStep } from '../components/WalkthroughVideo';

export const PLUGIN_WALKTHROUGHS: Record<string, { title: string; badge: string; steps: WalkthroughStep[] }> = {
  node: {
    title: 'Node.js Plugin Walkthrough',
    badge: 'avm-plugin-node',
    steps: [
      {
        id: 'node-install-plugin',
        stepNumber: '01',
        label: 'Add Plugin',
        command: 'avm plugin add node',
        comment: '# Fetch the precompiled native Node.js provider from the marketplace',
        output: 'Fetching \'node\' from PrajaNova/avm-plugin-node...\n✓ Installed node v0.2.0',
        timing: '0.9s'
      },
      {
        id: 'node-list-versions',
        stepNumber: '02',
        label: 'List Releases',
        command: 'avm node versions',
        comment: '# Queries nodejs.org/dist/index.json directly for stable & LTS releases',
        output: 'Available Node.js versions:\n  22.14.0 (LTS: Jod)\n  22.13.1 (LTS)\n  20.18.2 (LTS: Iron)\n  20.11.1 (LTS)\n  18.20.6 (LTS: Hydrogen)',
        timing: '210ms'
      },
      {
        id: 'node-install-version',
        stepNumber: '03',
        label: 'Download Runtime',
        command: 'avm node install 22.14.0',
        comment: '# Fast zero-compile binary fetch of the official Node distribution',
        output: 'Downloading https://nodejs.org/dist/v22.14.0/node-v22.14.0-darwin-arm64.tar.gz...\nExtracting to ~/.avm/tools/node/22.14.0...\n✓ Installed Node.js v22.14.0 (with npm 10.9.2, corepack, and shims)',
        timing: '1.8s'
      },
      {
        id: 'node-use-pin',
        stepNumber: '04',
        label: 'Pin Local Version',
        command: 'avm node use 22.14.0',
        comment: '# Pin Node version specifically for this project directory',
        output: '✓ Updated ./.avm.json:\n  "tools": {\n    "node": "22.14.0"\n  }\nDirectory activated: node v22.14.0',
        timing: '<1ms'
      },
      {
        id: 'node-package-scripts',
        stepNumber: '05',
        label: 'Auto Script Discovery',
        command: 'avm test',
        comment: '# Automatically discovers package.json scripts using detected package manager',
        output: 'Discovered package script "test" in ./package.json (detected pnpm)\n> vitest run\n\n ✓ tests/runtime.test.ts (8 tests)\n ✓ tests/resolver.test.ts (6 tests)\n Test Files  2 passed (2)\n      Tests  14 passed (14)\n   Duration  412ms',
        timing: '412ms'
      },
      {
        id: 'node-shim-verify',
        stepNumber: '06',
        label: 'Verify Shims',
        command: 'which node && node -v',
        comment: '# Transparent shim execution without running subshells or slow hooks',
        output: '/Users/dev/.avm/shims/node\nv22.14.0',
        timing: '<0.2ms'
      }
    ]
  },

  java: {
    title: 'OpenJDK Temurin Plugin Walkthrough',
    badge: 'avm-plugin-java',
    steps: [
      {
        id: 'java-install-plugin',
        stepNumber: '01',
        label: 'Add Plugin',
        command: 'avm plugin add java',
        comment: '# Install native OpenJDK provider powered by Eclipse Temurin & Foojay Disco API',
        output: 'Fetching \'java\' from PrajaNova/avm-plugin-java...\n✓ Installed java v0.2.0',
        timing: '0.8s'
      },
      {
        id: 'java-list-versions',
        stepNumber: '02',
        label: 'Browse JDKs',
        command: 'avm java versions',
        comment: '# Queries Foojay Disco API for certified Eclipse Temurin OpenJDK builds',
        output: 'Available OpenJDK versions (Eclipse Temurin):\n  openjdk-21.0.6+7  (LTS - Java 21)\n  openjdk-17.0.13+11 (LTS - Java 17)\n  openjdk-11.0.25+9  (LTS - Java 11)\n  openjdk-8u432-b06  (LTS - Java 8)',
        timing: '320ms'
      },
      {
        id: 'java-install-version',
        stepNumber: '03',
        label: 'Install OpenJDK 17',
        command: 'avm java install 17',
        comment: '# Bare major "17" automatically resolves to latest stable patch build',
        output: 'Resolved "17" → openjdk-17.0.13+11\nDownloading Eclipse Temurin tar.gz from github.com/adoptium/temurin17-binaries...\nExtracting to ~/.avm/tools/java/openjdk-17.0.13+11...\n✓ Successfully installed OpenJDK 17.0.13+11',
        timing: '3.1s'
      },
      {
        id: 'java-use-pin',
        stepNumber: '04',
        label: 'Pin Local Version',
        command: 'avm java use 17.0.13+11',
        comment: '# Pin OpenJDK specifically for this project directory',
        output: '✓ Updated ./.avm.json:\n  "tools": {\n    "java": "openjdk-17.0.13+11"\n  }\nDirectory activated: OpenJDK 17',
        timing: '<1ms'
      },
      {
        id: 'java-env-vars',
        stepNumber: '05',
        label: 'JAVA_HOME Injection',
        command: 'avm env',
        comment: '# Automatically exports and injects JAVA_HOME for Gradle, Maven, and Android builds',
        output: 'export JAVA_HOME="/Users/dev/.avm/tools/java/openjdk-17.0.13+11"\nexport PATH="/Users/dev/.avm/shims:$PATH"',
        timing: '<0.3ms'
      },
      {
        id: 'java-verify',
        stepNumber: '06',
        label: 'Verify Java',
        command: 'java -version',
        comment: '# Shims transparently route to the directory\'s pinned OpenJDK release',
        output: 'openjdk version "17.0.13" 2024-10-15\nOpenJDK Runtime Environment Temurin-17.0.13+11 (build 17.0.13+11)\nOpenJDK 64-Bit Server VM Temurin-17.0.13+11 (build 17.0.13+11, mixed mode)',
        timing: '<0.4ms'
      }
    ]
  },

  android: {
    title: 'Android SDK & Emulator Plugin Walkthrough',
    badge: 'avm-plugin-android',
    steps: [
      {
        id: 'android-install-plugin',
        stepNumber: '01',
        label: 'Add Plugin',
        command: 'avm plugin add android',
        comment: '# Install native Android SDK provider (cmdline-tools, platform-tools, avd manager)',
        output: 'Fetching \'android\' from PrajaNova/avm-plugin-android...\n✓ Installed android v0.2.0',
        timing: '1.0s'
      },
      {
        id: 'android-list-versions',
        stepNumber: '02',
        label: 'List API Levels',
        command: 'avm android versions',
        comment: '# Scans Google\'s official SDK repository index for stable platform levels',
        output: 'Available Android API Levels:\n  API 36  (Android 16)\n  API 35  (Android 15)\n  API 34  (Android 14 UpsideDownCake)\n  API 33  (Android 13 Tiramisu)\n  API 31  (Android 12 S)',
        timing: '280ms'
      },
      {
        id: 'android-install-sdk',
        stepNumber: '03',
        label: 'Install API 34',
        command: 'avm android install 34',
        comment: '# Auto-installs cmdline-tools, platform-tools (adb), build-tools, and emulator',
        output: 'Accepting Android SDK licenses...\nDownloading platform-tools (adb, fastboot)...\nDownloading platforms;android-34 & build-tools;34.0.0...\nDownloading emulator & system-images;android-34;google_apis;arm64-v8a...\n✓ Complete Android SDK 34 installed in ~/.avm/tools/android/34/sdk',
        timing: '4.8s'
      },
      {
        id: 'android-avd-create',
        stepNumber: '04',
        label: 'Create AVD',
        command: 'avm android avd create pixel_test --api 34',
        comment: '# Guided wizard creates hardware-accelerated Android Virtual Device',
        output: 'Creating AVD \'pixel_test\' (system-images;android-34;google_apis;arm64-v8a)...\nAuto-configuring hardware profile: medium_phone\n✓ Created AVD \'pixel_test\' in ~/.android/avd/pixel_test.avd',
        timing: '1.2s'
      },
      {
        id: 'android-avd-start',
        stepNumber: '05',
        label: 'Start Emulator',
        command: 'avm android avd start pixel_test',
        comment: '# Launches emulator with native Apple Silicon Hypervisor Framework (HVF)',
        output: 'INFO | Android emulator version 35.1.4.0\nINFO | Initializing gfxstream Metal backend (Apple M-Series)\nINFO | Hypervisor Framework (HVF) acceleration enabled\n✓ Android Virtual Device \'pixel_test\' online & connected to adb',
        timing: '2.1s'
      },
      {
        id: 'android-env-vars',
        stepNumber: '06',
        label: 'Environment Injection',
        command: 'avm env',
        comment: '# Injects ANDROID_HOME and PATH wrappers for adb, avdmanager, and emulator',
        output: 'export ANDROID_HOME="/Users/dev/.avm/tools/android/34/sdk"\nexport ANDROID_SDK_ROOT="/Users/dev/.avm/tools/android/34/sdk"\nexport PATH="/Users/dev/.avm/shims:$PATH"',
        timing: '<0.3ms'
      }
    ]
  }
};
