# Code Signing and Notarization Guide

This guide covers code signing and notarization for MockForge Desktop on Windows and macOS.

## Overview

Code signing ensures that:
- Users trust the application
- OS security warnings are minimized
- Auto-updates work properly
- Distribution is professional

## Windows Code Signing

### Prerequisites

1. **Code Signing Certificate**
   - Purchase from a Certificate Authority (CA)
   - Common CAs: DigiCert, Sectigo, GlobalSign
   - Cost: ~$200-500/year

2. **Certificate Format**
   - PFX/P12 format
   - Or install in Windows Certificate Store

### Configuration

1. **Install Certificate**
   ```powershell
   # Import PFX certificate
   Import-PfxCertificate -FilePath certificate.pfx -CertStoreLocation Cert:\LocalMachine\My
   ```

2. **Get Thumbprint**
   ```powershell
   Get-ChildItem -Path Cert:\LocalMachine\My | Where-Object {$_.Subject -like "*MockForge*"}
   ```

3. **Update tauri.conf.json**
   ```json
   {
     "tauri": {
       "bundle": {
         "windows": {
           "certificateThumbprint": "YOUR_CERTIFICATE_THUMBPRINT"
         }
       }
     }
   }
   ```

### Signing Process

Tauri automatically signs the MSI during build if:
- Certificate is in the store
- Thumbprint is configured
- Certificate has code signing capability

### Testing

```powershell
# Verify signature
Get-AuthenticodeSignature .\target\release\bundle\msi\MockForge_*.msi
```

## macOS Code Signing & Notarization

### Prerequisites

1. **Apple Developer Account**
   - Enroll in Apple Developer Program ($99/year)
   - Create certificates in Apple Developer Portal

2. **Certificates**
   - Developer ID Application certificate
   - Developer ID Installer certificate (for DMG)

### Setup

1. **Create Certificates**
   - Go to https://developer.apple.com/account
   - Certificates → Create
   - Select "Developer ID Application"
   - Download and install in Keychain

2. **Get Signing Identity**
   ```bash
   security find-identity -v -p codesigning
   ```

3. **Update tauri.conf.json**
   ```json
   {
     "tauri": {
       "bundle": {
         "macOS": {
           "signingIdentity": "Developer ID Application: Your Name (TEAM_ID)"
         }
       }
     }
   }
   ```

### Signing

Tauri automatically signs during build if:
- Signing identity is configured
- Certificate is in Keychain
- App entitlements are set

### Notarization

Notarization is required for distribution outside Mac App Store.

1. **Build and Sign**
   ```bash
   cargo tauri build
   ```

2. **Create Archive**
   ```bash
   # Archive the app
   ditto -c -k --keepParent \
     target/release/bundle/macos/MockForge.app \
     MockForge.zip
   ```

3. **Submit for Notarization**
   ```bash
   xcrun notarytool submit MockForge.zip \
     --apple-id your@email.com \
     --team-id YOUR_TEAM_ID \
     --password YOUR_APP_SPECIFIC_PASSWORD \
     --wait
   ```

4. **Staple Ticket**
   ```bash
   xcrun stapler staple target/release/bundle/macos/MockForge.app
   ```

### Entitlements

Create `entitlements.plist`:

```xml
<?xml version="1.0" encoding="UTF-8"?>
<!DOCTYPE plist PUBLIC "-//Apple//DTD PLIST 1.0//EN" "http://www.apple.com/DTDs/PropertyList-1.0.dtd">
<plist version="1.0">
<dict>
    <key>com.apple.security.cs.allow-jit</key>
    <true/>
    <key>com.apple.security.cs.allow-unsigned-executable-memory</key>
    <true/>
    <key>com.apple.security.cs.allow-dyld-environment-variables</key>
    <true/>
    <key>com.apple.security.cs.disable-library-validation</key>
    <true/>
</dict>
</plist>
```

Update `tauri.conf.json`:

```json
{
  "tauri": {
    "bundle": {
      "macOS": {
        "entitlements": "entitlements.plist"
      }
    }
  }
}
```

## Linux

Linux doesn't require code signing, but you can:
- Sign packages with GPG
- Use package repositories
- Provide checksums

## CI/CD Integration

### GitHub Actions Example

```yaml
- name: Sign Windows App
  if: matrix.os == 'windows'
  run: |
    signtool sign /f certificate.pfx /p ${{ secrets.CERT_PASSWORD }} /t http://timestamp.digicert.com app.msi

- name: Notarize macOS App
  if: matrix.os == 'macos'
  run: |
    xcrun notarytool submit app.zip \
      --apple-id ${{ secrets.APPLE_ID }} \
      --team-id ${{ secrets.APPLE_TEAM_ID }} \
      --password ${{ secrets.APPLE_APP_PASSWORD }} \
      --wait
```

## Cost Estimation

- **Windows Certificate**: $200-500/year
- **Apple Developer**: $99/year
- **Total**: ~$300-600/year

## Alternatives

### For Open Source Projects

1. **Windows**:
   - Self-signed certificate (shows warning)
   - Or use Windows Store (free for open source)

2. **macOS**:
   - Ad-hoc signing (free, but shows warning)
   - Or use Mac App Store (requires $99/year)

### For Testing

- Use self-signed certificates
- Users can bypass warnings
- Not suitable for production distribution

## Security Best Practices

1. **Store Secrets Securely**
   - Use environment variables
   - Use secret management (GitHub Secrets, etc.)
   - Never commit certificates

2. **Rotate Certificates**
   - Renew before expiration
   - Update thumbprints/identities
   - Revoke old certificates

3. **Monitor Signing**
   - Verify signatures after build
   - Test on clean systems
   - Check notarization status

## Troubleshooting

### Windows

**"Certificate not found"**
- Verify certificate is in LocalMachine\My store
- Check thumbprint matches
- Ensure certificate has code signing capability

**"Timestamp server unavailable"**
- Use alternative timestamp server
- Or skip timestamp (not recommended)

### macOS

**"Code object is not signed"**
- Verify signing identity
- Check certificate in Keychain
- Ensure entitlements file exists

**"Notarization failed"**
- Check Apple ID credentials
- Verify team ID
- Review notarization logs
- Fix any issues and resubmit

## Resources

- [Windows Code Signing](https://docs.microsoft.com/en-us/windows/win32/seccrypto/cryptography-tools)
- [macOS Code Signing](https://developer.apple.com/documentation/security/notarizing_macos_software_before_distribution)
- [Tauri Code Signing](https://tauri.app/v1/guides/building/code-signing)
