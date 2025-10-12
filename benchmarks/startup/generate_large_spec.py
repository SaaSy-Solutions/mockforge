#!/usr/bin/env python3
"""
Generate a large OpenAPI spec for startup performance testing.
This creates a spec with 100 endpoints to measure MockForge startup latency.
"""

import json
import sys

def generate_spec(num_endpoints=100):
    """Generate an OpenAPI spec with the specified number of endpoints."""
    spec = {
        "openapi": "3.0.0",
        "info": {
            "title": "Large API for Startup Performance Testing",
            "version": "1.0.0",
            "description": f"API with {num_endpoints} endpoints for testing MockForge startup latency"
        },
        "paths": {}
    }

    # Add diverse endpoint types
    categories = [
        "users", "products", "orders", "customers", "invoices",
        "payments", "shipments", "inventory", "analytics", "reports"
    ]

    endpoint_count = 0
    category_idx = 0

    while endpoint_count < num_endpoints:
        category = categories[category_idx % len(categories)]

        # Create CRUD operations for each category
        operations = [
            {
                "path": f"/{category}",
                "method": "get",
                "summary": f"List all {category}",
                "operationId": f"list_{category}_{endpoint_count}",
                "parameters": [
                    {
                        "name": "limit",
                        "in": "query",
                        "schema": {"type": "integer", "default": 20}
                    },
                    {
                        "name": "offset",
                        "in": "query",
                        "schema": {"type": "integer", "default": 0}
                    }
                ],
                "responses": {
                    "200": {
                        "description": "Success",
                        "content": {
                            "application/json": {
                                "schema": {
                                    "type": "array",
                                    "items": {"$ref": f"#/components/schemas/{category.capitalize()}"}
                                }
                            }
                        }
                    }
                }
            },
            {
                "path": f"/{category}",
                "method": "post",
                "summary": f"Create a {category[:-1]}",
                "operationId": f"create_{category[:-1]}_{endpoint_count}",
                "requestBody": {
                    "required": True,
                    "content": {
                        "application/json": {
                            "schema": {"$ref": f"#/components/schemas/{category.capitalize()}"}
                        }
                    }
                },
                "responses": {
                    "201": {
                        "description": "Created",
                        "content": {
                            "application/json": {
                                "schema": {"$ref": f"#/components/schemas/{category.capitalize()}"}
                            }
                        }
                    }
                }
            },
            {
                "path": f"/{category}/{{id}}",
                "method": "get",
                "summary": f"Get a {category[:-1]} by ID",
                "operationId": f"get_{category[:-1]}_by_id_{endpoint_count}",
                "parameters": [
                    {
                        "name": "id",
                        "in": "path",
                        "required": True,
                        "schema": {"type": "string"}
                    }
                ],
                "responses": {
                    "200": {
                        "description": "Success",
                        "content": {
                            "application/json": {
                                "schema": {"$ref": f"#/components/schemas/{category.capitalize()}"}
                            }
                        }
                    }
                }
            },
            {
                "path": f"/{category}/{{id}}",
                "method": "put",
                "summary": f"Update a {category[:-1]}",
                "operationId": f"update_{category[:-1]}_{endpoint_count}",
                "parameters": [
                    {
                        "name": "id",
                        "in": "path",
                        "required": True,
                        "schema": {"type": "string"}
                    }
                ],
                "requestBody": {
                    "required": True,
                    "content": {
                        "application/json": {
                            "schema": {"$ref": f"#/components/schemas/{category.capitalize()}"}
                        }
                    }
                },
                "responses": {
                    "200": {
                        "description": "Success",
                        "content": {
                            "application/json": {
                                "schema": {"$ref": f"#/components/schemas/{category.capitalize()}"}
                            }
                        }
                    }
                }
            },
            {
                "path": f"/{category}/{{id}}",
                "method": "delete",
                "summary": f"Delete a {category[:-1]}",
                "operationId": f"delete_{category[:-1]}_{endpoint_count}",
                "parameters": [
                    {
                        "name": "id",
                        "in": "path",
                        "required": True,
                        "schema": {"type": "string"}
                    }
                ],
                "responses": {
                    "204": {
                        "description": "Deleted"
                    }
                }
            }
        ]

        for op in operations:
            if endpoint_count >= num_endpoints:
                break

            path = op["path"]
            method = op["method"]

            # Initialize path if it doesn't exist
            if path not in spec["paths"]:
                spec["paths"][path] = {}

            # Add operation to path
            spec["paths"][path][method] = {
                "summary": op["summary"],
                "operationId": op["operationId"],
                "parameters": op.get("parameters", []),
                "responses": op["responses"]
            }

            if "requestBody" in op:
                spec["paths"][path][method]["requestBody"] = op["requestBody"]

            endpoint_count += 1

        category_idx += 1

    # Add component schemas
    spec["components"] = {
        "schemas": {}
    }

    for category in categories:
        spec["components"]["schemas"][category.capitalize()] = {
            "type": "object",
            "properties": {
                "id": {"type": "string"},
                "name": {"type": "string"},
                "description": {"type": "string"},
                "created_at": {"type": "string", "format": "date-time"},
                "updated_at": {"type": "string", "format": "date-time"},
                "status": {"type": "string", "enum": ["active", "inactive", "pending"]},
                "metadata": {
                    "type": "object",
                    "additionalProperties": True
                }
            },
            "required": ["id", "name"]
        }

    return spec

if __name__ == "__main__":
    num_endpoints = int(sys.argv[1]) if len(sys.argv) > 1 else 100
    spec = generate_spec(num_endpoints)

    # Print to stdout
    print(json.dumps(spec, indent=2))
