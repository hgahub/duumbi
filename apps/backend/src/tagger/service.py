"""Image analysis service - business logic layer."""
import time
from typing import BinaryIO, Optional

from .azure_client import AzureVisionService
from .config import get_settings
from .validators import validate_image_file, calculate_image_quality
from .models import (
    ImageAnalysisResult,
    ImageQualityScore,
    RoomType,
    ImageFeature,
)


class ImageAnalysisService:
    """Service for analyzing property images."""
    
    def __init__(self):
        """Initialize the service."""
        self.azure_service = AzureVisionService()
        self.settings = get_settings()
    
    async def analyze_image_from_url(self, image_url: str) -> ImageAnalysisResult:
        """
        Analyze image from URL.
        
        Args:
            image_url: URL of the image to analyze
            
        Returns:
            ImageAnalysisResult with quality scores, tags, and recommendations
            
        Raises:
            AzureVisionError: If Azure API call fails
        """
        start_time = time.time()
        
        # Call Azure AI Vision
        azure_result = await self.azure_service.analyze_image_url(image_url)
        
        # Process results
        result = self._process_azure_result(azure_result, None)
        
        # Calculate processing time
        processing_time_ms = int((time.time() - start_time) * 1000)
        result.processing_time_ms = processing_time_ms
        
        return result
    
    async def analyze_image_from_file(
        self,
        file: BinaryIO
    ) -> ImageAnalysisResult:
        """
        Analyze uploaded image file.
        
        Args:
            file: Binary file object
            
        Returns:
            ImageAnalysisResult with quality scores, tags, and recommendations
            
        Raises:
            ImageValidationError: If image validation fails
            AzureVisionError: If Azure API call fails
        """
        start_time = time.time()
        
        # Step 1: Validate image
        img, metadata = validate_image_file(file)
        
        # Step 2: Calculate local quality metrics
        quality_metrics = calculate_image_quality(img)
        
        # Step 3: Convert image to bytes for Azure
        file.seek(0)
        image_bytes = file.read()
        
        # Step 4: Call Azure AI Vision
        azure_result = await self.azure_service.analyze_image_data(image_bytes)
        
        # Step 5: Process and combine results
        result = self._process_azure_result(azure_result, quality_metrics)
        
        # Calculate processing time
        processing_time_ms = int((time.time() - start_time) * 1000)
        result.processing_time_ms = processing_time_ms
        
        return result
    
    def _process_azure_result(
        self,
        azure_result: dict,
        quality_metrics: Optional[dict]
    ) -> ImageAnalysisResult:
        """
        Process Azure Vision result and combine with quality metrics.
        
        Args:
            azure_result: Result from Azure AI Vision
            quality_metrics: Optional quality metrics from local analysis
            
        Returns:
            ImageAnalysisResult
        """
        # Extract tags from Azure
        tags = [tag["name"] for tag in azure_result.get("tags", [])]
        
        # Detect room type from tags
        room_type, room_confidence = self._detect_room_type(azure_result.get("tags", []))
        
        # Detect features from tags
        features = self._detect_features(azure_result.get("tags", []))
        
        # Get caption
        caption_data = azure_result.get("caption")
        caption = caption_data["text"] if caption_data else None
        
        # Create quality score
        if quality_metrics:
            quality = ImageQualityScore(
                overall_score=quality_metrics["overall_score"],
                brightness_score=quality_metrics["brightness_score"],
                sharpness_score=quality_metrics["sharpness_score"],
                composition_score=quality_metrics["composition_score"],
                issues=quality_metrics["issues"]
            )
            is_suitable = quality_metrics["is_acceptable"]
        else:
            # Default quality for URL-based analysis (no local validation)
            quality = ImageQualityScore(
                overall_score=7.0,
                brightness_score=7.0,
                sharpness_score=7.0,
                composition_score=7.0,
                issues=[]
            )
            is_suitable = True
        
        # Generate recommendations
        recommendations = self._generate_recommendations(
            quality_metrics if quality_metrics else {},
            tags,
            room_type
        )
        
        return ImageAnalysisResult(
            quality=quality,
            room_type=room_type,
            room_confidence=room_confidence,
            features=features,
            tags=tags,
            caption=caption,
            recommendations=recommendations,
            is_suitable=is_suitable,
            processing_time_ms=0  # Will be set by caller
        )

    def _detect_room_type(self, tags: list[dict]) -> tuple[Optional[RoomType], float]:
        """
        Detect room type from Azure tags.

        Args:
            tags: List of tags with name and confidence

        Returns:
            Tuple of (RoomType, confidence)
        """
        # Room type keywords mapping
        room_keywords = {
            RoomType.LIVING_ROOM: ["living room", "living", "lounge", "sitting room"],
            RoomType.KITCHEN: ["kitchen", "kitchenette"],
            RoomType.BEDROOM: ["bedroom", "bed room", "sleeping room"],
            RoomType.BATHROOM: ["bathroom", "bath room", "shower", "toilet"],
            RoomType.DINING_ROOM: ["dining room", "dining"],
            RoomType.OFFICE: ["office", "study", "workspace"],
            RoomType.BALCONY: ["balcony", "terrace", "patio"],
            RoomType.GARDEN: ["garden", "yard", "backyard"],
            RoomType.GARAGE: ["garage", "parking"],
            RoomType.HALLWAY: ["hallway", "corridor", "entrance"],
            RoomType.EXTERIOR: ["exterior", "building", "facade", "outside"],
        }

        # Find best matching room type
        best_match = None
        best_confidence = 0.0

        for tag in tags:
            tag_name = tag["name"].lower()
            tag_confidence = tag["confidence"]

            for room_type, keywords in room_keywords.items():
                for keyword in keywords:
                    if keyword in tag_name:
                        if tag_confidence > best_confidence:
                            best_match = room_type
                            best_confidence = tag_confidence

        # Only return if confidence is above threshold
        if best_confidence >= self.settings.room_detection_confidence_threshold:
            return best_match, best_confidence

        return None, 0.0

    def _detect_features(self, tags: list[dict]) -> list[ImageFeature]:
        """
        Detect property features from Azure tags.

        Args:
            tags: List of tags with name and confidence

        Returns:
            List of detected ImageFeature enums
        """
        # Feature keywords mapping
        feature_keywords = {
            ImageFeature.MODERN: ["modern", "contemporary", "minimalist"],
            ImageFeature.SPACIOUS: ["spacious", "large", "big", "roomy"],
            ImageFeature.BRIGHT: ["bright", "light", "sunny", "illuminated"],
            ImageFeature.FURNISHED: ["furniture", "furnished", "sofa", "chair", "table"],
            ImageFeature.RENOVATED: ["renovated", "new", "updated", "remodeled"],
            ImageFeature.NATURAL_LIGHT: ["window", "natural light", "daylight"],
            ImageFeature.HARDWOOD_FLOOR: ["hardwood", "wood floor", "wooden floor"],
            ImageFeature.TILE_FLOOR: ["tile", "tiled floor"],
            ImageFeature.HIGH_CEILING: ["high ceiling", "tall ceiling"],
        }

        detected_features = []
        threshold = self.settings.feature_detection_confidence_threshold

        for tag in tags:
            tag_name = tag["name"].lower()
            tag_confidence = tag["confidence"]

            if tag_confidence < threshold:
                continue

            for feature, keywords in feature_keywords.items():
                for keyword in keywords:
                    if keyword in tag_name and feature not in detected_features:
                        detected_features.append(feature)

        return detected_features

    def _generate_recommendations(
        self,
        quality_metrics: dict,
        tags: list[str],
        room_type: Optional[RoomType]
    ) -> list[str]:
        """
        Generate recommendations for improving the image.

        Args:
            quality_metrics: Quality metrics from local analysis
            tags: Tags from Azure
            room_type: Detected room type

        Returns:
            List of recommendation strings
        """
        recommendations = []

        # Quality-based recommendations
        if quality_metrics:
            issues = quality_metrics.get("issues", [])

            for issue in issues:
                if issue.value == "too_dark":
                    recommendations.append("Increase lighting or use flash")
                elif issue.value == "too_bright":
                    recommendations.append("Reduce exposure or avoid direct sunlight")
                elif issue.value == "blurry":
                    recommendations.append("Use a tripod or increase shutter speed")
                elif issue.value == "low_resolution":
                    recommendations.append("Use a higher resolution camera")
                elif issue.value == "poor_composition":
                    recommendations.append("Adjust framing and composition")

        # Content-based recommendations
        if room_type == RoomType.LIVING_ROOM:
            if "clutter" in tags or "messy" in tags:
                recommendations.append("Remove clutter before photographing")

        if not recommendations:
            recommendations.append("Image quality is good")

        return recommendations

