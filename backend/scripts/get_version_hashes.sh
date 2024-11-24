#! /bin/sh

IMAGE_URL=docker://ghcr.io/layr-labs/eigenda/opr-node
NAME=eigenda

skopeo list-tags $IMAGE_URL | jq .Tags | jq -c '.[]' | while read version;
do
    ver=${version%\"}
    ver=${ver#\"}
    arm_hash=`skopeo --override-arch arm64 --override-os linux inspect $IMAGE_URL:$ver | jq .Digest`
    amd_hash=`skopeo --override-arch amd64 --override-os linux inspect $IMAGE_URL:$ver | jq .Digest`

    arm_hash=${arm_hash%\"}
    arm_hash=${arm_hash#\"}
    amd_hash=${amd_hash%\"}
    amd_hash=${amd_hash#\"}
    if [ "$arm_hash" = "$amd_hash" ]; then
        echo "INSERT INTO avs_version_hash (avs_type, hash, version) VALUES ('$NAME', '$arm_hash', '$ver');"
    else
        if [ -n "$arm_hash" ]; then
            echo "INSERT INTO avs_version_hash (avs_type, hash, version) VALUES ('$NAME', '$arm_hash', '$ver');"
        fi
        if [ -n "$amd_hash" ]; then
            echo "INSERT INTO avs_version_hash (avs_type, hash, version) VALUES ('$NAME', '$amd_hash', '$ver');"
        fi
    fi
done
