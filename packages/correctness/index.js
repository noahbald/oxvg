const path = require("node:path");
const fs = require("node:fs/promises");

const { loadImage, createCanvas } = require("@napi-rs/canvas");

const argWidth = parseInt(process.argv[2]);
if (Number.isNaN(argWidth)) {
	throw new Error(`${process.argv[2]} is not a number!`);
}
const originalDir = process.argv[3];
if (!originalDir) {
	throw new Error("no directory for original provided");
}
const optimisedDir = process.argv[4];
if (!optimisedDir) {
	throw new Error("no directory for optimised provided");
}

const svgFileTree = async (subPath = ".") => {
	/** @type {{ original: string, optimised: string }[]} */
	const result = [];
	const originalPath = path.resolve(__dirname, originalDir, subPath);
	const stat = await fs.stat(originalPath);
	if (!stat.isDirectory()) {
		if (originalPath.endsWith(".svg")) {
			return [
				{
					original: originalPath,
					optimised: path.resolve(__dirname, optimisedDir, subPath),
				},
			];
		} else {
			return [];
		}
	}

	const dir = await fs.readdir(originalPath);
	await Promise.all(
		dir.map(async (symbol) => {
			const newSubPath = `${subPath}/${symbol}`;
			result.push(...(await svgFileTree(newSubPath)));
		}),
	);

	return result;
};

/**
 * @param svg {import("@napi-rs/canvas").Image}
 * @param method {"data" | "encode" | undefined}
 */
const drawSVG = async (svg, method = "data") => {
	const scale = argWidth / svg.width;
	const canvas = createCanvas(svg.width * scale, svg.height * scale);
	const context = canvas.getContext("2d");

	context.drawImage(svg, 0, 0, svg.width * scale, svg.height * scale);
	if (method === "data") {
		return canvas.data();
	} else {
		return await canvas.encode("png")
	}
};

// Math.sqrt(Math.pow(255, 2) * 4)
const MAX_DISTANCE = 510;

/**
 * @param i {number}
 * @param left {Uint8Array}
 * @param right {Uint8Array}
 * @returns the difference in percentage between the left colour and the right
 */
const difference = (i, left, right) => {
	if (
		left[i] === right[i] &&
		left[i + 1] === right[i + 1] &&
		left[i + 2] === right[i + 2] &&
		left[i + 3] === right[i + 3]
	) {
		// Most pixels will be equal, no need to process
		return 0;
	}
	const distance = Math.sqrt(
		Math.pow(right[i] - left[i], 2) +
			Math.pow(right[i + 1] - left[i + 1], 2) +
			Math.pow(right[i + 2] - left[i + 2], 2) +
			Math.pow(right[i + 3] - left[i + 3], 2),
	);

	return distance / MAX_DISTANCE;
};

/**
 * i.e. to replicate imagemagick's `compare -metric AE -fuzz 10% left.png right.png`
 * - Metric of absolute-error (i.e. number of different pixels)
 * - Fuzz by 10% (i.e. colours within 10% are considered equal)
 * @param left {Buffer<ArrayBufferLike>}
 * @param right {Buffer<ArrayBufferLike>}
 * @returns {string | undefined} Error message when the images are different enough to be considered broken
 */
const compare = (left, right) => {
	if (left.length !== right.length) {
		return "compared images are different sizes, may need manual comparison";
	}
	if (left.length % 4 !== 0) {
		return "image is not a quartet of RGBA values";
	}

	let errors = 0;
	const leftArray = new Uint8Array(left);
	const rightArray = new Uint8Array(right);
	for (let i = 0; i < left.length; i += 4) {
		const percentage = difference(i, leftArray, rightArray);
		if (percentage > 0.1) {
			errors += 1;
		}
	}
	const errorPercentage = errors / left.length;
	if (errorPercentage > 0.02) {
		return `errors exceeds threshold (${errorPercentage * 100}%)`;
	}
};

/**
 * @param item {{ original: string, optimised: string }}
 * @returns {Promise<"ignore" | "broken" | "ok">}
 */
const check = async (item) => {
	const [originalImage, optimisedImage] = await Promise.all([
		loadImage(item.original)
			.catch((e) => console.error(item.original, "missing:", e)),
		loadImage(item.optimised)
			.catch((e) => console.error(item.optimised, "ignored:", e)),
	]);
	if (!originalImage || !optimisedImage) {
		return "ignore";
	}

	const [original, optimised] = await Promise.all([
			drawSVG(originalImage),
			drawSVG(optimisedImage),
	]);
	const result = compare(original, optimised);
	if (result) {
		console.error(item.optimised, result);
	}
	if (result) {
		const time = Date.now();
		await Promise.all([
			drawSVG(originalImage, "encode")
				.then((original) => fs.writeFile(`./screenshots/${time}.${encodeURIComponent(item.original)}.original.png`, original)),
			drawSVG(optimisedImage, "encode")
				.then((optimised) => fs.writeFile(`./screenshots/${time}.${encodeURIComponent(item.optimised)}.optimised.png`, optimised)),
		]);
		return "broken";
	} else {
		return "ok";
	}
};

(async () => {
	const svgs = await svgFileTree();

	const results = await Promise.all(svgs.map(check));
	const brokenCount = results.filter((result) => result === "broken").length;

	if (svgs.length > 1) {
		console.log(
			brokenCount,
			`items broken (${100 * (brokenCount / svgs.length)}%)`,
		);
	} else {
		console.log(results[0]);
	}
})();

/*
 * If it crashed, try the following nu script

 ```nu
 let result = fd *.svg --glob --no-ignore
 	| lines
 	| where (str starts-with oxygen/)
 	| par-each { |e| [
 		try { node index.js 512 $e ($e | str replace "oxygen/" "oxygen-optimised/") } catch { "retry" }), $e]
 	] }

// Usually all retries work, might be worth double checking
let retries = $result
	| filter { $in.0 == "retry" }
	| par-each { node index.ts 512 $in.1 ($in.1 | str replace "oxygen/" "oxygen-optimised/") }

let total = $result | filter { $in.0 != "ignore" } | length
let ok = ($result | filter { $in.0 == "ok" } | length) + ($retries | filter { $in.0 == "ok" } | length)
{ count: $ok, percentage: $ok / $total }
 ```

 */
