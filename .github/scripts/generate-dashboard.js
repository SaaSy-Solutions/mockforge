#!/usr/bin/env node

/**
 * Generate HTML performance dashboard from benchmark results
 *
 * This script creates an interactive HTML dashboard showing:
 * - Current benchmark results
 * - Historical trends (if available)
 * - Performance metrics visualization
 * - Regression detection status
 */

const fs = require('fs');
const path = require('path');

const CRITERION_DIR = process.env.CRITERION_DIR || 'target/criterion';
const OUTPUT_FILE = process.env.DASHBOARD_OUTPUT || 'performance-dashboard.html';
const BASELINE_DIR = process.env.BASELINE_DIR || '.github/benchmarks';

/**
 * Parse Criterion benchmark results
 */
function parseCriterionResults(criterionPath) {
    const results = [];

    function walkDir(dir, prefix = '') {
        const entries = fs.readdirSync(dir, { withFileTypes: true });

        for (const entry of entries) {
            const fullPath = path.join(dir, entry.name);

            if (entry.isDirectory()) {
                walkDir(fullPath, prefix ? `${prefix}/${entry.name}` : entry.name);
            } else if (entry.name === 'benchmark.json') {
                try {
                    const data = JSON.parse(fs.readFileSync(fullPath, 'utf8'));
                    const benchName = prefix.replace(/\/(new|base|current)$/, '');

                    if (data.mean && data.mean.point_estimate) {
                        results.push({
                            name: benchName,
                            mean: data.mean.point_estimate,
                            stddev: data.std_dev ? data.std_dev.point_estimate : 0,
                            median: data.median ? data.median.point_estimate : data.mean.point_estimate,
                            unit: 'ns'
                        });
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
 * Load baseline if available
 */
function loadBaseline() {
    try {
        const baselinePath = path.join(BASELINE_DIR, 'baseline.json');
        const data = fs.readFileSync(baselinePath, 'utf8');
        return JSON.parse(data);
    } catch (err) {
        return null;
    }
}

/**
 * Generate HTML dashboard
 */
function generateDashboard(results, baseline) {
    const timestamp = new Date().toISOString();

    // Prepare data for charts
    const benchmarkNames = results.map(r => r.name);
    const benchmarkMeans = results.map(r => r.mean);

    // Calculate comparisons with baseline
    const comparisons = results.map(result => {
        if (!baseline || !baseline[result.name]) {
            return { name: result.name, status: 'new', change: 0 };
        }

        const baselineMean = baseline[result.name].new?.mean || baseline[result.name].main?.mean;
        const change = ((result.mean - baselineMean) / baselineMean) * 100;

        let status = 'stable';
        if (change > 5) status = 'regression';
        else if (change < -5) status = 'improvement';

        return { name: result.name, status, change };
    });

    const regressionCount = comparisons.filter(c => c.status === 'regression').length;
    const improvementCount = comparisons.filter(c => c.status === 'improvement').length;
    const stableCount = comparisons.filter(c => c.status === 'stable').length;
    const newCount = comparisons.filter(c => c.status === 'new').length;

    const html = `<!DOCTYPE html>
<html lang="en">
<head>
    <meta charset="UTF-8">
    <meta name="viewport" content="width=device-width, initial-scale=1.0">
    <title>MockForge Performance Dashboard</title>
    <script src="https://cdn.jsdelivr.net/npm/chart.js@4.4.0/dist/chart.umd.min.js"></script>
    <style>
        * {
            margin: 0;
            padding: 0;
            box-sizing: border-box;
        }

        body {
            font-family: -apple-system, BlinkMacSystemFont, 'Segoe UI', Roboto, Oxygen, Ubuntu, Cantarell, sans-serif;
            background: #0d1117;
            color: #c9d1d9;
            padding: 20px;
            line-height: 1.6;
        }

        .container {
            max-width: 1400px;
            margin: 0 auto;
        }

        header {
            background: linear-gradient(135deg, #1f6feb 0%, #0969da 100%);
            padding: 30px;
            border-radius: 12px;
            margin-bottom: 30px;
            box-shadow: 0 4px 6px rgba(0,0,0,0.3);
        }

        h1 {
            font-size: 2.5rem;
            margin-bottom: 10px;
        }

        .timestamp {
            opacity: 0.8;
            font-size: 0.9rem;
        }

        .stats {
            display: grid;
            grid-template-columns: repeat(auto-fit, minmax(250px, 1fr));
            gap: 20px;
            margin-bottom: 30px;
        }

        .stat-card {
            background: #161b22;
            padding: 20px;
            border-radius: 8px;
            border: 1px solid #30363d;
            transition: transform 0.2s;
        }

        .stat-card:hover {
            transform: translateY(-2px);
            border-color: #1f6feb;
        }

        .stat-value {
            font-size: 2.5rem;
            font-weight: bold;
            margin: 10px 0;
        }

        .stat-label {
            color: #8b949e;
            text-transform: uppercase;
            font-size: 0.85rem;
            letter-spacing: 0.5px;
        }

        .regression { color: #f85149; }
        .improvement { color: #3fb950; }
        .stable { color: #58a6ff; }
        .new { color: #d29922; }

        .charts {
            display: grid;
            grid-template-columns: repeat(auto-fit, minmax(600px, 1fr));
            gap: 30px;
            margin-bottom: 30px;
        }

        .chart-container {
            background: #161b22;
            padding: 25px;
            border-radius: 8px;
            border: 1px solid #30363d;
        }

        .chart-title {
            font-size: 1.3rem;
            margin-bottom: 20px;
            color: #58a6ff;
        }

        table {
            width: 100%;
            background: #161b22;
            border-radius: 8px;
            overflow: hidden;
            border: 1px solid #30363d;
        }

        th, td {
            padding: 15px;
            text-align: left;
            border-bottom: 1px solid #30363d;
        }

        th {
            background: #0d1117;
            color: #58a6ff;
            font-weight: 600;
            text-transform: uppercase;
            font-size: 0.85rem;
            letter-spacing: 0.5px;
        }

        tr:last-child td {
            border-bottom: none;
        }

        tr:hover {
            background: #0d1117;
        }

        .status-badge {
            display: inline-block;
            padding: 4px 10px;
            border-radius: 12px;
            font-size: 0.8rem;
            font-weight: 600;
            text-transform: uppercase;
        }

        .status-regression {
            background: rgba(248, 81, 73, 0.2);
            color: #f85149;
        }

        .status-improvement {
            background: rgba(63, 185, 80, 0.2);
            color: #3fb950;
        }

        .status-stable {
            background: rgba(88, 166, 255, 0.2);
            color: #58a6ff;
        }

        .status-new {
            background: rgba(210, 153, 34, 0.2);
            color: #d29922;
        }

        .footer {
            text-align: center;
            margin-top: 50px;
            padding: 20px;
            color: #8b949e;
            font-size: 0.9rem;
        }

        @media (max-width: 768px) {
            .charts {
                grid-template-columns: 1fr;
            }

            h1 {
                font-size: 1.8rem;
            }
        }
    </style>
</head>
<body>
    <div class="container">
        <header>
            <h1>🚀 MockForge Performance Dashboard</h1>
            <div class="timestamp">Generated: ${timestamp}</div>
        </header>

        <div class="stats">
            <div class="stat-card">
                <div class="stat-label">Total Benchmarks</div>
                <div class="stat-value">${results.length}</div>
            </div>
            <div class="stat-card">
                <div class="stat-label">Regressions</div>
                <div class="stat-value regression">${regressionCount}</div>
            </div>
            <div class="stat-card">
                <div class="stat-label">Improvements</div>
                <div class="stat-value improvement">${improvementCount}</div>
            </div>
            <div class="stat-card">
                <div class="stat-label">Stable</div>
                <div class="stat-value stable">${stableCount}</div>
            </div>
        </div>

        <div class="charts">
            <div class="chart-container">
                <h2 class="chart-title">Benchmark Performance (Mean Time)</h2>
                <canvas id="performanceChart"></canvas>
            </div>
            <div class="chart-container">
                <h2 class="chart-title">Status Distribution</h2>
                <canvas id="statusChart"></canvas>
            </div>
        </div>

        <div class="chart-container">
            <h2 class="chart-title">Detailed Results</h2>
            <table>
                <thead>
                    <tr>
                        <th>Benchmark</th>
                        <th>Mean Time</th>
                        <th>Std Dev</th>
                        <th>Status</th>
                        <th>Change</th>
                    </tr>
                </thead>
                <tbody>
                    ${results.map((result, i) => {
                        const comparison = comparisons[i];
                        const changeStr = comparison.status === 'new' ? 'N/A' :
                            `${comparison.change >= 0 ? '+' : ''}${comparison.change.toFixed(2)}%`;

                        return `
                        <tr>
                            <td>${result.name}</td>
                            <td>${formatTime(result.mean)}</td>
                            <td>${formatTime(result.stddev)}</td>
                            <td><span class="status-badge status-${comparison.status}">${comparison.status}</span></td>
                            <td class="${comparison.status}">${changeStr}</td>
                        </tr>
                        `;
                    }).join('')}
                </tbody>
            </table>
        </div>

        <div class="footer">
            <p>MockForge Performance Monitoring System</p>
            <p>Powered by Criterion.rs and GitHub Actions</p>
        </div>
    </div>

    <script>
        // Performance chart
        const perfCtx = document.getElementById('performanceChart').getContext('2d');
        new Chart(perfCtx, {
            type: 'bar',
            data: {
                labels: ${JSON.stringify(benchmarkNames)},
                datasets: [{
                    label: 'Mean Time (ns)',
                    data: ${JSON.stringify(benchmarkMeans)},
                    backgroundColor: 'rgba(88, 166, 255, 0.7)',
                    borderColor: 'rgba(88, 166, 255, 1)',
                    borderWidth: 1
                }]
            },
            options: {
                responsive: true,
                maintainAspectRatio: true,
                plugins: {
                    legend: {
                        labels: { color: '#c9d1d9' }
                    }
                },
                scales: {
                    y: {
                        beginAtZero: true,
                        ticks: { color: '#c9d1d9' },
                        grid: { color: '#30363d' }
                    },
                    x: {
                        ticks: {
                            color: '#c9d1d9',
                            maxRotation: 45,
                            minRotation: 45
                        },
                        grid: { color: '#30363d' }
                    }
                }
            }
        });

        // Status distribution chart
        const statusCtx = document.getElementById('statusChart').getContext('2d');
        new Chart(statusCtx, {
            type: 'doughnut',
            data: {
                labels: ['Regressions', 'Improvements', 'Stable', 'New'],
                datasets: [{
                    data: [${regressionCount}, ${improvementCount}, ${stableCount}, ${newCount}],
                    backgroundColor: [
                        'rgba(248, 81, 73, 0.8)',
                        'rgba(63, 185, 80, 0.8)',
                        'rgba(88, 166, 255, 0.8)',
                        'rgba(210, 153, 34, 0.8)'
                    ],
                    borderWidth: 2,
                    borderColor: '#161b22'
                }]
            },
            options: {
                responsive: true,
                maintainAspectRatio: true,
                plugins: {
                    legend: {
                        position: 'bottom',
                        labels: { color: '#c9d1d9', padding: 20 }
                    }
                }
            }
        });
    </script>
</body>
</html>`;

    return html;
}

/**
 * Main execution
 */
function main() {
    console.log('Generating performance dashboard...');

    // Parse current results
    const results = parseCriterionResults(CRITERION_DIR);
    console.log(`Found ${results.length} benchmark results`);

    // Load baseline
    const baseline = loadBaseline();
    if (baseline) {
        console.log('Loaded baseline for comparison');
    } else {
        console.log('No baseline available, showing current results only');
    }

    // Generate HTML
    const html = generateDashboard(results, baseline);

    // Write to file
    fs.writeFileSync(OUTPUT_FILE, html, 'utf8');
    console.log(`✅ Dashboard saved to ${OUTPUT_FILE}`);
}

main();
