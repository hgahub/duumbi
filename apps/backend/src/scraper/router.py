from fastapi import APIRouter

router = APIRouter()

@router.post("/trigger")
async def trigger_scraper():
    return {"message": "Scraper triggered", "job_id": "12345"}
