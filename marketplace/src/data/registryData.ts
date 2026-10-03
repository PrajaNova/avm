import { PluginDetail } from '../types';

export const EXTENDED_PLUGIN_METADATA: Record<string, PluginDetail> = {
  node: {
    envVars: ['PATH (via shims)'],
    highlights: [
      'Live version index fetched straight from nodejs.org/dist/index.json',
      'Automatic package.json script aliases (avm test, avm dev, etc.)',
      'Global packages (npm -g) stay reachable across local version switches',
      'Zero-compile binary fetch for your exact platform architecture'
    ],
    keyCommands: [
      { command: 'avm plugin add node', description: 'Install native Node.js provider binary' },
      { command: 'avm node versions', description: 'List recent available Node.js releases' },
      { command: 'avm node 22 versions', description: 'Filter versions by major line (e.g. Node 22.x)' },
      { command: 'avm node install 22.14.0', description: 'Download and install exact Node version' },
      { command: 'avm node use 22.14.0', description: 'Pin Node version for current project in .avm.json' },
      { command: 'avm node use 22.14.0 --global', description: 'Set machine-wide default Node version' }
    ],
    sampleConfig: `{
  "tools": {
    "node": "22.14.0"
  },
  "aliases": {
    "dev": "pnpm run dev",
    "build": "pnpm run build"
  }
}`
  },
  java: {
    envVars: ['JAVA_HOME', 'PATH (via shims)'],
    highlights: [
      'Eclipse Temurin builds powered by foojay Disco API',
      'Automatic in-process JAVA_HOME export upon version activation',
      'Exact build pin or bare major support (e.g. "17" resolves to latest)',
      'Installs with openjdk- prefix for seamless coexistence'
    ],
    keyCommands: [
      { command: 'avm plugin add java', description: 'Install native OpenJDK provider binary' },
      { command: 'avm java versions', description: 'Browse available OpenJDK Temurin builds' },
      { command: 'avm java 17 versions', description: 'List all patch builds for OpenJDK 17' },
      { command: 'avm java install 17', description: 'Install latest OpenJDK 17 and pin locally' },
      { command: 'avm java use 17.0.13+11', description: 'Select exact Java version in .avm.json' },
      { command: 'avm java list', description: 'Show currently selected and installed OpenJDK versions' }
    ],
    sampleConfig: `{
  "tools": {
    "java": "17.0.13+11"
  },
  "env": {
    "GRADLE_OPTS": "-Dorg.gradle.daemon=true"
  }
}`
  },
  android: {
    envVars: ['ANDROID_HOME', 'ANDROID_SDK_ROOT', 'PATH (adb, sdkmanager, avdmanager, emulator)'],
    highlights: [
      'Direct index querying from Google repository2-3.xml',
      'Auto-installs cmdline-tools, platform-tools, build-tools, emulator, and system-images',
      'Wrapper scripts for adb, sdkmanager, avdmanager, and emulator with correct ANDROID_HOME',
      'One command setup for full Android command-line builds and emulators'
    ],
    keyCommands: [
      { command: 'avm plugin add android', description: 'Install native Android SDK provider binary' },
      { command: 'avm android versions', description: 'List stable Android API levels (34, 35, 36, etc.)' },
      { command: 'avm android install 34', description: 'Install Android SDK 34 + system image + tools' },
      { command: 'avm android use 34', description: 'Pin Android API level for current project' },
      { command: 'avm android list', description: 'Show selected and installed Android SDK versions' }
    ],
    sampleConfig: `{
  "tools": {
    "java": "17",
    "android": "34"
  },
  "aliases": {
    "build:apk": "./gradlew assembleRelease",
    "emulator:start": "emulator -avd pixel-34"
  }
}`
  }
};
