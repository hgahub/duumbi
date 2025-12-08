# Tagger API - Usage Examples

Complete examples for using the Tagger API in different scenarios.

## Table of Contents
- [Basic Usage](#basic-usage)
- [Advanced Scenarios](#advanced-scenarios)
- [Error Handling](#error-handling)
- [Frontend Integration](#frontend-integration)

---

## Basic Usage

### 1. Analyze Image from URL

**Request:**
```bash
curl -X POST http://localhost:8000/api/tagger/analyze \
  -H "Content-Type: application/json" \
  -d '{
    "image_url": "https://example.com/living-room.jpg"
  }'
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
  "features": ["modern", "bright", "spacious", "furnished"],
  "tags": ["living room", "sofa", "window", "modern", "furniture"],
  "caption": "A modern living room with bright natural lighting and comfortable furniture",
  "recommendations": ["Image quality is good"],
  "is_suitable": true,
  "processing_time_ms": 250
}
```

### 2. Upload and Analyze Image

**Request:**
```bash
curl -X POST http://localhost:8000/api/tagger/analyze/upload \
  -F "file=@/path/to/property.jpg"
```

**Response:** Same structure as above

### 3. Batch Analysis

**Request:**
```bash
curl -X POST http://localhost:8000/api/tagger/analyze/batch \
  -H "Content-Type: application/json" \
  -d '{
    "image_urls": [
      "https://example.com/living-room.jpg",
      "https://example.com/kitchen.jpg",
      "https://example.com/bedroom.jpg"
    ]
  }'
```

**Response:**
```json
{
  "results": [
    {
      "quality": {...},
      "room_type": "living_room",
      ...
    },
    {
      "quality": {...},
      "room_type": "kitchen",
      ...
    },
    {
      "quality": {...},
      "room_type": "bedroom",
      ...
    }
  ],
  "total_processed": 3,
  "total_failed": 0,
  "processing_time_ms": 750
}
```

---

## Advanced Scenarios

### Scenario 1: Property Listing Workflow

**Step 1: Upload all property images**
```python
import httpx
from pathlib import Path

async def upload_property_images(image_paths: list[Path]):
    results = []
    
    async with httpx.AsyncClient() as client:
        for image_path in image_paths:
            with open(image_path, 'rb') as f:
                response = await client.post(
                    'http://localhost:8000/api/tagger/analyze/upload',
                    files={'file': f}
                )
                results.append(response.json())
    
    return results

# Usage
images = [
    Path('living-room.jpg'),
    Path('kitchen.jpg'),
    Path('bedroom.jpg')
]
results = await upload_property_images(images)
```

**Step 2: Filter suitable images**
```python
def filter_suitable_images(results):
    suitable = []
    needs_improvement = []
    
    for i, result in enumerate(results):
        if result['is_suitable']:
            suitable.append({
                'index': i,
                'room_type': result['room_type'],
                'quality_score': result['quality']['overall_score']
            })
        else:
            needs_improvement.append({
                'index': i,
                'issues': result['quality']['issues'],
                'recommendations': result['recommendations']
            })
    
    return suitable, needs_improvement

suitable, needs_improvement = filter_suitable_images(results)
print(f"Suitable images: {len(suitable)}")
print(f"Need improvement: {len(needs_improvement)}")
```

**Step 3: Generate property description**
```python
def generate_property_summary(results):
    rooms = {}
    all_features = set()
    
    for result in results:
        if result['room_type']:
            rooms[result['room_type']] = result['caption']
        all_features.update(result['features'])
    
    return {
        'rooms': rooms,
        'features': list(all_features),
        'total_images': len(results),
        'avg_quality': sum(r['quality']['overall_score'] for r in results) / len(results)
    }

summary = generate_property_summary(results)
```

### Scenario 2: Quality Control Dashboard

```python
from typing import List, Dict

class ImageQualityDashboard:
    def __init__(self):
        self.results: List[Dict] = []
    
    async def analyze_images(self, image_urls: List[str]):
        """Analyze multiple images and store results."""
        async with httpx.AsyncClient() as client:
            response = await client.post(
                'http://localhost:8000/api/tagger/analyze/batch',
                json={'image_urls': image_urls}
            )
            data = response.json()
            self.results = data['results']
    
    def get_statistics(self):
        """Calculate quality statistics."""
        if not self.results:
            return None
        
        return {
            'total_images': len(self.results),
            'suitable_count': sum(1 for r in self.results if r['is_suitable']),
            'avg_quality': sum(r['quality']['overall_score'] for r in self.results) / len(self.results),
            'avg_brightness': sum(r['quality']['brightness_score'] for r in self.results) / len(self.results),
            'common_issues': self._get_common_issues(),
            'room_distribution': self._get_room_distribution()
        }
    
    def _get_common_issues(self):
        """Find most common quality issues."""
        from collections import Counter
        all_issues = []
        for r in self.results:
            all_issues.extend(r['quality']['issues'])
        return Counter(all_issues).most_common(3)
    
    def _get_room_distribution(self):
        """Get distribution of room types."""
        from collections import Counter
        rooms = [r['room_type'] for r in self.results if r['room_type']]
        return dict(Counter(rooms))

# Usage
dashboard = ImageQualityDashboard()
await dashboard.analyze_images([...])
stats = dashboard.get_statistics()
```

---

## Error Handling

### Handle Validation Errors

```python
import httpx

async def analyze_with_error_handling(image_url: str):
    try:
        async with httpx.AsyncClient() as client:
            response = await client.post(
                'http://localhost:8000/api/tagger/analyze',
                json={'image_url': image_url}
            )
            response.raise_for_status()
            return response.json()

    except httpx.HTTPStatusError as e:
        if e.response.status_code == 400:
            print(f"Validation error: {e.response.json()['detail']}")
        elif e.response.status_code == 422:
            print(f"Invalid input: {e.response.json()['detail']}")
        elif e.response.status_code == 500:
            print(f"Server error: {e.response.json()['detail']}")
        return None

    except httpx.RequestError as e:
        print(f"Network error: {e}")
        return None
```

### Retry Logic for Azure Errors

```python
import asyncio
from tenacity import retry, stop_after_attempt, wait_exponential

@retry(
    stop=stop_after_attempt(3),
    wait=wait_exponential(multiplier=1, min=2, max=10)
)
async def analyze_with_retry(image_url: str):
    async with httpx.AsyncClient() as client:
        response = await client.post(
            'http://localhost:8000/api/tagger/analyze',
            json={'image_url': image_url},
            timeout=30.0
        )
        response.raise_for_status()
        return response.json()

# Usage
try:
    result = await analyze_with_retry('https://example.com/image.jpg')
except Exception as e:
    print(f"Failed after 3 retries: {e}")
```

---

## Frontend Integration

### React Component Example

```typescript
import React, { useState } from 'react';

interface AnalysisResult {
  quality: {
    overall_score: number;
    brightness_score: number;
    sharpness_score: number;
    composition_score: number;
    issues: string[];
  };
  room_type: string | null;
  features: string[];
  tags: string[];
  caption: string | null;
  is_suitable: boolean;
  recommendations: string[];
}

export const ImageAnalyzer: React.FC = () => {
  const [file, setFile] = useState<File | null>(null);
  const [result, setResult] = useState<AnalysisResult | null>(null);
  const [loading, setLoading] = useState(false);
  const [error, setError] = useState<string | null>(null);

  const handleFileChange = (e: React.ChangeEvent<HTMLInputElement>) => {
    if (e.target.files && e.target.files[0]) {
      setFile(e.target.files[0]);
    }
  };

  const analyzeImage = async () => {
    if (!file) return;

    setLoading(true);
    setError(null);

    const formData = new FormData();
    formData.append('file', file);

    try {
      const response = await fetch('/api/tagger/analyze/upload', {
        method: 'POST',
        body: formData,
      });

      if (!response.ok) {
        const errorData = await response.json();
        throw new Error(errorData.detail || 'Analysis failed');
      }

      const data = await response.json();
      setResult(data);
    } catch (err) {
      setError(err instanceof Error ? err.message : 'Unknown error');
    } finally {
      setLoading(false);
    }
  };

  return (
    <div className="image-analyzer">
      <input type="file" accept="image/*" onChange={handleFileChange} />
      <button onClick={analyzeImage} disabled={!file || loading}>
        {loading ? 'Analyzing...' : 'Analyze Image'}
      </button>

      {error && <div className="error">{error}</div>}

      {result && (
        <div className="results">
          <h3>Analysis Results</h3>

          <div className="quality">
            <h4>Quality Score: {result.quality.overall_score.toFixed(1)}/10</h4>
            <div className={result.is_suitable ? 'suitable' : 'not-suitable'}>
              {result.is_suitable ? '✓ Suitable' : '✗ Needs Improvement'}
            </div>
          </div>

          {result.room_type && (
            <div className="room-type">
              <strong>Room Type:</strong> {result.room_type.replace('_', ' ')}
            </div>
          )}

          {result.features.length > 0 && (
            <div className="features">
              <strong>Features:</strong>
              <ul>
                {result.features.map((feature, i) => (
                  <li key={i}>{feature.replace('_', ' ')}</li>
                ))}
              </ul>
            </div>
          )}

          {result.recommendations.length > 0 && (
            <div className="recommendations">
              <strong>Recommendations:</strong>
              <ul>
                {result.recommendations.map((rec, i) => (
                  <li key={i}>{rec}</li>
                ))}
              </ul>
            </div>
          )}
        </div>
      )}
    </div>
  );
};
```

### Vue.js Example

```vue
<template>
  <div class="image-analyzer">
    <input type="file" @change="handleFileChange" accept="image/*" />
    <button @click="analyzeImage" :disabled="!file || loading">
      {{ loading ? 'Analyzing...' : 'Analyze Image' }}
    </button>

    <div v-if="error" class="error">{{ error }}</div>

    <div v-if="result" class="results">
      <h3>Quality Score: {{ result.quality.overall_score.toFixed(1) }}/10</h3>
      <p>Room: {{ result.room_type }}</p>
      <p>Suitable: {{ result.is_suitable ? 'Yes' : 'No' }}</p>

      <div v-if="result.recommendations.length">
        <h4>Recommendations:</h4>
        <ul>
          <li v-for="(rec, i) in result.recommendations" :key="i">{{ rec }}</li>
        </ul>
      </div>
    </div>
  </div>
</template>

<script setup lang="ts">
import { ref } from 'vue';

const file = ref<File | null>(null);
const result = ref(null);
const loading = ref(false);
const error = ref<string | null>(null);

const handleFileChange = (e: Event) => {
  const target = e.target as HTMLInputElement;
  if (target.files && target.files[0]) {
    file.value = target.files[0];
  }
};

const analyzeImage = async () => {
  if (!file.value) return;

  loading.value = true;
  error.value = null;

  const formData = new FormData();
  formData.append('file', file.value);

  try {
    const response = await fetch('/api/tagger/analyze/upload', {
      method: 'POST',
      body: formData,
    });

    if (!response.ok) {
      const errorData = await response.json();
      throw new Error(errorData.detail);
    }

    result.value = await response.json();
  } catch (err) {
    error.value = err instanceof Error ? err.message : 'Unknown error';
  } finally {
    loading.value = false;
  }
};
</script>
```

---

## Best Practices

### 1. Image Optimization Before Upload

```python
from PIL import Image
from io import BytesIO

def optimize_image(image_path: str, max_size_mb: int = 2) -> BytesIO:
    """Optimize image before upload."""
    img = Image.open(image_path)

    # Resize if too large
    max_dimension = 1920
    if max(img.size) > max_dimension:
        ratio = max_dimension / max(img.size)
        new_size = tuple(int(dim * ratio) for dim in img.size)
        img = img.resize(new_size, Image.Resampling.LANCZOS)

    # Save with compression
    buffer = BytesIO()
    img.save(buffer, format='JPEG', quality=85, optimize=True)
    buffer.seek(0)

    return buffer
```

### 2. Progress Tracking for Batch

```python
from tqdm import tqdm

async def analyze_batch_with_progress(image_urls: list[str]):
    results = []

    async with httpx.AsyncClient() as client:
        for url in tqdm(image_urls, desc="Analyzing images"):
            try:
                response = await client.post(
                    'http://localhost:8000/api/tagger/analyze',
                    json={'image_url': url},
                    timeout=30.0
                )
                results.append(response.json())
            except Exception as e:
                print(f"Failed to analyze {url}: {e}")
                results.append(None)

    return results
```

### 3. Caching Results

```python
from functools import lru_cache
import hashlib

@lru_cache(maxsize=100)
def get_cached_analysis(image_url: str):
    """Cache analysis results by URL."""
    # In production, use Redis or similar
    return analyze_image(image_url)

def get_image_hash(image_bytes: bytes) -> str:
    """Get hash of image for caching."""
    return hashlib.md5(image_bytes).hexdigest()
```

---

## Performance Tips

1. **Use batch endpoint** for multiple images (up to 20)
2. **Compress images** before upload (quality 85-95)
3. **Resize large images** to 1920x1080 or similar
4. **Implement caching** for repeated analyses
5. **Use async/await** for concurrent requests
6. **Set appropriate timeouts** (30s recommended)

---

## Additional Resources

- [Tagger Module README](../src/tagger/README.md)
- [Azure Vision Setup Guide](./AZURE_VISION_SETUP.md)
- [API Documentation](http://localhost:8000/docs) (Swagger UI)
- [Test Examples](../tests/tagger/)

