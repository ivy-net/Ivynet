#!/bin/bash
set -euo pipefail

# Config
IMAGE_URL="docker://ghcr.io/layr-labs/eigenda/opr-node"
NAME="eigenda"
LOG_FILE="/var/log/eigenda_version.log"
ARCHITECTURES=("arm64" "amd64" "unknown")
TEMP_FILE=$(mktemp)
trap 'rm -f $TEMP_FILE' EXIT

log() {
    echo "[$(date '+%Y-%m-%d %H:%M:%S')] $1" | tee -a "$LOG_FILE"
}

fetch_tags() {
    if ! ALL_TAGS=$(skopeo list-tags "$IMAGE_URL" 2>"$TEMP_FILE" | jq -r '.Tags[]'); then
        log "Error fetching tags: $(cat "$TEMP_FILE")"
        exit 1
    fi
    echo "$ALL_TAGS"
}

fetch_hashes() {
    local arch=$1
    local override_args=""
    [[ $arch != "unknown" ]] && override_args="--override-arch $arch --override-os linux"

    for tag in $ALL_TAGS; do
        echo "{\"reference\":\"$IMAGE_URL:$tag\"}"
    done | {
        if ! skopeo $override_args inspect --raw-format=json @- 2>"$TEMP_FILE" | \
            jq -r '.[] | [.Reference, .Digest] | @tsv'; then
            log "Error fetching $arch hashes: $(cat "$TEMP_FILE")"
            return 1
        fi
    }
}

generate_sql() {
    local arch=$1
    local hashes=$2

    while IFS=$'\t' read -r ref hash; do
        version=${ref##*:}
        [[ -z $version || -z $hash ]] && continue
        echo "INSERT INTO avs_version_hash (avs_type, architecture, hash, version)
              VALUES ('$NAME', '$arch', '$hash', '$version')
              ON CONFLICT (avs_type, architecture, version) DO UPDATE
              SET hash = EXCLUDED.hash;"
    done <<< "$hashes"
}

main() {
    log "Starting version hash collection"

    ALL_TAGS=$(fetch_tags)
    [[ -z "$ALL_TAGS" ]] && { log "No tags found"; exit 1; }

    echo "BEGIN;"
    for arch in "${ARCHITECTURES[@]}"; do
        log "Processing $arch architecture"
        if hashes=$(fetch_hashes "$arch"); then
            generate_sql "$arch" "$hashes"
        fi
    done
    echo "COMMIT;"

    log "Completed successfully"
}

main
