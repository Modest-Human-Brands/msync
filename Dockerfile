# Use Bun official image
FROM oven/bun:latest

# Set working directory
WORKDIR /app

# Copy everything
COPY . .

# Install dependencies
RUN bun install --frozen-lockfile

# Run the same steps as CI
CMD bash -c "\
  echo '⚙️ Build' && bun run build && \
  echo '🧹 Lint' && bun run lint || true && \
  echo '🧼 Format' && bun run format \
"