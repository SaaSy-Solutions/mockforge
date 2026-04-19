export const onRequest: PagesFunction = async ({ request }) => {
  const url = new URL(request.url);
  const upstream = new URL(`https://api.mockforge.dev${url.pathname}${url.search}`);
  const init: RequestInit = {
    method: request.method,
    headers: request.headers,
    body: ["GET", "HEAD"].includes(request.method) ? undefined : request.body,
    redirect: "manual",
  };
  return fetch(upstream.toString(), init);
};
