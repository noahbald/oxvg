import { readFile } from "node:fs/promises";

import { Criterion } from "@folkol/criterion";
import { parseSvg } from "svgo/lib/parser.js";
import { invokePlugins } from "svgo/lib/svgo/plugins.js";
import convertPathData from "svgo/plugins/convertPathData.js";
import presetDefault from "svgo/plugins/preset-default.js";

const files = [
  "./fixtures/gnome-blobs.svg",
  "./fixtures/inkscape-isometric-madness.svg",
  "./fixtures/tldr-banner.svg",
  "./fixtures/trajans-column.svg",
];

const criterion = new Criterion();

const defaultJobs = criterion.group("default jobs");
const path = criterion.group("path");
files.forEach(async (file) => {
  const svg = await readFile(file, { encoding: "utf8" });
  let ast = parseSvg(svg, file);
  const info = {
    multipassCount: 1,
  };
  defaultJobs.bench(file, () => {
    invokePlugins(ast, info, [presetDefault], null, {});
  });

  ast = parseSvg(svg, file);
  path.bench(file, () => {
    invokePlugins(ast, info, [convertPathData], null, {});
  });
});
