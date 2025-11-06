# Publishing @mockforge/forgeconnect to npm

## Prerequisites

1. **NPM Account:**
   - Create account at https://www.npmjs.com
   - Join the `@mockforge` organization (contact maintainers)

2. **Authentication:**
   ```bash
   npm login
   ```

3. **Build:**
   ```bash
   npm run build
   ```

4. **Tests:**
   ```bash
   npm run test:unit
   ```

## Publishing Steps

### 1. Version Update

Update version in `package.json`:
- Patch: `0.1.0` → `0.1.1` (bug fixes)
- Minor: `0.1.0` → `0.2.0` (new features)
- Major: `0.1.0` → `1.0.0` (breaking changes)

Or use npm version:
```bash
npm version patch  # 0.1.0 → 0.1.1
npm version minor  # 0.1.0 → 0.2.0
npm version major  # 0.1.0 → 1.0.0
```

### 2. Build and Test

```bash
npm run build
npm run test:unit
```

### 3. Publish

```bash
npm publish --access public
```

### 4. Verify

Check the package on npm:
https://www.npmjs.com/package/@mockforge/forgeconnect

## Pre-release Checklist

- [ ] Update version in `package.json`
- [ ] Update CHANGELOG.md (if exists)
- [ ] Run `npm run build`
- [ ] Run `npm run test:unit`
- [ ] Verify `dist/` contains all necessary files
- [ ] Check `.npmignore` excludes source files
- [ ] Test installation: `npm install @mockforge/forgeconnect@latest`
- [ ] Update README if needed

## Post-release

- [ ] Create GitHub release
- [ ] Update main MockForge documentation
- [ ] Announce in community channels

## Troubleshooting

### "You do not have permission to publish"

- Ensure you're logged in: `npm whoami`
- Verify you're part of `@mockforge` organization
- Check package name matches organization scope

### "Package name already exists"

- Version already published, increment version
- Or unpublish if it was a mistake (within 72 hours)

### Build errors

- Ensure all dependencies are installed: `npm install`
- Check TypeScript compilation: `npx tsc --noEmit`
- Verify Rollup build: `npm run build`
