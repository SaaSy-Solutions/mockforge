//! Community portal API service
//!
//! Provides API client for showcase, learning resources, and community features

import { authenticatedFetch } from '../utils/apiClient';

export interface ShowcaseProject {
  id: string;
  title: string;
  author: string;
  author_avatar?: string;
  description: string;
  category: string;
  tags: string[];
  featured: boolean;
  screenshot?: string;
  demo_url?: string;
  source_url?: string;
  template_id?: string;
  scenario_id?: string;
  stats: {
    downloads: number;
    stars: number;
    forks: number;
    rating: number;
  };
  testimonials: Array<{
    author: string;
    company?: string;
    text: string;
  }>;
  created_at: string;
  updated_at: string;
}

export interface SuccessStory {
  id: string;
  title: string;
  company: string;
  industry: string;
  author: string;
  role: string;
  date: string;
  challenge: string;
  solution: string;
  results: string[];
  featured: boolean;
}

export interface LearningResource {
  id: string;
  title: string;
  description: string;
  category: string;
  resource_type: string; // tutorial, example, video, guide
  difficulty: string; // beginner, intermediate, advanced
  tags: string[];
  content_url?: string;
  video_url?: string;
  code_examples: Array<{
    title: string;
    language: string;
    code: string;
    description?: string;
  }>;
  author: string;
  views: number;
  rating: number;
  created_at: string;
  updated_at: string;
}

export interface ApiResponse<T> {
  success: boolean;
  data?: T;
  error?: string;
}

class CommunityApiService {
  private async fetchJson<T>(url: string, options?: RequestInit): Promise<ApiResponse<T>> {
    const response = await authenticatedFetch(url, options);
    if (!response.ok) {
      if (response.status === 401) {
        throw new Error('Authentication required');
      }
      if (response.status === 403) {
        throw new Error('Access denied');
      }
      const errorText = await response.text().catch(() => 'Unknown error');
      throw new Error(`API error: ${response.status} - ${errorText}`);
    }
    return response.json();
  }

  // Showcase API
  async getShowcaseProjects(params?: {
    category?: string;
    featured?: boolean;
    limit?: number;
    offset?: number;
  }): Promise<ApiResponse<ShowcaseProject[]>> {
    const queryParams = new URLSearchParams();
    if (params?.category) queryParams.append('category', params.category);
    if (params?.featured !== undefined) queryParams.append('featured', String(params.featured));
    if (params?.limit) queryParams.append('limit', String(params.limit));
    if (params?.offset) queryParams.append('offset', String(params.offset));

    const url = `/__mockforge/community/showcase/projects${queryParams.toString() ? `?${queryParams}` : ''}`;
    return this.fetchJson<ShowcaseProject[]>(url);
  }

  async getShowcaseProject(id: string): Promise<ApiResponse<ShowcaseProject>> {
    return this.fetchJson<ShowcaseProject>(`/__mockforge/community/showcase/projects/${id}`);
  }

  async getShowcaseCategories(): Promise<ApiResponse<string[]>> {
    return this.fetchJson<string[]>(`/__mockforge/community/showcase/categories`);
  }

  async getSuccessStories(params?: {
    featured?: boolean;
    limit?: number;
  }): Promise<ApiResponse<SuccessStory[]>> {
    const queryParams = new URLSearchParams();
    if (params?.featured !== undefined) queryParams.append('featured', String(params.featured));
    if (params?.limit) queryParams.append('limit', String(params.limit));

    const url = `/__mockforge/community/showcase/stories${queryParams.toString() ? `?${queryParams}` : ''}`;
    return this.fetchJson<SuccessStory[]>(url);
  }

  async submitShowcaseProject(project: Partial<ShowcaseProject>): Promise<ApiResponse<string>> {
    return this.fetchJson<string>('/__mockforge/community/showcase/submit', {
      method: 'POST',
      headers: { 'Content-Type': 'application/json' },
      body: JSON.stringify(project),
    });
  }

  // Learning Resources API
  async getLearningResources(params?: {
    category?: string;
    type?: string;
    difficulty?: string;
    limit?: number;
  }): Promise<ApiResponse<LearningResource[]>> {
    const queryParams = new URLSearchParams();
    if (params?.category) queryParams.append('category', params.category);
    if (params?.type) queryParams.append('type', params.type);
    if (params?.difficulty) queryParams.append('difficulty', params.difficulty);
    if (params?.limit) queryParams.append('limit', String(params.limit));

    const url = `/__mockforge/community/learning/resources${queryParams.toString() ? `?${queryParams}` : ''}`;
    return this.fetchJson<LearningResource[]>(url);
  }

  async getLearningResource(id: string): Promise<ApiResponse<LearningResource>> {
    return this.fetchJson<LearningResource>(`/__mockforge/community/learning/resources/${id}`);
  }

  async getLearningCategories(): Promise<ApiResponse<string[]>> {
    return this.fetchJson<string[]>(`/__mockforge/community/learning/categories`);
  }
}

export const communityApi = new CommunityApiService();
