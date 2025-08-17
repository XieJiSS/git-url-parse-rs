#!/bin/bash

cd "$(dirname "$0")"/../
GH_WORKFLOW_ID="$(gh run ls -R XieJiSS/git-url-parse-rs -b main -s success -u XieJiSS -w Maturin -L 1 --json 'databaseId' -q '.[].databaseId' | head -n 1)"

if [ -z "$GH_WORKFLOW_ID" ]; then
    echo "Error: GH_WORKFLOW_ID is empty. No successful Maturin workflow runs found."
    exit 1
fi

gh run download -R XieJiSS/git-url-parse-rs -D ./artifacts/$GH_WORKFLOW_ID -p "*" $GH_WORKFLOW_ID

# Restructure artifacts: move files from ./artifacts/id/artifact_name/ to ./artifacts/id/
for artifact_dir in ./artifacts/$GH_WORKFLOW_ID/*/; do
    if [ -d "$artifact_dir" ]; then
        mv "$artifact_dir"* ./artifacts/$GH_WORKFLOW_ID/ 2>/dev/null
        rmdir "$artifact_dir" 2>/dev/null
    fi
done
