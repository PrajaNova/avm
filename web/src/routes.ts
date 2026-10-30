import { type RouteConfig, index, route } from '@react-router/dev/routes';

export default [
  index('routes/home.tsx'),
  route('marketplace', 'routes/marketplace.tsx'),
  route('marketplace/:name', 'routes/plugin.tsx'),
  route('docs/*', 'routes/docs.tsx'),
  route('guides/:slug', 'routes/guide.tsx'),
  route('changelog', 'routes/changelog.tsx'),
  route('*', 'routes/not-found.tsx'),
] satisfies RouteConfig;
