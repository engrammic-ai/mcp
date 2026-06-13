# Engrammic Self-Hosted

Lite deployment (~3GB RAM total) with Memgraph, Qdrant, Redis, and PostgreSQL.

## Quick Start

```bash
# Start all services
docker compose up -d

# Check status
docker compose ps

# View logs
docker compose logs -f app
```

## Skills Catalog

Browse available MCP skills at [engrammic.ai/skills](https://engrammic.ai/skills). Skills are installed to `~/.agents/skills/` and work across Claude Code, Codex, Gemini, and other agent harnesses.

## Configuration

Copy `.env.example` to `.env` and set:

- `ENGRAMMIC_LICENSE_KEY` - Your license key (required)
- `POSTGRES_PASSWORD` - Database password (default: engrammic)

## Database Migrations

Migrations run automatically on container startup. To disable:

```yaml
# In docker-compose.yml, under app.environment:
- RUN_MIGRATIONS=false
```

Then run migrations manually when needed:

```bash
docker exec engrammic-app python -m alembic upgrade head
```

Check migration status:

```bash
docker exec engrammic-app python -m alembic current
```

## Scaling

The default configuration is sized for development/small teams. For larger deployments:

| Service   | Default | Production    |
|-----------|---------|---------------|
| app       | 512M    | 1-2G          |
| memgraph  | 1G      | 4-8G          |
| qdrant    | 512M    | 2-4G          |
| redis     | 128M    | 256-512M      |
| postgres  | 256M    | 512M-1G       |

Adjust `deploy.resources.limits.memory` in docker-compose.yml.

## Data Persistence

Data is stored in Docker volumes:

- `memgraph-data` - Graph database
- `qdrant-data` - Vector storage
- `redis-data` - Cache and queues
- `postgres-data` - Relational data

To backup:

```bash
docker compose stop
# Copy volumes or use docker cp
docker compose start
```

## Health Checks

All services have health checks. The app waits for dependencies before starting.

Check health:

```bash
curl http://localhost:8000/health
```

## Podman Support

Engrammic supports Podman via the Docker-compatible socket:

```bash
# Start the Podman socket (run once)
podman system service --time=0 unix:///tmp/podman.sock &
export DOCKER_HOST=unix:///tmp/podman.sock

# Then run the installer with --podman flag
engrammic selfhost --podman
```

The `--podman` flag:
- Skips Docker daemon check
- Uses Podman GPU syntax (CDI) in compose files
- Adds `:Z` suffix to volumes for SELinux compatibility

## Offline / Airgapped Setup

For environments without internet access, pull images and models before deployment:

```bash
# Pre-pull all required images
docker compose pull

# For local embedding models, pre-download via Ollama or HuggingFace
# and set EMBEDDING_MODEL_PATH in .env to the local path
```

Compose files are generated in the current directory when you run `engrammic selfhost`. You can inspect and modify them before starting services, or commit them to version control for reproducible airgapped deployments.

## Troubleshooting

**App won't start**: Check that all dependencies are healthy first:
```bash
docker compose ps
```

**Migration failed**: Check logs and run manually:
```bash
docker compose logs app
docker exec engrammic-app python -m alembic upgrade head
```

**Out of memory**: Increase container limits in docker-compose.yml.
