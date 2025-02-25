#!/bin/bash
#
IVYNET_PATH=/opt/ivynet

source ${IVYNET_PATH}/secrets/env

${IVYNET_PATH}/bin/ivynet-api --delete-old-logs --db-uri $DATABASE_URL
