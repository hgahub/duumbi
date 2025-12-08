"""Integration tests for Tagger module end-to-end workflows."""
import pytest
from io import BytesIO
from PIL import Image, ImageDraw
from fastapi.testclient import TestClient
from unittest.mock import patch, AsyncMock

from src.main import app
from src.tagger.models import ImageAnalysisResult, ImageQualityScore, RoomType, ImageFeature


client = TestClient(app)


class TestEndToEndWorkflows:
    """End-to-end integration tests for complete workflows."""
    
    @pytest.mark.asyncio
    async def test_complete_image_upload_workflow(self):
        """Test complete workflow: upload → validate → analyze → return result."""
        # Step 1: Create a realistic test image
        img = Image.new('RGB', (1920, 1080), color=(180, 180, 180))
        draw = ImageDraw.Draw(img)
        
        # Add some details to make it look more realistic
        for i in range(0, 1920, 100):
            draw.line([(i, 0), (i, 1080)], fill=(150, 150, 150), width=2)
        for i in range(0, 1080, 100):
            draw.line([(0, i), (1920, i)], fill=(150, 150, 150), width=2)
        
        buffer = BytesIO()
        img.save(buffer, format='JPEG', quality=95)
        buffer.seek(0)
        
        # Step 2: Mock Azure response
        mock_azure_result = {
            'tags': [
                {'name': 'living room', 'confidence': 0.95},
                {'name': 'modern', 'confidence': 0.90},
                {'name': 'bright', 'confidence': 0.88},
                {'name': 'furniture', 'confidence': 0.85},
                {'name': 'window', 'confidence': 0.82}
            ],
            'objects': [
                {
                    'name': 'sofa',
                    'confidence': 0.92,
                    'bounding_box': {'x': 100, 'y': 200, 'w': 400, 'h': 300}
                }
            ],
            'caption': {
                'text': 'A modern living room with bright natural lighting',
                'confidence': 0.89
            }
        }
        
        # Step 3: Execute full workflow with mock
        from src.tagger.service import ImageAnalysisService
        with patch.object(
            ImageAnalysisService,
            'analyze_image_from_file',
            new=AsyncMock()
        ) as mock_analyze:
            # Create expected result
            mock_result = ImageAnalysisResult(
                quality=ImageQualityScore(
                    overall_score=8.5,
                    brightness_score=9.0,
                    sharpness_score=8.0,
                    composition_score=8.5,
                    issues=[]
                ),
                room_type=RoomType.LIVING_ROOM,
                room_confidence=0.95,
                features=[
                    ImageFeature.MODERN,
                    ImageFeature.BRIGHT,
                    ImageFeature.FURNISHED,
                    ImageFeature.NATURAL_LIGHT
                ],
                tags=['living room', 'modern', 'bright', 'furniture', 'window'],
                caption='A modern living room with bright natural lighting',
                recommendations=['Image quality is good'],
                is_suitable=True,
                processing_time_ms=250
            )
            mock_analyze.return_value = mock_result
            
            # Step 4: Make API request
            response = client.post(
                '/api/tagger/analyze/upload',
                files={'file': ('test.jpg', buffer, 'image/jpeg')}
            )
            
            # Step 5: Verify response
            assert response.status_code == 200
            data = response.json()
            
            # Verify quality assessment
            assert data['quality']['overall_score'] == 8.5
            assert data['quality']['brightness_score'] == 9.0
            assert len(data['quality']['issues']) == 0
            
            # Verify room detection
            assert data['room_type'] == 'living_room'
            assert data['room_confidence'] == 0.95
            
            # Verify features
            assert 'modern' in data['features']
            assert 'bright' in data['features']
            assert 'furnished' in data['features']
            assert 'natural_light' in data['features']
            
            # Verify tags and caption
            assert 'living room' in data['tags']
            assert data['caption'] == 'A modern living room with bright natural lighting'
            
            # Verify suitability
            assert data['is_suitable'] is True
            assert 'Image quality is good' in data['recommendations']
    
    def test_poor_quality_image_workflow(self):
        """Test workflow with poor quality image (dark, blurry)."""
        # Create a dark, low-quality image
        img = Image.new('RGB', (800, 600), color=(30, 30, 30))
        buffer = BytesIO()
        img.save(buffer, format='JPEG', quality=50)
        buffer.seek(0)
        
        # Mock Azure response
        mock_result = ImageAnalysisResult(
            quality=ImageQualityScore(
                overall_score=3.5,
                brightness_score=2.0,
                sharpness_score=4.0,
                composition_score=5.0,
                issues=['too_dark', 'blurry', 'low_resolution']
            ),
            room_type=None,
            room_confidence=0.0,
            features=[],
            tags=['dark', 'indoor'],
            caption='A dark indoor space',
            recommendations=[
                'Increase lighting or use flash',
                'Use a tripod or increase shutter speed',
                'Use a higher resolution camera'
            ],
            is_suitable=False,
            processing_time_ms=180
        )
        
        from src.tagger.service import ImageAnalysisService
        with patch.object(
            ImageAnalysisService,
            'analyze_image_from_file',
            new=AsyncMock(return_value=mock_result)
        ):
            response = client.post(
                '/api/tagger/analyze/upload',
                files={'file': ('dark.jpg', buffer, 'image/jpeg')}
            )
            
            assert response.status_code == 200
            data = response.json()
            
            # Verify poor quality detected
            assert data['quality']['overall_score'] < 5.0
            assert 'too_dark' in data['quality']['issues']
            assert data['is_suitable'] is False
            
            # Verify recommendations provided
            assert len(data['recommendations']) > 0
            assert any('lighting' in rec.lower() for rec in data['recommendations'])

    def test_batch_processing_workflow(self):
        """Test batch processing of multiple images."""
        # Mock results for 3 images
        mock_results = [
            ImageAnalysisResult(
                quality=ImageQualityScore(
                    overall_score=8.0, brightness_score=8.5,
                    sharpness_score=7.5, composition_score=8.0, issues=[]
                ),
                room_type=RoomType.LIVING_ROOM, room_confidence=0.95,
                features=[ImageFeature.MODERN], tags=['living room'],
                caption='Living room', recommendations=['Image quality is good'],
                is_suitable=True, processing_time_ms=150
            ),
            ImageAnalysisResult(
                quality=ImageQualityScore(
                    overall_score=7.5, brightness_score=8.0,
                    sharpness_score=7.0, composition_score=7.5, issues=[]
                ),
                room_type=RoomType.KITCHEN, room_confidence=0.92,
                features=[ImageFeature.MODERN], tags=['kitchen'],
                caption='Kitchen', recommendations=['Image quality is good'],
                is_suitable=True, processing_time_ms=140
            ),
            ImageAnalysisResult(
                quality=ImageQualityScore(
                    overall_score=3.0, brightness_score=2.0,
                    sharpness_score=3.5, composition_score=4.0,
                    issues=['too_dark']
                ),
                room_type=None, room_confidence=0.0,
                features=[], tags=['dark'], caption='Dark room',
                recommendations=['Increase lighting or use flash'],
                is_suitable=False, processing_time_ms=130
            )
        ]

        from src.tagger.service import ImageAnalysisService

        # Mock the analyze_image_from_url to return results sequentially
        call_count = [0]

        async def mock_analyze(url):
            result = mock_results[call_count[0]]
            call_count[0] += 1
            return result

        with patch.object(
            ImageAnalysisService,
            'analyze_image_from_url',
            new=AsyncMock(side_effect=mock_analyze)
        ):
            response = client.post(
                '/api/tagger/analyze/batch',
                json={'image_urls': [
                    'https://example.com/living-room.jpg',
                    'https://example.com/kitchen.jpg',
                    'https://example.com/dark-room.jpg'
                ]}
            )

            assert response.status_code == 200
            data = response.json()

            # Verify batch results
            assert data['total_processed'] == 3
            assert data['total_failed'] == 0
            assert len(data['results']) == 3

            # Verify individual results
            assert data['results'][0]['room_type'] == 'living_room'
            assert data['results'][1]['room_type'] == 'kitchen'
            assert data['results'][2]['is_suitable'] is False

    def test_validation_error_workflow(self):
        """Test workflow with validation errors."""
        # Test 1: Image too small
        small_img = Image.new('RGB', (400, 300), color=(150, 150, 150))
        buffer = BytesIO()
        small_img.save(buffer, format='JPEG')
        buffer.seek(0)

        response = client.post(
            '/api/tagger/analyze/upload',
            files={'file': ('small.jpg', buffer, 'image/jpeg')}
        )

        assert response.status_code == 400
        assert 'width' in response.json()['detail'].lower() or 'height' in response.json()['detail'].lower()

        # Test 2: Invalid file type
        response = client.post(
            '/api/tagger/analyze/upload',
            files={'file': ('test.txt', BytesIO(b'not an image'), 'text/plain')}
        )

        assert response.status_code == 400
        assert 'invalid file type' in response.json()['detail'].lower()

    def test_multiple_room_types_workflow(self):
        """Test detection of different room types."""
        room_types = [
            (RoomType.BEDROOM, 'bedroom', 0.93),
            (RoomType.BATHROOM, 'bathroom', 0.91),
            (RoomType.KITCHEN, 'kitchen', 0.94),
            (RoomType.EXTERIOR, 'exterior', 0.88)
        ]

        from src.tagger.service import ImageAnalysisService

        for room_type, tag_name, confidence in room_types:
            mock_result = ImageAnalysisResult(
                quality=ImageQualityScore(
                    overall_score=7.5, brightness_score=8.0,
                    sharpness_score=7.0, composition_score=7.5, issues=[]
                ),
                room_type=room_type,
                room_confidence=confidence,
                features=[],
                tags=[tag_name],
                caption=f'A {tag_name}',
                recommendations=['Image quality is good'],
                is_suitable=True,
                processing_time_ms=150
            )

            with patch.object(
                ImageAnalysisService,
                'analyze_image_from_url',
                new=AsyncMock(return_value=mock_result)
            ):
                response = client.post(
                    '/api/tagger/analyze',
                    json={'image_url': f'https://example.com/{tag_name}.jpg'}
                )

                assert response.status_code == 200
                data = response.json()
                assert data['room_type'] == room_type.value
                assert data['room_confidence'] == confidence


class TestErrorHandling:
    """Integration tests for error handling scenarios."""

    def test_azure_api_error_handling(self):
        """Test handling of Azure API errors."""
        from src.tagger.exceptions import AzureVisionError
        from src.tagger.service import ImageAnalysisService

        with patch.object(
            ImageAnalysisService,
            'analyze_image_from_url',
            new=AsyncMock(side_effect=AzureVisionError('API rate limit exceeded', status_code=429))
        ):
            response = client.post(
                '/api/tagger/analyze',
                json={'image_url': 'https://example.com/test.jpg'}
            )

            assert response.status_code == 500
            assert 'Azure Vision API error' in response.json()['detail']

    def test_service_health_check(self):
        """Test service health check endpoint."""
        response = client.get('/api/tagger/health')

        assert response.status_code == 200
        data = response.json()
        assert data['status'] == 'healthy'
        assert data['service'] == 'tagger'
        assert isinstance(data['azure_configured'], bool)

