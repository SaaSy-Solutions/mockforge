# First mock: command and measurement

With Docker installed, start a working HTTP mock from the bundled OpenAPI spec:

```bash
docker run --rm -p 3000:3000 ghcr.io/saasy-solutions/mockforge:latest serve --spec examples/openapi-demo.json --http-port 3000
```

In another terminal:

```bash
curl http://localhost:3000/users
```

The response is a JSON users array. This path needs no Rust toolchain, account, or configuration file. Stop it with Ctrl-C when finished.

## Measured cold start

On October 11, 2026, the first verified `/users` response arrived **9.195 seconds** after starting an uncached image pull:

| Stage | Seconds |
| --- | ---: |
| Download the public image into an empty image cache | 8.392 |
| Start the container and verify the HTTP response | 0.803 |
| Total | **9.195** |

The target is less than 60 seconds from the image pull to the first valid response. The check used a fresh Linux Docker daemon with an empty data root, a new container, and no prior MockForge configuration. Host Docker installation and the preparation of that daemon were outside the timed interval. Download speed and host hardware affect the result.

The tested public image reported version **0.3.227**; its immutable reference was:

```text
ghcr.io/saasy-solutions/mockforge@sha256:c0e9a7d90ddb41b89340750bb6c54375c01a77d81f04bf22779f2a0def5ef7fd
```

The [recorded receipt](https://github.com/SaaSy-Solutions/mockforge/blob/main/docs/qa/first-mock-cold-20261011.json) contains the exact timing and response checks. This measures that published container, independently of newer native CLI releases.

## Reproduce the measurement

From a checkout of the repository, with Python 3 and Docker available:

```bash
python3 scripts/measure-first-mock.py \
  --image ghcr.io/saasy-solutions/mockforge@sha256:c0e9a7d90ddb41b89340750bb6c54375c01a77d81f04bf22779f2a0def5ef7fd \
  --receipt first-mock-receipt.json
```

The script needs permission to start a privileged Docker container. It creates an isolated nested Docker daemon, verifies that the image cache is empty, pulls the public image, starts the example, and checks the JSON response. It removes its own daemon, containers, and anonymous volumes on exit.

To measure the current public tag, omit `--image`. Each receipt records the version actually executed.

## Use your own OpenAPI spec

Save your spec as `api.yaml`, then run:

```bash
docker run --rm -p 3000:3000 --mount type=bind,source="$PWD/api.yaml",target=/app/api.yaml,readonly ghcr.io/saasy-solutions/mockforge:latest serve --spec /app/api.yaml --http-port 3000
```

With the native CLI installed, the equivalent command is `mockforge serve --spec api.yaml`.

For a runnable native project, run `mockforge init my-api`. The generated project includes an example OpenAPI spec and configuration; the command prints the startup command and the curl request to test it. Customizing protocols, chaos, or generated data can wait until the first response works.
