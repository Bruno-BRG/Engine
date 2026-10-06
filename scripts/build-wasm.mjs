// Compila o runtime do Nexo para WebAssembly e gera os bindings JS do viewport do editor.
import { execFileSync } from 'node:child_process';
import { homedir } from 'node:os';
import { join, dirname } from 'node:path';
import { fileURLToPath } from 'node:url';

const root = join(dirname(fileURLToPath(import.meta.url)), '..');
const release = process.argv.includes('--release');
const profile = release ? 'release' : 'debug';
const sep = process.platform === 'win32' ? ';' : ':';
const env = { ...process.env, PATH: join(homedir(), '.cargo', 'bin') + sep + process.env.PATH };

const run = (cmd, args) => execFileSync(cmd, args, { cwd: root, env, stdio: 'inherit' });

console.log('[nexo] compilando runtime WASM (' + profile + ')...');
run('cargo', [
  'build', '-p', 'nexo-runtime-web', '--target', 'wasm32-unknown-unknown',
  ...(release ? ['--release'] : []),
]);
run('wasm-bindgen', [
  join(root, 'target', 'wasm32-unknown-unknown', profile, 'nexo_runtime_web.wasm'),
  '--target', 'web',
  '--out-dir', join(root, 'editor', 'src', 'lib', 'runtime', 'pkg'),
]);
console.log('[nexo] runtime pronto.');
