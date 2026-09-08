# Upstream source

Source: https://github.com/localsend/localsend/tree/6279d3e30d1d1290caee3b81549f8128a8b01d9f/packages/core

Copied without source modifications on 2026-09-08, with the repository's MIT LICENSE.
Vendoring only this package avoids Cargo recursively downloading the upstream
Flutter SDK submodule. Update this directory from a reviewed upstream revision;
record any future local patches here.

Local patch: discovery emits `ProbeFailed` for failed announcement verification,
so the app can display a scoped diagnostic. Certificate verification and device
acceptance rules are unchanged.
