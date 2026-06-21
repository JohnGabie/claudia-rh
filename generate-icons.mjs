import { Resvg } from "@resvg/resvg-js";
import { writeFileSync } from "fs";

// Orange bg + white glasses SVG
function makeSvg(size) {
  const r = size / 2;
  // Scale the glasses geometry (designed at 660x360) to fit inside a square with padding
  const pad = size * 0.12;
  const inner = size - pad * 2;
  const scaleX = inner / 660;
  const scaleY = inner / 360;
  const scale = Math.min(scaleX, scaleY);
  const gW = 660 * scale;
  const gH = 360 * scale;
  const tx = (size - gW) / 2;
  const ty = (size - gH) / 2;

  return `<svg xmlns="http://www.w3.org/2000/svg" width="${size}" height="${size}" viewBox="0 0 ${size} ${size}">
  <rect width="${size}" height="${size}" rx="${Math.round(size * 0.21)}" fill="#D97757"/>
  <g transform="translate(${tx}, ${ty}) scale(${scale})"
     fill="none" stroke="#FFFFFF" stroke-linecap="round" stroke-width="39">
    <circle cx="160" cy="195" r="135"/>
    <circle cx="500" cy="195" r="135"/>
    <path d="M295 180 Q330 130 365 180"/>
    <path d="M10 195 L35 192"/>
    <path d="M650 195 L625 192"/>
  </g>
</svg>`;
}

const sizes = [
  { file: "src-tauri/icons/32x32.png", size: 32 },
  { file: "src-tauri/icons/128x128.png", size: 128 },
  { file: "src-tauri/icons/128x128@2x.png", size: 256 },
  { file: "src-tauri/icons/icon.png", size: 512 },
  { file: "src-tauri/icons/Square30x30Logo.png", size: 30 },
  { file: "src-tauri/icons/Square44x44Logo.png", size: 44 },
  { file: "src-tauri/icons/Square71x71Logo.png", size: 71 },
  { file: "src-tauri/icons/Square89x89Logo.png", size: 89 },
  { file: "src-tauri/icons/Square107x107Logo.png", size: 107 },
  { file: "src-tauri/icons/Square142x142Logo.png", size: 142 },
  { file: "src-tauri/icons/Square150x150Logo.png", size: 150 },
  { file: "src-tauri/icons/Square284x284Logo.png", size: 284 },
  { file: "src-tauri/icons/Square310x310Logo.png", size: 310 },
  { file: "src-tauri/icons/StoreLogo.png", size: 50 },
];

for (const { file, size } of sizes) {
  const svg = makeSvg(size);
  const resvg = new Resvg(svg, { fitTo: { mode: "width", value: size } });
  const data = resvg.render().asPng();
  writeFileSync(file, data);
  console.log(`✓ ${file} (${size}x${size})`);
}

console.log("Icons generated.");
