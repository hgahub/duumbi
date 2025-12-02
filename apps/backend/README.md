# Duumbi Backend

Modular Monolith Backend API for the Duumbi Platform, built with FastAPI and Python.

## Overview

This is the Python backend service for Duumbi, providing REST APIs for:

- **AVM (Automated Valuation Model)**: Property valuation services
- **Scraper**: Data collection and web scraping functionality
- **Tagger**: Content tagging and classification

## Technology Stack

- **Framework**: FastAPI
- **Server**: Uvicorn
- **ML/Data**: scikit-learn
- **Validation**: Pydantic
- **HTTP Client**: httpx
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
- `/api/tagger/*` - Content tagging endpoints

See `/docs` for complete API documentation.

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
