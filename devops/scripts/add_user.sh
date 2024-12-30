#!/bin/bash
#

EMAIL=$1
ORG=$2
PASSWORD="${PASSWORD:-$3}"

source .env
ivynet-backend --add-organization ${EMAIL}:${PASSWORD}/${ORG} --db-uri postgresql://ivy:${PGPASSWORD}@${HOST}:5432/ivynet
