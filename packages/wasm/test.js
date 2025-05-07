const { test } = require("node:test");
const assert = require("node:assert");

const { optimise, extend } = require("./dist/node/oxvg_wasm.js");

test("optimise basic svg", () => {
	const result = optimise(
		`<svg xmlns="http://www.w3.org/2000/svg">
    <!-- Should convert to hex -->
    <g color="black"/>
    <g color="BLACK"/>
    <path fill="rgb(64 64 64)"/>
    <path fill="rgb(64, 64, 64)"/>
    <path fill="rgb(86.27451%,86.666667%,87.058824%)"/>
    <path fill="rgb(-255,100,500)"/>
</svg>`,
	);

	// NOTE: All features are useless, so removed
	assert.equal(result, `<svg xmlns="http://www.w3.org/2000/svg"/>`);
});

test("optimise with config", () => {
	const result = optimise(
		`<svg xmlns="http://www.w3.org/2000/svg">
    <!-- Should convert to currentColor -->
    <g color="black"/>
    <g color="BLACK"/>
    <g color="none"/>
    <path fill="rgb(64, 64, 64)"/>
    <path fill="rgb(86.27451%,86.666667%,87.058824%)"/>
    <path fill="rgb(-255,100,500)"/>
    <path fill="none"/>
</svg>`,
		{ convertColors: { method: "currentColor" } },
	);

	assert.equal(
		result,
		`<svg xmlns="http://www.w3.org/2000/svg"><!-- Should convert to currentColor --><g color="currentColor"/><g color="currentColor"/><g color="none"/><path fill="currentColor"/><path fill="currentColor"/><path fill="currentColor"/><path fill="none"/></svg>`,
	);
});
