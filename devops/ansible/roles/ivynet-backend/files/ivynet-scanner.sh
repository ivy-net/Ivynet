#!/bin/bash
#
IVYNET_PATH=/opt/ivynet

source ${IVYNET_PATH}/secrets/env

${IVYNET_PATH}/bin/ivynet-backend --add-node-version-hashes --db-uri $DATABASE_URL
${IVYNET_PATH}/bin/ivynet-backend --update-node-data-versions --db-uri $DATABASE_URL
