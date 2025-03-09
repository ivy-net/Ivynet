#!/bin/bash
#
IVYNET_PATH=/opt/ivynet

source ${IVYNET_PATH}/secrets/env.api

${IVYNET_PATH}/bin/ivynet-api --add-node-version-hashes --db-uri $DATABASE_URL
${IVYNET_PATH}/bin/ivynet-api --update-node-data-versions --db-uri $DATABASE_URL
