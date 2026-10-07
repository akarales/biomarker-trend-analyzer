FROM node:24-slim AS build
RUN corepack enable
WORKDIR /app
# pnpm-workspace.yaml carries minimumReleaseAge — without it the policy is skipped
COPY frontend/package.json frontend/pnpm-lock.yaml frontend/pnpm-workspace.yaml ./
RUN --mount=type=cache,target=/root/.local/share/pnpm/store \
    pnpm install --frozen-lockfile
COPY frontend/ ./
RUN pnpm build

# non-root nginx (uid 101), listens on 8080; same pinned image as app #1
FROM nginxinc/nginx-unprivileged:1.31.6-alpine
COPY frontend/nginx.conf /etc/nginx/conf.d/default.conf
COPY --from=build /app/dist /usr/share/nginx/html
EXPOSE 8080
