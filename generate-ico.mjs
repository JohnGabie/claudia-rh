import pngToIco from "png-to-ico";
import { writeFileSync, readFileSync } from "fs";

const buf = await pngToIco(["src-tauri/icons/32x32.png", "src-tauri/icons/128x128.png"]);
writeFileSync("src-tauri/icons/icon.ico", buf);
console.log("icon.ico generated");
