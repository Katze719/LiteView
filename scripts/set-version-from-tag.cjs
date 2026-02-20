const ref = process.env.REF_NAME || '';
const v = ref.startsWith('v') ? ref.slice(1) : ref;
if (!v) {
  console.error('REF_NAME env var required (e.g. v1.0.0)');
  process.exit(1);
}

const fs = require('fs');

const tauri = JSON.parse(fs.readFileSync('src-tauri/tauri.conf.json', 'utf8'));
tauri.version = v;
fs.writeFileSync('src-tauri/tauri.conf.json', JSON.stringify(tauri, null, 2));

const pkg = JSON.parse(fs.readFileSync('package.json', 'utf8'));
pkg.version = v;
fs.writeFileSync('package.json', JSON.stringify(pkg, null, 2));

const cargo = fs.readFileSync('src-tauri/Cargo.toml', 'utf8');
fs.writeFileSync(
  'src-tauri/Cargo.toml',
  cargo.replace(/^version = ".*"/m, `version = "${v}"`)
);

console.log('Version set to', v);
