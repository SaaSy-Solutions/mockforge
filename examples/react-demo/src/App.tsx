import React, { useState } from 'react';
import { useGetUsers, useCreateUser, useGetPosts, useCreatePost } from './generated/hooks';
import './App.css';

function App() {
  const [newUser, setNewUser] = useState({ name: '', email: '' });
  const [newPost, setNewPost] = useState({ title: '', content: '', authorId: 1 });

  // Use generated hooks
  const { data: users, loading: usersLoading, error: usersError } = useGetUsers();
  const { execute: createUser, loading: createUserLoading } = useCreateUser();
  const { data: posts, loading: postsLoading, error: postsError } = useGetPosts();
  const { execute: createPost, loading: createPostLoading } = useCreatePost();

  const handleCreateUser = async (e: React.FormEvent) => {
    e.preventDefault();
    try {
      await createUser(newUser);
      setNewUser({ name: '', email: '' });
      alert('User created successfully!');
    } catch (error) {
      alert('Failed to create user');
    }
  };

  const handleCreatePost = async (e: React.FormEvent) => {
    e.preventDefault();
    try {
      await createPost(newPost);
      setNewPost({ title: '', content: '', authorId: 1 });
      alert('Post created successfully!');
    } catch (error) {
      alert('Failed to create post');
    }
  };

  return (
    <div className="App">
      <header className="App-header">
        <h1>User Management System</h1>
        <p>Generated with MockForge React Client</p>
      </header>

      <main className="App-main">
        {/* Users Section */}
        <section className="section">
          <h2>Users</h2>

          {usersLoading && <p>Loading users...</p>}
          {usersError && <p className="error">Error: {usersError.message}</p>}

          {users && (
            <div className="users-grid">
              {users.map((user: any) => (
                <div key={user.id} className="user-card">
                  <h3>{user.name}</h3>
                  <p>{user.email}</p>
                  <small>Created: {new Date(user.createdAt).toLocaleDateString()}</small>
                </div>
              ))}
            </div>
          )}

          {/* Create User Form */}
          <form onSubmit={handleCreateUser} className="form">
            <h3>Create New User</h3>
            <div className="form-group">
              <label htmlFor="userName">Name:</label>
              <input
                id="userName"
                type="text"
                value={newUser.name}
                onChange={(e) => setNewUser({ ...newUser, name: e.target.value })}
                required
              />
            </div>
            <div className="form-group">
              <label htmlFor="userEmail">Email:</label>
              <input
                id="userEmail"
                type="email"
                value={newUser.email}
                onChange={(e) => setNewUser({ ...newUser, email: e.target.value })}
                required
              />
            </div>
            <button type="submit" disabled={createUserLoading}>
              {createUserLoading ? 'Creating...' : 'Create User'}
            </button>
          </form>
        </section>

        {/* Posts Section */}
        <section className="section">
          <h2>Posts</h2>

          {postsLoading && <p>Loading posts...</p>}
          {postsError && <p className="error">Error: {postsError.message}</p>}

          {posts && (
            <div className="posts-list">
              {posts.map((post: any) => (
                <div key={post.id} className="post-card">
                  <h3>{post.title}</h3>
                  <p>{post.content}</p>
                  <small>
                    Author ID: {post.authorId} |
                    Published: {post.published ? 'Yes' : 'No'} |
                    Created: {new Date(post.createdAt).toLocaleDateString()}
                  </small>
                </div>
              ))}
            </div>
          )}

          {/* Create Post Form */}
          <form onSubmit={handleCreatePost} className="form">
            <h3>Create New Post</h3>
            <div className="form-group">
              <label htmlFor="postTitle">Title:</label>
              <input
                id="postTitle"
                type="text"
                value={newPost.title}
                onChange={(e) => setNewPost({ ...newPost, title: e.target.value })}
                required
              />
            </div>
            <div className="form-group">
              <label htmlFor="postContent">Content:</label>
              <textarea
                id="postContent"
                value={newPost.content}
                onChange={(e) => setNewPost({ ...newPost, content: e.target.value })}
                required
              />
            </div>
            <div className="form-group">
              <label htmlFor="postAuthorId">Author ID:</label>
              <input
                id="postAuthorId"
                type="number"
                value={newPost.authorId}
                onChange={(e) => setNewPost({ ...newPost, authorId: parseInt(e.target.value) })}
                required
              />
            </div>
            <button type="submit" disabled={createPostLoading}>
              {createPostLoading ? 'Creating...' : 'Create Post'}
            </button>
          </form>
        </section>
      </main>
    </div>
  );
}

export default App;
