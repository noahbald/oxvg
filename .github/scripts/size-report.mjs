import { readFileSync, writeFileSync } from "node:fs";
import { join } from "node:path";
import { argv, env, exit } from "node:process";
import { gzipSync } from "node:zlib";

/** Marker used to find and update a previous report comment */
export const MARKER = "<!-- oxvg-size-report -->";

/** Release artifacts to measure, by path relative to a checkout */
export const ARTIFACTS = [
	{
		label: "oxvg (x86_64-unknown-linux-gnu)",
		path: "target/x86_64-unknown-linux-gnu/release/oxvg",
	},
	{
		label: "oxvg_wasm_bg.wasm (web)",
		path: "packages/wasm/dist/oxvg_wasm_bg.wasm",
	},
	{
		label: "oxvg_wasm_bg.wasm (node)",
		path: "packages/wasm/dist/node/oxvg_wasm_bg.wasm",
	},
];

export function formatBytes(bytes) {
	if (bytes < 1024) {
		return `${bytes} B`;
	}
	if (bytes < 1024 * 1024) {
		return `${(bytes / 1024).toFixed(1)} KiB`;
	}
	return `${(bytes / (1024 * 1024)).toFixed(2)} MiB`;
}

export function formatChange(base, head) {
	if (base === undefined) {
		return "new";
	}
	const delta = head - base;
	if (delta === 0) {
		return "no change";
	}
	const sign = delta > 0 ? "+" : "-";
	const size = `${sign}${formatBytes(Math.abs(delta))}`;
	if (base === 0) {
		return size;
	}
	return `${size} (${sign}${((Math.abs(delta) / base) * 100).toFixed(2)}%)`;
}

export function renderReport({ base, head, baseSha, headSha }) {
	const rows = Object.entries(head).map(([label, headSize]) => {
		const baseSize = base[label];
		return [
			`\`${label}\``,
			baseSize ? formatBytes(baseSize.bytes) : "—",
			formatBytes(headSize.bytes),
			formatChange(baseSize?.bytes, headSize.bytes),
			baseSize ? formatBytes(baseSize.gzip) : "—",
			formatBytes(headSize.gzip),
			formatChange(baseSize?.gzip, headSize.gzip),
		].join(" | ");
	});

	return [
		MARKER,
		"## Binary size comparison",
		"",
		`Base \`${baseSha.slice(0, 8)}\` compared with PR \`${headSha.slice(0, 8)}\`.`,
		"",
		"| Artifact | Base | PR | Change | Base (gzip) | PR (gzip) | Change (gzip) |",
		"| --- | ---: | ---: | ---: | ---: | ---: | ---: |",
		...rows.map((row) => `| ${row} |`),
		"",
	].join("\n");
}

export function measure(root) {
	const sizes = {};
	for (const { label, path } of ARTIFACTS) {
		const file = join(root, path);
		let contents;
		try {
			contents = readFileSync(file);
		} catch {
			throw new Error(`Expected a build to have produced \`${file}\``);
		}
		sizes[label] = {
			bytes: contents.byteLength,
			gzip: gzipSync(contents, { level: 9 }).byteLength,
		};
	}
	return sizes;
}

function main([command, ...args]) {
	switch (command) {
		case "measure": {
			const [root, out] = args;
			writeFileSync(out, `${JSON.stringify(measure(root), null, "\t")}\n`);
			return;
		}
		case "report": {
			const [basePath, headPath, out] = args;
			const report = renderReport({
				base: JSON.parse(readFileSync(basePath, "utf8")),
				head: JSON.parse(readFileSync(headPath, "utf8")),
				baseSha: env.BASE_SHA ?? "",
				headSha: env.HEAD_SHA ?? "",
			});
			writeFileSync(out, report);
			return;
		}
		default:
			throw new Error(
				`Expected \`measure <root> <out>\` or \`report <base> <head> <out>\`, got \`${command ?? ""}\``,
			);
	}
}

if (import.meta.filename === argv[1]) {
	try {
		main(argv.slice(2));
	} catch (err) {
		console.error(err.message);
		exit(1);
	}
}
