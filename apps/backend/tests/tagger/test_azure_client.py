"""Tests for Azure AI Vision client."""
import pytest
from unittest.mock import Mock, patch

from src.tagger.azure_client import AzureVisionService
from src.tagger.exceptions import AzureVisionError


class TestAzureVisionService:
    """Tests for AzureVisionService class."""
    
    def test_initialization(self):
        """Test service initialization."""
        service = AzureVisionService()
        
        assert service.client is not None
        assert service.timeout == 30
        assert service.max_retries == 3
        assert service.retry_delay == 1.0
    
    def test_parse_result_with_tags(self):
        """Test parsing Azure result with tags."""
        service = AzureVisionService()
        
        # Create mock result
        mock_tag1 = Mock()
        mock_tag1.name = 'living room'
        mock_tag1.confidence = 0.95
        
        mock_tag2 = Mock()
        mock_tag2.name = 'furniture'
        mock_tag2.confidence = 0.88
        
        mock_tags = Mock()
        mock_tags.list = [mock_tag1, mock_tag2]
        
        mock_result = Mock()
        mock_result.tags = mock_tags
        mock_result.objects = None
        mock_result.caption = None
        
        parsed = service._parse_result(mock_result)
        
        assert len(parsed['tags']) == 2
        assert parsed['tags'][0]['name'] == 'living room'
        assert parsed['tags'][0]['confidence'] == 0.95
        assert parsed['tags'][1]['name'] == 'furniture'
        assert parsed['tags'][1]['confidence'] == 0.88
    
    def test_parse_result_with_objects(self):
        """Test parsing Azure result with objects."""
        service = AzureVisionService()
        
        # Create mock object
        mock_obj_tag = Mock()
        mock_obj_tag.name = 'sofa'
        mock_obj_tag.confidence = 0.92
        
        mock_bbox = Mock()
        mock_bbox.x = 100
        mock_bbox.y = 150
        mock_bbox.w = 300
        mock_bbox.h = 200
        
        mock_obj = Mock()
        mock_obj.tags = [mock_obj_tag]
        mock_obj.bounding_box = mock_bbox
        
        mock_objects = Mock()
        mock_objects.list = [mock_obj]
        
        mock_result = Mock()
        mock_result.tags = None
        mock_result.objects = mock_objects
        mock_result.caption = None
        
        parsed = service._parse_result(mock_result)
        
        assert len(parsed['objects']) == 1
        assert parsed['objects'][0]['name'] == 'sofa'
        assert parsed['objects'][0]['confidence'] == 0.92
        assert parsed['objects'][0]['bounding_box']['x'] == 100
        assert parsed['objects'][0]['bounding_box']['y'] == 150
    
    def test_parse_result_with_caption(self):
        """Test parsing Azure result with caption."""
        service = AzureVisionService()
        
        mock_caption = Mock()
        mock_caption.text = 'A modern living room'
        mock_caption.confidence = 0.89
        
        mock_result = Mock()
        mock_result.tags = None
        mock_result.objects = None
        mock_result.caption = mock_caption
        
        parsed = service._parse_result(mock_result)
        
        assert parsed['caption']['text'] == 'A modern living room'
        assert parsed['caption']['confidence'] == 0.89
    
    @pytest.mark.asyncio
    async def test_analyze_image_url_error_handling(self):
        """Test error handling for analyze_image_url."""
        service = AzureVisionService()
        
        # This will fail with placeholder credentials
        with pytest.raises(AzureVisionError):
            await service.analyze_image_url('https://example.com/test.jpg')
    
    @pytest.mark.asyncio
    async def test_analyze_image_data_error_handling(self):
        """Test error handling for analyze_image_data."""
        service = AzureVisionService()
        
        # This will fail with placeholder credentials
        with pytest.raises(AzureVisionError):
            await service.analyze_image_data(b'invalid image data')
    
    @pytest.mark.asyncio
    async def test_analyze_image_url_with_mock(self):
        """Test analyze_image_url with mocked Azure client."""
        service = AzureVisionService()
        
        # Mock the _analyze_sync method
        mock_result = {
            'tags': [{'name': 'test', 'confidence': 0.9}],
            'objects': [],
            'caption': {'text': 'Test image', 'confidence': 0.8}
        }
        
        with patch.object(service, '_analyze_sync', return_value=mock_result):
            result = await service.analyze_image_url('https://example.com/test.jpg')
            
            assert result == mock_result
            assert len(result['tags']) == 1
            assert result['caption']['text'] == 'Test image'
    
    @pytest.mark.asyncio
    async def test_analyze_image_data_with_mock(self):
        """Test analyze_image_data with mocked Azure client."""
        service = AzureVisionService()
        
        mock_result = {
            'tags': [{'name': 'bedroom', 'confidence': 0.95}],
            'objects': [],
            'caption': None
        }
        
        with patch.object(service, '_analyze_sync', return_value=mock_result):
            result = await service.analyze_image_data(b'fake image bytes')
            
            assert result == mock_result
            assert result['tags'][0]['name'] == 'bedroom'

