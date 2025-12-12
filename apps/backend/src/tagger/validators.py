"""Image validation utilities."""
from io import BytesIO
from typing import BinaryIO, Union
from PIL import Image, ImageStat
import numpy as np

from .config import get_settings
from .exceptions import (
    ImageTooLargeError,
    ImageTooSmallError,
    UnsupportedImageFormatError,
    ImageProcessingError,
)
from .models import ImageQualityIssue


SUPPORTED_FORMATS = {"JPEG", "PNG", "WEBP"}


def validate_image_file(file: Union[BinaryIO, bytes]) -> tuple[Image.Image, dict]:
    """
    Validate uploaded image file.
    
    Args:
        file: Binary file object or bytes
        
    Returns:
        Tuple of (PIL Image, metadata dict)
        
    Raises:
        UnsupportedImageFormatError: If format is not supported
        ImageTooLargeError: If image exceeds size limits
        ImageTooSmallError: If image is below minimum resolution
        ImageProcessingError: If image cannot be opened
    """
    settings = get_settings()
    
    # Convert bytes to BytesIO if needed
    if isinstance(file, bytes):
        file = BytesIO(file)
    
    # Get file size
    file.seek(0, 2)  # Seek to end
    file_size_bytes = file.tell()
    file.seek(0)  # Reset to beginning
    
    # Check file size
    if file_size_bytes > settings.max_image_size_bytes:
        raise ImageTooLargeError(
            actual_size=file_size_bytes,
            max_size=settings.max_image_size_bytes,
            dimension_type="file_size"
        )
    
    # Load image
    try:
        img = Image.open(file)
        img.load()  # Force load to catch truncated images
    except Exception as e:
        raise ImageProcessingError(
            f"Cannot open image: {str(e)}",
            operation="open"
        )
    
    # Check format
    if img.format not in SUPPORTED_FORMATS:
        raise UnsupportedImageFormatError(
            format_name=img.format or "UNKNOWN",
            supported_formats=list(SUPPORTED_FORMATS)
        )
    
    # Check dimensions
    width, height = img.size
    
    if width > settings.max_image_width:
        raise ImageTooLargeError(
            actual_size=width,
            max_size=settings.max_image_width,
            dimension_type="width"
        )
    
    if height > settings.max_image_height:
        raise ImageTooLargeError(
            actual_size=height,
            max_size=settings.max_image_height,
            dimension_type="height"
        )
    
    if width < settings.min_image_width:
        raise ImageTooSmallError(
            actual_size=width,
            min_size=settings.min_image_width,
            dimension_type="width"
        )
    
    if height < settings.min_image_height:
        raise ImageTooSmallError(
            actual_size=height,
            min_size=settings.min_image_height,
            dimension_type="height"
        )
    
    # Metadata
    metadata = {
        "format": img.format,
        "width": width,
        "height": height,
        "mode": img.mode,
        "size_bytes": file_size_bytes,
        "size_mb": round(file_size_bytes / (1024 * 1024), 2),
    }
    
    return img, metadata


def calculate_image_quality(img: Image.Image) -> dict:
    """
    Calculate basic image quality metrics using PIL and numpy.
    
    Args:
        img: PIL Image object
        
    Returns:
        Dict with quality metrics including scores and detected issues
    """
    settings = get_settings()
    
    # Convert to RGB if needed
    if img.mode != 'RGB':
        img = img.convert('RGB')
    
    # Convert to numpy array
    img_array = np.array(img)
    
    # 1. Brightness Analysis
    brightness = float(np.mean(img_array))
    
    # Score brightness (0-10 scale)
    if 80 <= brightness <= 180:
        # Optimal range
        brightness_score = 10.0
    elif 50 <= brightness < 80 or 180 < brightness <= 200:
        # Acceptable range
        brightness_score = 7.0
    elif brightness < 50:
        # Too dark
        brightness_score = max(0.0, (brightness / 50.0) * 5.0)
    else:
        # Too bright
        brightness_score = max(0.0, 10.0 - ((brightness - 200) / 55.0) * 5.0)
    
    # 2. Sharpness Analysis (using standard deviation as proxy)
    gray = img.convert('L')
    stat = ImageStat.Stat(gray)
    sharpness_variance = stat.stddev[0]

    # Score sharpness (0-10 scale)
    # Higher variance = sharper image
    if sharpness_variance >= 50:
        sharpness_score = 10.0
    elif sharpness_variance >= 30:
        sharpness_score = 7.0 + ((sharpness_variance - 30) / 20.0) * 3.0
    elif sharpness_variance >= 15:
        sharpness_score = 5.0 + ((sharpness_variance - 15) / 15.0) * 2.0
    else:
        sharpness_score = (sharpness_variance / 15.0) * 5.0

    # 3. Composition Analysis (simple aspect ratio check)
    width, height = img.size
    aspect_ratio = width / height

    # Prefer landscape or portrait, penalize extreme ratios
    if 0.5 <= aspect_ratio <= 2.0:
        composition_score = 8.0
    elif 0.3 <= aspect_ratio < 0.5 or 2.0 < aspect_ratio <= 3.0:
        composition_score = 6.0
    else:
        composition_score = 4.0

    # 4. Overall Score (weighted average)
    overall_score = (
        brightness_score * 0.35 +
        sharpness_score * 0.45 +
        composition_score * 0.20
    )

    # 5. Detect Quality Issues
    issues = []

    if brightness < 50:
        issues.append(ImageQualityIssue.TOO_DARK)
    elif brightness > 200:
        issues.append(ImageQualityIssue.TOO_BRIGHT)

    if sharpness_variance < 15:
        issues.append(ImageQualityIssue.BLURRY)

    if width < 1024 or height < 768:
        issues.append(ImageQualityIssue.LOW_RESOLUTION)

    if aspect_ratio < 0.3 or aspect_ratio > 3.0:
        issues.append(ImageQualityIssue.POOR_COMPOSITION)

    # 6. Check if quality is acceptable
    is_acceptable = settings.is_quality_acceptable(
        overall_score=overall_score,
        brightness_score=brightness_score,
        sharpness_score=sharpness_score
    )

    return {
        "brightness": round(brightness, 2),
        "brightness_score": round(brightness_score, 2),
        "sharpness": round(sharpness_variance, 2),
        "sharpness_score": round(sharpness_score, 2),
        "composition_score": round(composition_score, 2),
        "overall_score": round(overall_score, 2),
        "issues": issues,
        "is_acceptable": is_acceptable,
        "aspect_ratio": round(aspect_ratio, 2),
    }

