from fastapi import FastAPI
from src.avm.router import router as avm_router
from src.scraper.router import router as scraper_router
from src.tagger.router import router as tagger_router

app = FastAPI(
    title="Duumbi Backend API",
    description="Modular Monolith for Duumbi Platform",
    version="0.1.0"
)

@app.get("/")
async def root():
    return {"message": "Welcome to Duumbi Backend API"}

@app.get("/health")
async def health_check():
    return {"status": "healthy"}

# Mount routers
app.include_router(avm_router, prefix="/api/avm", tags=["AVM"])
app.include_router(scraper_router, prefix="/api/scraper", tags=["Scraper"])
app.include_router(tagger_router, prefix="/api")  # Tagger router has its own /tagger prefix

if __name__ == "__main__":
    import uvicorn
    uvicorn.run(app, host="0.0.0.0", port=8000)
