# Publishing MockForge VS Code Extension

This guide explains how to package and publish the MockForge VS Code extension to the marketplace.

## Prerequisites

1. **VS Code Extension Manager (vsce)**: Already installed as a dev dependency
2. **Azure DevOps Personal Access Token**: Required for publishing
3. **Publisher Account**: The extension uses publisher `saasy-solutions`

## Packaging

To create a `.vsix` package:

```bash
cd vscode-extension
npm run compile
npm run package
```

This will create `mockforge-vscode-0.1.0.vsix` in the `vscode-extension` directory.

## Publishing to Marketplace

### First-Time Publishing

1. **Install vsce globally** (if not already installed):
   ```bash
   npm install -g @vscode/vsce
   ```

2. **Login to Azure DevOps**:
   ```bash
   vsce login saasy-solutions
   ```
   
   You'll need a Personal Access Token (PAT) from Azure DevOps with the "Marketplace (manage)" scope.

3. **Publish the extension**:
   ```bash
   vsce publish
   ```

   Or publish a specific version:
   ```bash
   vsce publish 0.1.0
   ```

### Updating an Existing Extension

1. **Update version** in `package.json`
2. **Update CHANGELOG.md** with new features/fixes
3. **Package and publish**:
   ```bash
   npm run package
   vsce publish
   ```

## Publishing Options

### Preview Release

To publish as a preview (pre-release):

```bash
vsce publish --pre-release
```

### Minor/Patch Updates

For minor or patch updates, just increment the version:

```bash
# Update version in package.json (e.g., 0.1.0 -> 0.1.1)
npm run package
vsce publish minor  # or patch
```

### Major Updates

For major version updates:

```bash
# Update version in package.json (e.g., 0.1.0 -> 1.0.0)
npm run package
vsce publish major
```

## Verification

After publishing, verify the extension:

1. Visit the [VS Code Marketplace](https://marketplace.visualstudio.com/vscode)
2. Search for "MockForge"
3. Verify the extension page shows correct information
4. Test installation from the marketplace

## Marketplace URL

Once published, the extension will be available at:
```
https://marketplace.visualstudio.com/items?itemName=saasy-solutions.mockforge-vscode
```

## Troubleshooting

### Authentication Issues

If you encounter authentication errors:
1. Generate a new PAT from Azure DevOps
2. Run `vsce login saasy-solutions` again
3. Ensure the PAT has "Marketplace (manage)" scope

### Version Conflicts

If you get version conflicts:
- Ensure the version in `package.json` is unique
- Check existing versions on the marketplace
- Use `vsce publish --yarn` if using yarn

### Package Size

The extension package should be under 50MB. Current package size: ~77KB (well within limits).

## CI/CD Integration

To automate publishing in CI/CD:

1. Store the Azure DevOps PAT as a secret
2. Add a publish step to your CI workflow:
   ```yaml
   - name: Publish Extension
     run: |
       cd vscode-extension
       npm install -g @vscode/vsce
       echo "${{ secrets.AZURE_DEVOPS_PAT }}" | vsce login saasy-solutions
       vsce publish
     if: github.ref == 'refs/heads/main' && github.event_name == 'push'
   ```

## Current Status

- ✅ Extension is packaged and ready
- ✅ README.md updated with all features
- ✅ CHANGELOG.md created
- ✅ package.json configured with publisher and metadata
- ⏳ Ready for first-time publishing to marketplace

