#!/bin/bash

if [ $# -eq 0 ]; then
    echo "Usage: $0 <GH_WORKFLOW_ID>"
    exit 1
fi

GH_WORKFLOW_ID=$1

cd "$(dirname "$0")"/../
poetry run python3 -m twine upload "./artifacts/$GH_WORKFLOW_ID/*"
