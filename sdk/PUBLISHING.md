# SDK publishing runbook

The five client SDKs each publish to their language's package registry from
its own GitHub Actions workflow under `.github/workflows/`. All five follow
the same pattern: tag push triggers a release, manual dispatch supports
dry-runs, secrets live as repo secrets, and a concurrency group serialises
in-flight publishes.

| SDK     | Workflow                      | Trigger tag       | Registry             | Required secrets |
|---------|-------------------------------|-------------------|----------------------|------------------|
| Node.js | `npm-publish-sdk.yml`         | `sdk-v*`          | npm (`@mockforge-dev/sdk`) | `NPM_TOKEN` |
| Python  | `python-publish-sdk.yml`      | `python-sdk-v*`   | PyPI (`mockforge-sdk`) | Trusted Publishers env `pypi`, or `PYPI_API_TOKEN` |
| Java    | `maven-publish-sdk.yml`       | `java-sdk-v*`     | Maven Central (`com.mockforge:mockforge-sdk`) | `OSSRH_USERNAME`, `OSSRH_TOKEN`, `MAVEN_GPG_PRIVATE_KEY`, `MAVEN_GPG_PASSPHRASE` |
| .NET    | `nuget-publish-sdk.yml`       | `dotnet-sdk-v*`   | NuGet (`MockForge.Sdk`)    | `NUGET_API_KEY` |
| Go      | `go-publish-sdk.yml`          | `go-sdk-v*`       | proxy.golang.org (anon)    | none |

## Cutting a release

```bash
# 1. Bump the SDK's version field
#    - sdk/nodejs/package.json
#    - sdk/python/setup.py
#    - sdk/java/pom.xml         <version>
#    - sdk/dotnet/MockForge.Sdk/MockForge.Sdk.csproj   <Version>
#    - sdk/go/go.mod is implicit (tag = version)
git commit -am "sdk(<lang>): release vX.Y.Z"

# 2. Tag (each registry uses its own tag prefix so they're independent)
git tag sdk-v0.2.1                # Node
git tag python-sdk-v0.1.1         # Python
git tag java-sdk-v0.1.1           # Java
git tag dotnet-sdk-v0.1.1         # .NET
git tag go-sdk-v0.1.1             # Go

# 3. Push — the workflow takes over
git push origin sdk-v0.2.1
```

## Dry-run before tagging

Every workflow has a `workflow_dispatch` entry with a `dry_run` boolean
input. Trigger it from the Actions tab; it builds + tests + packages
without pushing to the registry. Useful when validating new metadata or
signing-key rotations.

## First-time setup checklist

Until each registry's secrets are added to the repo, the corresponding
workflow will fail at the publish step. The audit (#674) ships the
workflows themselves; the secrets are an operational follow-up.

- [x] npm — `NPM_TOKEN` added 2024 (pre-existing)
- [ ] PyPI — configure Trusted Publishers against environment `pypi`
- [ ] Maven Central — register OSSRH portal account + GPG key
- [ ] NuGet — register `MockForge.Sdk` package id + add `NUGET_API_KEY`
- [x] Go — no setup needed (anonymous proxy)
