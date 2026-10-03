export interface PluginDetail {
  envVars: string[];
  keyCommands: {
    command: string;
    description: string;
  }[];
  sampleConfig: string;
  highlights: string[];
}

export interface CliCommand {
  command: string;
  args?: string;
  description: string;
  category: 'Core' | 'Plugins' | 'Versions' | 'Aliases' | 'Shims' | 'Env';
  example: string;
  outputExample?: string;
}

export interface ComparisonItem {
  feature: string;
  avm: string;
  asdf: string;
  vfox: string;
  mise: string;
  proto: string;
}
