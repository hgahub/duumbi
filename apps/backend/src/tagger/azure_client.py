"""Azure AI Vision integration."""
import asyncio
from typing import Optional
from azure.ai.vision.imageanalysis import ImageAnalysisClient
from azure.ai.vision.imageanalysis.models import VisualFeatures
from azure.core.credentials import AzureKeyCredential

from .config import get_settings
from .exceptions import AzureVisionError


class AzureVisionService:
    """Service for Azure AI Vision API."""
    
    def __init__(self):
        """Initialize Azure Vision client."""
        settings = get_settings()
        
        try:
            self.client = ImageAnalysisClient(
                endpoint=settings.azure_vision_endpoint,
                credential=AzureKeyCredential(settings.azure_vision_key)
            )
        except Exception as e:
            raise AzureVisionError(
                f"Failed to initialize Azure Vision client: {str(e)}"
            )
        
        self.timeout = settings.timeout_seconds
        self.max_retries = settings.max_retries
        self.retry_delay = settings.retry_delay_seconds
    
    async def analyze_image_url(self, image_url: str) -> dict:
        """
        Analyze image from URL using Azure AI Vision.
        
        Args:
            image_url: URL of the image
            
        Returns:
            Analysis result dict with tags, objects, caption, etc.
            
        Raises:
            AzureVisionError: If API call fails
        """
        try:
            # Run synchronous Azure SDK call in thread pool
            loop = asyncio.get_event_loop()
            result = await loop.run_in_executor(
                None,
                self._analyze_sync,
                image_url,
                None
            )
            return result
        except AzureVisionError:
            raise
        except Exception as e:
            raise AzureVisionError(f"Image analysis failed: {str(e)}")
    
    async def analyze_image_data(self, image_data: bytes) -> dict:
        """
        Analyze image from binary data.
        
        Args:
            image_data: Image binary data
            
        Returns:
            Analysis result dict with tags, objects, caption, etc.
            
        Raises:
            AzureVisionError: If API call fails
        """
        try:
            loop = asyncio.get_event_loop()
            result = await loop.run_in_executor(
                None,
                self._analyze_sync,
                None,
                image_data
            )
            return result
        except AzureVisionError:
            raise
        except Exception as e:
            raise AzureVisionError(f"Image analysis failed: {str(e)}")
    
    def _analyze_sync(
        self,
        image_url: Optional[str],
        image_data: Optional[bytes]
    ) -> dict:
        """
        Synchronous image analysis.
        
        Args:
            image_url: URL of image (mutually exclusive with image_data)
            image_data: Binary image data
            
        Returns:
            Parsed analysis result
            
        Raises:
            AzureVisionError: If API call fails
        """
        # Define features to extract
        visual_features = [
            VisualFeatures.TAGS,
            VisualFeatures.OBJECTS,
            VisualFeatures.CAPTION,
        ]
        
        try:
            # Call Azure API
            if image_url:
                result = self.client.analyze_from_url(
                    image_url=image_url,
                    visual_features=visual_features,
                    language="en"
                )
            else:
                result = self.client.analyze(
                    image_data=image_data,
                    visual_features=visual_features,
                    language="en"
                )
            
            # Parse and return result
            return self._parse_result(result)
            
        except Exception as e:
            error_msg = str(e)
            status_code = None
            
            # Try to extract status code from error
            if hasattr(e, 'status_code'):
                status_code = e.status_code
            
            raise AzureVisionError(error_msg, status_code=status_code)
    
    def _parse_result(self, result) -> dict:
        """
        Parse Azure Vision API result.
        
        Args:
            result: Azure Vision API response object
            
        Returns:
            Parsed dict with tags, objects, caption, etc.
        """
        parsed = {
            "tags": [],
            "objects": [],
            "caption": None,
        }
        
        # Extract tags
        if result.tags and result.tags.list:
            parsed["tags"] = [
                {"name": tag.name, "confidence": tag.confidence}
                for tag in result.tags.list
            ]

        # Extract objects
        if result.objects and result.objects.list:
            parsed["objects"] = [
                {
                    "name": obj.tags[0].name if obj.tags else "unknown",
                    "confidence": obj.tags[0].confidence if obj.tags else 0.0,
                    "bounding_box": {
                        "x": obj.bounding_box.x,
                        "y": obj.bounding_box.y,
                        "w": obj.bounding_box.w,
                        "h": obj.bounding_box.h,
                    }
                }
                for obj in result.objects.list
            ]

        # Extract caption
        if result.caption:
            parsed["caption"] = {
                "text": result.caption.text,
                "confidence": result.caption.confidence
            }

        return parsed

