# Run a mock in GitHub Actions

MockForge ships an action that starts a mock inside a workflow job, so a test
suite can run against an API that behaves as the spec says, with nothing to
host.

```yaml
jobs:
  api-tests:
    runs-on: ubuntu-latest
    steps:
      - uses: actions/checkout@v7
      - uses: SaaSy-Solutions/mockforge/.github/actions/setup@main
        id: mock
        with:
          spec: ./openapi.json
      - run: curl http://127.0.0.1:3000/users
      - name: Stop the mock
        if: always()
        env:
          CONTAINER: ${{ steps.mock.outputs.container }}
        run: docker rm --force "$CONTAINER"
```

The action pulls the published image, starts the mock with the spec mounted
read-only, and waits until the port answers. It fails with the container's
last lines when the mock does not start.

Pin it to a release tag or a commit, as for any action, once the release
that carries it is out. `@main` is the branch, and it moves.

## What it needs

A Linux runner. The action runs a container, and of GitHub's hosted runners
only `ubuntu-latest` has Docker. The image is built for `linux/amd64`.

The action is in this repository, under `.github/actions/setup`. Publishing
it to the GitHub Marketplace as `mockforge/setup` needs a repository of its
own for it, which is not there yet.

## Inputs

| input | default | what it is |
| --- | --- | --- |
| `spec` | required | The OpenAPI file the mock serves, such as `./openapi.json`. |
| `port` | `3000` | The port the mock listens on, on the runner. |
| `version` | `latest` | The tag of the image to run, such as `0.3.224`. `latest` is the newest image published. |
| `image` | `ghcr.io/saasy-solutions/mockforge` | The image to run. |
| `timeout` | `60` | Seconds to wait for the mock to answer. |
| `env` | none | Settings for the mock, one `NAME=value` a line. |

## Outputs

| output | what it is |
| --- | --- |
| `url` | Where the mock answers, such as `http://127.0.0.1:3000`. |
| `image` | The image that was run, with its tag. |
| `container` | The id of the container. |

## Settings

The image turns simulated latency on, as it does when you run it by hand. A
test suite usually wants it off. Pass settings through `env`; each line is
given to the mock as it is:

```yaml
      - uses: SaaSy-Solutions/mockforge/.github/actions/setup@main
        with:
          spec: ./openapi.json
          env: |
            MOCKFORGE_LATENCY_ENABLED=false
            MOCKFORGE_RESPONSE_TEMPLATE_EXPAND=false
```

The settings are the environment variables in [the configuration guide](../configuration/environment.md).

## After the job

The mock listens on `127.0.0.1` only, so nothing outside the job reaches it.
The container keeps running until something stops it. On a hosted runner that
is the end of the job. On a runner of your own, stop it in a last step with
`if: always()`, as the example above does.
