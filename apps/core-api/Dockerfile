FROM node:20-alpine AS builder

WORKDIR /app

# Copy package files
COPY package.json package-lock.json ./

# Install dependencies
RUN npm ci

# Copy source
COPY . .

# Build the app
# This will output to apps/core-api/dist based on project.json configuration
RUN npx nx run core-api:build --configuration=production

# Generate package.json for the app
RUN npx nx run core-api:prune-lockfile

# Production image
FROM node:20-alpine

WORKDIR /app

# Copy built artifacts
COPY --from=builder /app/apps/core-api/dist ./

# Install production dependencies
RUN npm install --omit=dev

EXPOSE 3000

CMD ["node", "main.js"]
