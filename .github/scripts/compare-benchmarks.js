#!/usr/bin/env node

/**
 * Compare benchmark results between PR and baseline
 *
 * This script:
 * 1. Reads benchmark results from Criterion output
 * 2. Compares against baseline stored in git
 * 3. Detects performance regressions >5%
 * 4. Generates detailed comparison report
 * 5. Returns exit code 1 if significant regression detected
 */

const fs = require('fs');
const path = require('path');

// Configuration
const REGRESSION_THRESHOLD = parseFloat(process.env.REGRESSION_THRESHOLD || '5.0'); // Percentage
const BASELINE_DIR = process.env.BASELINE_DIR || '.github/benchmarks';
const CRITERION_DIR = process.env.CRITERION_DIR || 'target/criterion';

/**
 * Parse Criterion benchmark results
 */
function parseCriterionResults(criterionPath) {
    const results = {};

    // Walk through criterion directory structure
    function walkDir(dir, prefix = '') {
        const entries = fs.readdirSync(dir, { withFileTypes: true });

        for (const entry of entries) {
            const fullPath = path.join(dir, entry.name);

            if (entry.isDirectory()) {
                // Skip "change" directories - these contain comparison artifacts, not benchmark results
                if (entry.name === 'change') {
                    continue;
                }
                walkDir(fullPath, prefix ? `${prefix}/${entry.name}` : entry.name);
            } else if (entry.name === 'estimates.json') {
                // Read and parse estimates.json (actual timing data)
                try {
                    const data = JSON.parse(fs.readFileSync(fullPath, 'utf8'));
                    const benchName = prefix.replace(/\/(new|base|current|main)$/, '');

                    // Skip if this is a "change" entry (shouldn't happen due to directory skip, but double-check)
                    if (benchName.includes('/change')) {
                        continue;
                    }

                    if (!results[benchName]) {
                        results[benchName] = {};
                    }

                    // Extract mean time estimate
                    if (data.mean && data.mean.point_estimate) {
                        const dirType = path.basename(path.dirname(fullPath));

                        // Skip "change" directory type - these are comparison artifacts, not benchmark results
                        if (dirType === 'change') {
                            continue;
                        }

                        results[benchName][dirType] = {
                            mean: data.mean.point_estimate,
                            stddev: data.std_dev ? data.std_dev.point_estimate : 0,
                            unit: 'ns'
                        };
                    }
                } catch (err) {
                    console.warn(`Warning: Failed to parse ${fullPath}: ${err.message}`);
                }
            }
        }
    }

    walkDir(criterionPath);
    return results;
}

/**
 * Load baseline from git
 */
function loadBaseline(baselinePath) {
    try {
        const data = fs.readFileSync(baselinePath, 'utf8');
        return JSON.parse(data);
    } catch (err) {
        console.warn(`Warning: No baseline found at ${baselinePath}`);
        return {};
    }
}

/**
 * Save new baseline
 */
function saveBaseline(baselinePath, results) {
    fs.mkdirSync(path.dirname(baselinePath), { recursive: true });
    fs.writeFileSync(baselinePath, JSON.stringify(results, null, 2), 'utf8');
    console.log(`Baseline saved to ${baselinePath}`);
}

/**
 * Format time in human-readable format
 */
function formatTime(nanoseconds) {
    if (nanoseconds < 1000) {
        return `${nanoseconds.toFixed(2)} ns`;
    } else if (nanoseconds < 1000000) {
        return `${(nanoseconds / 1000).toFixed(2)} µs`;
    } else if (nanoseconds < 1000000000) {
        return `${(nanoseconds / 1000000).toFixed(2)} ms`;
    } else {
        return `${(nanoseconds / 1000000000).toFixed(2)} s`;
    }
}

/**
 * Compare benchmark results and generate report
 */
function compareBenchmarks(current, baseline) {
    const regressions = [];
    const improvements = [];
    const stable = [];
    const newBenchmarks = [];

    // Compare each benchmark
    for (const [name, data] of Object.entries(current)) {
        const currentMean = data.new ? data.new.mean : data.main?.mean;
        if (!currentMean) continue;

        const baselineMean = baseline[name]?.new?.mean || baseline[name]?.main?.mean;

        if (!baselineMean) {
            newBenchmarks.push({
                name,
                mean: currentMean,
                formatted: formatTime(currentMean)
            });
            continue;
        }

        const diff = currentMean - baselineMean;
        const percentChange = (diff / baselineMean) * 100;

        const result = {
            name,
            current: currentMean,
            baseline: baselineMean,
            diff,
            percentChange,
            formattedCurrent: formatTime(currentMean),
            formattedBaseline: formatTime(baselineMean)
        };

        if (percentChange > REGRESSION_THRESHOLD) {
            regressions.push(result);
        } else if (percentChange < -REGRESSION_THRESHOLD) {
            improvements.push(result);
        } else {
            stable.push(result);
        }
    }

    return { regressions, improvements, stable, newBenchmarks };
}

/**
 * Generate markdown report
 */
function generateReport(comparison) {
    let report = '# 📊 Performance Benchmark Report\n\n';

    // Summary
    const totalBenchmarks =
        comparison.regressions.length +
        comparison.improvements.length +
        comparison.stable.length +
        comparison.newBenchmarks.length;

    report += '## Summary\n\n';
    report += `- **Total Benchmarks**: ${totalBenchmarks}\n`;
    report += `- **Regressions**: ${comparison.regressions.length} ⚠️\n`;
    report += `- **Improvements**: ${comparison.improvements.length} ✅\n`;
    report += `- **Stable**: ${comparison.stable.length} ➡️\n`;
    report += `- **New**: ${comparison.newBenchmarks.length} 🆕\n\n`;

    // Regression threshold
    report += `**Regression Threshold**: ${REGRESSION_THRESHOLD}%\n\n`;

    // Regressions (if any)
    if (comparison.regressions.length > 0) {
        report += '## ⚠️ Performance Regressions\n\n';
        report += '| Benchmark | Baseline | Current | Change | % Change |\n';
        report += '|-----------|----------|---------|--------|----------|\n';

        for (const reg of comparison.regressions.sort((a, b) => b.percentChange - a.percentChange)) {
            report += `| ${reg.name} | ${reg.formattedBaseline} | ${reg.formattedCurrent} | +${formatTime(Math.abs(reg.diff))} | **+${reg.percentChange.toFixed(2)}%** |\n`;
        }
        report += '\n';
    }

    // Improvements
    if (comparison.improvements.length > 0) {
        report += '## ✅ Performance Improvements\n\n';
        report += '| Benchmark | Baseline | Current | Change | % Change |\n';
        report += '|-----------|----------|---------|--------|----------|\n';

        for (const imp of comparison.improvements.sort((a, b) => a.percentChange - b.percentChange)) {
            report += `| ${imp.name} | ${imp.formattedBaseline} | ${imp.formattedCurrent} | -${formatTime(Math.abs(imp.diff))} | ${imp.percentChange.toFixed(2)}% |\n`;
        }
        report += '\n';
    }

    // New benchmarks
    if (comparison.newBenchmarks.length > 0) {
        report += '## 🆕 New Benchmarks\n\n';
        report += '| Benchmark | Mean Time |\n';
        report += '|-----------|----------|\n';

        for (const bench of comparison.newBenchmarks) {
            report += `| ${bench.name} | ${bench.formatted} |\n`;
        }
        report += '\n';
    }

    // Stable (collapsed)
    if (comparison.stable.length > 0) {
        report += '<details>\n';
        report += '<summary>➡️ Stable Benchmarks (click to expand)</summary>\n\n';
        report += '| Benchmark | Baseline | Current | Change |\n';
        report += '|-----------|----------|---------|--------|\n';

        for (const stab of comparison.stable) {
            const sign = stab.percentChange >= 0 ? '+' : '';
            report += `| ${stab.name} | ${stab.formattedBaseline} | ${stab.formattedCurrent} | ${sign}${stab.percentChange.toFixed(2)}% |\n`;
        }
        report += '\n</details>\n\n';
    }

    return report;
}

/**
 * Main execution
 */
function main() {
    const args = process.argv.slice(2);
    const command = args[0];

    if (command === 'compare') {
        // Load current results
        console.log('Loading current benchmark results...');
        const currentResults = parseCriterionResults(CRITERION_DIR);

        // Load baseline
        console.log('Loading baseline...');
        const baselinePath = path.join(BASELINE_DIR, 'baseline.json');
        const baseline = loadBaseline(baselinePath);

        // Compare
        console.log('Comparing benchmarks...');
        const comparison = compareBenchmarks(currentResults, baseline);

        // Generate report
        const report = generateReport(comparison);

        // Save report
        const reportPath = process.env.REPORT_PATH || 'benchmark-report.md';
        fs.writeFileSync(reportPath, report, 'utf8');
        console.log(`Report saved to ${reportPath}`);

        // Print to console
        console.log('\n' + report);

        // Exit with error if regressions found
        if (comparison.regressions.length > 0) {
            console.error(`\n❌ Found ${comparison.regressions.length} performance regression(s) exceeding ${REGRESSION_THRESHOLD}% threshold`);
            process.exit(1);
        } else {
            console.log(`\n✅ No significant performance regressions detected`);
            process.exit(0);
        }

    } else if (command === 'save-baseline') {
        // Save current results as baseline
        console.log('Saving current results as baseline...');
        const currentResults = parseCriterionResults(CRITERION_DIR);
        const baselinePath = path.join(BASELINE_DIR, 'baseline.json');
        saveBaseline(baselinePath, currentResults);
        console.log('✅ Baseline saved successfully');
        process.exit(0);

    } else {
        console.error('Usage:');
        console.error('  compare-benchmarks.js compare      - Compare current results against baseline');
        console.error('  compare-benchmarks.js save-baseline - Save current results as new baseline');
        process.exit(1);
    }
}

main();
