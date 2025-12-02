from fastapi import APIRouter

router = APIRouter()

@router.get("/estimate")
async def estimate_property():
    return {"message": "AVM Estimate endpoint", "value": 1000000}
