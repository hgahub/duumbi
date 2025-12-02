FROM python:3.11-slim

WORKDIR /app

# Install uv for fast package management
RUN pip install uv

# Copy project files
COPY pyproject.toml .
COPY src ./src

# Install dependencies
RUN uv pip install --system .

# Expose port
EXPOSE 8000

# Run the application
CMD ["uvicorn", "src.main:app", "--host", "0.0.0.0", "--port", "8000"]
