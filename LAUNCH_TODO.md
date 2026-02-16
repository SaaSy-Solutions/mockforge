# Launch Readiness TODO

## Completed in this pass
- [x] Fixed `ConfigPage` unit test stability and coverage regressions.
- [x] Added missing mocks and resilient patterns in `ConfigPage` tests.
- [x] Fixed pending port config persistence behavior in `ConfigPage` so local pending values are not immediately overwritten.
- [x] Updated logs page tests to current UI behavior and selectors.
- [x] Updated services page tests to current store contract (`fetchServices`, `isLoading`, `error`, `clearError`).
- [x] Updated sync status indicator tests to current badge/tooltip behavior.
- [x] Revalidated passing suites:
  - `src/pages/__tests__/ConfigPage.test.tsx`
  - `src/pages/__tests__/PluginsPage.test.tsx`
  - `src/pages/__tests__/WorkspacesPage.test.tsx`
  - `src/pages/__tests__/LogsPage.test.tsx`
  - `src/pages/__tests__/ServicesPage.test.tsx`
  - `src/components/workspace/__tests__/SyncStatusIndicator.test.tsx`
- [x] `pnpm type-check` passes.

## Outstanding items (still blocking full green test run)
- [ ] `src/components/__tests__/dashboard/Dashboard.test.tsx`
  - Contains invalid/unstable test structure; currently no tests run due transform/runtime issues and outdated expectations.
- [ ] `src/pages/__tests__/TestingPage.test.tsx`
  - Requires full modernization to avoid deprecated `require(...)` patterns and duplicated/ambiguous text assertions.
- [ ] `src/stores/__tests__/useFixtureStore.test.ts`
  - Diff assertion expects old shape (`fixtureId`) while current store returns `id`, `name`, `old_content`, `new_content`, `changes`, `timestamp`.
- [ ] `src/components/testing/__tests__/WorkflowValidator.test.tsx`
  - Assertions still expect legacy pass-message text that no longer matches rendered output.
- [ ] `src/pages/__tests__/ScenarioStateMachineEditor.test.tsx`
  - Mocking is partially fixed but still needs full assertion alignment against current editor/ReactFlow integration.

## Non-test launch follow-ups
- [ ] Complete i18n rollout beyond shell + key pages (many UI strings still hardcoded English).
- [ ] Add/refresh navigation and route smoke checks for newly added pages/components.
- [ ] Replace broad brittle text-match assertions with role/semantic queries across legacy suites.

