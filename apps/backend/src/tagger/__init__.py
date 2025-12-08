"""Tagger module for image analysis."""
from .models import (
    RoomType,
    ImageQualityIssue,
    ImageFeature,
    ImageAnalysisRequest,
    ImageQualityScore,
    ImageAnalysisResult,
    BatchAnalysisRequest,
    BatchAnalysisResult,
)
from .config import TaggerSettings, get_settings
from .validators import validate_image_file, calculate_image_quality
from .azure_client import AzureVisionService
from .service import ImageAnalysisService
from .exceptions import (
    TaggerException,
    ImageValidationError,
    UnsupportedImageFormatError,
    ImageTooLargeError,
    ImageTooSmallError,
    AzureVisionError,
    ImageProcessingError,
    BatchProcessingError,
)

__all__ = [
    # Models
    "RoomType",
    "ImageQualityIssue",
    "ImageFeature",
    "ImageAnalysisRequest",
    "ImageQualityScore",
    "ImageAnalysisResult",
    "BatchAnalysisRequest",
    "BatchAnalysisResult",
    # Config
    "TaggerSettings",
    "get_settings",
    # Validators
    "validate_image_file",
    "calculate_image_quality",
    # Azure Client
    "AzureVisionService",
    # Service
    "ImageAnalysisService",
    # Exceptions
    "TaggerException",
    "ImageValidationError",
    "UnsupportedImageFormatError",
    "ImageTooLargeError",
    "ImageTooSmallError",
    "AzureVisionError",
    "ImageProcessingError",
    "BatchProcessingError",
]

