"""Tests for main application endpoints."""
from fastapi.testclient import TestClient
from src.main import app

client = TestClient(app)


def test_root_endpoint():
    """Test the root endpoint returns welcome message."""
    response = client.get("/")
    assert response.status_code == 200
    assert response.json() == {"message": "Welcome to Duumbi Backend API"}


def test_health_check():
    """Test the health check endpoint."""
    response = client.get("/health")
    assert response.status_code == 200
    assert response.json() == {"status": "healthy"}


def test_openapi_docs():
    """Test that OpenAPI documentation is available."""
    response = client.get("/docs")
    assert response.status_code == 200
