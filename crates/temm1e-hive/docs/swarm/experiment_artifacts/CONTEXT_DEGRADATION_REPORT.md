# Context Degradation Benchmark

Date: 2026-03-17 18:10:16 UTC
Model: gemini-3.1-pro-preview

## Hypothesis

Single agent accumulating conversation history degrades on later functions.
Swarm with fresh context maintains consistent quality.

## Results

| Metric | Single Agent | Swarm |
|--------|-------------|-------|
| **Functions passing** | **12/12** | **12/12** |
| Wall clock | 111537ms | 17998ms |
| Speedup | — | 6.20x |
| Tokens | 7183 | 2130 |
| Cost | $0.002370 | $0.000703 |
