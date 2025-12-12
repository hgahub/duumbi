"""Configuration for Tagger module."""
from functools import lru_cache
from pydantic_settings import BaseSettings, SettingsConfigDict


class TaggerSettings(BaseSettings):
    """Settings for image analysis service."""
    
    model_config = SettingsConfigDict(
        env_file=".env",
        env_prefix="TAGGER_",
        case_sensitive=False,
        extra="ignore"
    )
    
    # Azure AI Vision Configuration
    azure_vision_endpoint: str = "https://placeholder.cognitiveservices.azure.com/"
    azure_vision_key: str = "placeholder-key"
    
    # Image Size Constraints (in bytes and pixels)
    max_image_size_mb: int = 10
    max_image_width: int = 4096
    max_image_height: int = 4096
    min_image_width: int = 640
    min_image_height: int = 480
    
    # Quality Thresholds (0-10 scale)
    min_quality_score: float = 5.0
    min_brightness_score: float = 4.0
    min_sharpness_score: float = 5.0
    
    # Processing Configuration
    timeout_seconds: int = 30
    max_retries: int = 3
    retry_delay_seconds: float = 1.0
    
    # Batch Processing
    max_batch_size: int = 20
    batch_timeout_seconds: int = 120
    
    # Feature Detection Thresholds
    room_detection_confidence_threshold: float = 0.6
    feature_detection_confidence_threshold: float = 0.7
    tag_confidence_threshold: float = 0.5
    
    @property
    def max_image_size_bytes(self) -> int:
        """Convert max image size from MB to bytes."""
        return self.max_image_size_mb * 1024 * 1024
    
    def is_quality_acceptable(
        self,
        overall_score: float,
        brightness_score: float,
        sharpness_score: float
    ) -> bool:
        """
        Check if image quality meets minimum thresholds.
        
        Args:
            overall_score: Overall quality score (0-10)
            brightness_score: Brightness score (0-10)
            sharpness_score: Sharpness score (0-10)
            
        Returns:
            True if quality is acceptable, False otherwise
        """
        return (
            overall_score >= self.min_quality_score and
            brightness_score >= self.min_brightness_score and
            sharpness_score >= self.min_sharpness_score
        )


@lru_cache
def get_settings() -> TaggerSettings:
    """
    Get cached settings instance.
    
    This function uses lru_cache to ensure only one instance
    of settings is created and reused throughout the application.
    
    Returns:
        TaggerSettings instance
    """
    return TaggerSettings()

