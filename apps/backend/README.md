# Duumbi Backend

Modular Monolith Backend API for the Duumbi Platform, built with FastAPI and Python.

## Overview

This is the Python backend service for Duumbi, providing REST APIs for:

- **AVM (Automated Valuation Model)**: Property valuation services
- **Scraper**: Data collection and web scraping functionality
- **Tagger**: AI-powered image analysis for property photos (✅ **Implemented**)

## Technology Stack

- **Framework**: FastAPI
- **Server**: Uvicorn
- **ML/Data**: scikit-learn
- **Validation**: Pydantic v2
- **HTTP Client**: httpx
- **Image Processing**: Pillow, NumPy
- **AI Services**: Azure AI Vision
- **Package Manager**: uv
- **Python**: >=3.9

## Getting Started

### Prerequisites

- Python 3.9 or higher
- uv package manager

### Installation

Install dependencies:

```bash
nx run backend:install
```

Or directly with uv:

```bash
cd apps/backend
uv sync
```

### Development

Start the development server with hot reload:

```bash
nx run backend:serve
```

The API will be available at `http://localhost:8000`

- API docs: `http://localhost:8000/docs`
- Alternative docs: `http://localhost:8000/redoc`
- Health check: `http://localhost:8000/health`

## Project Structure

```
apps/backend/
├── src/
│   ├── main.py           # FastAPI application entry point
│   ├── avm/              # AVM module
│   │   └── router.py
│   ├── scraper/          # Scraper module
│   │   └── router.py
│   └── tagger/           # Tagger module
│       └── router.py
├── pyproject.toml        # Python dependencies and config
├── project.json          # Nx project configuration
└── Dockerfile            # Container configuration
```

## Available Commands

All commands can be run via Nx from the workspace root:

- `nx run backend:serve` - Start development server
- `nx run backend:build` - Build distribution package
- `nx run backend:test` - Run tests with pytest
- `nx run backend:lint` - Lint code with ruff

## API Endpoints

### Core Endpoints

- `GET /` - Welcome message
- `GET /health` - Health check endpoint

### Module Endpoints

- `/api/avm/*` - AVM (Automated Valuation Model) endpoints
- `/api/scraper/*` - Web scraping endpoints
- `/api/tagger/*` - **Image analysis endpoints** ✅
  - `POST /api/tagger/analyze` - Analyze image from URL
  - `POST /api/tagger/analyze/upload` - Analyze uploaded image
  - `POST /api/tagger/analyze/batch` - Batch analysis (max 20 images)
  - `GET /api/tagger/health` - Health check

See `/docs` for complete API documentation.

## Modules

### Tagger Module ✅ (Implemented)

AI-powered image analysis for property listings.

**Features:**
- Quality assessment (brightness, sharpness, composition)
- Room type detection (13 types)
- Feature detection (10 features)
- Azure AI Vision integration
- Smart recommendations

**Documentation:**
- [Module README](src/tagger/README.md)
- [API Examples](docs/TAGGER_API_EXAMPLES.md)
- [Azure Setup Guide](docs/AZURE_VISION_SETUP.md)
- [Test Documentation](tests/tagger/README.md)

**Quick Start:**
```bash
# Set Azure credentials in .env
TAGGER_AZURE_VISION_ENDPOINT=https://your-resource.cognitiveservices.azure.com/
TAGGER_AZURE_VISION_KEY=your-api-key

# Test the API
curl -X POST http://localhost:8000/api/tagger/analyze \
  -H "Content-Type: application/json" \
  -d '{"image_url": "https://example.com/property.jpg"}'
```

## Testing

Run tests:

```bash
nx run backend:test
```

## Linting

Check code quality:

```bash
nx run backend:lint
```

## Building

Create distribution package:

```bash
nx run backend:build
```

## Docker

Build and run with Docker:

```bash
docker build -t duumbi-backend apps/backend
docker run -p 8000:8000 duumbi-backend
```

## Contributing

Follow the project guidelines defined in `AGENTS.md` and the monorepo conventions.
