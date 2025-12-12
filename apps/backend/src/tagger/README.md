# Tagger Module

AI-powered image analysis module for property listings using Azure AI Vision.

## Overview

The Tagger module provides comprehensive image analysis for real estate property photos, including:
- **Quality Assessment**: Brightness, sharpness, composition scoring
- **Room Type Detection**: Automatic identification of room types (living room, kitchen, etc.)
- **Feature Detection**: Property features (modern, spacious, bright, etc.)
- **Smart Recommendations**: Actionable suggestions for improving image quality

## Features

### ✅ Image Quality Analysis
- Brightness scoring (0-10 scale)
- Sharpness detection (blur detection)
- Composition analysis (aspect ratio)
- Quality issue detection (too dark, too bright, blurry, low resolution)
- Overall quality score with weighted metrics

### ✅ Room Type Detection
Supports 13 room types:
- Living Room, Kitchen, Bedroom, Bathroom
- Dining Room, Office, Balcony, Terrace
- Garden, Garage, Hallway, Exterior, Other

### ✅ Feature Detection
Detects 10 property features:
- Modern, Spacious, Bright, Furnished
- Renovated, Natural Light, Hardwood Floor
- Tile Floor, High Ceiling, Balcony View

### ✅ Azure AI Vision Integration
- Object detection with bounding boxes
- Tag generation with confidence scores
- AI-generated image captions
- Multi-language support (English)

## Architecture

```
tagger/
├── models.py          # Pydantic models (Request/Response)
├── config.py          # Configuration (Settings)
├── exceptions.py      # Custom exceptions
├── validators.py      # Image validation & quality calculation
├── azure_client.py    # Azure AI Vision integration
├── service.py         # Business logic layer
└── router.py          # FastAPI endpoints
```

## API Endpoints

### POST /api/tagger/analyze
Analyze image from URL.

**Request:**
```json
{
  "image_url": "https://example.com/property.jpg"
}
```

**Response:**
```json
{
  "quality": {
    "overall_score": 8.5,
    "brightness_score": 9.0,
    "sharpness_score": 8.0,
    "composition_score": 8.5,
    "issues": []
  },
  "room_type": "living_room",
  "room_confidence": 0.95,
  "features": ["modern", "bright", "spacious"],
  "tags": ["living room", "furniture", "window"],
  "caption": "A modern living room with bright natural lighting",
  "recommendations": ["Image quality is good"],
  "is_suitable": true,
  "processing_time_ms": 250
}
```

### POST /api/tagger/analyze/upload
Analyze uploaded image file.

**Request:**
- Multipart form data
- Field: `file` (image/jpeg, image/png, image/webp)

**Response:** Same as `/analyze`

### POST /api/tagger/analyze/batch
Analyze multiple images in batch (max 20).

**Request:**
```json
{
  "image_urls": [
    "https://example.com/img1.jpg",
    "https://example.com/img2.jpg"
  ]
}
```

**Response:**
```json
{
  "results": [...],
  "total_processed": 2,
  "total_failed": 0,
  "processing_time_ms": 500
}
```

### GET /api/tagger/health
Health check endpoint.

**Response:**
```json
{
  "status": "healthy",
  "service": "tagger",
  "azure_configured": true
}
```

## Configuration

Environment variables (`.env`):

```bash
# Azure AI Vision
TAGGER_AZURE_VISION_ENDPOINT=https://your-resource.cognitiveservices.azure.com/
TAGGER_AZURE_VISION_KEY=your-api-key

# Image Constraints
TAGGER_MAX_IMAGE_SIZE_MB=10
TAGGER_MAX_IMAGE_WIDTH=4096
TAGGER_MAX_IMAGE_HEIGHT=4096
TAGGER_MIN_IMAGE_WIDTH=640
TAGGER_MIN_IMAGE_HEIGHT=480

# Quality Thresholds
TAGGER_MIN_QUALITY_SCORE=5.0
TAGGER_MIN_BRIGHTNESS_SCORE=4.0
TAGGER_MIN_SHARPNESS_SCORE=5.0

# Processing
TAGGER_TIMEOUT_SECONDS=30
TAGGER_MAX_RETRIES=3
```

See `.env.example` for all configuration options.

## Usage Examples

### Python SDK

```python
from src.tagger.service import ImageAnalysisService
from io import BytesIO

# Initialize service
service = ImageAnalysisService()

# Analyze from URL
result = await service.analyze_image_from_url(
    "https://example.com/property.jpg"
)

# Analyze from file
with open("property.jpg", "rb") as f:
    result = await service.analyze_image_from_file(f)

print(f"Room: {result.room_type}")
print(f"Quality: {result.quality.overall_score}/10")
print(f"Suitable: {result.is_suitable}")
```

### cURL

```bash
# Analyze from URL
curl -X POST http://localhost:8000/api/tagger/analyze \
  -H "Content-Type: application/json" \
  -d '{"image_url": "https://example.com/property.jpg"}'

# Upload file
curl -X POST http://localhost:8000/api/tagger/analyze/upload \
  -F "file=@property.jpg"
```

### JavaScript/TypeScript

```typescript
// Analyze from URL
const response = await fetch('/api/tagger/analyze', {
  method: 'POST',
  headers: { 'Content-Type': 'application/json' },
  body: JSON.stringify({
    image_url: 'https://example.com/property.jpg'
  })
});
const result = await response.json();

// Upload file
const formData = new FormData();
formData.append('file', fileInput.files[0]);

const uploadResponse = await fetch('/api/tagger/analyze/upload', {
  method: 'POST',
  body: formData
});
const uploadResult = await uploadResponse.json();
```

## Image Requirements

### Supported Formats
- JPEG (.jpg, .jpeg)
- PNG (.png)
- WEBP (.webp)

### Size Constraints
- **File size**: Max 10MB (configurable)
- **Dimensions**: 640x480 to 4096x4096 pixels (configurable)
- **Aspect ratio**: Any (optimal: 0.5 - 2.0)

### Quality Guidelines
For best results:
- ✅ Good lighting (natural or artificial)
- ✅ Sharp focus (avoid blur)
- ✅ Proper exposure (not too dark/bright)
- ✅ Clean composition (minimal clutter)
- ✅ High resolution (1024x768 or higher)

## Quality Scoring

### Overall Score (0-10)
Weighted average of:
- **Brightness** (35%): Optimal range 80-180
- **Sharpness** (45%): Based on image variance
- **Composition** (20%): Aspect ratio analysis

### Quality Issues
Automatically detected:
- `too_dark`: Brightness < 50
- `too_bright`: Brightness > 200
- `blurry`: Sharpness variance < 15
- `low_resolution`: Width < 1024 or Height < 768
- `poor_composition`: Extreme aspect ratio

### Suitability
Image is suitable if:
- Overall score ≥ 5.0
- Brightness score ≥ 4.0
- Sharpness score ≥ 5.0

## Error Handling

### HTTP Status Codes
- `200`: Success
- `400`: Bad Request (validation error)
- `413`: Payload Too Large (image too big)
- `422`: Unprocessable Entity (Pydantic validation)
- `500`: Internal Server Error (Azure API error)

### Error Response Format
```json
{
  "detail": "Error message description"
}
```

### Common Errors

**ImageValidationError (400)**
```json
{
  "detail": "Image width 500px is below minimum 640px"
}
```

**AzureVisionError (500)**
```json
{
  "detail": "Azure Vision API error: Rate limit exceeded"
}
```

## Performance

### Benchmarks
- **Single image analysis**: ~150-300ms
- **Batch (10 images)**: ~2-3 seconds
- **File upload + analysis**: ~200-400ms

### Optimization Tips
- Use batch endpoint for multiple images
- Compress images before upload (quality 85-95)
- Use appropriate image dimensions (1920x1080 recommended)

## Testing

Run tests:
```bash
# All tests
pytest tests/tagger/ -v

# Unit tests only
pytest tests/tagger/test_validators.py -v

# Integration tests
pytest tests/tagger/test_integration.py -v
```

See `tests/tagger/README.md` for detailed test documentation.

## Azure Setup

See `docs/AZURE_VISION_SETUP.md` for:
- Creating Azure AI Vision resource
- Getting API credentials
- Configuration guide
- Pricing information
- Troubleshooting

## Development

### Adding New Features

1. **Add to models.py**: Define Pydantic models
2. **Update service.py**: Implement business logic
3. **Add to router.py**: Create API endpoint
4. **Write tests**: Add unit and integration tests
5. **Update docs**: Document the new feature

### Code Style
- Follow PEP 8
- Use type hints
- Write docstrings
- Keep functions focused (single responsibility)

## Troubleshooting

### Issue: "Access denied due to invalid subscription key"
**Solution**: Check `TAGGER_AZURE_VISION_KEY` in `.env`

### Issue: "Image too large"
**Solution**: Reduce image size or increase `TAGGER_MAX_IMAGE_SIZE_MB`

### Issue: "Low quality score"
**Solution**: Improve lighting, focus, or resolution

### Issue: "Room type not detected"
**Solution**: Ensure image clearly shows room features, check confidence threshold

## License

Part of the Duumbi platform. See main repository for license information.

## Support

For issues or questions:
- Check documentation: `docs/` folder
- Run tests: `pytest tests/tagger/`
- Review logs: Check FastAPI logs for errors

