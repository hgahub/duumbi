from fastapi import APIRouter

router = APIRouter()

@router.post("/analyze")
async def analyze_image():
    return {"message": "Image analysis started", "tags": ["living_room", "modern"]}
