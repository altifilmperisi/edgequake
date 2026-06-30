#!/usr/bin/env python3
"""Minimal OpenAI-compatible embedding server using sentence-transformers."""
import asyncio
from fastapi import FastAPI
from pydantic import BaseModel
from sentence_transformers import SentenceTransformer
import uvicorn

app = FastAPI()
model = SentenceTransformer('all-MiniLM-L6-v2')
DIM = model.get_sentence_embedding_dimension()

class EmbeddingRequest(BaseModel):
    input: str | list[str]
    model: str = "text-embedding-ada-002"

class EmbeddingResponse(BaseModel):
    object: str = "list"
    data: list[dict]
    model: str
    usage: dict

@app.post("/v1/embeddings")
async def embeddings(req: EmbeddingRequest):
    texts = [req.input] if isinstance(req.input, str) else req.input
    vecs = model.encode(texts, normalize_embeddings=True).tolist()
    data = [{"object": "embedding", "index": i, "embedding": v} for i, v in enumerate(vecs)]
    return EmbeddingResponse(
        data=data, model=req.model,
        usage={"prompt_tokens": sum(len(t.split()) for t in texts), "total_tokens": sum(len(t.split()) for t in texts)}
    )

@app.get("/v1/models")
async def list_models():
    return {"object": "list", "data": [{"id": "text-embedding-ada-002", "object": "model"}]}

if __name__ == "__main__":
    print(f"Embedding server on :1234 (dim={DIM})")
    uvicorn.run(app, host="0.0.0.0", port=1234)
