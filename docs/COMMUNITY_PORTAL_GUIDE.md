# Community Portal - Complete Implementation Guide

Comprehensive guide for creating a community portal with showcase, templates library, and learning resources for MockForge.

## Table of Contents

- [Overview](#overview)
- [Architecture](#architecture)
- [Showcase](#showcase)
- [Templates Library](#templates-library)
- [Learning Resources](#learning-resources)
- [Community Features](#community-features)
- [Implementation](#implementation)
- [Content Strategy](#content-strategy)

---

## Overview

The MockForge Community Portal provides a centralized hub for:

- **Showcase**: Featured projects, success stories, and community highlights
- **Templates Library**: Curated collection of templates, scenarios, and plugins
- **Learning Resources**: Tutorials, guides, examples, and documentation
- **Community Engagement**: Forums, discussions, and collaboration

### Key Features

✅ **Marketplace Integration**: Templates, scenarios, and plugins
✅ **Showcase Gallery**: Featured projects and use cases
✅ **Learning Hub**: Tutorials, guides, and examples
✅ **Community Forum**: Discussions and Q&A
✅ **Contributor Recognition**: Credits and badges
✅ **Search & Discovery**: Find resources easily

---

## Architecture

### Portal Structure

```
┌─────────────────────────────────────────────────────────────┐
│                    Community Portal                          │
├─────────────────────────────────────────────────────────────┤
│                                                               │
│  ┌──────────────┐  ┌──────────────┐  ┌──────────────┐      │
│  │   Showcase   │  │   Templates  │  │   Learning   │      │
│  │   Gallery    │  │   Library    │  │   Resources  │      │
│  └──────┬───────┘  └──────┬───────┘  └──────┬───────┘      │
│         │                  │                  │              │
│  ┌──────▼──────────────────▼──────────────────▼──────┐      │
│  │           Marketplace Backend                      │      │
│  │  (Templates, Scenarios, Plugins)                  │      │
│  └────────────────────────────────────────────────────┘      │
│                                                               │
│  ┌──────────────┐  ┌──────────────┐  ┌──────────────┐      │
│  │   Forums     │  │   Examples   │  │   Blog       │      │
│  │   & Q&A      │  │   Gallery    │  │   & News     │      │
│  └──────────────┘  └──────────────┘  └──────────────┘      │
│                                                               │
└───────────────────────────────────────────────────────────────┘
```

### Technology Stack

- **Frontend**: React (existing Admin UI)
- **Backend**: Rust (existing registry server)
- **Database**: PostgreSQL (existing schema)
- **Storage**: S3-compatible (existing)
- **Search**: Full-text search with PostgreSQL

---

## Showcase

### Featured Projects

Highlight exceptional community projects and use cases.

**Showcase Entry Structure:**

```yaml
# showcase/projects/ecommerce-platform.yaml
title: "E-commerce Platform Mock"
author: "community-user"
description: "Complete e-commerce API mock with shopping carts, orders, and payments"
category: "ecommerce"
tags:
  - ecommerce
  - shopping-cart
  - payments
  - rest-api
featured: true
screenshot: "https://example.com/screenshot.png"
demo_url: "https://demo.mockforge.dev/ecommerce"
source_url: "https://github.com/user/ecommerce-mock"
template_id: "ecommerce-store@1.0.0"
scenario_id: "ecommerce-scenario@1.0.0"
stats:
  downloads: 1250
  stars: 45
  forks: 12
  rating: 4.8
testimonials:
  - author: "John Doe"
    company: "Acme Corp"
    text: "This template saved us weeks of development time!"
```

**Showcase API Endpoints:**

```rust
// GET /api/v1/showcase/projects
// Returns featured projects with pagination

// GET /api/v1/showcase/projects/{id}
// Returns detailed project information

// POST /api/v1/showcase/projects
// Submit a project for showcase (requires authentication)

// GET /api/v1/showcase/categories
// Returns available showcase categories
```

### Success Stories

Real-world use cases and testimonials.

**Success Story Format:**

```yaml
# showcase/stories/acme-corp.yaml
title: "Acme Corp: Accelerating API Development"
company: "Acme Corporation"
industry: "E-commerce"
author: "Jane Smith"
role: "Lead API Developer"
date: "2024-01-15"
challenge: |
  Acme Corp needed to develop a new payment API but couldn't
  wait for backend services to be ready.
solution: |
  Used MockForge to create realistic payment mocks with
  various scenarios (success, failure, retry).
results:
  - "Reduced development time by 60%"
  - "Enabled parallel frontend/backend development"
  - "Improved test coverage with edge cases"
tags:
  - payments
  - fintech
  - api-development
featured: true
```

---

## Templates Library

### Template Categories

Organize templates by category and use case:

1. **E-commerce**
   - Shopping carts
   - Product catalogs
   - Order management
   - Payment processing

2. **Fintech**
   - Banking APIs
   - Payment gateways
   - Trading platforms
   - Financial reporting

3. **Social Media**
   - User profiles
   - Posts and feeds
   - Messaging
   - Notifications

4. **IoT**
   - Device management
   - Sensor data
   - Telemetry
   - Command/control

5. **Healthcare**
   - Patient records
   - Appointment scheduling
   - Medical devices
   - Telemedicine

6. **Enterprise**
   - CRM systems
   - HR management
   - Document management
   - Collaboration tools

### Template Discovery

**Search & Filter:**

```typescript
// Template search interface
interface TemplateSearch {
  query?: string;
  category?: string;
  tags?: string[];
  minRating?: number;
  sortBy?: 'popularity' | 'rating' | 'recent' | 'downloads';
  page?: number;
  limit?: number;
}
```

**Featured Templates:**

- Curated by MockForge team
- High-quality, well-documented
- Regularly updated
- Community-vetted

### Template Details Page

Each template should include:

- **Overview**: Description, use cases, features
- **Screenshots/Demos**: Visual examples
- **Installation**: Quick start guide
- **Documentation**: Full documentation
- **Examples**: Code examples and tutorials
- **Reviews**: User reviews and ratings
- **Versions**: Version history and changelog
- **Related**: Similar templates and scenarios

---

## Learning Resources

### Tutorials

Step-by-step guides for common tasks.

**Tutorial Structure:**

```markdown
# Tutorial: Building Your First Mock API

## Prerequisites
- MockForge installed
- Basic understanding of REST APIs

## Step 1: Create a New Project
...

## Step 2: Define Your API
...

## Step 3: Add Mock Data
...

## Step 4: Test Your Mock
...

## Next Steps
- Learn about advanced features
- Explore templates
- Join the community
```

**Tutorial Categories:**

1. **Getting Started**
   - Installation
   - First mock
   - Basic concepts

2. **Advanced Features**
   - Multi-protocol mocking
   - AI-driven mocking
   - Chaos engineering

3. **Integration**
   - CI/CD integration
   - Framework integration
   - Cloud deployment

4. **Best Practices**
   - Mock design patterns
   - Testing strategies
   - Performance optimization

### Examples Gallery

Code examples for common scenarios.

**Example Format:**

```yaml
# examples/rest-api-basic.yaml
title: "Basic REST API Mock"
description: "Simple REST API with CRUD operations"
difficulty: "beginner"
language: "yaml"
category: "rest-api"
tags:
  - rest
  - crud
  - beginner
code: |
  http:
    port: 3000
    routes:
      - path: /users
        method: GET
        response:
          status: 200
          body:
            - id: 1
              name: "Alice"
            - id: 2
              name: "Bob"
```

### Video Tutorials

- YouTube integration
- Embedded video player
- Playlists by topic
- Transcripts and captions

### Documentation Hub

- API reference
- CLI reference
- SDK documentation
- Architecture guides
- FAQ

---

## Community Features

### Forums & Discussions

**Forum Categories:**

1. **General Discussion**
   - Introductions
   - Announcements
   - Feature requests

2. **Help & Support**
   - Getting started
   - Troubleshooting
   - Q&A

3. **Show & Tell**
   - Share your mocks
   - Project showcases
   - Tips & tricks

4. **Contributions**
   - Plugin development
   - Template creation
   - Documentation

### User Profiles

**Profile Features:**

- Avatar and bio
- Contribution history
- Published templates/scenarios
- Reviews and ratings given
- Badges and achievements
- Activity feed

### Badges & Achievements

**Badge Types:**

- **Contributor**: Published templates/scenarios
- **Helper**: Answered questions
- **Expert**: High-rated contributions
- **Early Adopter**: Early community member
- **Power User**: Advanced feature usage

---

## Implementation

### Database Schema

**Showcase Tables:**

```sql
-- Showcase projects
CREATE TABLE showcase_projects (
    id UUID PRIMARY KEY DEFAULT gen_random_uuid(),
    title VARCHAR(255) NOT NULL,
    author_id UUID NOT NULL REFERENCES users(id),
    description TEXT NOT NULL,
    category VARCHAR(50) NOT NULL,
    tags TEXT[] DEFAULT ARRAY[]::TEXT[],
    featured BOOLEAN DEFAULT FALSE,
    screenshot_url TEXT,
    demo_url TEXT,
    source_url TEXT,
    template_id UUID REFERENCES templates(id),
    scenario_id UUID REFERENCES scenarios(id),
    stats_json JSONB DEFAULT '{"downloads": 0, "stars": 0, "forks": 0, "rating": 0.0}'::jsonb,
    approved BOOLEAN DEFAULT FALSE,
    created_at TIMESTAMPTZ NOT NULL DEFAULT NOW(),
    updated_at TIMESTAMPTZ NOT NULL DEFAULT NOW()
);

-- Success stories
CREATE TABLE success_stories (
    id UUID PRIMARY KEY DEFAULT gen_random_uuid(),
    title VARCHAR(255) NOT NULL,
    company VARCHAR(255) NOT NULL,
    industry VARCHAR(100),
    author_name VARCHAR(255) NOT NULL,
    author_role VARCHAR(255),
    challenge TEXT NOT NULL,
    solution TEXT NOT NULL,
    results TEXT[] DEFAULT ARRAY[]::TEXT[],
    tags TEXT[] DEFAULT ARRAY[]::TEXT[],
    featured BOOLEAN DEFAULT FALSE,
    approved BOOLEAN DEFAULT FALSE,
    created_at TIMESTAMPTZ NOT NULL DEFAULT NOW()
);

-- Tutorials
CREATE TABLE tutorials (
    id UUID PRIMARY KEY DEFAULT gen_random_uuid(),
    title VARCHAR(255) NOT NULL,
    author_id UUID NOT NULL REFERENCES users(id),
    description TEXT NOT NULL,
    category VARCHAR(50) NOT NULL,
    difficulty VARCHAR(20) CHECK (difficulty IN ('beginner', 'intermediate', 'advanced')),
    content_markdown TEXT NOT NULL,
    tags TEXT[] DEFAULT ARRAY[]::TEXT[],
    featured BOOLEAN DEFAULT FALSE,
    views INTEGER DEFAULT 0,
    rating DECIMAL(3,2) DEFAULT 0.0,
    rating_count INTEGER DEFAULT 0,
    published BOOLEAN DEFAULT FALSE,
    created_at TIMESTAMPTZ NOT NULL DEFAULT NOW(),
    updated_at TIMESTAMPTZ NOT NULL DEFAULT NOW()
);

-- Examples
CREATE TABLE examples (
    id UUID PRIMARY KEY DEFAULT gen_random_uuid(),
    title VARCHAR(255) NOT NULL,
    author_id UUID NOT NULL REFERENCES users(id),
    description TEXT NOT NULL,
    category VARCHAR(50) NOT NULL,
    difficulty VARCHAR(20),
    language VARCHAR(50),
    code TEXT NOT NULL,
    tags TEXT[] DEFAULT ARRAY[]::TEXT[],
    featured BOOLEAN DEFAULT FALSE,
    views INTEGER DEFAULT 0,
    rating DECIMAL(3,2) DEFAULT 0.0,
    rating_count INTEGER DEFAULT 0,
    published BOOLEAN DEFAULT FALSE,
    created_at TIMESTAMPTZ NOT NULL DEFAULT NOW(),
    updated_at TIMESTAMPTZ NOT NULL DEFAULT NOW()
);
```

### API Endpoints

**Showcase API:**

```rust
// GET /api/v1/showcase/projects
// Query parameters: category, tags, featured, page, limit
pub async fn list_showcase_projects(
    Query(params): Query<ShowcaseQuery>,
) -> Result<Json<PaginatedResponse<ShowcaseProject>>>

// GET /api/v1/showcase/projects/{id}
pub async fn get_showcase_project(
    Path(id): Path<Uuid>,
) -> Result<Json<ShowcaseProject>>

// POST /api/v1/showcase/projects
// Requires authentication
pub async fn create_showcase_project(
    user: AuthenticatedUser,
    Json(payload): Json<CreateShowcaseProjectRequest>,
) -> Result<Json<ShowcaseProject>>

// GET /api/v1/showcase/stories
pub async fn list_success_stories(
    Query(params): Query<StoryQuery>,
) -> Result<Json<Vec<SuccessStory>>>
```

**Learning Resources API:**

```rust
// GET /api/v1/tutorials
pub async fn list_tutorials(
    Query(params): Query<TutorialQuery>,
) -> Result<Json<PaginatedResponse<Tutorial>>>

// GET /api/v1/tutorials/{id}
pub async fn get_tutorial(
    Path(id): Path<Uuid>,
) -> Result<Json<Tutorial>>

// GET /api/v1/examples
pub async fn list_examples(
    Query(params): Query<ExampleQuery>,
) -> Result<Json<PaginatedResponse<Example>>>
```

### Frontend Components

**Showcase Gallery:**

```typescript
// components/showcase/ShowcaseGallery.tsx
export function ShowcaseGallery() {
  const [projects, setProjects] = useState<ShowcaseProject[]>([]);
  const [filter, setFilter] = useState<ShowcaseFilter>({});

  useEffect(() => {
    fetchShowcaseProjects(filter).then(setProjects);
  }, [filter]);

  return (
    <div className="grid grid-cols-1 md:grid-cols-2 lg:grid-cols-3 gap-6">
      {projects.map(project => (
        <ShowcaseCard key={project.id} project={project} />
      ))}
    </div>
  );
}
```

**Template Library:**

```typescript
// components/templates/TemplateLibrary.tsx
export function TemplateLibrary() {
  const [templates, setTemplates] = useState<Template[]>([]);
  const [search, setSearch] = useState('');
  const [category, setCategory] = useState<string | null>(null);

  return (
    <div>
      <TemplateSearchBar
        search={search}
        onSearchChange={setSearch}
        category={category}
        onCategoryChange={setCategory}
      />
      <TemplateGrid templates={templates} />
    </div>
  );
}
```

**Learning Hub:**

```typescript
// components/learning/LearningHub.tsx
export function LearningHub() {
  return (
    <div>
      <TutorialList />
      <ExampleGallery />
      <VideoTutorials />
      <DocumentationLinks />
    </div>
  );
}
```

---

## Content Strategy

### Initial Content

**Templates (20+):**
- E-commerce (5)
- Fintech (4)
- Social Media (3)
- IoT (3)
- Healthcare (2)
- Enterprise (3)

**Tutorials (15+):**
- Getting Started (5)
- Advanced Features (5)
- Integration (3)
- Best Practices (2)

**Examples (30+):**
- REST API (10)
- gRPC (5)
- WebSocket (5)
- GraphQL (5)
- Other protocols (5)

**Success Stories (5+):**
- Real-world use cases
- Testimonials
- Case studies

### Content Curation

**Quality Standards:**

- Well-documented
- Tested and verified
- Follows best practices
- Includes examples
- Regular updates

**Review Process:**

1. Community submission
2. Automated validation
3. Manual review
4. Approval/rejection
5. Publication

### Community Contributions

**Contribution Guidelines:**

- Code of conduct
- Submission process
- Review criteria
- Recognition system

**Incentives:**

- Contributor badges
- Featured placement
- Recognition in release notes
- Community spotlight

---

## Integration with Existing Systems

### Marketplace Integration

The community portal integrates with existing marketplace infrastructure:

- **Templates**: Uses existing template marketplace
- **Scenarios**: Uses existing scenario marketplace
- **Plugins**: Uses existing plugin marketplace
- **Reviews**: Uses existing review system

### User System

- **Authentication**: Uses existing auth system
- **Profiles**: Extends user profiles
- **Permissions**: Uses existing RBAC

### Search

- **Full-text search**: PostgreSQL full-text search
- **Filtering**: Category, tags, ratings
- **Sorting**: Popularity, rating, recent

---

## Roadmap

### Phase 1: Foundation (Current)
- [x] Marketplace infrastructure
- [x] Template/scenario/plugin systems
- [x] Review system
- [ ] Showcase gallery
- [ ] Basic tutorials

### Phase 2: Content (Next)
- [ ] Curated template library
- [ ] Comprehensive tutorials
- [ ] Example gallery
- [ ] Success stories

### Phase 3: Community (Future)
- [ ] Forums & discussions
- [ ] User profiles & badges
- [ ] Contribution system
- [ ] Community events

### Phase 4: Advanced (Future)
- [ ] Video tutorials
- [ ] Interactive examples
- [ ] Certification program
- [ ] Community marketplace

---

## Summary

The Community Portal provides:

- ✅ **Showcase**: Featured projects and success stories
- ✅ **Templates Library**: Curated collection of templates
- ✅ **Learning Resources**: Tutorials, examples, documentation
- ✅ **Community Features**: Forums, profiles, badges
- ✅ **Integration**: Works with existing marketplace

**Status**: Infrastructure exists, content and UI components needed

---

**Last Updated**: 2024-01-01
**Version**: 1.0
