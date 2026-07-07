#!/usr/bin/env node

const { spawn } = require('child_process');
const path = require('path');
const os = require('os');
const fs = require('fs');
const https = require('https');

if (process.platform !== 'win32') {
  console.error('Claudia RH is a Windows-only application.');
  process.exit(1);
}

const PKG_VERSION = require('../package.json').version;
const BIN_DIR = path.join(os.homedir(), '.claudia-rh');
const EXE_PATH = path.join(BIN_DIR, 'claudia-rh.exe');
const VERSION_FILE = path.join(BIN_DIR, 'version.txt');

function launch() {
  spawn(EXE_PATH, [], { detached: true, stdio: 'ignore' }).unref();
}

function download(url, dest, cb) {
  https.get(url, (res) => {
    if (res.statusCode === 301 || res.statusCode === 302) {
      return download(res.headers.location, dest, cb);
    }
    if (res.statusCode !== 200) {
      return cb(new Error(`HTTP ${res.statusCode}`));
    }
    const file = fs.createWriteStream(dest);
    res.pipe(file);
    file.on('finish', () => file.close(cb));
    file.on('error', (err) => { fs.unlink(dest, () => {}); cb(err); });
  }).on('error', cb);
}

const cached = fs.existsSync(EXE_PATH) &&
  fs.existsSync(VERSION_FILE) &&
  fs.readFileSync(VERSION_FILE, 'utf8').trim() === PKG_VERSION;

if (cached) {
  launch();
} else {
  fs.mkdirSync(BIN_DIR, { recursive: true });
  const url = `https://github.com/JohnGabie/claudia-rh/releases/download/v${PKG_VERSION}/claudia-rh.exe`;
  process.stdout.write(`Downloading Claudia RH v${PKG_VERSION}... `);
  download(url, EXE_PATH, (err) => {
    if (err) {
      console.error('\nDownload failed:', err.message);
      console.error(`Get it manually at: https://github.com/JohnGabie/claudia-rh/releases/tag/v${PKG_VERSION}`);
      process.exit(1);
    }
    fs.writeFileSync(VERSION_FILE, PKG_VERSION);
    console.log('done.');
    launch();
  });
}
