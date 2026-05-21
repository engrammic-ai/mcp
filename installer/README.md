# Engrammic MCP Installer

Scripts and landing page for `get.engrammic.ai`.

## Files

- `install.sh` - macOS/Linux installer
- `install.ps1` - Windows PowerShell installer  
- `index.html` - Landing page with copy-paste commands
- `Dockerfile` - nginx container for Cloud Run
- `nginx.conf` - serves scripts as text/plain

## Deployment (Cloud Run)

```bash
cd installer

# Build and push
gcloud builds submit --tag gcr.io/engrammic/get-engrammic

# Deploy
gcloud run deploy get-engrammic \
  --image gcr.io/engrammic/get-engrammic \
  --platform managed \
  --region us-central1 \
  --allow-unauthenticated

# Map custom domain
gcloud run domain-mappings create \
  --service get-engrammic \
  --domain get.engrammic.ai \
  --region us-central1
```

Then add DNS A/AAAA records per gcloud output.

## PyPI

The `install` command is built into `engrammic-mcp`:

```bash
uvx engrammic-mcp install
```

## Usage

```bash
# macOS/Linux
curl -fsSL https://get.engrammic.ai/install.sh | sh

# Windows PowerShell
irm https://get.engrammic.ai/install.ps1 | iex

# Via PyPI
uvx engrammic-mcp install
```
