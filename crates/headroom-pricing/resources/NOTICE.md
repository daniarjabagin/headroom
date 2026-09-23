# Third-party pricing data

The bundled price snapshots are trimmed copies of public catalogs. Only OpenAI and Anthropic chat
models and the price fields Headroom reads are kept; the trimming rules are the same ones
`headroom_pricing::refresh` applies to live downloads, so the `data` member of a refreshed cache file
can replace a snapshot as-is.

| file | source | license | retrieved |
| --- | --- | --- | --- |
| `litellm.json` | [BerriAI/litellm](https://github.com/BerriAI/litellm) `model_prices_and_context_window.json` (main) | MIT | 2026-09-23, ETag `c3875b05cd818b7ce842f77819f3dcc12f85831761db53add7b83224e31f01fc` |
| `models_dev.json` | [models.dev](https://models.dev) `api.json` ([sst/models.dev](https://github.com/sst/models.dev)) | MIT | 2026-09-23, ETag `864d9ed6fa52341270ac975b421e7754` |

`supplement.json` is Headroom's own file. Its tier multipliers, the Codex alias rules and the
`codex-mini-latest` price follow the pricing supplement and LiteLLM snapshot published by
[OpenQuota](https://github.com/deviffyy/OpenQuota) (MIT), cross-checked against the LiteLLM
`*_priority` rates and the Azure OpenAI entries of the snapshot above.
