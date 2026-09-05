import assert from "node:assert";
import { join } from "node:path";
import { describe, test } from "node:test";

import {
	ARTIFACTS,
	formatBytes,
	formatChange,
	measure,
	renderReport,
} from "./size-report.mjs";

describe("formatBytes", () => {
	test("uses bytes below a kibibyte", () => {
		assert.strictEqual(formatBytes(0), "0 B");
		assert.strictEqual(formatBytes(1023), "1023 B");
	});

	test("uses kibibytes below a mebibyte", () => {
		assert.strictEqual(formatBytes(1024), "1.0 KiB");
		assert.strictEqual(formatBytes(1024 * 1024 - 1), "1024.0 KiB");
	});

	test("uses mebibytes from a mebibyte", () => {
		assert.strictEqual(formatBytes(1024 * 1024), "1.00 MiB");
		assert.strictEqual(formatBytes(5 * 1024 * 1024 + 512 * 1024), "5.50 MiB");
	});
});

describe("formatChange", () => {
	test("reports growth with a signed size and percentage", () => {
		assert.strictEqual(
			formatChange(1024 * 100, 1024 * 101),
			"+1.0 KiB (+1.00%)",
		);
	});

	test("reports shrinkage with a signed size and percentage", () => {
		assert.strictEqual(
			formatChange(1024 * 100, 1024 * 99),
			"-1.0 KiB (-1.00%)",
		);
	});

	test("reports equal sizes as unchanged", () => {
		assert.strictEqual(formatChange(2048, 2048), "no change");
	});

	test("reports a missing base as new", () => {
		assert.strictEqual(formatChange(undefined, 2048), "new");
	});

	test("omits the percentage when the base is empty", () => {
		assert.strictEqual(formatChange(0, 2048), "+2.0 KiB");
	});
});

describe("renderReport", () => {
	const base = {
		"oxvg (x86_64-unknown-linux-gnu)": { bytes: 1024 * 100, gzip: 1024 * 40 },
		"gone.wasm": { bytes: 10, gzip: 10 },
	};
	const head = {
		"oxvg (x86_64-unknown-linux-gnu)": { bytes: 1024 * 101, gzip: 1024 * 40 },
		"new.wasm": { bytes: 2048, gzip: 1024 },
	};
	const report = renderReport({
		base,
		head,
		baseSha: "0123456789abcdef",
		headSha: "fedcba9876543210",
	});

	test("carries the marker used to find a previous comment", () => {
		assert.match(report, /^<!-- oxvg-size-report -->\n/);
	});

	test("abbreviates both revisions", () => {
		assert.match(report, /Base `01234567` compared with PR `fedcba98`\./);
	});

	test("lists every measured artifact once", () => {
		assert.match(
			report,
			/\| `oxvg \(x86_64-unknown-linux-gnu\)` \| 100\.0 KiB \| 101\.0 KiB \| \+1\.0 KiB \(\+1\.00%\) \|/,
		);
		assert.match(report, /\| `new\.wasm` \| — \| 2\.0 KiB \| new \|/);
	});

	test("drops artifacts the pull request no longer builds", () => {
		assert.doesNotMatch(report, /gone\.wasm/);
	});
});

describe("measure", () => {
	test("names the artifact a build failed to produce", () => {
		assert.throws(() => measure("nowhere"), {
			message: `Expected a build to have produced \`${join("nowhere", ARTIFACTS[0].path)}\``,
		});
	});
});
