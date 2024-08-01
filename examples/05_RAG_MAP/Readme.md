# Example: Retrieval Augmented Generation with a Map

This directory shows how to use the Tron framework for simple Retrieval Augmented Generation for a set of PDF documents.

This example takes a set of pre-computed chunks of text extracted from a set of PDF files in JSONL format with their full embedding vectors and a UMAP projection in 2 dimensions. This example demonstrates how to use the Tron framework to incorporate useful JavaScript libraries (e.g., d3.js used in this example) and compute embeddings and large language models in the backend as a service.

![d3 scatter plot for UMAP](./images/d3_umap_plot.png "The UMAP highlighted the hits of the query using embedding vector search.")

![dialog](./images/dialog.png "Query the database with 'Find Related Text' and a user can ask an LLM to summarize the results or perform other related follow-up queries.")