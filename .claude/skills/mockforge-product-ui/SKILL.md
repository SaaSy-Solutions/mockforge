---
name: mockforge-product-ui
description: Use when building or restyling the AUTHENTICATED MockForge app (app.mockforge.dev / crates/mockforge-ui) — dashboards, data tables, settings, forms, and multi-step wizards. This is the product-UI complement to the marketing taste-skill (design-taste-frontend / mockforge-marketing-design), which explicitly excludes dense product UI. Invoke before adding or changing admin-UI components.
---

# MockForge Product UI

> For the **authenticated product app** (the admin UI in `crates/mockforge-ui/ui`, served at app.mockforge.dev): dashboards, data tables, settings panels, forms, multi-step wizards, detail views.
> **Not** for marketing/landing/docs pages — use `mockforge-marketing-design` (the taste-skill) for those. The taste-skill is explicitly out of scope for dashboards and tables (its Section 13); this skill fills that gap.
> Shares the brand (burnt-orange `#D35400`, slate-navy `#2C3E50`, Geist + JetBrains Mono) so the app feels like the same product as the site — but product UI is **denser, status-rich, and state-complete**, which changes several rules.

---

## 0. The real stack (build on it, don't fight it)

Confirmed from `crates/mockforge-ui/ui/package.json` + source:

- **React 19 + TypeScript** (strict), **Vite 6**, **Tailwind CSS 4** (`@tailwindcss/postcss`, `@tailwindcss/typography`).
- **Component primitives: Radix UI** — `dialog`, `dropdown-menu`, `select`, `switch`, `tabs`, `toast`, `context-menu`, `slot`. Plus **CVA** (`class-variance-authority`) + `clsx` + `tailwind-merge`. That combination is the **shadcn/ui pattern**: own your components, built on Radix + Tailwind, variants via CVA, classes merged via a `cn()` helper.
- **Icons: `lucide-react`.** (Note: the marketing site uses Phosphor; the app uses lucide. Keep **one family per surface** — in the app, use lucide, do not introduce Phosphor.)
- **Toasts: `sonner`.** **Charts: `chart.js` + `react-chartjs-2`.** **Node/flow graphs: `@xyflow/react`.**
- **Data/server state: `@tanstack/react-query` v5.** **Client/UI state: `zustand` v5.** **Validation: `zod`.** **Routing: `react-router-dom` v7.** **HTTP: `axios`.** **Markdown: `react-markdown`.** **API reference: `@scalar/api-reference-react`.**
- **No `@tanstack/react-table`** yet — complex tables are hand-rolled. **No `react-hook-form`** — forms use local state + `zod`.

### 0.A The consistency problem to fix: MUI vs Radix
The app **also imports `@mui/material` + `@mui/icons-material`** in a chunk of screens, alongside the Radix/shadcn/Tailwind system. That is the exact "two design systems in one tree" anti-pattern. **Directive:**
- **Build all new UI in the Tailwind + Radix + CVA (shadcn-style) system.** Do **not** add new `@mui/*` usage.
- When you touch a screen that uses MUI, migrate that surface to the Tailwind/Radix equivalents (MUI `Button`→ CVA button, MUI `Dialog`→ Radix Dialog, MUI `Select`→ Radix Select, MUI icons→ lucide). Incremental is fine; net MUI usage should only go down.
- This is what makes the app match the (Tailwind-based) marketing site. One system, one token set.

### 0.B Dependency verification
Before importing a new library, check `package.json`. Prefer the libraries already present (Radix, lucide, sonner, chart.js, TanStack Query, zustand, zod). For complex tables, **add `@tanstack/react-table`** (headless, pairs with the existing system) rather than a heavyweight grid — output the install command first.

---

## 1. Dials for product UI (different from marketing)

Marketing baseline is `VARIANCE 8 / MOTION 6 / DENSITY 4`. Product UI is the opposite shape:

* **`DESIGN_VARIANCE: 3`** — predictable, aligned, scannable. Users operate this daily; surprise is a cost, not a delight. Symmetric grids, consistent card/section rhythm.
* **`MOTION_INTENSITY: 2-3`** — functional only: state transitions, optimistic feedback, skeleton shimmer, menu/dialog enter. **No scroll-reveal, no decorative animation** — data must be visible immediately, not faded in on scroll.
* **`VISUAL_DENSITY: 6-7`** — tight, information-rich. `py-3`/`py-4` section rhythm, compact tables, `text-sm` as the workhorse size. Whitespace is earned, not default.

---

## 2. Tokens & color (status-rich, unlike marketing)

The marketing "max one accent" rule does **not** apply. Product UI must communicate state, so it needs a small, fixed semantic palette:

- **Primary action:** brand orange `#D35400` (`brand-orange`). Primary buttons, active nav, key links, focus accents, primary chart series.
- **Neutrals:** slate/zinc ramp. Surfaces, borders, text. Body text darker than marketing (readability at small sizes): `text-slate-700`/`text-slate-200` (dark).
- **Semantic (lock these, use everywhere consistently):**
  - success = `emerald-600` / `emerald-400` (dark)
  - warning = `amber-600` / `amber-400`
  - error/destructive = `red-600` / `red-400`
  - info = `blue-600` / `blue-400`
- **Never rely on color alone** for status — pair with an icon and/or label (a11y + colorblind). A green dot needs "Healthy" next to it.
- **Numerics, IDs, timestamps, code, durations → `font-mono` (JetBrains Mono).** UI text → Geist (`font-sans`).
- **Radius:** one scale — inputs/buttons `rounded-lg` (8px), cards/panels `rounded-xl` (12px), pills `rounded-full`. Be consistent.
- **Dark mode is first-class** (operators work at night). Tailwind 4 dark strategy as configured in the app; design and test both modes. Off-black surfaces (zinc-900/950), never pure `#000`.

---

## 3. App shell & page structure

- **Shell:** left sidebar nav (sections + active state in brand-orange, collapsible on small screens) + slim top bar (context, environment switcher, account/notifications) + scrollable content region. Sidebar/topbar are persistent chrome; only the content region scrolls.
- **Page header:** page title (left) + primary actions (right, primary action is the orange button, secondaries are outline/ghost). Optional breadcrumb above title. Keep on one row at desktop.
- **Content width:** product content is full-width within the shell (not `max-w-7xl` centered like marketing). Use a comfortable inner max (`max-w-[1600px]`) only on very wide monitors so line lengths and tables stay sane.
- **Filters/toolbar:** above tables/lists — search, filter chips, view toggles, density toggle. Sticky if the list is long.
- **Consistent spacing scale** (4px grid: `gap-2/3/4/6`, `p-3/4/6`). Don't free-style margins.

---

## 4. Data tables (the core product surface)

Tables are where product UI lives or dies. Rules:

- **Pick the right engine:** simple, static, < ~50 rows → semantic `<table>` with Tailwind. Sorting / filtering / pagination / column control / virtualization → **`@tanstack/react-table`** (headless; you keep full Tailwind control). Don't hand-roll sort/filter/paginate logic that TanStack Table already solves.
- **Header:** sticky (`sticky top-0`), subtle background (`bg-slate-50`/`dark:bg-slate-900`), uppercase-small or medium-weight labels, sortable columns show a lucide chevron/arrow and are keyboard-activatable.
- **Rows:** separate with `divide-y` hairlines (one separator between rows — **not** `border-t`+`border-b` on every row). Row hover (`hover:bg-slate-50/50`) for scannability. Row height compact by default; offer a density toggle (comfortable/compact).
- **Alignment & type:** text left; **numbers/IDs/dates right-aligned and `font-mono` with tabular figures** (`tabular-nums`) so columns line up. Truncate long cells with `truncate` + title/tooltip; don't wrap unpredictably.
- **Row actions:** a trailing actions column — an icon button opening a Radix `DropdownMenu`, or inline icon buttons for 1-2 frequent actions. Destructive actions confirm (see §5).
- **Selection:** checkbox column with a header select-all; show a bulk-action bar when ≥1 selected.
- **States (mandatory — never ship only the happy path):**
  - **Loading:** skeleton rows that match column widths (not a centered spinner).
  - **Empty:** a composed empty state — icon + one line of what this is + a primary action to populate it. Distinguish "no data yet" from "no results for this filter" (the latter offers "Clear filters").
  - **Error:** inline error row/panel with a retry, plus a `sonner` toast for transient fetch failures.
- **Pagination or virtualization:** paginate, or virtualize (TanStack Virtual) past ~100 rows. Never render thousands of DOM rows.
- **No giant fixed-track progress bars or decorative dots** in cells — show real values; a tiny inline bar without a heavy background track is fine for ratios.

---

## 5. Forms & multi-step wizards

- **Field layout:** label **above** input (`gap-1.5`), helper text below in muted, error text below in `red-600`/`dark:red-400` with an icon. **Never** placeholder-as-label.
- **Components:** Radix `Select`/`Switch`/`Tabs`/`Dialog`/`RadioGroup`-style built with CVA; consistent input styling (border, `focus:ring-brand-orange/40`, disabled state). Reuse the existing input components — don't restyle inputs per form.
- **Validation:** `zod` schema per form; validate on blur + on submit; show all errors inline; focus the first invalid field. Disable submit while invalid only if it won't trap the user — prefer showing errors on submit attempt.
- **Async submit:** button shows pending state (spinner + disabled), use TanStack Query mutations; on success a `sonner` success toast + navigate/close; on error inline + toast. Optimistic updates where safe, with rollback on failure.
- **Destructive actions:** Radix `AlertDialog` confirmation ("Delete mock server X?") with the destructive button in red and the safe action focused by default. Never delete on a single un-confirmed click.
- **Multi-step wizards:**
  - Show a **stepper** (numbered steps with current/done/upcoming states; done steps get a check). Steps are labeled by their content ("Choose protocol", "Configure", "Review"), not "Step 1/2/3".
  - **Validate per step** before allowing Next; keep entered data when going Back (don't reset state). Persist wizard state in a store (zustand) or form state so a misclick doesn't lose work.
  - Back/Next at the bottom; primary (Next/Finish) on the right. Final step is a **Review** summary before the committing action.
  - Long/optional flows: allow "Save draft" / resume where it makes sense.

---

## 6. Every state, always (the product discipline)

For any data-driven view, implement the full set — this is the #1 thing that separates product UI from a demo:

- **Loading** → skeletons shaped like the final content (cards, table rows, chart placeholder). No bare spinners for full sections.
- **Empty** → composed, actionable, distinguishes empty-dataset vs empty-filter.
- **Error** → inline, with retry; transient ops also toast (`sonner`).
- **Partial/stale** → TanStack Query `isFetching` background refresh indicator; don't flash the whole page on refetch.
- **Success / optimistic** → immediate feedback, rollback on failure.
- **Permission/disabled** → explain why a disabled action is disabled (tooltip), don't just grey it out silently.

---

## 7. Dashboards, metrics & charts

- **Stat tiles:** label + large `font-mono tabular-nums` value + optional delta (▲/▼ with semantic color + the comparison period). Real data only — **no fabricated precision** (`92.4%`, `4.1×`) unless it comes from the API; mark mocked sample data clearly in dev.
- **Charts:** `react-chartjs-2`. Primary series in brand-orange; multi-series use a defined categorical palette (don't random-pick colors per chart). Always: axis labels, units, legend when >1 series, accessible tooltips, empty/loading states. Respect dark mode (grid/axis colors). Don't chart-junk — no 3D, no needless gradients.
- **`@xyflow/react`** for topology/flow views: themed nodes (brand tokens), readable edges, fit-to-view, reduced-motion aware.
- **Card grid:** consistent card (`rounded-xl border bg-white dark:bg-slate-900 p-4/6 shadow-sm`); align tiles to a grid; group related metrics. Cards earn their elevation — for pure data, hairline `divide`/`border` sections can beat boxes.

---

## 8. Navigation, feedback, motion

- **Sidebar nav:** sections with clear active state (orange left-border or filled pill), lucide icons + labels, collapsible; keyboard navigable. Current route highlighted.
- **Tabs:** Radix `Tabs` for in-page section switching; don't deep-nest tabs-in-tabs.
- **Toasts (`sonner`):** transient confirmations and async errors. Don't toast things the user can already see; don't use toasts for errors that belong inline in a form.
- **Motion:** functional and quick (120–200ms). State changes, dialog/menu enter (Radix-driven), optimistic toggles, skeleton shimmer, `layout` shifts on list reorder. **No** scroll-reveal, parallax, or decorative loops in the app. Honor `prefers-reduced-motion`.

---

## 9. Accessibility (product owes more than marketing)

- Radix primitives give you focus management, roving tabindex, ARIA, escape/dismiss — use them instead of hand-rolling menus/dialogs/selects.
- **Keyboard:** every action reachable; tables support arrow/tab navigation for interactive cells; visible focus rings (`focus-visible:ring-2 ring-brand-orange/50`).
- **Contrast:** WCAG AA for text and UI; check small `text-sm`/muted text especially in dark mode.
- **Status never by color alone** (icon + label). **Forms:** label association (`htmlFor`/`id`), `aria-invalid`, error text linked via `aria-describedby`.

---

## 10. Reuse & consistency

- Use the project's `cn()` (clsx + tailwind-merge) helper and CVA variants; **don't invent one-off class soups** that drift from existing components.
- Extend the shared component library (button/input/card/dialog/table) rather than restyling inline per screen.
- Keep tokens in the Tailwind config / CSS variables; reference them, don't hardcode hexes in components (except the documented brand values).
- When in doubt, match an existing well-built screen rather than inventing a new pattern.

---

## 11. Pre-flight check (run before shipping product UI)

- [ ] Built in the **Tailwind + Radix + CVA** system; **no new `@mui/*`** added (and ideally some removed)?
- [ ] Icons are **lucide** (not Phosphor); one family in this surface?
- [ ] **All states** present: loading (skeleton), empty (actionable, filter-aware), error (inline + toast), success/optimistic, disabled-explained?
- [ ] **Tables:** sticky header, `divide-y` (not double borders per row), numbers right-aligned `font-mono tabular-nums`, row actions in a menu, destructive confirmed, paginated/virtualized past ~100 rows, TanStack Table for sort/filter?
- [ ] **Forms:** label-above, zod validation, inline errors below + focus first invalid, async pending state, destructive uses Radix AlertDialog?
- [ ] **Wizard:** labeled steps (not "Step 1/2/3"), per-step validation, data preserved on Back, Review step before commit?
- [ ] **Semantic colors** used consistently (emerald/amber/red/blue) and **never color-only** for status?
- [ ] **Numbers/IDs/dates** in `font-mono tabular-nums`; **no fabricated precision**?
- [ ] **Charts:** brand-orange primary, defined categorical palette, axis labels/units, empty+loading states, dark-mode aware?
- [ ] **Density** appropriate (compact, `text-sm` workhorse), aligned grids, 4px spacing scale?
- [ ] **Dark mode** designed and tested; off-black surfaces, AA contrast?
- [ ] **Motion** functional only (no scroll-reveal/decoration), `prefers-reduced-motion` honored?
- [ ] **A11y:** Radix primitives for menus/dialogs/selects, keyboard reachable, visible focus rings, labeled fields?
- [ ] Reused `cn()` + CVA + shared components; no drift from existing patterns?
- [ ] Brand alignment with the site: Geist UI text, JetBrains Mono data, `#D35400` primary, `#2C3E50` heading ink?

If a box can't be honestly ticked, it's not done.

---

## 12. Boundary with the marketing skill

| Surface | Skill |
|---|---|
| Landing, pricing, compare, blog/notes, docs marketing | `mockforge-marketing-design` (taste): high variance, generous space, Phosphor icons, single accent, scroll-reveal, hero discipline. |
| Authenticated app: dashboards, tables, settings, forms, wizards, detail views | **this skill**: low variance, dense, lucide icons, semantic status palette, every-state discipline, functional motion. |

Same brand, opposite ergonomics. If a screen is a marketing surface that happens to live behind auth (e.g., an upgrade/billing splash), the marketing taste rules can apply to that splash specifically — but the surrounding app chrome stays product-UI.
