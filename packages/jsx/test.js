const { test, describe } = require("node:test");
const assert = require("node:assert");

const { transform } = require("./index.js");

test("default config", () => {
	const result = transform(
		`<svg><g /></svg>`,
	);

	assert.equal(result, `import * as React from "react";
const SvgComponent = (props)=><svg {...props}/>;
export default SvgComponent;
`);
});

test("basic config", () => {
	const result = transform(
		`<svg><g /></svg>`,
		{ icon: { type: "Bool", field0: true }, oxvg: false }
	);

	assert.equal(result, `import * as React from "react";
const SvgComponent = (props)=><svg width="1em" height="1em" {...props}><g/></svg>;
export default SvgComponent;
`);
})

test("template config", () => {
	const result = transform(
		`<svg><g /></svg>`,
		{
			oxvg: false,
			template: ({ imports, interfaces, componentName, props, jsx, exports }) => `
${imports}
import PropTypes from 'prop-types';
${interfaces}

function ${componentName}(${props}) {
	return ${jsx};
}

${componentName}.propTypes = {
	title: PropTypes.string,
};

${exports}
` });

	assert.equal(result, `
import * as React from "react";

import PropTypes from 'prop-types';


function SvgComponent(props) {
	return <svg {...props}><g/></svg>;
}

SvgComponent.propTypes = {
	title: PropTypes.string,
};

export default SvgComponent;

`);
})

test("config with state", () => {
	const result = transform(
		`<svg><g /></svg>`,
		{ oxvg: false },
		{ componentName: "MySVG" }
	);

	assert.equal(result, `import * as React from "react";
const MySVG = (props)=><svg {...props}><g/></svg>;
export default MySVG;
`);
})


describe("optimise", () => {
	test("oxvg default", () => {
		const result = transform(
			`<svg><path d="M0 0l0 1"/></svg>`,
		);

		assert.equal(result, `import * as React from "react";
const SvgComponent = (props)=><svg {...props}><path d="M0 0v1"/></svg>;
export default SvgComponent;
`);
	})

	test("disable oxvg", () => {
		const result = transform(
			`<svg><path d="M0 0l0 1"/></svg>`,
			{ oxvg: false },
		);

		assert.equal(result, `import * as React from "react";
const SvgComponent = (props)=><svg {...props}><path d="M0 0l0 1"/></svg>;
export default SvgComponent;
`);
	})

	test("alias svgo", () => {
		const result = transform(
			`<svg><path d="M0 0l0 1"/></svg>`,
			{ svgo: true }
		);

		assert.equal(result, `import * as React from "react";
const SvgComponent = (props)=><svg {...props}><path d="M0 0v1"/></svg>;
export default SvgComponent;
`);
	})

	test("oxvg config", () => {
		const result = transform(
			`<svg><path d="M0 0l0 1"/></svg>`,
			{ oxvg: true, oxvgConfig: { convertPathData: undefined } }
		);

		assert.equal(result, `import * as React from "react";
const SvgComponent = (props)=><svg {...props}><path d="M0 0l0 1"/></svg>;
export default SvgComponent;
`);
	})
})
