#!/bin/bash
# Get version hashes for the eigenda node - unsure if it works fully because Claude wrote it

IMAGE_URL=docker://ghcr.io/layr-labs/eigenda/opr-node
NAME=eigenda

# Get all tags at once
ALL_TAGS=$(skopeo list-tags $IMAGE_URL | jq -r '.Tags[]')

# Get all arm64 hashes in one request
ARM_HASHES=$(for tag in $ALL_TAGS; do
    echo "{\"reference\":\"$IMAGE_URL:$tag\"}"
done | skopeo --override-arch arm64 --override-os linux inspect --raw-format=json @- | jq -r '.[] | [.Reference, .Digest] | @tsv')

# Get all amd64 hashes in one request
AMD_HASHES=$(for tag in $ALL_TAGS; do
    echo "{\"reference\":\"$IMAGE_URL:$tag\"}"
done | skopeo --override-arch amd64 --override-os linux inspect --raw-format=json @- | jq -r '.[] | [.Reference, .Digest] | @tsv')

# Generate SQL
echo "BEGIN;"
while IFS=$'\t' read -r ref hash; do
    version=${ref##*:}
    echo "INSERT INTO avs_version_hash (avs_type, hash, version) VALUES ('$NAME', '$hash', '$version');"
done <<< "$ARM_HASHES"
while IFS=$'\t' read -r ref hash; do
    version=${ref##*:}
    echo "INSERT INTO avs_version_hash (avs_type, hash, version) VALUES ('$NAME', '$hash', '$version');"
done <<< "$AMD_HASHES"
echo "COMMIT;"
