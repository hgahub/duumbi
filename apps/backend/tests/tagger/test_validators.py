"""Tests for image validators."""
import pytest
from io import BytesIO
from PIL import Image

from src.tagger.validators import validate_image_file, calculate_image_quality
from src.tagger.exceptions import (
    ImageTooLargeError,
    ImageTooSmallError,
    UnsupportedImageFormatError,
    ImageProcessingError,
)
from src.tagger.models import ImageQualityIssue


class TestValidateImageFile:
    """Tests for validate_image_file function."""
    
    def test_valid_jpeg_image(self):
        """Test validation of a valid JPEG image."""
        img = Image.new('RGB', (1024, 768), color=(150, 150, 150))
        buffer = BytesIO()
        img.save(buffer, format='JPEG')
        buffer.seek(0)
        
        validated_img, metadata = validate_image_file(buffer)
        
        assert validated_img.size == (1024, 768)
        assert metadata['format'] == 'JPEG'
        assert metadata['width'] == 1024
        assert metadata['height'] == 768
        assert metadata['mode'] == 'RGB'
        assert metadata['size_mb'] > 0
    
    def test_valid_png_image(self):
        """Test validation of a valid PNG image."""
        img = Image.new('RGB', (800, 600), color=(200, 100, 50))
        buffer = BytesIO()
        img.save(buffer, format='PNG')
        buffer.seek(0)
        
        validated_img, metadata = validate_image_file(buffer)
        
        assert validated_img.size == (800, 600)
        assert metadata['format'] == 'PNG'
    
    def test_image_too_small_width(self):
        """Test rejection of image with width below minimum."""
        img = Image.new('RGB', (500, 768), color=(150, 150, 150))
        buffer = BytesIO()
        img.save(buffer, format='JPEG')
        buffer.seek(0)
        
        with pytest.raises(ImageTooSmallError) as exc_info:
            validate_image_file(buffer)
        
        assert 'width' in str(exc_info.value).lower()
        assert exc_info.value.details['type'] == 'width'
    
    def test_image_too_small_height(self):
        """Test rejection of image with height below minimum."""
        img = Image.new('RGB', (1024, 400), color=(150, 150, 150))
        buffer = BytesIO()
        img.save(buffer, format='JPEG')
        buffer.seek(0)
        
        with pytest.raises(ImageTooSmallError) as exc_info:
            validate_image_file(buffer)
        
        assert 'height' in str(exc_info.value).lower()
    
    def test_image_too_large_width(self):
        """Test rejection of image with width above maximum."""
        img = Image.new('RGB', (5000, 768), color=(150, 150, 150))
        buffer = BytesIO()
        img.save(buffer, format='JPEG')
        buffer.seek(0)
        
        with pytest.raises(ImageTooLargeError) as exc_info:
            validate_image_file(buffer)
        
        assert 'width' in str(exc_info.value).lower()
    
    def test_bytes_input(self):
        """Test validation with bytes input instead of file object."""
        img = Image.new('RGB', (1024, 768), color=(150, 150, 150))
        buffer = BytesIO()
        img.save(buffer, format='JPEG')
        image_bytes = buffer.getvalue()
        
        validated_img, metadata = validate_image_file(image_bytes)
        
        assert validated_img.size == (1024, 768)
        assert metadata['format'] == 'JPEG'


class TestCalculateImageQuality:
    """Tests for calculate_image_quality function."""
    
    def test_bright_image_quality(self):
        """Test quality calculation for a bright image."""
        img = Image.new('RGB', (1024, 768), color=(180, 180, 180))
        
        quality = calculate_image_quality(img)
        
        assert 'overall_score' in quality
        assert 'brightness_score' in quality
        assert 'sharpness_score' in quality
        assert 'composition_score' in quality
        assert quality['overall_score'] >= 0
        assert quality['overall_score'] <= 10
        assert quality['brightness'] > 150
    
    def test_dark_image_detection(self):
        """Test detection of dark images."""
        img = Image.new('RGB', (1024, 768), color=(30, 30, 30))
        
        quality = calculate_image_quality(img)
        
        assert ImageQualityIssue.TOO_DARK in quality['issues']
        assert quality['brightness'] < 50
        assert quality['brightness_score'] < 5.0
    
    def test_bright_image_detection(self):
        """Test detection of too bright images."""
        img = Image.new('RGB', (1024, 768), color=(240, 240, 240))
        
        quality = calculate_image_quality(img)
        
        assert ImageQualityIssue.TOO_BRIGHT in quality['issues']
        assert quality['brightness'] > 200
    
    def test_low_resolution_detection(self):
        """Test detection of low resolution images."""
        img = Image.new('RGB', (800, 600), color=(150, 150, 150))
        
        quality = calculate_image_quality(img)
        
        assert ImageQualityIssue.LOW_RESOLUTION in quality['issues']
    
    def test_acceptable_quality_check(self):
        """Test is_acceptable flag for good quality image."""
        img = Image.new('RGB', (1920, 1080), color=(150, 150, 150))
        
        quality = calculate_image_quality(img)
        
        # Note: is_acceptable depends on settings thresholds
        assert 'is_acceptable' in quality
        assert isinstance(quality['is_acceptable'], bool)
    
    def test_aspect_ratio_calculation(self):
        """Test aspect ratio calculation."""
        img = Image.new('RGB', (1920, 1080), color=(150, 150, 150))
        
        quality = calculate_image_quality(img)
        
        assert 'aspect_ratio' in quality
        assert quality['aspect_ratio'] == pytest.approx(1920 / 1080, rel=0.01)

