"""Tests for image analysis service."""
import pytest
from io import BytesIO
from PIL import Image
from unittest.mock import patch, AsyncMock

from src.tagger.service import ImageAnalysisService
from src.tagger.models import RoomType, ImageFeature, ImageQualityIssue


class TestImageAnalysisService:
    """Tests for ImageAnalysisService class."""
    
    def test_initialization(self):
        """Test service initialization."""
        service = ImageAnalysisService()
        
        assert service.azure_service is not None
        assert service.settings is not None
    
    def test_detect_room_type_living_room(self):
        """Test room type detection for living room."""
        service = ImageAnalysisService()
        
        tags = [
            {'name': 'living room', 'confidence': 0.95},
            {'name': 'furniture', 'confidence': 0.88}
        ]
        
        room_type, confidence = service._detect_room_type(tags)
        
        assert room_type == RoomType.LIVING_ROOM
        assert confidence == 0.95
    
    def test_detect_room_type_kitchen(self):
        """Test room type detection for kitchen."""
        service = ImageAnalysisService()
        
        tags = [
            {'name': 'kitchen', 'confidence': 0.92},
            {'name': 'appliance', 'confidence': 0.85}
        ]
        
        room_type, confidence = service._detect_room_type(tags)
        
        assert room_type == RoomType.KITCHEN
        assert confidence == 0.92
    
    def test_detect_room_type_low_confidence(self):
        """Test room type detection with low confidence."""
        service = ImageAnalysisService()
        
        tags = [
            {'name': 'living room', 'confidence': 0.3},  # Below threshold
        ]
        
        room_type, confidence = service._detect_room_type(tags)
        
        assert room_type is None
        assert confidence == 0.0
    
    def test_detect_room_type_no_match(self):
        """Test room type detection with no matching tags."""
        service = ImageAnalysisService()
        
        tags = [
            {'name': 'random', 'confidence': 0.95},
            {'name': 'object', 'confidence': 0.88}
        ]
        
        room_type, confidence = service._detect_room_type(tags)
        
        assert room_type is None
        assert confidence == 0.0
    
    def test_detect_features(self):
        """Test feature detection from tags."""
        service = ImageAnalysisService()
        
        tags = [
            {'name': 'modern', 'confidence': 0.90},
            {'name': 'bright', 'confidence': 0.85},
            {'name': 'window', 'confidence': 0.80},
            {'name': 'furniture', 'confidence': 0.75}
        ]
        
        features = service._detect_features(tags)
        
        assert ImageFeature.MODERN in features
        assert ImageFeature.BRIGHT in features
        assert ImageFeature.NATURAL_LIGHT in features
        assert ImageFeature.FURNISHED in features
    
    def test_detect_features_low_confidence(self):
        """Test feature detection filters low confidence tags."""
        service = ImageAnalysisService()
        
        tags = [
            {'name': 'modern', 'confidence': 0.5},  # Below threshold (0.7)
        ]
        
        features = service._detect_features(tags)
        
        assert ImageFeature.MODERN not in features
    
    def test_generate_recommendations_dark_image(self):
        """Test recommendations for dark image."""
        service = ImageAnalysisService()
        
        quality_metrics = {
            'issues': [ImageQualityIssue.TOO_DARK]
        }
        
        recommendations = service._generate_recommendations(
            quality_metrics,
            ['living room'],
            RoomType.LIVING_ROOM
        )
        
        assert 'Increase lighting or use flash' in recommendations
    
    def test_generate_recommendations_blurry_image(self):
        """Test recommendations for blurry image."""
        service = ImageAnalysisService()
        
        quality_metrics = {
            'issues': [ImageQualityIssue.BLURRY]
        }
        
        recommendations = service._generate_recommendations(
            quality_metrics,
            [],
            None
        )
        
        assert 'Use a tripod or increase shutter speed' in recommendations
    
    def test_generate_recommendations_good_quality(self):
        """Test recommendations for good quality image."""
        service = ImageAnalysisService()
        
        quality_metrics = {'issues': []}
        
        recommendations = service._generate_recommendations(
            quality_metrics,
            [],
            None
        )
        
        assert 'Image quality is good' in recommendations
    
    @pytest.mark.asyncio
    async def test_analyze_image_from_file(self):
        """Test full image analysis from file with mocked Azure."""
        service = ImageAnalysisService()
        
        # Create test image
        img = Image.new('RGB', (1024, 768), color=(150, 150, 150))
        buffer = BytesIO()
        img.save(buffer, format='JPEG')
        buffer.seek(0)
        
        # Mock Azure response
        mock_azure_result = {
            'tags': [
                {'name': 'living room', 'confidence': 0.95},
                {'name': 'modern', 'confidence': 0.90}
            ],
            'objects': [],
            'caption': {'text': 'A modern living room', 'confidence': 0.88}
        }
        
        with patch.object(
            service.azure_service,
            'analyze_image_data',
            new=AsyncMock(return_value=mock_azure_result)
        ):
            result = await service.analyze_image_from_file(buffer)
            
            assert result.room_type == RoomType.LIVING_ROOM
            assert result.room_confidence == 0.95
            assert ImageFeature.MODERN in result.features
            assert 'living room' in result.tags
            assert result.caption == 'A modern living room'
            assert result.processing_time_ms > 0

