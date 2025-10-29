<template>
  <div class="app">
    <header class="app-header">
      <h1>User Management System</h1>
      <p>Generated with MockForge Vue Client</p>
    </header>

    <main class="app-main">
      <!-- Users Section -->
      <section class="section">
        <h2>Users</h2>

        <div v-if="usersLoading" class="loading">Loading users...</div>
        <div v-else-if="usersError" class="error">Error: {{ usersError.message }}</div>

        <div v-else-if="users" class="users-grid">
          <div v-for="user in users" :key="user.id" class="user-card">
            <h3>{{ user.name }}</h3>
            <p>{{ user.email }}</p>
            <small>Created: {{ formatDate(user.createdAt) }}</small>
          </div>
        </div>

        <!-- Create User Form -->
        <form @submit.prevent="handleCreateUser" class="form">
          <h3>Create New User</h3>
          <div class="form-group">
            <label for="userName">Name:</label>
            <input
              id="userName"
              v-model="newUser.name"
              type="text"
              required
            />
          </div>
          <div class="form-group">
            <label for="userEmail">Email:</label>
            <input
              id="userEmail"
              v-model="newUser.email"
              type="email"
              required
            />
          </div>
          <button type="submit" :disabled="createUserLoading">
            {{ createUserLoading ? 'Creating...' : 'Create User' }}
          </button>
        </form>
      </section>

      <!-- Posts Section -->
      <section class="section">
        <h2>Posts</h2>

        <div v-if="postsLoading" class="loading">Loading posts...</div>
        <div v-else-if="postsError" class="error">Error: {{ postsError.message }}</div>

        <div v-else-if="posts" class="posts-list">
          <div v-for="post in posts" :key="post.id" class="post-card">
            <h3>{{ post.title }}</h3>
            <p>{{ post.content }}</p>
            <small>
              Author ID: {{ post.authorId }} |
              Published: {{ post.published ? 'Yes' : 'No' }} |
              Created: {{ formatDate(post.createdAt) }}
            </small>
          </div>
        </div>

        <!-- Create Post Form -->
        <form @submit.prevent="handleCreatePost" class="form">
          <h3>Create New Post</h3>
          <div class="form-group">
            <label for="postTitle">Title:</label>
            <input
              id="postTitle"
              v-model="newPost.title"
              type="text"
              required
            />
          </div>
          <div class="form-group">
            <label for="postContent">Content:</label>
            <textarea
              id="postContent"
              v-model="newPost.content"
              required
            />
          </div>
          <div class="form-group">
            <label for="postAuthorId">Author ID:</label>
            <input
              id="postAuthorId"
              v-model.number="newPost.authorId"
              type="number"
              required
            />
          </div>
          <button type="submit" :disabled="createPostLoading">
            {{ createPostLoading ? 'Creating...' : 'Create Post' }}
          </button>
        </form>
      </section>
    </main>
  </div>
</template>

<script setup lang="ts">
import { ref } from 'vue';
import { useGetUsers, useCreateUser, useGetPosts, useCreatePost } from './generated/composables';

// Reactive state for forms
const newUser = ref({ name: '', email: '' });
const newPost = ref({ title: '', content: '', authorId: 1 });

// Use generated composables
const { data: users, loading: usersLoading, error: usersError } = useGetUsers();
const { execute: createUser, loading: createUserLoading } = useCreateUser();
const { data: posts, loading: postsLoading, error: postsError } = useGetPosts();
const { execute: createPost, loading: createPostLoading } = useCreatePost();

// Event handlers
const handleCreateUser = async () => {
  try {
    await createUser(newUser.value);
    newUser.value = { name: '', email: '' };
    alert('User created successfully!');
  } catch (error) {
    alert('Failed to create user');
  }
};

const handleCreatePost = async () => {
  try {
    await createPost(newPost.value);
    newPost.value = { title: '', content: '', authorId: 1 };
    alert('Post created successfully!');
  } catch (error) {
    alert('Failed to create post');
  }
};

// Utility function
const formatDate = (dateString: string) => {
  return new Date(dateString).toLocaleDateString();
};
</script>

<style scoped>
.app {
  text-align: center;
  max-width: 1200px;
  margin: 0 auto;
  padding: 20px;
}

.app-header {
  background-color: #2c3e50;
  padding: 20px;
  color: white;
  margin-bottom: 30px;
  border-radius: 8px;
}

.app-header h1 {
  margin: 0 0 10px 0;
  font-size: 2.5rem;
}

.app-header p {
  margin: 0;
  opacity: 0.8;
  font-size: 1.1rem;
}

.app-main {
  display: grid;
  grid-template-columns: 1fr 1fr;
  gap: 30px;
}

.section {
  background: #f8f9fa;
  padding: 20px;
  border-radius: 8px;
  border: 1px solid #e9ecef;
}

.section h2 {
  margin-top: 0;
  color: #495057;
  border-bottom: 2px solid #42b883;
  padding-bottom: 10px;
}

.users-grid {
  display: grid;
  grid-template-columns: repeat(auto-fill, minmax(250px, 1fr));
  gap: 15px;
  margin-bottom: 20px;
}

.user-card {
  background: white;
  padding: 15px;
  border-radius: 6px;
  border: 1px solid #dee2e6;
  box-shadow: 0 2px 4px rgba(0,0,0,0.1);
}

.user-card h3 {
  margin: 0 0 8px 0;
  color: #42b883;
}

.user-card p {
  margin: 0 0 8px 0;
  color: #6c757d;
}

.user-card small {
  color: #adb5bd;
  font-size: 0.85rem;
}

.posts-list {
  display: flex;
  flex-direction: column;
  gap: 15px;
  margin-bottom: 20px;
}

.post-card {
  background: white;
  padding: 20px;
  border-radius: 6px;
  border: 1px solid #dee2e6;
  box-shadow: 0 2px 4px rgba(0,0,0,0.1);
  text-align: left;
}

.post-card h3 {
  margin: 0 0 10px 0;
  color: #42b883;
}

.post-card p {
  margin: 0 0 10px 0;
  color: #495057;
  line-height: 1.5;
}

.post-card small {
  color: #6c757d;
  font-size: 0.9rem;
}

.form {
  background: white;
  padding: 20px;
  border-radius: 6px;
  border: 1px solid #dee2e6;
  text-align: left;
}

.form h3 {
  margin-top: 0;
  color: #495057;
}

.form-group {
  margin-bottom: 15px;
}

.form-group label {
  display: block;
  margin-bottom: 5px;
  font-weight: 500;
  color: #495057;
}

.form-group input,
.form-group textarea {
  width: 100%;
  padding: 8px 12px;
  border: 1px solid #ced4da;
  border-radius: 4px;
  font-size: 14px;
  box-sizing: border-box;
}

.form-group textarea {
  height: 80px;
  resize: vertical;
}

.form button {
  background-color: #42b883;
  color: white;
  border: none;
  padding: 10px 20px;
  border-radius: 4px;
  cursor: pointer;
  font-size: 14px;
  font-weight: 500;
}

.form button:hover:not(:disabled) {
  background-color: #369870;
}

.form button:disabled {
  background-color: #6c757d;
  cursor: not-allowed;
}

.loading {
  color: #6c757d;
  font-style: italic;
}

.error {
  color: #dc3545;
  background-color: #f8d7da;
  border: 1px solid #f5c6cb;
  padding: 10px;
  border-radius: 4px;
  margin: 10px 0;
}

@media (max-width: 768px) {
  .app-main {
    grid-template-columns: 1fr;
  }

  .users-grid {
    grid-template-columns: 1fr;
  }
}
</style>
