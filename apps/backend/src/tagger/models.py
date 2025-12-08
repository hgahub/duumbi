"""Pydantic models for image analysis."""
from enum import Enum
from typing import Optional
from pydantic import BaseModel, Field, HttpUrl, field_validator


class RoomType(str, Enum):
    """Supported room types for property images."""
    LIVING_ROOM = "living_room"
    KITCHEN = "kitchen"
    BEDROOM = "bedroom"
    BATHROOM = "bathroom"
    DINING_ROOM = "dining_room"
    OFFICE = "office"
    BALCONY = "balcony"
    TERRACE = "terrace"
    GARDEN = "garden"
    GARAGE = "garage"
    HALLWAY = "hallway"
    EXTERIOR = "exterior"
    OTHER = "other"


class ImageQualityIssue(str, Enum):
    """Image quality issues detected."""
    TOO_DARK = "too_dark"
    TOO_BRIGHT = "too_bright"
    BLURRY = "blurry"
    LOW_RESOLUTION = "low_resolution"
    POOR_COMPOSITION = "poor_composition"
    CLUTTERED = "cluttered"


class ImageFeature(str, Enum):
    """Property features detected in images."""
    MODERN = "modern"
    SPACIOUS = "spacious"
    BRIGHT = "bright"
    FURNISHED = "furnished"
    RENOVATED = "renovated"
    NATURAL_LIGHT = "natural_light"
    HARDWOOD_FLOOR = "hardwood_floor"
    TILE_FLOOR = "tile_floor"
    HIGH_CEILING = "high_ceiling"
    BALCONY_VIEW = "balcony_view"


class ImageAnalysisRequest(BaseModel):
    """Request model for image analysis."""
    image_url: Optional[HttpUrl] = Field(
        None,
        description="URL of the image to analyze"
    )
    
    @field_validator('image_url')
    @classmethod
    def validate_image_url(cls, v: Optional[HttpUrl]) -> Optional[HttpUrl]:
        """Validate image URL format."""
        if v and not str(v).lower().endswith(('.jpg', '.jpeg', '.png', '.webp')):
            raise ValueError("Image URL must end with .jpg, .jpeg, .png, or .webp")
        return v


class ImageQualityScore(BaseModel):
    """Image quality assessment."""
    overall_score: float = Field(
        ...,
        ge=0.0,
        le=10.0,
        description="Overall quality score (0-10)"
    )
    brightness_score: float = Field(..., ge=0.0, le=10.0)
    sharpness_score: float = Field(..., ge=0.0, le=10.0)
    composition_score: float = Field(..., ge=0.0, le=10.0)
    issues: list[ImageQualityIssue] = Field(
        default_factory=list,
        description="List of detected quality issues"
    )


class ImageAnalysisResult(BaseModel):
    """Complete image analysis result."""
    
    # Quality Assessment
    quality: ImageQualityScore = Field(
        ...,
        description="Image quality metrics"
    )
    
    # Room Detection
    room_type: Optional[RoomType] = Field(
        None,
        description="Detected room type"
    )
    room_confidence: float = Field(
        0.0,
        ge=0.0,
        le=1.0,
        description="Confidence score for room type detection"
    )
    
    # Features
    features: list[ImageFeature] = Field(
        default_factory=list,
        description="Detected property features"
    )
    
    # Tags from Azure AI Vision
    tags: list[str] = Field(
        default_factory=list,
        description="Raw tags from Azure AI Vision"
    )
    
    # Caption
    caption: Optional[str] = Field(
        None,
        description="AI-generated image caption"
    )
    
    # Recommendations
    recommendations: list[str] = Field(
        default_factory=list,
        description="Suggestions for improving the image"
    )
    
    # Metadata
    is_suitable: bool = Field(
        ...,
        description="Whether the image is suitable for listing"
    )
    processing_time_ms: int = Field(
        ...,
        description="Processing time in milliseconds"
    )


class BatchAnalysisRequest(BaseModel):
    """Request for analyzing multiple images."""
    image_urls: list[HttpUrl] = Field(
        ...,
        min_length=1,
        max_length=20,
        description="List of image URLs to analyze (max 20)"
    )


class BatchAnalysisResult(BaseModel):
    """Result of batch image analysis."""
    results: list[ImageAnalysisResult] = Field(
        ...,
        description="Analysis results for each image"
    )
    total_processed: int = Field(..., description="Total images processed")
    total_failed: int = Field(..., description="Total images failed")
    processing_time_ms: int = Field(..., description="Total processing time")

