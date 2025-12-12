"""Custom exceptions for Tagger module."""
from typing import Optional


class TaggerException(Exception):
    """Base exception for Tagger module."""
    
    def __init__(self, message: str, details: Optional[dict] = None):
        """
        Initialize exception.
        
        Args:
            message: Error message
            details: Optional additional details about the error
        """
        self.message = message
        self.details = details or {}
        super().__init__(self.message)
    
    def __str__(self) -> str:
        """String representation of the exception."""
        if self.details:
            details_str = ", ".join(f"{k}={v}" for k, v in self.details.items())
            return f"{self.message} ({details_str})"
        return self.message


class ImageValidationError(TaggerException):
    """Raised when image validation fails."""
    pass


class UnsupportedImageFormatError(ImageValidationError):
    """Raised when image format is not supported."""
    
    def __init__(self, format_name: str, supported_formats: Optional[list[str]] = None):
        """
        Initialize exception.
        
        Args:
            format_name: The unsupported format
            supported_formats: List of supported formats
        """
        supported = supported_formats or ["JPEG", "PNG", "WEBP"]
        message = (
            f"Unsupported image format: {format_name}. "
            f"Supported formats: {', '.join(supported)}"
        )
        super().__init__(message, {"format": format_name, "supported": supported})


class ImageTooLargeError(ImageValidationError):
    """Raised when image exceeds size limits."""
    
    def __init__(
        self,
        actual_size: int,
        max_size: int,
        dimension_type: str = "file_size"
    ):
        """
        Initialize exception.
        
        Args:
            actual_size: Actual size/dimension
            max_size: Maximum allowed size/dimension
            dimension_type: Type of dimension (file_size, width, height)
        """
        if dimension_type == "file_size":
            message = (
                f"Image file size {actual_size / (1024*1024):.2f}MB exceeds "
                f"maximum {max_size / (1024*1024):.2f}MB"
            )
        else:
            message = (
                f"Image {dimension_type} {actual_size}px exceeds "
                f"maximum {max_size}px"
            )
        super().__init__(
            message,
            {
                "actual": actual_size,
                "max": max_size,
                "type": dimension_type
            }
        )


class ImageTooSmallError(ImageValidationError):
    """Raised when image is below minimum resolution."""
    
    def __init__(self, actual_size: int, min_size: int, dimension_type: str):
        """
        Initialize exception.
        
        Args:
            actual_size: Actual dimension
            min_size: Minimum required dimension
            dimension_type: Type of dimension (width, height)
        """
        message = (
            f"Image {dimension_type} {actual_size}px is below "
            f"minimum {min_size}px"
        )
        super().__init__(
            message,
            {
                "actual": actual_size,
                "min": min_size,
                "type": dimension_type
            }
        )


class AzureVisionError(TaggerException):
    """Raised when Azure AI Vision API fails."""
    
    def __init__(self, message: str, status_code: Optional[int] = None):
        """
        Initialize exception.
        
        Args:
            message: Error message
            status_code: HTTP status code if available
        """
        details = {}
        if status_code:
            details["status_code"] = status_code
        super().__init__(f"Azure Vision API error: {message}", details)


class ImageProcessingError(TaggerException):
    """Raised when image processing fails."""
    
    def __init__(self, message: str, operation: Optional[str] = None):
        """
        Initialize exception.
        
        Args:
            message: Error message
            operation: The operation that failed
        """
        details = {}
        if operation:
            details["operation"] = operation
        super().__init__(f"Image processing error: {message}", details)


class BatchProcessingError(TaggerException):
    """Raised when batch processing fails."""
    
    def __init__(
        self,
        message: str,
        total_images: int,
        failed_count: int,
        failed_indices: Optional[list[int]] = None
    ):
        """
        Initialize exception.
        
        Args:
            message: Error message
            total_images: Total number of images in batch
            failed_count: Number of failed images
            failed_indices: Indices of failed images
        """
        super().__init__(
            message,
            {
                "total": total_images,
                "failed": failed_count,
                "failed_indices": failed_indices or []
            }
        )

