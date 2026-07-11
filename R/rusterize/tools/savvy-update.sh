#!/usr/bin/env bash
# Regenerate savvy bindings and docs after development

set -euo pipefail

pkg="$(cd "$(dirname "${BASH_SOURCE[0]}")/.." && pwd)"

Rscript -e "savvy::savvy_update('${pkg}'); devtools::document('${pkg}')"
