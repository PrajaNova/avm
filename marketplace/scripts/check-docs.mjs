import assert from 'node:assert/strict';
import { readFile } from 'node:fs/promises';
import ts from 'typescript';

const read = (path) => readFile(new URL(path, import.meta.url), 'utf8');
async function data(path) {
  const source = await read(path);
  const { outputText } = ts.transpileModule(source, { compilerOptions: { module: ts.ModuleKind.ESNext } });
  return import(`data:text/javascript;base64,${Buffer.from(outputText).toString('base64')}`);
}
const { CLI_COMMANDS } = await data('../src/data/cliReference.ts');
const { FEATURE_GUIDES } = await data('../src/data/featureGuides.ts');
const commands = CLI_COMMANDS.map(({ command }) => command);
assert.equal(new Set(commands).size, commands.length, 'Duplicate command reference entry');
const args = await read('../../crates/avm-cli/src/cli/args.rs');
const hidden = new Set(['ExecShim', 'Pa', 'Ea', 'UpdateCheck', 'PluginCommand']);
for (const [enumName, prefix] of [['Commands', 'avm'], ['PluginCommands', 'avm plugin'], ['AliasCommands', 'avm alias'], ['EnvCommands', 'avm env'], ['ShimsCommands', 'avm shims']]) {
  const body = args.split(`pub enum ${enumName} {`)[1]?.split('\n}')[0];
  assert.ok(body, `Missing CLI enum ${enumName}`);
  for (const [, variant] of body.matchAll(/^    ([A-Z]\w*)\s*[{(,]/gm)) {
    if (hidden.has(variant)) continue;
    const name = variant.replace(/([a-z])([A-Z])/g, '$1-$2').toLowerCase();
    const command = `${prefix} ${name}`;
    assert.ok(commands.some((entry) => entry === command || entry.startsWith(`${command} `)), `Undocumented command: ${command}`);
  }
}
const sidebar = await read('../src/components/docs/DocsSidebar.tsx');
const page = await read('../src/pages/DocsPage.tsx');
const ids = [...sidebar.matchAll(/id: '([^']+)'/g)].map((match) => match[1]);
for (const id of ids) {
  if (id === 'github-issues' || id.startsWith('plugin-') || id === 'contribute-docs') continue;
  assert.ok(FEATURE_GUIDES[id] || page.includes(`activeSection === '${id}'`), `Sidebar page has no content: ${id}`);
}
for (const [id, guide] of Object.entries(FEATURE_GUIDES)) {
  assert.ok(ids.includes(id), `Guide missing from navigation: ${id}`);
  assert.ok(guide.title && guide.intro && guide.sections.length, `Empty guide: ${id}`);
  for (const section of guide.sections) {
    // JSON examples must be safe to copy into .avm.json.
    if (section.code?.startsWith('{')) JSON.parse(section.code);
  }
}
console.log(`Documentation coverage passed: ${commands.length} command entries, ${Object.keys(FEATURE_GUIDES).length} feature guides.`);
