#!/bin/bash
#
source ~/.env

PGPASSWORD=$PGPASSWORD psql -h ${HOST} -U ivy -d ivynet
