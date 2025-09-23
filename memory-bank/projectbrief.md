# MockForge Project Brief

## Core Mission
MockForge is a comprehensive mocking framework for APIs, gRPC services, and WebSockets that provides a unified interface for creating, managing, and deploying mock servers across different protocols with advanced data generation capabilities.

## Key Objectives
- **Multi-Protocol Support**: HTTP REST APIs, gRPC services, and WebSocket connections
- **Advanced Data Synthesis**: Intelligent mock data generation with smart field inference, deterministic seeding, RAG-driven generation, relationship awareness, and schema graph extraction
- **Production Ready**: Comprehensive testing, security audits, and automated releases
- **Developer Experience**: Modern web-based Admin UI, flexible configuration, and extensible plugin architecture

## Current Status
✅ **Completed Features**:
- CLI infrastructure with import commands (postman, insomnia, curl)
- Postman collection import (fully functional)
- Curl command import (fully functional)
- Format detection with confidence scoring
- Admin UI with modern dashboard
- Multi-protocol server support (HTTP, WebSocket, gRPC)
- Advanced data generation with templates
- OpenAPI spec validation and route generation

🔄 **Next Phase**: Implement Insomnia import and UI integration for import functionality

## Architecture Principles
- **Modular Design**: Separate crates for CLI, core logic, HTTP, WebSocket, gRPC, data generation, and UI
- **Extensible**: Plugin system for custom response generators and data sources
- **Configuration-Driven**: YAML/JSON config with environment variable overrides
- **Performance-Focused**: Async runtime with latency simulation and failure injection

## Success Criteria
- Import functionality works seamlessly across Postman, Insomnia, and curl formats
- UI provides intuitive import experience with preview and selective import
- Generated mock configurations are accurate and production-ready
- Comprehensive test coverage and documentation
