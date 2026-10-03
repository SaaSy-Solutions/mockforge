import type { User } from '../types';
import { authApi } from './authApi';

/** Save editable profile fields; cloud authorization remains enforced by the registry. */
export async function saveUserProfile(current: User | null, userData: User): Promise<User> {
  const patch: { username?: string; email?: string } = {};
  if (current?.username !== userData.username) patch.username = userData.username;
  if (current?.email !== userData.email) patch.email = userData.email;

  let updatedUser = userData;

  // Cloud mode persists to the registry; local mode keeps client-side state
  // since the OSS admin doesn't back users with a mutable profile store.
  if (authApi.isCloud() && Object.keys(patch).length > 0) {
    const profile = await authApi.updateProfile(patch);
    updatedUser = {
      ...userData,
      id: profile.user_id,
      username: profile.username,
      email: profile.email,
    };
  }
  return updatedUser;
}
