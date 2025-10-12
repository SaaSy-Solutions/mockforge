🎨 MockForge Design System Brief
1. Overall Style Goals

Modern SaaS admin dashboard aesthetic (inspired by Grafana, Datadog, Linear).

Professional, clean, minimal — but with strong brand integration.

High readability and visual hierarchy for technical users.

2. Typography

Headings: Inter or Poppins, semi-bold, 18–24px.

Body text: Inter, Roboto, or Open Sans, 14–16px, medium weight.

Monospace (logs/requests): JetBrains Mono or Source Code Pro, 13px.

Consistent vertical rhythm (line-height 1.5–1.6).

3. Color Palette

Primary Brand Color: Use the orange tone from the MockForge logo (#D35400).

Secondary Accent: Dark slate gray (#2C3E50).

Backgrounds:

Main background: light neutral (#F9FAFB).

Panels/cards: white (#FFFFFF).

Status Indicators:

Running → Green (#27AE60)

Warning → Yellow/Amber (#F1C40F)

Error → Red (#E74C3C)

Text Colors:

Primary text: #2E2E2E

Secondary text: #6B7280

4. Layout & Spacing

Grid system: 12-column responsive grid.

Spacing scale: 4px baseline (use multiples: 8, 12, 16, 24, 32).

Cards/Panels: Rounded corners (8px–12px), soft shadows for depth.

Whitespace: Generous margins/padding for dashboard clarity.

5. Navigation & Tabs

Move the tabs (Dashboard, Services, Fixtures, etc.) into a left sidebar or top nav bar.

Use icons (e.g., Lucide or Feather icons) for quick recognition.

Add hover states & active highlights (brand orange underlines or filled pill-style background).

6. Dashboard Components

System Status:

Present as a card with clear metrics (uptime, CPU, memory, threads).

Use badges with color-coded indicators.

Servers:

Display in a table or list with status pills (green/red).

Recent Requests:

Show in a collapsible, scrollable card with timestamps, method, endpoint, and status color.

7. Brand Integration

Place the MockForge logo smaller in top-left as branding, not dominating half the screen.

Apply the orange/black/steel-gray brand theme consistently across accent elements.

8. Micro-Interactions

Hover effects on buttons, links, and tabs.

Subtle animations (fade-in for new request logs).

Refresh button styled as a modern icon button, not just text.

9. Implementation Notes

Use a UI library if possible (Tailwind CSS, Chakra UI, or Material UI).

Keep accessibility in mind: WCAG AA contrast compliance.

Provide a dark mode toggle with inverted palette (dark gray background, orange accents).
