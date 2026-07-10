#!/bin/sh
# MockForge container entrypoint (issue #930).
#
# The image sets ENTRYPOINT to this script and CMD to the default subcommand,
# so the documented compose form works:
#
#     command: serve --config /config/mockforge.yaml
#
# Before #930 the image only set CMD, so a user-supplied `command:` replaced
# the whole vector and Docker tried to exec `serve` as a binary:
#
#     exec: "serve": executable file not found in $PATH
#
# The documented workaround was to prefix the binary name
# (`command: mockforge serve ...`). We strip that leading argument here so the
# workaround keeps working and nobody's existing compose file breaks.
#
# To get a shell in the image, override the entrypoint:
#     docker run --entrypoint /bin/sh -it ghcr.io/saasy-solutions/mockforge
set -e

if [ "$1" = "mockforge" ] || [ "$1" = "/usr/local/bin/mockforge" ]; then
    shift
fi

exec /usr/local/bin/mockforge "$@"
