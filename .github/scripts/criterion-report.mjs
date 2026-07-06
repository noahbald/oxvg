#!/usr/bin/env node

import { readdir, readFile } from "node:fs/promises";
import os from "node:os";
import path from "node:path";
import process from "node:process";

const DEFAULT_THRESHOLD = 5;
const STATUS_LABELS = {
  added: "added",
  improved: "🟢 improved",
  noise: "unchanged",
  regressed: "🔴 regressed",
  removed: "removed",
};

async function findResultDirectories(directory) {
  const entries = await readdir(directory, { withFileTypes: true });
  const results = await Promise.all(entries.map(async (entry) => {
    if (!entry.isDirectory()) {
      return [];
    }

    const child = path.join(directory, entry.name);
    return entry.name === "new" ? [child] : findResultDirectories(child);
  }));

  return results.flat();
}

function benchmarkName({ group_id, function_id, value_str }) {
  return [group_id, function_id, value_str].filter(Boolean).join("/");
}

async function extract(directory) {
  const resultDirectories = await findResultDirectories(directory);
  const benchmarks = await Promise.all(resultDirectories.map(async (resultDirectory) => {
    const [benchmark, estimates] = await Promise.all([
      readFile(path.join(resultDirectory, "benchmark.json"), "utf8").then(JSON.parse),
      readFile(path.join(resultDirectory, "estimates.json"), "utf8").then(JSON.parse),
    ]);

    return {
      name: benchmarkName(benchmark),
      mean_ns: estimates.mean.point_estimate,
    };
  }));

  if (benchmarks.length === 0) {
    throw new Error(`no Criterion results found in ${directory}`);
  }

  benchmarks.sort((left, right) => left.name.localeCompare(right.name));
  return {
    metadata: {
      sha: process.env.GITHUB_SHA ?? null,
      runner: process.env.RUNNER_NAME ?? os.hostname(),
      cpu: os.cpus()[0]?.model ?? "unknown",
      platform: `${os.platform()} ${os.arch()}`,
    },
    benchmarks,
  };
}

function compare(base, head, threshold) {
  const baseByName = new Map(base.benchmarks.map((item) => [item.name, item]));
  const headByName = new Map(head.benchmarks.map((item) => [item.name, item]));
  const names = [...new Set([...baseByName.keys(), ...headByName.keys()])].sort();

  return names.map((name) => {
    const baseResult = baseByName.get(name);
    const headResult = headByName.get(name);

    if (!baseResult) {
      return { name, head: headResult, status: "added" };
    }
    if (!headResult) {
      return { name, base: baseResult, status: "removed" };
    }

    const change = ((headResult.mean_ns / baseResult.mean_ns) - 1) * 100;
    const status = change >= threshold
      ? "regressed"
      : change <= -threshold
        ? "improved"
        : "noise";
    return { name, base: baseResult, head: headResult, change, status };
  });
}

function formatDuration(nanoseconds) {
  const [divisor, unit] = [
    [1_000_000_000, "s"],
    [1_000_000, "ms"],
    [1_000, "µs"],
    [1, "ns"],
  ].find(([minimum]) => nanoseconds >= minimum);
  const value = nanoseconds / divisor;
  return `${value.toFixed(value >= 100 ? 1 : value >= 10 ? 2 : 3)} ${unit}`;
}

function escapeMarkdown(value) {
  return String(value).replaceAll("|", "\\|").replaceAll("\n", " ");
}

function renderRow(row) {
  const base = row.base ? formatDuration(row.base.mean_ns) : "—";
  const head = row.head ? formatDuration(row.head.mean_ns) : "—";
  const change = row.change === undefined
    ? "—"
    : `${row.change >= 0 ? "+" : ""}${row.change.toFixed(2)}%`;
  return `| ${escapeMarkdown(row.name)} | ${base} | ${head} | ${change} | ${STATUS_LABELS[row.status]} |`;
}

function renderReport(base, head, threshold) {
  const rows = compare(base, head, threshold);
  const count = (status) => rows.filter((row) => row.status === status).length;
  const runnerNote = base.metadata.cpu === head.metadata.cpu
    ? "Base and PR benchmarks ran concurrently on separate GitHub-hosted runners. Small changes may reflect runner variance."
    : `Runner CPU models differ. Treat this comparison as advisory: base used \`${escapeMarkdown(base.metadata.cpu)}\`, PR used \`${escapeMarkdown(head.metadata.cpu)}\`.`;

  return `<!-- oxvg-benchmark-report -->
## Benchmark comparison

Base \`${base.metadata.sha?.slice(0, 8) ?? "unknown"}\` compared with PR \`${head.metadata.sha?.slice(0, 8) ?? "unknown"}\`. Changes within ±${threshold}% are treated as noise.

Summary: ${count("regressed")} regressed, ${count("improved")} improved, ${count("noise")} unchanged, ${count("added")} added, ${count("removed")} removed.

> ${runnerNote}

| Benchmark | Base mean | PR mean | Change | Result |
| --- | ---: | ---: | ---: | --- |
${rows.map(renderRow).join("\n")}

<details>
<summary>Runner details</summary>

- Base: \`${escapeMarkdown(base.metadata.runner)}\`, \`${escapeMarkdown(base.metadata.cpu)}\`, \`${escapeMarkdown(base.metadata.platform)}\`
- PR: \`${escapeMarkdown(head.metadata.runner)}\`, \`${escapeMarkdown(head.metadata.cpu)}\`, \`${escapeMarkdown(head.metadata.platform)}\`

</details>
`;
}

async function main() {
  const [command, argument] = process.argv.slice(2);

  if (command === "extract" && argument) {
    process.stdout.write(`${JSON.stringify(await extract(argument))}\n`);
    return;
  }

  if (command === "report") {
    const threshold = Number(process.env.BENCHMARK_THRESHOLD_PERCENT ?? DEFAULT_THRESHOLD);
    if (!Number.isFinite(threshold) || threshold < 0) {
      throw new Error("BENCHMARK_THRESHOLD_PERCENT must be a non-negative number");
    }

    const base = JSON.parse(process.env.BASE_RESULTS ?? "");
    const head = JSON.parse(process.env.HEAD_RESULTS ?? "");
    process.stdout.write(renderReport(base, head, threshold));
    return;
  }

  throw new Error("usage: criterion-report.mjs extract <criterion-directory> | report");
}

main().catch((error) => {
  console.error(error);
  process.exitCode = 1;
});
