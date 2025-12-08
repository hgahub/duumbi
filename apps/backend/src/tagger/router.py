"""FastAPI router for image analysis endpoints."""
from fastapi import APIRouter, UploadFile, File, HTTPException, status

from .service import ImageAnalysisService
from .models import (
    ImageAnalysisRequest,
    ImageAnalysisResult,
    BatchAnalysisRequest,
    BatchAnalysisResult,
)
from .exceptions import (
    ImageValidationError,
    AzureVisionError
)

router = APIRouter(prefix="/tagger", tags=["Image Analysis"])

# Initialize service (singleton pattern)
_service: ImageAnalysisService | None = None


def get_service() -> ImageAnalysisService:
    """Get or create ImageAnalysisService instance."""
    global _service
    if _service is None:
        _service = ImageAnalysisService()
    return _service


@router.post(
    "/analyze",
    response_model=ImageAnalysisResult,
    summary="Analyze image from URL",
    description="Analyze a property image from a URL using Azure AI Vision",
    responses={
        200: {"description": "Image analyzed successfully"},
        400: {"description": "Invalid image URL or validation error"},
        500: {"description": "Azure Vision API error or internal error"},
    }
)
async def analyze_image_url(request: ImageAnalysisRequest) -> ImageAnalysisResult:
    """
    Analyze image from URL.

    Args:
        request: ImageAnalysisRequest with image_url

    Returns:
        ImageAnalysisResult with quality scores, tags, and recommendations

    Raises:
        HTTPException: If validation or analysis fails
    """
    if not request.image_url:
        raise HTTPException(
            status_code=status.HTTP_400_BAD_REQUEST,
            detail="image_url is required"
        )

    service = get_service()

    try:
        result = await service.analyze_image_from_url(str(request.image_url))
        return result
    except AzureVisionError as e:
        raise HTTPException(
            status_code=status.HTTP_500_INTERNAL_SERVER_ERROR,
            detail=f"Azure Vision API error: {e.message}"
        )
    except Exception as e:
        raise HTTPException(
            status_code=status.HTTP_500_INTERNAL_SERVER_ERROR,
            detail=f"Internal error: {str(e)}"
        )


@router.post(
    "/analyze/upload",
    response_model=ImageAnalysisResult,
    summary="Analyze uploaded image",
    description="Analyze an uploaded property image file",
    responses={
        200: {"description": "Image analyzed successfully"},
        400: {"description": "Invalid image file or validation error"},
        413: {"description": "Image file too large"},
        500: {"description": "Azure Vision API error or internal error"},
    }
)
async def analyze_image_upload(
    file: UploadFile = File(..., description="Image file (JPEG, PNG, WEBP)")
) -> ImageAnalysisResult:
    """
    Analyze uploaded image file.

    Args:
        file: Uploaded image file

    Returns:
        ImageAnalysisResult with quality scores, tags, and recommendations

    Raises:
        HTTPException: If validation or analysis fails
    """
    # Validate file type
    if not file.content_type or not file.content_type.startswith("image/"):
        raise HTTPException(
            status_code=status.HTTP_400_BAD_REQUEST,
            detail=f"Invalid file type: {file.content_type}. Must be an image."
        )

    service = get_service()

    try:
        # Read file content
        content = await file.read()

        # Convert to BytesIO for service
        from io import BytesIO
        file_buffer = BytesIO(content)

        # Analyze image
        result = await service.analyze_image_from_file(file_buffer)
        return result

    except ImageValidationError as e:
        # Handle validation errors (too large, too small, wrong format)
        status_code = status.HTTP_400_BAD_REQUEST
        if "exceeds" in e.message.lower():
            status_code = status.HTTP_413_REQUEST_ENTITY_TOO_LARGE

        raise HTTPException(
            status_code=status_code,
            detail=e.message
        )
    except AzureVisionError as e:
        raise HTTPException(
            status_code=status.HTTP_500_INTERNAL_SERVER_ERROR,
            detail=f"Azure Vision API error: {e.message}"
        )
    except Exception as e:
        raise HTTPException(
            status_code=status.HTTP_500_INTERNAL_SERVER_ERROR,
            detail=f"Internal error: {str(e)}"
        )
    finally:
        await file.close()


@router.post(
    "/analyze/batch",
    response_model=BatchAnalysisResult,
    summary="Analyze multiple images",
    description="Analyze multiple property images from URLs in batch",
    responses={
        200: {"description": "Batch analysis completed (may include partial failures)"},
        400: {"description": "Invalid request or too many images"},
        500: {"description": "Internal error"},
    }
)
async def analyze_batch(request: BatchAnalysisRequest) -> BatchAnalysisResult:
    """
    Analyze multiple images in batch.

    Args:
        request: BatchAnalysisRequest with list of image URLs

    Returns:
        BatchAnalysisResult with results for each image

    Raises:
        HTTPException: If request is invalid
    """
    service = get_service()

    results = []
    failed_count = 0
    start_time = __import__('time').time()

    # Process each image
    for image_url in request.image_urls:
        try:
            result = await service.analyze_image_from_url(str(image_url))
            results.append(result)
        except Exception as e:
            # Log error but continue processing
            failed_count += 1
            # Create a failed result placeholder
            from .models import ImageQualityScore
            failed_result = ImageAnalysisResult(
                quality=ImageQualityScore(
                    overall_score=0.0,
                    brightness_score=0.0,
                    sharpness_score=0.0,
                    composition_score=0.0,
                    issues=[]
                ),
                room_type=None,
                room_confidence=0.0,
                features=[],
                tags=[],
                caption=f"Analysis failed: {str(e)[:100]}",
                recommendations=[],
                is_suitable=False,
                processing_time_ms=0
            )
            results.append(failed_result)

    processing_time_ms = int((__import__('time').time() - start_time) * 1000)

    return BatchAnalysisResult(
        results=results,
        total_processed=len(results),
        total_failed=failed_count,
        processing_time_ms=processing_time_ms
    )


@router.get(
    "/health",
    summary="Health check",
    description="Check if the Tagger service is healthy",
    responses={
        200: {"description": "Service is healthy"},
        503: {"description": "Service is unhealthy"},
    }
)
async def health_check():
    """
    Health check endpoint.

    Returns:
        Health status
    """
    try:
        service = get_service()
        return {
            "status": "healthy",
            "service": "tagger",
            "azure_configured": bool(service.azure_service.client)
        }
    except Exception as e:
        raise HTTPException(
            status_code=status.HTTP_503_SERVICE_UNAVAILABLE,
            detail=f"Service unhealthy: {str(e)}"
        )
