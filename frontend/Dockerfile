# Multi-stage Dockerfile for production-ready Vite/React frontend

# Stage 1: Build stage
FROM node:20-alpine AS builder

WORKDIR /app

# Enable pnpm via Corepack instead of installing it globally
RUN corepack enable

# Copy package files first for better caching
COPY package.json pnpm-lock.yaml ./

# Install dependencies
RUN pnpm install --frozen-lockfile

# Copy source files
COPY . .

# Build the application
RUN pnpm build

# Stage 2: Production stage
FROM nginxinc/nginx-unprivileged:alpine

# Copy built assets from builder
COPY --from=builder /app/dist /usr/share/nginx/html

# Copy custom nginx configuration
COPY nginx.conf /etc/nginx/conf.d/default.conf

# Expose port 8000
EXPOSE 8080

# Start nginx
CMD ["nginx", "-g", "daemon off;"]
