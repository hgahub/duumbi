"""Tests for FastAPI router endpoints."""
import pytest
from io import BytesIO
from PIL import Image
from fastapi.testclient import TestClient
from unittest.mock import patch, AsyncMock

from src.main import app
from src.tagger.models import (
    ImageAnalysisResult,
    ImageQualityScore,
    RoomType,
    ImageFeature,
)


client = TestClient(app)


class TestTaggerEndpoints:
    """Tests for Tagger API endpoints."""
    
    def test_health_check(self):
        """Test health check endpoint."""
        response = client.get('/api/tagger/health')
        
        assert response.status_code == 200
        data = response.json()
        assert data['status'] == 'healthy'
        assert data['service'] == 'tagger'
        assert 'azure_configured' in data
    
    def test_analyze_url_missing_url(self):
        """Test analyze endpoint with missing URL."""
        response = client.post('/api/tagger/analyze', json={})
        
        assert response.status_code == 400
        assert 'image_url is required' in response.json()['detail']
    
    def test_analyze_url_invalid_format(self):
        """Test analyze endpoint with invalid URL format."""
        response = client.post(
            '/api/tagger/analyze',
            json={'image_url': 'https://example.com/image.gif'}
        )
        
        assert response.status_code == 422  # Pydantic validation error
    
    def test_analyze_url_valid_format(self):
        """Test analyze endpoint with valid URL format (will fail on Azure)."""
        response = client.post(
            '/api/tagger/analyze',
            json={'image_url': 'https://example.com/image.jpg'}
        )
        
        # Will fail with Azure error (placeholder credentials)
        assert response.status_code == 500
        assert 'Azure Vision API error' in response.json()['detail']
    
    def test_upload_invalid_file_type(self):
        """Test upload endpoint with invalid file type."""
        response = client.post(
            '/api/tagger/analyze/upload',
            files={'file': ('test.txt', BytesIO(b'not an image'), 'text/plain')}
        )
        
        assert response.status_code == 400
        assert 'Invalid file type' in response.json()['detail']
    
    def test_upload_valid_image_with_mock(self):
        """Test upload endpoint with valid image and mocked service."""
        # Create test image
        img = Image.new('RGB', (1024, 768), color=(150, 150, 150))
        buffer = BytesIO()
        img.save(buffer, format='JPEG')
        buffer.seek(0)
        
        # Create mock result
        mock_result = ImageAnalysisResult(
            quality=ImageQualityScore(
                overall_score=7.5,
                brightness_score=8.0,
                sharpness_score=7.0,
                composition_score=7.5,
                issues=[]
            ),
            room_type=RoomType.LIVING_ROOM,
            room_confidence=0.95,
            features=[ImageFeature.MODERN],
            tags=['living room', 'modern'],
            caption='A modern living room',
            recommendations=['Image quality is good'],
            is_suitable=True,
            processing_time_ms=150
        )
        
        # Patch the service
        from src.tagger.service import ImageAnalysisService
        with patch.object(
            ImageAnalysisService,
            'analyze_image_from_file',
            new=AsyncMock(return_value=mock_result)
        ):
            response = client.post(
                '/api/tagger/analyze/upload',
                files={'file': ('test.jpg', buffer, 'image/jpeg')}
            )
            
            assert response.status_code == 200
            data = response.json()
            assert data['room_type'] == 'living_room'
            assert data['quality']['overall_score'] == 7.5
            assert 'living room' in data['tags']
            assert data['is_suitable'] is True
    
    def test_batch_too_many_images(self):
        """Test batch endpoint with too many images."""
        response = client.post(
            '/api/tagger/analyze/batch',
            json={'image_urls': ['https://example.com/img.jpg'] * 25}
        )
        
        assert response.status_code == 422  # Validation error
    
    def test_batch_valid_request(self):
        """Test batch endpoint with valid request."""
        response = client.post(
            '/api/tagger/analyze/batch',
            json={'image_urls': [
                'https://example.com/img1.jpg',
                'https://example.com/img2.png'
            ]}
        )
        
        assert response.status_code == 200
        data = response.json()
        assert data['total_processed'] == 2
        assert data['total_failed'] == 2  # Will fail with placeholder credentials
        assert 'processing_time_ms' in data
    
    def test_batch_empty_list(self):
        """Test batch endpoint with empty image list."""
        response = client.post(
            '/api/tagger/analyze/batch',
            json={'image_urls': []}
        )
        
        assert response.status_code == 422  # Validation error (min_length=1)


class TestOpenAPIDocumentation:
    """Tests for OpenAPI documentation."""
    
    def test_openapi_schema_generation(self):
        """Test OpenAPI schema is generated correctly."""
        response = client.get('/openapi.json')
        
        assert response.status_code == 200
        openapi = response.json()
        assert openapi['info']['title'] == 'Duumbi Backend API'
        assert '/api/tagger/analyze' in openapi['paths']
        assert '/api/tagger/analyze/upload' in openapi['paths']
        assert '/api/tagger/analyze/batch' in openapi['paths']
        assert '/api/tagger/health' in openapi['paths']
    
    def test_docs_endpoint(self):
        """Test Swagger UI docs endpoint."""
        response = client.get('/docs')
        
        assert response.status_code == 200
        assert 'swagger' in response.text.lower()
    
    def test_redoc_endpoint(self):
        """Test ReDoc endpoint."""
        response = client.get('/redoc')
        
        assert response.status_code == 200
        assert 'redoc' in response.text.lower()

