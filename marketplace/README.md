# Legacy registry path

Only `registry.json` lives here, as a copy of `../registry.json` that keeps the `release_tag` pins older releases read. avm releases published
before the registry moved to the repo root fetch it from this path on `main`, so it has to stay.
`scripts/check-registry.sh` (run in CI) fails if the plugin lists differ (pins aside). Never remove the pins here. The website is in `../web`.
