#!/usr/bin/env node
const {execSync} = require('child_process');
const env = {
  ...process.env,
  PATH: 'D:\\msvc_x86_64\\bin;' + process.env.PATH,
  GSTREAMER_1_0_ROOT_MSVC_X86_64: 'D:\\msvc_x86_64\\',
  PKG_CONFIG_PATH: 'D:\\msvc_x86_64\\lib\\pkgconfig',
  PKG_CONFIG: 'D:\\msvc_x86_64\\bin\\pkg-config.exe',
  CARGO_TARGET_DIR: 'e:\\screencast-build',
};
const cmd = process.argv.slice(2).join(' ');
try {
  const result = execSync(cmd, {env, encoding: 'utf8', stdio: 'pipe'});
  console.log(result);
} catch(e) {
  console.error(e.stderr || e.stdout || e.message);
  process.exit(e.status || 1);
}
