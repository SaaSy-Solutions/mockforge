"""Turn a CVE profile into a MockForge mock of the guarded route.

The mock makes `mockforge serve` answer the profile's route with a small,
fixed 200 body, so a load balancer in front of it sees a target that *looks*
like the guarded application. MockForge serves canned responses only: none of
the vulnerable code runs, and the spec carries no payload. This is a
convenience for manually exploring a rule against a realistic origin; the
automated `cve_pack.py run` uses the owned correlation recorder instead.
"""

from cve_pack_profile import validate


def to_openapi(profile):
    """Return an OpenAPI 3.0.3 mock document for the profile's route."""
    profile = validate(profile)
    body = {"receiver": "mockforge-cve-pack", "cve": profile["cve"]}
    return {
        "openapi": "3.0.3",
        "info": {
            "title": f"{profile['cve']} rule-test mock",
            "version": "1.0.0",
            "description": (
                "Mocks the guarded route so a load balancer sees a realistic "
                "origin. Canned response only; no vulnerable behavior is run."
            ),
        },
        "paths": {
            profile["route"]: {
                "get": {
                    "operationId": "cvePackGuardedRoute",
                    "responses": {
                        "200": {
                            "description": "Mock origin reached; no exploit simulated",
                            "content": {
                                "application/json": {
                                    "schema": {
                                        "type": "object",
                                        "required": ["receiver", "cve"],
                                        "properties": {
                                            "receiver": {"type": "string"},
                                            "cve": {"type": "string"},
                                        },
                                    },
                                    "example": body,
                                }
                            },
                        }
                    },
                }
            }
        },
    }
