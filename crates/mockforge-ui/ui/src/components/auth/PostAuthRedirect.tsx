import { Navigate, useSearchParams } from 'react-router-dom';
import { resolvePostAuthTarget } from '@/utils/postAuthRedirect';

/** Rendered at an auth entry path once the user is signed in. */
export function PostAuthRedirect({ fallback }: { fallback: string }) {
  const [searchParams] = useSearchParams();
  return <Navigate to={resolvePostAuthTarget(searchParams, fallback)} replace />;
}
